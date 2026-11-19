# Release gate — 1.0.0

**Decision:** APPROVED — project owner approval, 2026-11-19. The owner authorized
the `1.0.0` release in this chat ("approve"), after the benchmark results and the
known limitations were reported.
**Release:** `@sendust/ts@1.0.0`, dist-tag `latest`.
**Related:** [release notes](../releases/1.0.0.md), ADR-0021, ADR-0022,
[benchmark](../qa/benchmark-nodemailer.md),
[audit](../audits/2026-11-18-autonomous-delivery-audit.md).

## 1. Scope

This release publishes the autonomous delivery engine (ADR-0021) and SMTP
session reuse (ADR-0022) as the first stable version. It is a **major** release
because a concurrent `send()` is queued instead of rejecting with
`TRANSPORT_BUSY` (versioning policy: changed lifecycle semantics are major).

## 2. Approval checklist

| Check | Evidence |
| --- | --- |
| Owner approval recorded | This record, section title |
| ADRs accepted | ADR-0021, ADR-0022, both in the ADR index |
| Rust suite, clippy, fmt | CI `Rust workspace` job, plus a local full run |
| TypeScript typecheck, lint, format, 68 tests | CI `TypeScript` jobs on three Node versions |
| Runtime behaviour on Node.js, Bun, Deno | `tests/runtime/behaviour.mjs`, 16/16 |
| Five native targets | CI `Native artifact` jobs |
| Benchmark performed | [benchmark-nodemailer.md](../qa/benchmark-nodemailer.md) |
| Documentation updated | README boundaries, public API §8, guides, changelog, release notes |
| Version consistent | `scripts/check-versions.mjs` |
| Rollback plan | Deprecate `1.0.0` and publish a patch; never republish a version |

## 3. Binary verification for this release

`release/1.0.0/approved.json` pins the approved commit and the facade tarball
hash. It does **not** pin per-target binary hashes: those exist only in the
approval bundle's `SHA256SUMS`, and `publish-release` verifies every packed
binary against the bundle that was built in the same approved run, plus the
approved facade hash and build inputs. Byte-for-byte reproducibility is not
claimed for this release (the Windows DLL was already non-reproducible).

## 4. Known limitations at approval

- The benchmark is a loopback comparison on one machine; no provider or real
  relay measurement is claimed.
- A few extra session re-establishments appeared in the benchmark and are
  recorded, not explained away.
- Provider quota policies remain a backlog item.
- The npm publish path still uses the owner's bypass-2FA token until the
  January 2027 deadline is addressed.

## 5. Publication

Publication runs from the CI `publish` job with the `publish` input enabled; it
is never run from a developer machine ([release strategy](../engineering/release-strategy.md)
§10). The tag `v1.0.0` is created on the approved commit after the gate, and is
never moved.
