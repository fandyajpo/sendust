# Rust Performance Audit — 2026-11-27

**Result:** eight findings, all fixed in `crates/sendust-core`; one finding is a
missing metric rather than a defect. Emitted bytes are unchanged, no public API,
result, error code, limit or default moved, and the full workspace suite passes.

**Scope:** the local cost the core owns — MIME composition and wire accounting,
base64 encoding, attachment reads, DATA transparency, and DKIM body
canonicalization. Not contact with a relay. Budgets and method are the ones in
[the performance budget](../qa/performance-budget.md); the remaining
optimisation candidates are in
[the improvement roadmap](../../audit/code-audit/IMPROVEMENT_ROADMAP.md) §3.4.

## 1. Findings

| ID | Where | Finding | Fix |
| --- | --- | --- | --- |
| P-01 | `mime/mod.rs` `push_base64` | One `String` per 57-byte chunk, so a 10 MiB attachment allocated about 184 000 short strings | One line buffer reused for the whole payload |
| P-02 | `mime/mod.rs` `compose`, `Body::as_part`, `build_attachment_part` | Every buffer started empty and doubled its way up to the wire size, so a 10 MiB attachment copied the payload several times and peaked at 61 MiB | `preflight` returns the measured encoded size and each buffer is reserved once |
| P-03 | `mime/mod.rs` `normalize_body`, `normalized_len` | Both walked characters, so a 1 MiB body cost two UTF-8 decode passes | Byte runs: `\r` and `\n` are ASCII and a multi-byte sequence contains neither |
| P-04 | `mime/headers.rs` `render`, `fold` | Every ASCII value was copied into a `String` before folding, and the prefix was built with `format!` | ASCII folds the borrowed value; the prefix is pushed in place |
| P-05 | `mime/mod.rs` `push_multipart` | A `format!` allocation per boundary line | The three segments are pushed directly |
| P-06 | `mime/mod.rs` `Body::Single` | An owned `String` copied from a literal | `&'static str` |
| P-07 | `attachments/mod.rs` `read_bounded` | Reading into `Vec::new()` let the growth path copy the file, and the buffer could reach twice the file's size | Reserves the file's own length, capped by the limit and still bounded by `take` |
| P-08 | `dkim/mod.rs` `canonical_body` | A `Vec<u8>` per body line plus a second full copy, so a signed 10 MiB attachment allocated about 184 000 buffers | One output buffer, one reusable line buffer, and a deferred-separator counter for the trailing blank lines |
| P-09 | `docs/qa/performance-budget.md` | The signing path has no metric, so a DKIM change cannot be gated on timing | Recorded here: P-08 is verified by the independent verifier in `crates/sendust-core/tests/dkim.rs`, not by a timing claim |

## 2. Measurement

Both runs use the release profile the budget requires (`lto = "thin"`,
`codegen-units = 1`), the pinned toolchain, `--test-threads=1`, and the two
harnesses in [the budget](../qa/performance-budget.md) §4. "Before" is the
tree at `3966eea`, "after" is `0df916b`; both are the same macOS reference
host, and a metric that moved more than 20% between the two after-runs is
reported as a range rather than a single value.

| Metric (median µs) | Before | After |
| --- | ---: | ---: |
| `serialize.small_text` | 20 | 9 |
| `serialize.1mib_text` | 5 346 | 2 150–3 248 |
| `serialize.10mib_attachment` | 32 858 | 30 376–30 674 |
| `serialize.unicode_long_headers` | 316 | 136–137 |
| `linearity.1mib` | 5 102 | 2 151–2 377 |
| `read.10mib_file` | 2 360 | 1 037–1 406 |
| `serialize.attachment_10mib` | 33 796 | 30 441–30 481 |
| `serialize.attachment_20mib` | 66 659 | 61 235–61 302 |

| Peak memory | Before | After |
| --- | ---: | ---: |
| `read.10mib_file` | 24.0 MiB | 10.0 MiB |
| `serialize.attachment_10mib` | 61.1 MiB | 27.4 MiB |
| `serialize.attachment_20mib` | 81.1 MiB | 54.7 MiB |

`copy.10mib` (1 467 → 1 350 µs) measures `Vec::clone` on unchanged code, so it
is noise and no improvement is claimed for it.

## 3. What was deliberately not changed

- **The base64 `simd` feature.** It would double the encoder's throughput on
  x86-64 and AArch64, but it changes the pinned feature set in the
  [dependency policy](../engineering/dependency-policy.md) §2 and needs evidence
  on all seven targets. It stays a candidate for a later release.
- **Streaming composition.** The wire still exists twice in memory before
  `DATA`; removing that copy is
  [roadmap §3.4](../../audit/code-audit/IMPROVEMENT_ROADMAP.md) and needs an
  ADR before code.
- **The base64 encoder itself.** The portable engine is the remaining cost of a
  large attachment: about 30 ms of the 30.4 ms measured for 13.4 MiB of output.
  No dependency change was made for it.

## 4. Verification

- `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`.
- `cargo test --workspace --all-targets --locked`: 14 suites, no failure. The
  MIME fixtures, the byte-exact serializer tests, the DATA transparency tests,
  and the DKIM verifier that re-derives the body hash and the signature with a
  second implementation all pass unchanged, which is what holds the emitted bytes
  identical.
- Both performance harnesses ran twice on the reference host; the correctness
  bounds in them (peak memory, residual memory after 20 × 10 MiB, wire size)
  pass with the values in section 2.

## 5. Method and limitations

- Read the core's composition, encoding, attachment, transparency and signing
  paths, measured the two harnesses before changing anything, and re-measured
  after each change landed on the reference host.
- The review is static plus measured; it is not a gate approval. Only the
  reference host is represented, and no relay latency, TLS handshake, or delivery
  time is claimed.
- Not reviewed: the TypeScript facade, the binding's buffer copy, and the SMTP
  client's I/O paths. The signing path has no budget (P-09).
