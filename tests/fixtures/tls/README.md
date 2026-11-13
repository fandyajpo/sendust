# TLS test fixtures

Generated material for Sendust tests only. These certificates and leaf keys
have no value outside this repository: never trust them, never install them, and
never use them in a real client or server.

The trust-anchor and intermediate *signing* keys are deliberately not stored
here. `scripts/generate-tls-fixtures.sh` writes them to a temporary directory and
discards them, so nothing in this repository can sign a new certificate. Only the
leaf server keys remain, because tests must serve those certificates.

Regenerate deterministically with:

```text
scripts/generate-tls-fixtures.sh tests/fixtures/tls
```

The fixture catalog and expected verification results are defined in
[docs/engineering/test-harness-contract.md](../../../docs/engineering/test-harness-contract.md).
