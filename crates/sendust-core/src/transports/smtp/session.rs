//! SMTP session state machine: AUTH, MAIL, RCPT, DATA, QUIT (P06-003).
//!
//! The session returns raw replies for the transaction so the delivery adapter
//! classifies outcomes (ADR-0005); only protocol-level requirements become
//! errors here.

use crate::errors::{Error, Stage};
use crate::transports::smtp::SmtpConfig;
use crate::transports::smtp::client::connection::Connection;
use crate::transports::smtp::proto::capabilities::Capabilities;
use crate::transports::smtp::proto::command::Command;
use crate::transports::smtp::proto::response::Reply;
use crate::transports::smtp::proto::transcript;

/// One open SMTP session over a native connection.
pub struct Session {
    connection: Connection,
}

impl Session {
    pub async fn open(config: &SmtpConfig) -> Result<Self, Error> {
        Ok(Self {
            connection: Connection::connect(config).await?,
        })
    }

    pub fn capabilities(&self) -> &Capabilities {
        self.connection.capabilities()
    }

    pub fn is_encrypted(&self) -> bool {
        self.connection.is_encrypted()
    }

    /// Performs the required STARTTLS upgrade and the re-EHLO that follows it.
    pub async fn starttls(&mut self) -> Result<(), Error> {
        self.connection.starttls().await
    }

    pub async fn authenticate(&mut self, user: &str, password: &str) -> Result<(), Error> {
        self.connection.authenticate(user, password).await
    }

    pub async fn mail_from(&mut self, address: &str) -> Result<Reply, Error> {
        self.connection
            .command(
                Command::MailFrom {
                    address: address.to_owned(),
                },
                Stage::Mail,
            )
            .await
    }

    pub async fn rcpt_to(&mut self, address: &str) -> Result<Reply, Error> {
        self.connection
            .command(
                Command::RcptTo {
                    address: address.to_owned(),
                },
                Stage::Rcpt,
            )
            .await
    }

    pub async fn data_command(&mut self) -> Result<Reply, Error> {
        self.connection
            .command(Command::Data, Stage::DataCommand)
            .await
    }

    pub async fn data_body(&mut self, wire: &[u8]) -> Result<Reply, Error> {
        self.connection.write_raw(&transcript::encode(wire)).await?;
        self.connection.read_reply(Stage::DataCommand).await
    }

    pub async fn quit(&mut self) -> Result<Reply, Error> {
        self.connection.command(Command::Quit, Stage::Cleanup).await
    }
}
