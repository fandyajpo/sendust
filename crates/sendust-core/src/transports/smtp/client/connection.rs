//! Native SMTP connection: greeting, EHLO, implicit TLS, STARTTLS, reply I/O.
//!
//! Every read and write is bounded by `SendDeadlines`. Replies are parsed by
//! the P06-001 parser, so a malformed or oversized reply is a protocol error
//! rather than a hang or an unbounded allocation.

use std::pin::Pin;
use std::sync::Arc;
use std::time::{Duration, Instant};

use rustls::ClientConfig;
use rustls::pki_types::ServerName;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio_rustls::TlsConnector;
use tokio_rustls::client::TlsStream;

use crate::config::SendDeadlines;
use crate::errors::{Error, ErrorCode, ErrorDetails, Stage};
use crate::transports::smtp::proto::capabilities::Capabilities;
use crate::transports::smtp::proto::command::Command;
use crate::transports::smtp::proto::response::{MAX_LINE_BYTES, Reply, ReplyBuilder};
use crate::transports::smtp::{SmtpConfig, SmtpSecurity};

use super::tls;

enum Transport {
    Plain(TcpStream),
    Tls(Box<TlsStream<TcpStream>>),
}

impl AsyncRead for Transport {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        match self.get_mut() {
            Self::Plain(stream) => Pin::new(stream).poll_read(cx, buf),
            Self::Tls(stream) => Pin::new(stream).poll_read(cx, buf),
        }
    }
}

impl AsyncWrite for Transport {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<std::io::Result<usize>> {
        match self.get_mut() {
            Self::Plain(stream) => Pin::new(stream).poll_write(cx, buf),
            Self::Tls(stream) => Pin::new(stream).poll_write(cx, buf),
        }
    }

    fn poll_flush(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        match self.get_mut() {
            Self::Plain(stream) => Pin::new(stream).poll_flush(cx),
            Self::Tls(stream) => Pin::new(stream).poll_flush(cx),
        }
    }

    fn poll_shutdown(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        match self.get_mut() {
            Self::Plain(stream) => Pin::new(stream).poll_shutdown(cx),
            Self::Tls(stream) => Pin::new(stream).poll_shutdown(cx),
        }
    }
}

pub struct Connection {
    stream: Option<Transport>,
    buffer: Vec<u8>,
    capabilities: Capabilities,
    hello: String,
    server_name: ServerName<'static>,
    tls_config: Arc<ClientConfig>,
    deadlines: SendDeadlines,
}

impl Connection {
    pub async fn connect(config: &SmtpConfig) -> Result<Self, Error> {
        let deadlines = SendDeadlines::start(config.timeouts(), Instant::now());
        let server_name = tls::server_name(config.host())?;
        let tls_config = Arc::new(tls::client_config(config)?);
        let hello = config.hello_name().to_string();

        let connect_budget = config
            .timeouts()
            .connect()
            .min(deadlines.remaining_total(Instant::now()));
        if connect_budget.is_zero() {
            return Err(deadlines.timeout_error(Instant::now(), false, Stage::ConnectionSetup));
        }

        let tcp = tokio::time::timeout(
            connect_budget,
            TcpStream::connect((config.host(), config.port())),
        )
        .await
        .map_err(|_| deadlines.timeout_error(Instant::now(), false, Stage::ConnectionSetup))?
        .map_err(io_error)?;

        let transport = match config.security() {
            SmtpSecurity::ImplicitTls => {
                let connector = TlsConnector::from(Arc::clone(&tls_config));
                let stream = tokio::time::timeout(
                    connect_budget,
                    connector.connect(server_name.clone(), tcp),
                )
                .await
                .map_err(|_| {
                    deadlines.timeout_error(Instant::now(), false, Stage::ConnectionSetup)
                })?
                .map_err(tls_error)?;
                Transport::Tls(Box::new(stream))
            }
            SmtpSecurity::StartTls => Transport::Plain(tcp),
        };

        let mut connection = Self {
            stream: Some(transport),
            buffer: Vec::new(),
            capabilities: Capabilities::default(),
            hello,
            server_name,
            tls_config,
            deadlines,
        };

        let greeting = connection.read_reply().await?;
        expect(&greeting, 220, Stage::ConnectionSetup)?;
        connection.rehandshake().await?;
        Ok(connection)
    }

    pub fn capabilities(&self) -> &Capabilities {
        &self.capabilities
    }

    async fn rehandshake(&mut self) -> Result<(), Error> {
        let ehlo = self
            .command(
                Command::Ehlo {
                    identity: self.hello.clone(),
                },
                Stage::ConnectionSetup,
            )
            .await?;
        expect(&ehlo, 250, Stage::ConnectionSetup)?;
        self.capabilities = Capabilities::parse(&ehlo);
        Ok(())
    }

