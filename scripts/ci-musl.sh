#!/bin/sh
# Builds and verifies one Linux musl target inside the musl image.
#
# The `artifacts-musl` CI job runs this with `docker run node:<pin>-alpine` on a
# runner of the same architecture, so the binary is produced *and loaded* by a
# musl userland instead of being cross-compiled (ADR-0020). It is CI plumbing,
# not part of the product: nothing here ships.
set -eu

apk add --no-cache bash build-base curl git perl xz

# rustup resolves rust-toolchain.toml. Inside this image the host triple is
# <arch>-unknown-linux-musl, which is exactly the target being shipped.
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal
export PATH="$HOME/.cargo/bin:$PATH"

npm install --global npm@12.2.2 >/dev/null
node --version
rustc --version
cargo --version

# musl targets default to a static crt, and rustc refuses to produce a cdylib
# for one ("does not support these crate types"). A Node addon has to link the
# same dynamic musl libc that Node itself uses, so the crt is dynamic here.
export RUSTFLAGS="-C target-feature=-crt-static"

node scripts/build-artifact.mjs

node -e "
  const { readdirSync } = require('node:fs');
  const file = readdirSync('artifacts').find((name) => name.endsWith('.node'));
  if (!file) throw new Error('no artifact produced');
  const addon = require('./artifacts/' + file);
  if (!addon.Transport) throw new Error('no Transport export in ' + file);
  console.log('loaded', file);
"

npm ci --no-audit --no-fund

# The package build bundles this addon and the suite loads it as the bundled
# path, so both run against this image's own binary.
cp artifacts/*.node ts/sendust.node
npm --workspace ./ts run build
node scripts/prepare-publish.mjs
node scripts/assemble-rc.mjs

npm --workspace ./ts run test

# A clean consumer: the packed facade plus this platform package, resolved
# through the loader's fallback path. Bun and Deno are not claimed on musl, so
# those checks skip themselves.
node scripts/verify-platform.mjs
node tests/runtime/behaviour.mjs
