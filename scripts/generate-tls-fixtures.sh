#!/usr/bin/env bash
# Generate the committed TLS fixtures used by Sendust tests.
#
# Deterministic: fixed subjects, fixed validity windows, EC P-256 keys, and no
# network access. The generated key material is test-only and has no value
# outside this repository.
#
# Usage: scripts/generate-tls-fixtures.sh [output-dir]
set -euo pipefail

dest="${1:-tests/fixtures/tls}"
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
mkdir -p "$dest"

not_before="20200101000000Z"
not_after="20400101000000Z"
expired_before="20180101000000Z"
expired_after="20200101000000Z"
subject="/C=US/O=Sendust Test"

fail() {
  echo "generate-tls-fixtures: $*" >&2
  exit 1
}

command -v openssl >/dev/null 2>&1 || fail "openssl is required"

key() { openssl genpkey -algorithm EC -pkeyopt ec_paramgen_curve:P-256 -out "$1" 2>/dev/null; }
csr() { openssl req -new -key "$1" -out "$2" -subj "$3" 2>/dev/null; }

# Extensions used when signing leaf certificates with the test CA.
cat > "$work/server.ext" <<'EXT'
basicConstraints=critical,CA:FALSE
keyUsage=critical,digitalSignature
extendedKeyUsage=serverAuth
subjectAltName=DNS:localhost,IP:127.0.0.1
EXT

cat > "$work/wrong-host.ext" <<'EXT'
basicConstraints=critical,CA:FALSE
keyUsage=critical,digitalSignature
extendedKeyUsage=serverAuth
subjectAltName=DNS:wrong.example
EXT

cat > "$work/intermediate.ext" <<'EXT'
basicConstraints=critical,CA:TRUE,pathlen:0
keyUsage=critical,keyCertSign,cRLSign
EXT

# --- Trust anchor ----------------------------------------------------------
key "$dest/ca.key"
openssl req -x509 -new -key "$dest/ca.key" -out "$dest/ca.crt" \
  -not_before "$not_before" -not_after "$not_after" \
  -subj "$subject/CN=Sendust Test Root CA" \
  -addext "basicConstraints=critical,CA:TRUE" \
  -addext "keyUsage=critical,keyCertSign,cRLSign" 2>/dev/null

# --- Valid server certificate signed by the trust anchor -------------------
key "$dest/valid-server.key"
csr "$dest/valid-server.key" "$work/valid.csr" "$subject/CN=localhost"
openssl x509 -req -in "$work/valid.csr" -CA "$dest/ca.crt" -CAkey "$dest/ca.key" \
  -set_serial 1001 -not_before "$not_before" -not_after "$not_after" \
  -extfile "$work/server.ext" -out "$dest/valid-server.crt" 2>/dev/null

# --- Self-signed server certificate ----------------------------------------
key "$dest/self-signed.key"
openssl req -x509 -new -key "$dest/self-signed.key" -out "$dest/self-signed.crt" \
  -not_before "$not_before" -not_after "$not_after" \
  -subj "$subject/CN=localhost" \
  -addext "basicConstraints=critical,CA:FALSE" \
  -addext "keyUsage=critical,digitalSignature" \
  -addext "extendedKeyUsage=serverAuth" \
  -addext "subjectAltName=DNS:localhost,IP:127.0.0.1" 2>/dev/null

# --- Expired server certificate (validity window in the past) --------------
key "$dest/expired.key"
csr "$dest/expired.key" "$work/expired.csr" "$subject/CN=localhost"
openssl x509 -req -in "$work/expired.csr" -CA "$dest/ca.crt" -CAkey "$dest/ca.key" \
  -set_serial 1002 -not_before "$expired_before" -not_after "$expired_after" \
  -extfile "$work/server.ext" -out "$dest/expired.crt" 2>/dev/null

# --- Certificate for the wrong hostname ------------------------------------
key "$dest/wrong-host.key"
csr "$dest/wrong-host.key" "$work/wrong-host.csr" "$subject/CN=wrong.example"
openssl x509 -req -in "$work/wrong-host.csr" -CA "$dest/ca.crt" -CAkey "$dest/ca.key" \
  -set_serial 1003 -not_before "$not_before" -not_after "$not_after" \
  -extfile "$work/wrong-host.ext" -out "$dest/wrong-host.crt" 2>/dev/null

# --- Incomplete chain: leaf issued by an intermediate whose CA is withheld -#
key "$dest/intermediate.key"
csr "$dest/intermediate.key" "$work/intermediate.csr" "$subject/CN=Sendust Test Intermediate CA"
openssl x509 -req -in "$work/intermediate.csr" -CA "$dest/ca.crt" -CAkey "$dest/ca.key" \
  -set_serial 2001 -not_before "$not_before" -not_after "$not_after" \
  -extfile "$work/intermediate.ext" -out "$dest/intermediate.crt" 2>/dev/null

key "$dest/incomplete-chain.key"
csr "$dest/incomplete-chain.key" "$work/incomplete.csr" "$subject/CN=localhost"
openssl x509 -req -in "$work/incomplete.csr" -CA "$dest/intermediate.crt" -CAkey "$dest/intermediate.key" \
  -set_serial 2002 -not_before "$not_before" -not_after "$not_after" \
  -extfile "$work/server.ext" -out "$dest/incomplete-chain.crt" 2>/dev/null

# --- Malformed certificate: well-formed PEM wrapping, invalid DER -----------
printf '%s\n' \
  '-----BEGIN CERTIFICATE-----' \
  'c2VuZHVzdC10ZXN0LW1hbGZvcm1lZC1jZXJ0aWZpY2F0ZQ==' \
  '-----END CERTIFICATE-----' > "$dest/malformed.pem"

echo "generated TLS fixtures in $dest"
