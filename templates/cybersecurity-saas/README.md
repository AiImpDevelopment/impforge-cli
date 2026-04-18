# Cybersecurity Multi-Tenant SaaS

Multi-tenant SOC / EDR / SIEM platform scaffold with alert / incident / threat-intel tables, OAuth 2.1 + PKCE auth, and Stripe-billed enterprise subscriptions.

**Industry:** `cybersecurity`  ·  **Category:** `saas`  ·  **Framework:** `next-15`  ·  **License:** `MIT`

## Compliance

This template ships **100** Crown-Jewel compliance rules across the following regimes:

> SOC2, ISO-27001, NIST-CSF, GDPR, FEDRAMP, CMMC

The full machine-readable rule set lives next to this README at `compliance-rules.json` and conforms to the [`compliance-rules.json` v1 spec](https://github.com/AiImpDevelopment/impforge-mcp-manifests/blob/main/spec/compliance-rules.json.v1.md).

## Run locally

```bash
bun install && bun run dev
```

The preview is reachable at `http://localhost:3000` once it's ready.

## Production build

```bash
bun run build
```

## Provenance

This pack was rendered by the ImpForge engine (https://github.com/AiImpDevelopment/impforge). The `template.json` manifest in this directory conforms to the [v1 spec](https://github.com/AiImpDevelopment/impforge-mcp-manifests/blob/main/spec/template.json.v1.md).
