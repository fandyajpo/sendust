//! Self-tests for the deterministic fake SMTP harness in `tests/support`.
//!
//! These prove the harness contract: loopback-only binding, scripted replies,
//! per-recipient responses, the three post-DATA dispositions, injected delays,
//! and deterministic shutdown. They do not test Sendust itself.

mod support;

use std::io::{BufRead, BufReader, Write};
use std::net::{SocketAddr, TcpStream};
use std::time::{Duration, Instant};

use support::{AfterBody, FakeSmtp, Reply, Script, Stage};

struct Client {
    stream: TcpStream,
    reader: BufReader<TcpStream>,
}

impl Client {
    fn connect(addr: SocketAddr) -> Self {
        let stream = TcpStream::connect(addr).expect("connect to fake SMTP");
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .expect("read timeout");
        let reader = BufReader::new(stream.try_clone().expect("clone stream"));
        Self { stream, reader }
    }

    fn send(&mut self, line: &str) {
        self.stream
            .write_all(format!("{line}\r\n").as_bytes())
            .expect("write command");
        self.stream.flush().expect("flush command");
    }

    fn read(&mut self) -> Option<String> {
        let mut line = String::new();
        match self.reader.read_line(&mut line) {
            Ok(0) | Err(_) => None,
            Ok(_) => Some(line.trim_end_matches(['\r', '\n']).to_string()),
        }
    }

    fn expect(&mut self, code: &str) -> String {
        let line = self.read().expect("a reply line");
        assert!(line.starts_with(code), "expected {code}, got {line}");
        line
    }

    fn ehlo(&mut self) {
        self.send("EHLO test");
        loop {
            let line = self.read().expect("ehlo reply line");
            assert!(line.starts_with("250"), "unexpected EHLO line: {line}");
            if line.starts_with("250 ") {
                break;
            }
        }
    }

    fn envelope(&mut self, recipients: &[&str]) {
        self.ehlo();
        self.send("MAIL FROM:<alice@example.test>");
        self.expect("250");
        for recipient in recipients {
            self.send(&format!("RCPT TO:<{recipient}>"));
        }
    }

    fn send_body(&mut self, body: &str) {
        for line in body.lines() {
            self.send(line);
        }
        self.send(".");
    }
}

#[test]
fn accepted_dialogue_is_scripted_and_traced() {
    let server = FakeSmtp::start(Script::default()).expect("start fake smtp");
    let mut client = Client::connect(server.addr());

    client.expect("220");
    client.envelope(&["bob@example.test"]);
    client.expect("250");
    client.send("DATA");
    client.expect("354");
    client.send_body("Subject: hello\n\nHi there");
    client.expect("250");
    client.send("QUIT");
    client.expect("221");

    let trace = server.trace();
    let lines = trace.lines();
    assert!(
        lines
            .iter()
            .any(|line| line.starts_with("MAIL FROM:<alice@example.test>")),
        "missing MAIL FROM in {lines:?}"
    );
    assert_eq!(trace.count_prefix("RCPT"), 1);
    assert_eq!(trace.body_text(), "Subject: hello\r\n\r\nHi there\r\n");
}

#[test]
fn mixed_recipient_replies_are_served_per_rcpt() {
    let script = Script::default().rcpt_replies(vec![
        Reply::line("250 2.1.5 Recipient ok"),
        Reply::line("550 5.1.1 No such user"),
    ]);
    let server = FakeSmtp::start(script).expect("start fake smtp");
    let mut client = Client::connect(server.addr());

    client.expect("220");
    client.envelope(&["ok@example.test"]);
    client.expect("250");
    client.send("RCPT TO:<missing@example.test>");
    client.expect("550");

    assert_eq!(server.trace().count_prefix("RCPT"), 2);
}

#[test]
fn explicit_final_data_rejection_is_delivered() {
    let script =
        Script::default().after_body(AfterBody::Reply(Reply::line("451 4.3.0 Try again later")));
    let server = FakeSmtp::start(script).expect("start fake smtp");
    let mut client = Client::connect(server.addr());

    client.expect("220");
    client.envelope(&["bob@example.test"]);
    client.expect("250");
    client.send("DATA");
    client.expect("354");
    client.send_body("Subject: x\n\nbody");
    client.expect("451");
}

#[test]
fn connection_dropped_after_terminator_has_no_final_reply() {
    let script = Script::default().after_body(AfterBody::Close);
    let server = FakeSmtp::start(script).expect("start fake smtp");
    let mut client = Client::connect(server.addr());

    client.expect("220");
    client.envelope(&["bob@example.test"]);
    client.expect("250");
    client.send("DATA");
    client.expect("354");
    client.send_body("Subject: x\n\nbody");

    assert_eq!(
        client.read(),
        None,
        "connection must end without a final reply"
    );
    assert!(server.trace().body_text().contains("body"));
}

#[test]
fn injected_greeting_delay_is_observable() {
    let script = Script::default().delay(Stage::Greeting, Duration::from_millis(120));
    let started = Instant::now();
    let server = FakeSmtp::start(script).expect("start fake smtp");
    let mut client = Client::connect(server.addr());
    client.expect("220");
    assert!(
        started.elapsed() >= Duration::from_millis(120),
        "banner arrived before the injected delay"
    );
}

#[test]
fn injected_stage_delay_is_observable() {
    let script = Script::default().delay(Stage::Mail, Duration::from_millis(100));
    let server = FakeSmtp::start(script).expect("start fake smtp");
    let mut client = Client::connect(server.addr());

    client.expect("220");
    client.ehlo();
    let started = Instant::now();
    client.send("MAIL FROM:<alice@example.test>");
    client.expect("250");
    assert!(
        started.elapsed() >= Duration::from_millis(100),
        "MAIL reply arrived before the injected delay"
    );
}

#[test]
fn servers_bind_distinct_ephemeral_loopback_ports() {
    let first = FakeSmtp::start(Script::default()).expect("first server");
    let second = FakeSmtp::start(Script::default()).expect("second server");

    assert!(
        first.addr().ip().is_loopback(),
        "harness must bind loopback only"
    );
    assert_ne!(first.addr().port(), second.addr().port());
    assert_ne!(first.addr().port(), 0);
}
