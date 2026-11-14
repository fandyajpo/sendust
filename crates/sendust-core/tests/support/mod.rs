#![allow(dead_code)]
//! Deterministic fake SMTP server for Sendust integration tests.
//!
//! Binds loopback only, on an ephemeral port, and never contacts an external
//! network. Dialogues are scripted per stage, delays are injected rather than
//! slept for arbitrarily, and every received line plus the DATA body is
//! captured for assertions. See `docs/engineering/test-harness-contract.md`.

use std::io::{self, Read, Write};
use std::net::{Shutdown, SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use rustls::{ServerConnection, StreamOwned};

pub mod tls;

/// The only address the harness ever binds.
pub const LOOPBACK: &str = "127.0.0.1";

/// SMTP stages that a script can answer or delay.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    Greeting,
    Ehlo,
    StartTls,
    Auth,
    Mail,
    Rcpt,
    Data,
    Quit,
}

/// A complete, possibly multiline, SMTP reply.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reply {
    lines: Vec<String>,
}

impl Reply {
    pub fn line(line: impl Into<String>) -> Self {
        Self {
            lines: vec![line.into()],
        }
    }

    pub fn lines(lines: &[&str]) -> Self {
        Self {
            lines: lines.iter().map(|line| (*line).to_string()).collect(),
        }
    }

    pub fn is_final(&self, code: &str) -> bool {
        self.lines.last().is_some_and(|line| line.starts_with(code))
    }
}

/// What the server does after the message body terminator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AfterBody {
    /// Write this final reply. A 2xx means accepted, 4xx/5xx means rejected.
    Reply(Reply),
    /// Drop the connection without a final reply: the client cannot know the
    /// disposition and must report `unknown`. A hard TCP reset is produced by
    /// P06's TLS/fault tests when a socket is closed with unread data; the
    /// observable contract here is identical: no final reply.
    Close,
}

/// Scripted behavior for every connection the harness accepts.
#[derive(Debug, Clone)]
pub struct Script {
    pub greeting: Reply,
    pub ehlo: Reply,
    pub start_tls: Reply,
    pub auth: Reply,
    /// Replies for a multi-step AUTH exchange (334 challenges, final 235/5xx).
    pub auth_flow: Vec<Reply>,
    pub mail: Reply,
    /// Reply per `RCPT TO` index; the last entry repeats.
    pub rcpt: Vec<Reply>,
    pub data: Reply,
    pub after_body: AfterBody,
    pub quit: Reply,
    pub unknown: Reply,
    /// Injected delay applied before the reply of a stage.
    pub delays: Vec<(Stage, Duration)>,
    /// Stages where the server closes the connection instead of replying.
    pub hangup: Vec<Stage>,
    /// When set, the harness completes an implicit TLS handshake first.
    pub tls: Option<Arc<rustls::ServerConfig>>,
    /// Serve plaintext and upgrade on STARTTLS instead of implicit TLS.
    pub starttls_mode: bool,
}

impl Default for Script {
    fn default() -> Self {
        Self {
            greeting: Reply::line("220 sendust.test ESMTP ready"),
            ehlo: Reply::lines(&["250-sendust.test", "250-PIPELINING", "250 8BITMIME"]),
            start_tls: Reply::line("220 2.0.0 Ready to start TLS"),
            auth: Reply::line("235 2.7.0 Authentication successful"),
            auth_flow: Vec::new(),
            mail: Reply::line("250 2.1.0 Sender ok"),
            rcpt: vec![Reply::line("250 2.1.5 Recipient ok")],
            data: Reply::line("354 End data with <CR><LF>.<CR><LF>"),
            after_body: AfterBody::Reply(Reply::line("250 2.0.0 Message accepted")),
            quit: Reply::line("221 2.0.0 Bye"),
            unknown: Reply::line("500 5.5.2 Command unrecognized"),
            delays: Vec::new(),
            hangup: Vec::new(),
            tls: None,
            starttls_mode: false,
        }
    }
}

impl Script {
    pub fn rcpt_reply(mut self, reply: Reply) -> Self {
        self.rcpt = vec![reply];
        self
    }

    pub fn rcpt_replies(mut self, replies: Vec<Reply>) -> Self {
        self.rcpt = replies;
        self
    }

    pub fn after_body(mut self, after_body: AfterBody) -> Self {
        self.after_body = after_body;
        self
    }

    pub fn delay(mut self, stage: Stage, duration: Duration) -> Self {
        self.delays.push((stage, duration));
        self
    }

