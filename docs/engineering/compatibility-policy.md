# Post-MVP compatibility and deprecation policy (P18-004)

**Status:** active policy for releases after `0.1.0-rc.1`.
**Date:** 2026-11-18.
**Companions:** [versioning policy](versioning-policy.md) (versions, tags, changelog)
and [release strategy](release-strategy.md) (publication and rollback).

The versioning policy defines how a version is numbered and published. This
policy defines what a release may change without breaking a caller, how a
deprecation is announced, and which parts of the MVP contract are frozen.

## 1. What the semver level means here

| Change | Level |
| --- | --- |
| Documentation, error message wording, internal refactor with identical observable behaviour | patch |
| New optional capability that no existing caller must adopt, with unchanged defaults | minor |
| Changed default, removed capability, changed status semantics, changed error code, changed cap, changed timeout, changed TLS posture | major |
| Any change that turns `unknown` into a success, retries automatically, or claims delivery | not permitted; it contradicts the product |

The published artifact and its hashes are immutable. A fix ships as a new version,
never as a replacement under an existing one.

## 2. Frozen MVP contract

These are not subject to silent change, at any version:

- `accepted` means relay acceptance only; there is no `delivered`, `queued`,
  `opened`, or `bounced` status.
- `rejected` is an explicit negative final reply; `unknown` is a missing or
  indeterminate final reply after `DATA`, and it is never retried by the library.
- `TRANSPORT_BUSY` fails fast with no dial and no queue.
- `close()` is explicit, idempotent, and bounded by 125 s; it never depends on GC.
- Timeouts stay finite and lower-only: connect 10 s, operation 30 s, total 120 s,
  close 125 s.
- Caps stay hard: 100 recipients, 10 MiB per attachment, 20 MiB aggregate raw,
  30 MiB wire.
- Attachments stay opt-in, regular-file only, with no URL or remote form.
- TLS verification is always on; there is no bypass and no plaintext mode.
- Envelope local parts are ASCII; domains are IDNA-converted; SMTPUTF8 is absent.
- No public `AbortSignal` (C-17); cancellation is `close()`.
- One active send per transport; no pool, queue, worker, scheduler, or adapter.

Changing any line above requires an approved ADR, new tests, updated guides,
release notes, and a release-gate record. A "small" change is still a change.

## 3. Deprecation process

1. **Announce** in the changelog under `Deprecated`, name the replacement, and
   state the first version in which removal may happen.
2. **Mark** the surface in the type declarations with `@deprecated` and a pointer
   to the replacement; the runtime behaviour stays unchanged.
3. **Keep it working** for at least one minor release and 90 days, whichever is
   longer, before removal in a major release.
4. **Document** the migration in the release notes of both the deprecating and the
   removing release.
5. **Remove** only in a major release, with the migration notes and the gate
   record naming the removal.

A deprecation cannot be used to change semantics quietly: the old surface keeps
its old behaviour until it is removed.

## 4. Support and compatibility windows

| Item | Promise |
| --- | --- |
| Node.js | The pinned `engines.node` line and the CI-tested versions; widening requires a full suite and behaviour run |
| Bun, Deno | The versions in the toolchain baseline; a new version is a claim only after CI passes |
| Platform targets | Only the targets in the platform matrix, built and loaded natively in CI |
| Pre-releases | `0.1.0-x` releases may iterate; the first stable release fixes the baseline for the rules above |
| Dist-tags | `next` for pre-releases; `latest` moves only on a stable release |

## 5. Release-gate consequences

- Every release record names the version, commit, hashes, and the tests that
  prove the behaviour it claims.
- A breaking-change release names its ADR and its migration notes.
- A release that touches the frozen list in section 2 is a major release, even if
  the diff is small.
- Post-publication defects follow the rollback and deprecation path; they are
  never patched in place.
