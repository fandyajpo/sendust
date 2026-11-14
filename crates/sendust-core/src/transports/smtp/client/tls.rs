//! TLS configuration for the native SMTP client (ADR-0003, ADR-0017).
//!
//! Verification is never optional: the trust store is the bundled Mozilla
//! roots plus any caller-supplied PEM anchors, hostname checks are on, and TLS
//! 1.2 is the floor. `dangerous_*` switches do not exist here.

use std::sync::Arc;

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use rustls::pki_types::{CertificateDer, ServerName};
use rustls::{ClientConfig, RootCertStore};
use tokio_rustls::TlsConnector;

use crate::errors::{Error, ErrorCode};
use crate::transports::smtp::SmtpConfig;

pub(crate) fn connector(config: &SmtpConfig) -> Result<TlsConnector, Error> {
    Ok(TlsConnector::from(Arc::new(client_config(config)?)))
}

pub(crate) fn client_config(config: &SmtpConfig) -> Result<ClientConfig, Error> {
    let mut roots = RootCertStore::empty();
    roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());

    for pem in config.root_certificates() {
        for certificate in parse_certificates(pem)? {
            roots.add(certificate).map_err(|_| tls_error())?;
        }
    }

    let config =
        ClientConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
            .with_safe_default_protocol_versions()
            .map_err(|_| tls_error())?
            .with_root_certificates(roots)
            .with_no_client_auth();

    Ok(config)
}

pub(crate) fn server_name(host: &str) -> Result<ServerName<'static>, Error> {
    ServerName::try_from(host.to_string()).map_err(|_| tls_error())
}

/// Validates one PEM anchor by adding it to a root store.
pub(crate) fn validate_anchor(pem: &[u8]) -> Result<(), Error> {
    let mut roots = RootCertStore::empty();
    for certificate in parse_certificates(pem)? {
        roots.add(certificate).map_err(|_| tls_error())?;
    }
    Ok(())
}

fn parse_certificates(pem: &[u8]) -> Result<Vec<CertificateDer<'static>>, Error> {
    let text = std::str::from_utf8(pem).map_err(|_| tls_error())?;
    let mut certificates = Vec::new();
    let mut current = String::new();
    let mut inside = false;

    for line in text.lines() {
        let line = line.trim();
        if line == "-----BEGIN CERTIFICATE-----" {
            inside = true;
            current.clear();
        } else if line == "-----END CERTIFICATE-----" {
            if !inside {
                return Err(tls_error());
            }
            let der = STANDARD
                .decode(current.as_bytes())
                .map_err(|_| tls_error())?;
            certificates.push(CertificateDer::from(der));
            inside = false;
        } else if inside {
            current.push_str(line);
        }
    }

    if inside || certificates.is_empty() {
        return Err(tls_error());
    }
    Ok(certificates)
}

fn tls_error() -> Error {
    Error::new(ErrorCode::TlsHandshake)
}