    /// Closes the connection at `stage` instead of writing its reply.
    pub fn hangup(mut self, stage: Stage) -> Self {
        self.hangup.push(stage);
        self
    }

    fn delay_for(&self, stage: Stage) -> Option<Duration> {
        self.delays
            .iter()
            .filter(|(configured, _)| *configured == stage)
            .map(|(_, duration)| *duration)
            .max()
    }
}

/// Everything the client sent, in order.
#[derive(Debug, Default)]
pub struct Trace {
    lines: Mutex<Vec<String>>,
    body: Mutex<Vec<u8>>,
}

impl Trace {
    pub fn lines(&self) -> Vec<String> {
        self.lines
            .lock()
            .map(|lines| lines.clone())
            .unwrap_or_default()
    }

    pub fn body(&self) -> Vec<u8> {
        self.body
            .lock()
            .map(|body| body.clone())
            .unwrap_or_default()
    }

    pub fn body_text(&self) -> String {
        String::from_utf8_lossy(&self.body()).into_owned()
    }

    pub fn count_prefix(&self, prefix: &str) -> usize {
        self.lines()
            .iter()
            .filter(|line| line.to_ascii_uppercase().starts_with(prefix))
            .count()
    }

    fn push_line(&self, line: &str) {
        if let Ok(mut lines) = self.lines.lock() {
            lines.push(line.to_string());
        }
    }

    fn push_body(&self, bytes: &[u8]) {
        if let Ok(mut body) = self.body.lock() {
            body.extend_from_slice(bytes);
        }
    }
}

/// A running fake SMTP server on an ephemeral loopback port.
pub struct FakeSmtp {
    addr: SocketAddr,
    stop: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
    trace: Arc<Trace>,
}

impl FakeSmtp {
    pub fn start(script: Script) -> io::Result<Self> {
        let listener = TcpListener::bind((LOOPBACK, 0))?;
        let addr = listener.local_addr()?;
        let stop = Arc::new(AtomicBool::new(false));
        let trace = Arc::new(Trace::default());

        let thread_stop = Arc::clone(&stop);
        let thread_trace = Arc::clone(&trace);
        let handle = thread::Builder::new()
            .name("sendust-fake-smtp".to_string())
            .spawn(move || {
                for incoming in listener.incoming() {
                    if thread_stop.load(Ordering::SeqCst) {
                        break;
                    }
                    match incoming {
                        Ok(stream) => serve(stream, &script, &thread_trace),
                        Err(_) => break,
                    }
                }
            })?;

        Ok(Self {
            addr,
            stop,
            handle: Some(handle),
            trace,
        })
    }

    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    pub fn trace(&self) -> &Trace {
        &self.trace
    }

    fn stop_inner(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        // Wake the blocking accept so the thread can observe `stop`.
        let _ = TcpStream::connect(self.addr);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

impl Drop for FakeSmtp {
    fn drop(&mut self) {
        self.stop_inner();
    }
}

/// A connection the harness can talk over: plaintext or implicit TLS.
enum Conn {
    Plain(TcpStream),
    Tls(Box<StreamOwned<ServerConnection, TcpStream>>),
}

impl Read for Conn {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        match self {
            Self::Plain(stream) => stream.read(buffer),
            Self::Tls(stream) => stream.read(buffer),
        }
    }
}

impl Write for Conn {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        match self {
            Self::Plain(stream) => stream.write(buffer),
            Self::Tls(stream) => stream.write(buffer),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match self {
            Self::Plain(stream) => stream.flush(),
            Self::Tls(stream) => stream.flush(),
        }
    }
}

impl Conn {
    /// Replaces a plaintext connection with a TLS session using the script config.
    fn upgrade(self, config: &Arc<rustls::ServerConfig>) -> Option<Conn> {
        match self {
            Conn::Plain(stream) => {
                let session = ServerConnection::new(Arc::clone(config)).ok()?;
                Some(Conn::Tls(Box::new(StreamOwned::new(session, stream))))
            }
            Conn::Tls(existing) => Some(Conn::Tls(existing)),
        }
    }

