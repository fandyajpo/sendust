//! TLS support for the fake SMTP harness.
//!
//! Uses the committed test fixtures: the harness serves `valid-server` and test
//! clients trust `ca.crt`. This is the only place the harness touches TLS, and
//! it is test-only code.

use std::fs;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::path::Path;
use std::sync::Arc;

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer, ServerName};
use rustls::{ClientConfig, ClientConnection, RootCertStore, ServerConfig, StreamOwned};

fn der_blocks(pem: &str, label: &str) -> Vec<Vec<u8>> {
    let begin = format!("-----BEGIN {label}-----");
    let end = format!("-----END {label}-----");
    let mut blocks = Vec::new();
    let mut inside = false;
    let mut current = String::new();

    for line in pem.lines() {
        let trimmed = line.trim();
        if trimmed == begin {
            inside = true;
            current.clear();
        } else if trimmed == end {
            inside = false;
            if let Ok(bytes) = STANDARD.decode(current.trim()) {
                blocks.push(bytes);
            }
        } else if inside {
            current.push_str(trimmed);
        }
    }
    blocks
}

fn read(pem_path: &Path, label: &str) -> Vec<Vec<u8>> {
    let pem = fs::read_to_string(pem_path).expect("read fixture");
    der_blocks(&pem, label)
}

fn install_provider() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        let _ = rustls::crypto::ring::default_provider().install_default();
    });
}

/// Server config serving `valid-server` with the committed key.
pub fn server_config(tls_dir: &Path) -> Arc<ServerConfig> {
    server_config_named(tls_dir, "valid-server")
}

/// Server config serving one named fixture pair, e.g. `self-signed`.
pub fn server_config_named(tls_dir: &Path, name: &str) -> Arc<ServerConfig> {
    install_provider();

    let chain = read(&tls_dir.join(format!("{name}.crt")), "CERTIFICATE")
        .into_iter()
        .map(CertificateDer::from)
        .collect::<Vec<_>>();
    let key = read(&tls_dir.join(format!("{name}.key")), "PRIVATE KEY")
        .into_iter()
        .next()
        .map(|bytes| PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(bytes)))
        .expect("server key");

    Arc::new(
        ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(chain, key)
            .expect("valid server certificate"),
    )
}

/// Client config trusting only the committed test CA.
pub fn client_config(tls_dir: &Path) -> Arc<ClientConfig> {
    install_provider();

    let mut roots = RootCertStore::empty();
    for der in read(&tls_dir.join("ca.crt"), "CERTIFICATE") {
        roots.add(CertificateDer::from(der)).expect("trust anchor");
    }

    Arc::new(
        ClientConfig::builder()
            .with_root_certificates(roots)
            .with_no_client_auth(),
    )
}

/// A client that trusts nothing, used to prove verification fails closed.
pub fn untrusting_client_config() -> Arc<ClientConfig> {
    install_provider();
    Arc::new(
        ClientConfig::builder()
            .with_root_certificates(RootCertStore::empty())
            .with_no_client_auth(),
    )
}

/// Connects to the harness over implicit TLS as `localhost`.
pub fn connect(
    addr: SocketAddr,
    config: Arc<ClientConfig>,
) -> StreamOwned<ClientConnection, TcpStream> {
    let stream = TcpStream::connect(addr).expect("tcp connect");
    stream
        .set_read_timeout(Some(std::time::Duration::from_secs(5)))
        .expect("read timeout");
    let server_name = ServerName::try_from("localhost").expect("server name");
    let session = ClientConnection::new(config, server_name).expect("client session");
    StreamOwned::new(session, stream)
}

/// Reads one CRLF-terminated line from any reader, for the harness.
pub fn read_line<S: Read>(stream: &mut S, out: &mut String) -> std::io::Result<bool> {
    let mut bytes = Vec::new();
    let mut byte = [0u8; 1];
    loop {
        match stream.read(&mut byte)? {
            0 => return Ok(false),
            _ if byte[0] == b'\n' => break,
            _ => {
                bytes.push(byte[0]);
                if bytes.len() > 8192 {
                    return Ok(false);
                }
            }
        }
    }
    out.clear();
    out.push_str(String::from_utf8_lossy(&bytes).trim_end_matches('\r'));
    Ok(true)
}

/// Writes a raw byte string and flushes.
pub fn write_all<S: Write>(stream: &mut S, data: &[u8]) -> std::io::Result<()> {
    stream.write_all(data)?;
    stream.flush()
}