    pub async fn starttls(&mut self) -> Result<(), Error> {
        if !self.capabilities.starttls() {
            return Err(Error::new(ErrorCode::TlsHandshake));
        }

        let reply = self
            .command(Command::StartTls, Stage::ConnectionSetup)
            .await?;
        expect(&reply, 220, Stage::ConnectionSetup)?;
        if !self.buffer.is_empty() {
            return Err(protocol_error());
        }

        let transport = self.stream.take().ok_or_else(protocol_error)?;
        let tcp = match transport {
            Transport::Plain(stream) => stream,
            Transport::Tls(_) => return Err(protocol_error()),
        };

        let connector = TlsConnector::from(Arc::clone(&self.tls_config));
        let budget = self.remaining(self.deadlines.connect_by());
        let stream = tokio::time::timeout(budget, connector.connect(self.server_name.clone(), tcp))
            .await
            .map_err(|_| {
                self.deadlines.timeout_error(
                    std::time::Instant::now(),
                    false,
                    Stage::ConnectionSetup,
                )
            })?
            .map_err(tls_error)?;

        self.stream = Some(Transport::Tls(Box::new(stream)));
        self.capabilities = Capabilities::default();
        self.rehandshake().await
    }
}

impl Connection {
    /// Sends one command and reads its complete reply.
    pub async fn command(&mut self, command: Command, stage: Stage) -> Result<Reply, Error> {
        let bytes = command.to_bytes()?;
        let budget = self.budget();
        let deadline = self.deadlines;

        let stream = self.transport_mut()?;
        tokio::time::timeout(budget, async {
            stream.write_all(&bytes).await?;
            stream.flush().await
        })
        .await
        .map_err(|_| deadline.timeout_error(Instant::now(), true, stage))?
        .map_err(io_error)?;

        self.read_reply().await
    }

    async fn read_reply(&mut self) -> Result<Reply, Error> {
        let mut builder = ReplyBuilder::new();
        loop {
            let line = self.read_line().await?;
            if builder.push_line(&line)? {
                return builder.finish();
            }
        }
    }

    async fn read_line(&mut self) -> Result<Vec<u8>, Error> {
        loop {
            if let Some(position) = self.buffer.iter().position(|byte| *byte == b'\n') {
                return Ok(self.buffer.drain(..=position).collect());
            }
            if self.buffer.len() >= MAX_LINE_BYTES {
                return Err(protocol_error());
            }

            let mut chunk = [0u8; 1024];
            let budget = self.budget();
            let deadline = self.deadlines;
            let read = tokio::time::timeout(budget, self.transport_mut()?.read(&mut chunk))
                .await
                .map_err(|_| deadline.timeout_error(Instant::now(), true, Stage::ConnectionSetup))?
                .map_err(io_error)?;

            if read == 0 {
                return Err(Error::new(ErrorCode::ProtocolConnectionReset));
            }
            self.buffer.extend_from_slice(&chunk[..read]);
        }
    }

    fn budget(&self) -> Duration {
        let now = Instant::now();
        self.deadlines
            .operation_deadline(now)
            .saturating_duration_since(now)
            .min(self.deadlines.remaining_total(now))
    }

    fn remaining(&self, deadline: Instant) -> Duration {
        let now = Instant::now();
        deadline
            .saturating_duration_since(now)
            .min(self.deadlines.remaining_total(now))
    }

    fn transport_mut(&mut self) -> Result<&mut Transport, Error> {
        self.stream.as_mut().ok_or_else(protocol_error)
    }
}

fn expect(reply: &Reply, code: u16, stage: Stage) -> Result<(), Error> {
    if reply.value() == code {
        return Ok(());
    }
    Err(
        Error::new(ErrorCode::ProtocolUnexpectedResponse).with_details(
            ErrorDetails::default()
                .stage(stage)
                .smtp_code(reply.value()),
        ),
    )
}

fn protocol_error() -> Error {
    Error::new(ErrorCode::ProtocolUnexpectedResponse)
}

fn io_error(error: std::io::Error) -> Error {
    if error.kind() == std::io::ErrorKind::TimedOut {
        return Error::new(ErrorCode::TimeoutOperation);
    }
    Error::new(ErrorCode::ProtocolConnectionReset)
}

fn tls_error(error: std::io::Error) -> Error {
    if let Some(inner) = error.get_ref()
        && let Some(rustls_error) = inner.downcast_ref::<rustls::Error>()
    {
        return Error::new(match rustls_error {
            rustls::Error::InvalidCertificate(_) => ErrorCode::TlsCertificate,
            _ => ErrorCode::TlsHandshake,
        });
    }
    Error::new(ErrorCode::TlsHandshake)
}