    fn shutdown(self) {
        match self {
            Self::Plain(stream) => {
                let _ = stream.shutdown(Shutdown::Both);
            }
            Self::Tls(stream) => {
                let _ = stream.sock.shutdown(Shutdown::Both);
            }
        }
    }
}

fn serve(stream: TcpStream, script: &Script, trace: &Trace) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(10)));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(10)));

    let mut conn = match (&script.tls, script.starttls_mode) {
        (Some(config), false) => match ServerConnection::new(Arc::clone(config)) {
            Ok(session) => Conn::Tls(Box::new(StreamOwned::new(session, stream))),
            Err(_) => return,
        },
        _ => Conn::Plain(stream),
    };

    if script.hangup.contains(&Stage::Greeting) {
        conn.shutdown();
        return;
    }
    if let Some(delay) = script.delay_for(Stage::Greeting) {
        thread::sleep(delay);
    }
    if write_reply(&mut conn, &script.greeting).is_err() {
        conn.shutdown();
        return;
    }

    let mut in_data = false;
    let mut in_auth = false;
    let mut auth_index = 0usize;
    let mut rcpt_index = 0usize;

    loop {
        let mut line = String::new();
        match tls::read_line(&mut conn, &mut line) {
            Ok(true) => {}
            Ok(false) | Err(_) => {
                conn.shutdown();
                return;
            }
        }
        let trimmed = line;

        if in_data {
            if trimmed == "." {
                in_data = false;
                trace.push_line("<DATA-END>");
                match &script.after_body {
                    AfterBody::Reply(reply) => {
                        let _ = write_reply(&mut conn, reply);
                    }
                    AfterBody::Close => {
                        conn.shutdown();
                        return;
                    }
                }
                continue;
            }
            trace.push_body(trimmed.as_bytes());
            trace.push_body(b"\r\n");
            continue;
        }

        trace.push_line(&trimmed);
        let (verb, rest) = split_command(&trimmed);

        if in_auth {
            let reply = script
                .auth_flow
                .get(auth_index)
                .cloned()
                .unwrap_or_else(|| Reply::line("535 5.7.8 Authentication failed"));
            auth_index += 1;
            in_auth = reply.is_final("334");
            if write_reply(&mut conn, &reply).is_err() {
                conn.shutdown();
                return;
            }
            continue;
        }

        if verb == "AUTH" && !script.auth_flow.is_empty() {
            let reply = script
                .auth_flow
                .first()
                .cloned()
                .unwrap_or_else(|| Reply::line("535 5.7.8 Authentication failed"));
            auth_index = 1;
            in_auth = reply.is_final("334");
            if write_reply(&mut conn, &reply).is_err() {
                conn.shutdown();
                return;
            }
            continue;
        }

        if verb == "NOOP" || verb == "RSET" {
            let _ = write_reply(&mut conn, &Reply::line("250 2.0.0 OK"));
            continue;
        }

        let selection = match verb.as_str() {
            "EHLO" | "HELO" => Some((Stage::Ehlo, &script.ehlo)),
            "STARTTLS" => Some((Stage::StartTls, &script.start_tls)),
            "AUTH" => Some((Stage::Auth, &script.auth)),
            "MAIL" => Some((Stage::Mail, &script.mail)),
            "RCPT" => {
                let reply = script
                    .rcpt
                    .get(rcpt_index)
                    .or_else(|| script.rcpt.last())
                    .unwrap_or(&script.mail);
                rcpt_index += 1;
                Some((Stage::Rcpt, reply))
            }
            "DATA" => Some((Stage::Data, &script.data)),
            "QUIT" => Some((Stage::Quit, &script.quit)),
            _ => None,
        };

        let Some((stage, reply)) = selection else {
            let _ = write_reply(&mut conn, &script.unknown);
            let _ = rest;
            continue;
        };

        if script.hangup.contains(&stage) {
            conn.shutdown();
            return;
        }
        if let Some(delay) = script.delay_for(stage) {
            thread::sleep(delay);
        }
        if write_reply(&mut conn, reply).is_err() {
            conn.shutdown();
            return;
        }

        if stage == Stage::StartTls
            && reply.is_final("220")
            && let Some(config) = &script.tls
        {
            conn = match conn.upgrade(config) {
                Some(next) => next,
                None => return,
            };
        }

        if stage == Stage::Data {
            in_data = reply.is_final("354");
        }
        if stage == Stage::Quit {
            conn.shutdown();
            return;
        }
    }
}

fn split_command(line: &str) -> (String, String) {
    let mut parts = line.splitn(2, ' ');
    let verb = parts.next().unwrap_or_default().to_ascii_uppercase();
    let rest = parts.next().unwrap_or_default().to_string();
    (verb, rest)
}

fn write_reply<W: Write>(stream: &mut W, reply: &Reply) -> io::Result<()> {
    let mut out = String::new();
    for line in &reply.lines {
        out.push_str(line);
        out.push_str("\r\n");
    }
    stream.write_all(out.as_bytes())?;
    stream.flush()
}
