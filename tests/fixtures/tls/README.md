# TLS test fixtures

Generated material for Sendust tests only. These keys and certificates have no
value outside this repository: never trust them, never install them, and never
use them in a real client or server.

Regenerate deterministically with:

```text
scripts/generate-tls-fixtures.sh tests/fixtures/tls
```

The fixture catalog and expected verification results are defined in
[docs/engineering/test-harness-contract.md](../../../docs/engineering/test-harness-contract.md).
