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

use crate::config::{SendDeadlines, Timeouts};
use crate::errors::{Error, ErrorCode, ErrorDetails, Stage};
use crate::transports::smtp::proto::capabilities::Capabilities;
use crate::transports::smtp::proto::command::{
    AuthMechanism, Command, auth_plain_initial, auth_xoauth2_initial,
};
use crate::transports::smtp::proto::response::{MAX_LINE_BYTES, Reply, ReplyBuilder};
use crate::transports::smtp::{SmtpConfig, SmtpSecurity};
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;

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
    encrypted: bool,
    /// True until greeting, EHLO and any implicit TLS finish, so those reads
    /// are bounded by the connect window (ADR-0007).
    in_setup: bool,
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
                // The handshake shares the one absolute connect deadline; it must
                // not restart a full budget after TCP already consumed part of it.
                let tls_budget = deadlines
                    .connect_by()
                    .saturating_duration_since(Instant::now())
                    .min(deadlines.remaining_total(Instant::now()));
                let stream =
                    tokio::time::timeout(tls_budget, connector.connect(server_name.clone(), tcp))
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
            encrypted: matches!(config.security(), SmtpSecurity::ImplicitTls),
            in_setup: true,
        };

        let greeting = connection.read_reply(Stage::ConnectionSetup).await?;
        expect(&greeting, 220, Stage::ConnectionSetup)?;
        connection.rehandshake().await?;
        connection.in_setup = false;
        Ok(connection)
    }

    pub fn capabilities(&self) -> &Capabilities {
        &self.capabilities
    }

    pub fn is_encrypted(&self) -> bool {
        self.encrypted
    }

    /// Restarts the send budgets for a session that is reused across messages.
    ///
    /// The session was established under the budget of the send that opened it;
    /// a later message gets its own, exactly as a fresh connection would.
    pub fn begin_transaction(&mut self, timeouts: Timeouts) {
        self.deadlines = SendDeadlines::start(timeouts, Instant::now());
        self.in_setup = false;
    }

    /// Writes raw protocol bytes (the DATA body) under the operation budget.
    pub async fn write_raw(&mut self, bytes: &[u8]) -> Result<(), Error> {
        let budget = self.budget();
        let deadline = self.deadlines;
        let stream = self.transport_mut()?;
        tokio::time::timeout(budget, async {
            stream.write_all(bytes).await?;
            stream.flush().await
        })
        .await
        .map_err(|_| deadline.timeout_error(Instant::now(), true, Stage::DataBody))?
        .map_err(io_error)
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
        if ehlo.value() == 250 {
            self.capabilities = Capabilities::parse(&ehlo);
            return Ok(());
        }

        // RFC 5321 §4.1.1.1: a server may refuse EHLO, and the client must
        // then try HELO. A server that answers only HELO advertises no
        // extensions, so STARTTLS, AUTH and SIZE are all unavailable — the
        // client records exactly that instead of guessing at them. A transient
        // 4xx is reported, because retrying the greeting cannot help it.
        if !(500..600).contains(&ehlo.value()) {
            return Err(unexpected(&ehlo, Stage::ConnectionSetup));
        }
        let helo = self
            .command(
                Command::Helo {
                    identity: self.hello.clone(),
                },
                Stage::ConnectionSetup,
            )
            .await?;
        expect(&helo, 250, Stage::ConnectionSetup)?;
        self.capabilities = Capabilities::default();
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
        self.encrypted = true;
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
        let connected = !self.in_setup;

        let stream = self.transport_mut()?;
        tokio::time::timeout(budget, async {
            stream.write_all(&bytes).await?;
            stream.flush().await
        })
        .await
        .map_err(|_| deadline.timeout_error(Instant::now(), connected, stage))?
        .map_err(io_error)?;

        self.read_reply(stage).await
    }

    pub async fn read_reply(&mut self, stage: Stage) -> Result<Reply, Error> {
        let mut builder = ReplyBuilder::new();
        loop {
            let line = self.read_line(stage).await?;
            if builder.push_line(&line)? {
                return builder.finish();
            }
        }
    }

    async fn read_line(&mut self, stage: Stage) -> Result<Vec<u8>, Error> {
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
            let connected = !self.in_setup;
            let read = tokio::time::timeout(budget, self.transport_mut()?.read(&mut chunk))
                .await
                .map_err(|_| deadline.timeout_error(Instant::now(), connected, stage))?
                .map_err(io_error)?;

            if read == 0 {
                return Err(Error::new(ErrorCode::ProtocolConnectionReset));
            }
            self.buffer.extend_from_slice(&chunk[..read]);
        }
    }

    /// Budget for one read or write. During connection setup the connect window
    /// governs greeting, EHLO and TLS; afterwards the operation budget does,
    /// always capped by the remaining total.
    fn budget(&self) -> Duration {
        let now = Instant::now();
        if self.in_setup {
            return self.remaining(self.deadlines.connect_by());
        }
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
    Err(unexpected(reply, stage))
}

