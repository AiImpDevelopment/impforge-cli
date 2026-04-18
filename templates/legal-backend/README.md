# Legal Backend (Postgres 17)

Postgres 17 backend with legal-flavored seed migration (matter, conflict_check, trust_account_ledger) and read-replica routing default.

**Industry:** `legal`  ·  **Category:** `backend`  ·  **Framework:** `rust-1.95`  ·  **License:** `MIT`

## Compliance

This template ships **100** Crown-Jewel compliance rules across the following regimes:

> ABA-MRPC, GDPR, ISO-27001, SOC2

The full machine-readable rule set lives next to this README at `compliance-rules.json` and conforms to the [`compliance-rules.json` v1 spec](https://github.com/AiImpDevelopment/impforge-mcp-manifests/blob/main/spec/compliance-rules.json.v1.md).

## Run locally

```bash
cargo run
```

The preview is reachable at `http://localhost:8080` once it's ready.

## Production build

```bash
cargo build --release
```

## Provenance

This pack was rendered by the ImpForge engine (https://github.com/AiImpDevelopment/impforge). The `template.json` manifest in this directory conforms to the [v1 spec](https://github.com/AiImpDevelopment/impforge-mcp-manifests/blob/main/spec/template.json.v1.md).
