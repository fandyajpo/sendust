//! Sends one real message through a configured SMTP relay.
//!
//! This is a driver for the Rust core: the Node-API binding and the TypeScript
//! facade in `ts/` are how applications consume Sendust. Inputs come from
//! environment variables, or from a `.env` file in the working directory (real
//! environment variables win), so no credential lives in the source tree. Start
//! from the committed `.env.example`.
//!
//! A successful result means the relay accepted responsibility; it is not a
//! mailbox delivery confirmation.

use std::collections::HashMap;
use std::env;

use sendust_core::{
    AttachmentPolicy, Delivery, Limits, Mailbox, OutgoingMessage, SmtpConfig, SmtpCredentials,
    SmtpSecurity, SmtpTransport,
};

/// Environment plus an optional `.env` file, with real variables taking priority.
struct Settings {
    file: HashMap<String, String>,
}

impl Settings {
    fn load() -> Self {
        let mut file = HashMap::new();
        if let Ok(contents) = std::fs::read_to_string(".env") {
            for line in contents.lines() {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }
                let Some((key, value)) = line.split_once('=') else {
                    continue;
                };
                let value = value.trim();
                let value = value
                    .strip_prefix('"')
                    .and_then(|inner| inner.strip_suffix('"'))
                    .or_else(|| {
                        value
                            .strip_prefix('\'')
                            .and_then(|inner| inner.strip_suffix('\''))
                    })
                    .unwrap_or(value);
                file.insert(key.trim().to_string(), value.to_string());
            }
        }
        Self { file }
    }

    fn get(&self, key: &str) -> Option<String> {
        match env::var(key) {
            Ok(value) if !value.is_empty() => Some(value),
            _ => self
                .file
                .get(key)
                .filter(|value| !value.is_empty())
                .cloned(),
        }
    }

    fn required(&self, key: &str) -> Result<String, String> {
        self.get(key)
            .ok_or_else(|| format!("{key} is required (see .env.example)"))
    }

    fn number(&self, key: &str, default: u16) -> Result<u16, String> {
        match self.get(key) {
            Some(value) => value
                .parse()
                .map_err(|_| format!("{key} must be a port number")),
            None => Ok(default),
        }
    }
}

/// Values shipped in `.env.example`; an unedited copy must not send.
fn is_placeholder(value: &str) -> bool {
    let lowered = value.to_ascii_lowercase();
    lowered.contains("example.com")
        || lowered.contains("your-name")
        || lowered.contains("your-16-char-app-password")
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let settings = Settings::load();
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(run(&settings))
}

async fn run(settings: &Settings) -> Result<(), Box<dyn std::error::Error>> {
    let security = match settings.get("SENDUST_SECURITY").as_deref().unwrap_or("tls") {
        "tls" => SmtpSecurity::ImplicitTls,
        "starttls" => SmtpSecurity::StartTls,
        other => {
            return Err(format!("unknown SENDUST_SECURITY {other}; use tls or starttls").into());
        }
    };
    let mode = match security {
        SmtpSecurity::ImplicitTls => "tls",
        SmtpSecurity::StartTls => "starttls",
    };

    for key in [
        "SENDUST_HOST",
        "SENDUST_USER",
        "SENDUST_PASS",
        "SENDUST_FROM",
        "SENDUST_TO",
    ] {
        if let Some(value) = settings.get(key)
            && is_placeholder(&value)
        {
            return Err(format!(
                "{key} still contains an example placeholder; edit .env (see .env.example)"
            )
            .into());
        }
    }

    let host = settings.required("SENDUST_HOST")?;
    let port = settings.number("SENDUST_PORT", 465)?;
    if (security == SmtpSecurity::ImplicitTls && port == 587)
        || (security == SmtpSecurity::StartTls && port == 465)
    {
        eprintln!(
            "warning: port {port} is unusual for {mode}; 465 is implicit TLS, 587 is STARTTLS"
        );
    }

    let mut config = SmtpConfig::new(host.clone(), port, security)?;

    let user = settings.get("SENDUST_USER");
    let password = match (user.as_ref(), settings.get("SENDUST_PASS")) {
        (Some(_), None) => {
            return Err(
                "SENDUST_USER is set but SENDUST_PASS is empty; set the password or unset both"
                    .into(),
            );
        }
        (None, Some(_)) => {
            return Err(
                "SENDUST_PASS is set but SENDUST_USER is empty; set the user or unset both".into(),
            );
        }
        (Some(_), Some(value))
            if host.to_ascii_lowercase().ends_with("gmail.com") && value.contains(' ') =>
        {
            println!("note: removed spaces from the Gmail app password");
            Some(value.replace(' ', ""))
        }
        (_, other) => other,
    };
    if let (Some(user), Some(password)) = (user, password) {
        config = config.with_credentials(SmtpCredentials::new(user, password)?);
    }

    if let Some(path) = settings.get("SENDUST_EXTRA_ROOT") {
        config = config.with_root_certificate_pem(std::fs::read(path)?)?;
    }

    let from = Mailbox::parse(&settings.required("SENDUST_FROM")?)?;
    let recipients = settings
        .required("SENDUST_TO")?
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(Mailbox::parse)
        .collect::<Result<Vec<_>, _>>()?;

    let message = OutgoingMessage {
        from: from.clone(),
        to: recipients.clone(),
        cc: Vec::new(),
        bcc: Vec::new(),
        reply_to: Vec::new(),
        subject: settings
            .get("SENDUST_SUBJECT")
            .unwrap_or_else(|| "Hello from Sendust".to_string()),
        text: Some(
            settings
                .get("SENDUST_TEXT")
                .unwrap_or_else(|| "Sent by the Sendust core send path.".to_string()),
        ),
        html: None,
        headers: Vec::new(),
        attachments: Vec::new(),
    };

    println!(
        "relay: {host}:{port} {mode}, auth: {}",
        if config.credentials().is_some() {
            "yes"
        } else {
            "no"
        }
    );
    println!("from: {from}");
    println!(
        "to: {}",
        recipients
            .iter()
            .map(|mailbox| mailbox.address())
            .collect::<Vec<_>>()
            .join(", ")
    );

    let mut transport = SmtpTransport::new(config);
    let mut delivery = Delivery::new(
        transport.session(),
        Limits::default(),
        AttachmentPolicy::Disabled,
    );
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
