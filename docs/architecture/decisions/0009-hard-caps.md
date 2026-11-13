# ADR-0009 — Hard caps for recipients, attachments, and wire size

**Status:** ACCEPTED (planning)  
**Date:** 2026-11-13  
**Deciders:** Architecture review (P02-005)  
**Validated by:** P02-004, P04-003, P08-003, P13-003

## Context

An email library that buffers whole messages can be used to exhaust memory. Relays also impose practical limits. Without explicit caps, behavior depends on the relay and on available memory, which is unpredictable and untestable.

## Decision

| Cap | Default | Enforced |
| --- | --- | --- |
| Recipients per message (To + CC + BCC) | 100 | Before connect |
| Per attachment raw bytes | 10 MiB | Before MIME composition |
| Aggregate raw attachment bytes | 20 MiB | Before MIME composition |
| Serialized wire message | 30 MiB | Before DATA |
| Active sends per transport | 1 | Before connect |

- Caps are hard and documented.
- Exceeding a cap fails with a stable limit error; no truncation.
- Caps may be lowered through configuration only if the implementation supports it; raising above the default is not permitted in the MVP.
- The exact measured peak memory is recorded in P13-003.

## Alternatives

1. **No caps.** Unbounded memory and relay-dependent behavior; rejected.
2. **Advisory caps with truncation.** Silent data loss; rejected.
3. **Streaming with backpressure.** Would remove the buffering need, but is post-MVP.
4. **Relay-discovered limits only.** Unreliable across providers; rejected.

## Consequences

- Memory use is bounded and testable.
- Large legitimate messages require a future streaming feature.
- Documentation must state the caps and the error codes.
- Tests must cover exactly-at-limit and one-byte-over for every cap.

## Evidence required

- Boundary tests for 100/101 recipients, 10 MiB/10 MiB+1, 20 MiB/20 MiB+1, 30 MiB/30 MiB+1.
- Memory measurements in P13-003.
- Documentation of caps and errors in P15-002/P15-004.
