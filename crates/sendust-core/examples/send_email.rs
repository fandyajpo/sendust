//! Sends one real message through a configured SMTP relay.
//!
//! This is the only runnable entry point in the repository today: the Node-API
//! binding and the public TypeScript API arrive in P05 and P09. It reads its
//! inputs from the environment so no credential ever lives in the source tree:
//!
//! ```text
//! SENDUST_HOST=relay.example.test
//! SENDUST_PORT=465                  # default 465
//! SENDUST_SECURITY=tls             # tls (implicit) or starttls
//! SENDUST_USER=...                  # optional; both user and pass or neither
//! SENDUST_PASS=...
//! SENDUST_FROM=alice@example.test
//! SENDUST_TO=bob@example.test,carol@example.test
//! SENDUST_SUBJECT=Hello from Sendust
//! SENDUST_TEXT=Body text
//! SENDUST_EXTRA_ROOT=path/to/ca.pem  # optional extra trust anchor, for local tests
//! ```
//!
//! A successful result means the relay accepted responsibility; it is not a
//! mailbox delivery confirmation.

use std::env;

use sendust_core::{
    Delivery, Limits, Mailbox, OutgoingMessage, SmtpConfig, SmtpCredentials, SmtpSecurity,
    SmtpTransport,
};

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let host = env::var("SENDUST_HOST").map_err(|_| "SENDUST_HOST is required")?;
    let port: u16 = env::var("SENDUST_PORT")
        .unwrap_or_else(|_| "465".to_string())
        .parse()?;
    let security = match env::var("SENDUST_SECURITY").as_deref().unwrap_or("tls") {
        "tls" => SmtpSecurity::ImplicitTls,
        "starttls" => SmtpSecurity::StartTls,
        other => {
            return Err(format!("unknown SENDUST_SECURITY {other}; use tls or starttls").into());
        }
    };

    let mut config = SmtpConfig::new(host, port, security)?;

    match (env::var("SENDUST_USER"), env::var("SENDUST_PASS")) {
        (Ok(user), Ok(password)) => {
            config = config.with_credentials(SmtpCredentials::new(user, password)?);
        }
        (Err(_), Err(_)) => {}
        _ => return Err("set both SENDUST_USER and SENDUST_PASS, or neither".into()),
    }

    if let Ok(path) = env::var("SENDUST_EXTRA_ROOT") {
        config = config.with_root_certificate_pem(std::fs::read(path)?)?;
    }

    let from = Mailbox::parse(&env::var("SENDUST_FROM").map_err(|_| "SENDUST_FROM is required")?)?;
    let to = env::var("SENDUST_TO").map_err(|_| "SENDUST_TO is required")?;
    let recipients = to
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(Mailbox::parse)
        .collect::<Result<Vec<_>, _>>()?;

    let message = OutgoingMessage {
        from,
        to: recipients,
        cc: Vec::new(),
        bcc: Vec::new(),
        reply_to: Vec::new(),
        subject: env::var("SENDUST_SUBJECT").unwrap_or_else(|_| "Hello from Sendust".to_string()),
        text: Some(
            env::var("SENDUST_TEXT")
                .unwrap_or_else(|_| "Sent by the Sendust core send path.".to_string()),
        ),
        html: None,
        headers: Vec::new(),
        attachments: Vec::new(),
    };

    let mut transport = SmtpTransport::new(config);
    let mut delivery = Delivery::new(transport.session(), Limits::default(), false);
    let outcome = delivery.send(message).await?;

    println!("status: {}", outcome.status().as_str());
    for recipient in outcome.recipients() {
        println!("  {}: {}", recipient.address, recipient.status.as_str());
    }
    if let Some(response) = outcome.response() {
        println!("relay: {} {}", response.code, response.message);
    }

    Ok(())
}