fn unexpected(reply: &Reply, stage: Stage) -> Error {
    Error::new(ErrorCode::ProtocolUnexpectedResponse).with_details(
        ErrorDetails::default()
            .stage(stage)
            .smtp_code(reply.value()),
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

impl Connection {
    /// Authenticates after TLS with PLAIN or LOGIN, per ADR-0017.
    ///
    /// `preferred` forces a mechanism when the caller configured one; an
    /// unadvertised preference fails instead of silently falling back.
    pub async fn authenticate(
        &mut self,
        user: &str,
        password: &str,
        preferred: Option<AuthMechanism>,
    ) -> Result<(), Error> {
        if !self.encrypted {
            return Err(auth_unsupported());
        }

        let mechanism = choose_auth(
            self.capabilities.supports_auth(AuthMechanism::Plain),
            self.capabilities.supports_auth(AuthMechanism::Login),
            self.capabilities.supports_auth(AuthMechanism::XOAuth2),
            preferred,
        )
        .ok_or_else(auth_unsupported)?;

        match mechanism {
            AuthMechanism::Plain => {
                let reply = self
                    .command(
                        Command::Auth {
                            mechanism: AuthMechanism::Plain,
                            initial_response: Some(auth_plain_initial(user, password)),
                        },
                        Stage::Auth,
                    )
                    .await?;
                confirm_auth(reply)
            }
            AuthMechanism::Login => {
                let reply = self
                    .command(
                        Command::Auth {
                            mechanism: AuthMechanism::Login,
                            initial_response: None,
                        },
                        Stage::Auth,
                    )
                    .await?;
                if reply.value() != 334 {
                    return confirm_auth(reply);
                }

                let mut challenges = 0;
                let mut reply = self
                    .command(
                        Command::AuthResponse {
                            response: STANDARD.encode(user),
                        },
                        Stage::Auth,
                    )
                    .await?;

                while reply.value() == 334 {
                    challenges += 1;
                    if challenges > 1 {
                        return Err(auth_error(ErrorCode::AuthFailed, &reply));
                    }
                    reply = self
                        .command(
                            Command::AuthResponse {
                                response: STANDARD.encode(password),
                            },
                            Stage::Auth,
                        )
                        .await?;
                }

                confirm_auth(reply)
            }
            AuthMechanism::XOAuth2 => {
                // `password` carries the bearer token for this mechanism.
                let reply = self
                    .command(
                        Command::Auth {
                            mechanism: AuthMechanism::XOAuth2,
                            initial_response: Some(auth_xoauth2_initial(user, password)),
                        },
                        Stage::Auth,
                    )
                    .await?;
                if reply.value() != 334 {
                    return confirm_auth(reply);
                }

                // RFC 7628: the 334 carries a base64 JSON error description that
                // the client must not parse. Answer with an empty line so the
                // relay sends its final failure reply, and report that.
                let final_reply = self
                    .command(
                        Command::AuthResponse {
                            response: String::new(),
                        },
                        Stage::Auth,
                    )
                    .await?;
                confirm_auth(final_reply)
            }
        }
    }
}

/// Chooses the AUTH mechanism: an explicit preference must be advertised,
/// otherwise PLAIN is preferred over LOGIN. XOAUTH2 is never chosen implicitly
/// because it needs a bearer token rather than a password.
fn choose_auth(
    plain: bool,
    login: bool,
    xoauth2: bool,
    preferred: Option<AuthMechanism>,
) -> Option<AuthMechanism> {
    match preferred {
        Some(AuthMechanism::Plain) if plain => Some(AuthMechanism::Plain),
        Some(AuthMechanism::Login) if login => Some(AuthMechanism::Login),
        Some(AuthMechanism::XOAuth2) if xoauth2 => Some(AuthMechanism::XOAuth2),
        Some(_) => None,
        None if plain => Some(AuthMechanism::Plain),
        None if login => Some(AuthMechanism::Login),
        None => None,
    }
}

fn confirm_auth(reply: Reply) -> Result<(), Error> {
    if reply.value() == 235 {
        return Ok(());
    }
    Err(auth_error(
        if reply.value() == 334 || reply.value() == 535 {
            ErrorCode::AuthFailed
        } else {
            ErrorCode::ProtocolUnexpectedResponse
        },
        &reply,
    ))
}

/// The relay's AUTH failure, with the enhanced status it supplied (RFC 3463).
///
/// The enhanced code is what `535` usually explains itself with ("5.7.8 …"), and
/// it is copied only when the relay sent one, so `details` stays honest.
fn auth_error(code: ErrorCode, reply: &Reply) -> Error {
    let mut details = ErrorDetails::default()
        .stage(Stage::Auth)
        .smtp_code(reply.value());
    if let Some(enhanced) = reply.enhanced() {
        details = details.smtp_enhanced_code(enhanced.raw());
    }
    Error::new(code).with_details(details)
}

/// The relay offered no mechanism this client can use, so no reply exists.
fn auth_unsupported() -> Error {
    Error::new(ErrorCode::AuthUnsupported).with_details(ErrorDetails::default().stage(Stage::Auth))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_engine_prefers_plain_then_login() {
        assert_eq!(
            choose_auth(true, true, false, None),
            Some(AuthMechanism::Plain)
        );
        assert_eq!(
            choose_auth(false, true, false, None),
            Some(AuthMechanism::Login)
        );
        assert_eq!(choose_auth(false, false, false, None), None);
    }

    #[test]
    fn xoauth2_is_never_chosen_implicitly() {
        assert_eq!(
            choose_auth(false, false, true, None),
            None,
            "a bearer token is not a password, so the caller has to ask for it"
        );
        assert_eq!(
            choose_auth(true, false, true, Some(AuthMechanism::XOAuth2)),
            Some(AuthMechanism::XOAuth2)
        );
    }

    #[test]
    fn an_explicit_mechanism_must_be_advertised() {
        assert_eq!(
            choose_auth(true, true, false, Some(AuthMechanism::Login)),
            Some(AuthMechanism::Login)
        );
        assert_eq!(
            choose_auth(true, false, false, Some(AuthMechanism::Login)),
            None
        );
        assert_eq!(
            choose_auth(false, true, false, Some(AuthMechanism::Plain)),
            None
        );
        assert_eq!(
            choose_auth(true, true, false, Some(AuthMechanism::XOAuth2)),
            None,
            "XOAUTH2 requested but not advertised"
        );
    }
}
