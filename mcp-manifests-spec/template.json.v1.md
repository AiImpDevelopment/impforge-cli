# template.json — v1 specification

**Status:** Pre-launch (v1.0.0-rc.1)
**License:** MIT
**Maintainer:** AiImp Technology (ImpForge)
**Authors:** AiImp + community
**Discussion:** [GitHub issues](https://github.com/AiImpDevelopment/impforge-mcp-manifests/issues)

---

## Summary

`template.json` is a **minimum-viable contract** that any scaffold can ship to be discoverable, validatable, and customisable by any AI coding tool that supports the spec.

The spec answers four questions:

1. **What does this template scaffold?** (category + industry + framework)
2. **What does it expect from the user?** (parameters + secrets)
3. **What guarantees does it make?** (compliance rule IDs + safety class)
4. **How do we run it?** (preview command + production build command)

A template that conforms to this spec works with [ImpForge][impforge], Cursor, Claude Code, Codex CLI, Gemini CLI, JetBrains AI, Copilot, Windsurf, and any future tool that registers the [`impforge-templates` MCP server][mcp].

[impforge]: https://github.com/AiImpDevelopment/impforge
[mcp]: https://github.com/AiImpDevelopment/impforge-templates

---

## File location

A conforming template is a directory containing — at the root — exactly one `template.json` file. Everything else is template content (source files, READMEs, tests, etc.).

```
my-template/
├── template.json     ← the manifest (this spec)
├── README.md         ← human-readable description
├── src/              ← scaffold content
└── tests/            ← validator tests (optional, recommended)
```

---

## Schema (JSON Schema draft-2020-12)

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://impforge.dev/spec/template.json.v1.json",
  "type": "object",
  "required": ["spec_version", "id", "name", "description", "category", "industry", "framework", "license", "compliance"],
  "additionalProperties": false,
  "properties": {
    "spec_version": { "const": "1" },
    "id":            { "type": "string", "pattern": "^[a-z][a-z0-9-]{2,63}$" },
    "name":          { "type": "string", "minLength": 1, "maxLength": 100 },
    "description":   { "type": "string", "minLength": 10, "maxLength": 500 },
    "category":      { "enum": ["web", "saas", "backend", "mobile", "desktop", "cli", "browser_extension", "bot", "firmware", "single_page", "ecommerce", "crm"] },
    "industry":      { "enum": ["aerospace", "agriculture", "automotive", "construction", "cybersecurity", "education", "energy", "engines", "fintech", "food", "government", "healthcare", "hospitality", "insurance", "legal", "logistics", "manufacturing", "maritime", "media", "mining", "nonprofit", "pharma", "professional", "proptech", "retail", "telecom", "general"] },
    "framework":     { "type": "string", "examples": ["next-15", "svelte-5", "vue-3.5", "astro-5", "remix-3", "tauri-2", "react-native-0.76", "rust-1.95", "node-22"] },
    "language":      { "type": "string", "examples": ["typescript", "rust", "python", "go", "kotlin"] },
    "license":       { "type": "string", "examples": ["MIT", "Apache-2.0", "BSD-3-Clause", "ELv2", "BUSL-1.1"] },
    "version":       { "type": "string", "pattern": "^\\d+\\.\\d+\\.\\d+(?:-[a-z0-9.]+)?$" },

    "tags":          { "type": "array", "items": { "type": "string" }, "maxItems": 16 },
    "homepage":      { "type": "string", "format": "uri" },
    "repository":    { "type": "string", "format": "uri" },

    "parameters": {
      "type": "object",
      "additionalProperties": {
        "type": "object",
        "required": ["type", "description"],
        "additionalProperties": false,
        "properties": {
          "type":        { "enum": ["string", "integer", "boolean", "enum", "secret"] },
          "description": { "type": "string" },
          "required":    { "type": "boolean", "default": false },
          "default":     { "description": "Default value (any JSON type matching `type`)" },
          "enum":        { "type": "array", "description": "Required when `type=\"enum\"`" },
          "pattern":     { "type": "string", "description": "Optional regex constraint for `type=\"string\"`" },
          "secret_env":  { "type": "string", "description": "Required when `type=\"secret\"` — the env-var name to populate" }
        }
      }
    },

    "compliance": {
      "type": "array",
      "items": { "type": "string", "pattern": "^[A-Z]+(?:-[A-Z0-9]+)*$" },
      "description": "Compliance regimes this template claims conformance with (HIPAA, GDPR, SOC2, FINRA, PCI-DSS, GoBD, ISO-27001, …)"
    },

    "safety_class": {
      "enum": ["t0_pure", "t1_filesystem", "t2_network", "t3_system"],
      "default": "t1_filesystem",
      "description": "Trust tier per arXiv:2602.12430 — t0 = no side effects, t3 = system-level operations"
    },

    "preview": {
      "type": "object",
      "required": ["command"],
      "additionalProperties": false,
      "properties": {
        "command":        { "type": "string", "description": "Shell command to launch local preview (e.g. `bun run dev`)" },
        "ready_url":      { "type": "string", "description": "URL the preview is reachable at when ready (default `http://localhost:5173`)" },
        "ready_timeout":  { "type": "integer", "default": 30000, "description": "Milliseconds to wait for ready_url before declaring failure" }
      }
    },

    "production": {
      "type": "object",
      "additionalProperties": false,
      "properties": {
        "build_command":  { "type": "string" },
        "output_path":    { "type": "string", "description": "Relative path to the build artefact directory" },
        "deploy_targets": { "type": "array", "items": { "enum": ["vercel", "netlify", "railway", "fly", "cloudflare-pages", "self-hosted"] } }
      }
    },

    "tests": {
      "type": "object",
      "additionalProperties": false,
      "properties": {
        "command":  { "type": "string", "description": "Shell command to run the conformance test suite" },
        "coverage_floor_pct": { "type": "integer", "minimum": 0, "maximum": 100 }
      }
    },

    "ai_metadata": {
      "type": "object",
      "additionalProperties": false,
      "properties": {
        "summary_for_llm":          { "type": "string", "maxLength": 1000, "description": "1-2 sentence pitch the AI uses when proposing this template to a user" },
        "trigger_keywords":         { "type": "array", "items": { "type": "string" }, "maxItems": 32 },
        "estimated_complexity":     { "enum": ["trivial", "small", "medium", "large", "xl"] },
        "estimated_minutes_to_run": { "type": "integer", "minimum": 1, "maximum": 600 }
      }
    }
  }
}
```

---

## Minimal example

```json
{
  "spec_version": "1",
  "id": "fintech-saas",
  "name": "Fintech SaaS — Multi-Tenant",
  "description": "Production-grade multi-tenant SaaS scaffold with Stripe Subscriptions, OAuth 2.1, RLS, and FINRA-aware audit logging.",
  "category": "saas",
  "industry": "fintech",
  "framework": "svelte-5",
  "language": "typescript",
  "license": "MIT",
  "version": "1.0.0",
  "compliance": ["FINRA", "SOC2", "GDPR"],
  "safety_class": "t1_filesystem",
  "preview": {
    "command": "bun run dev",
    "ready_url": "http://localhost:5173"
  }
}
```

---

## Full example (with parameters + production + tests + ai_metadata)

```json
{
  "spec_version": "1",
  "id": "fintech-saas",
  "name": "Fintech SaaS — Multi-Tenant",
  "description": "Production-grade multi-tenant SaaS scaffold for fintech.",
  "category": "saas",
  "industry": "fintech",
  "framework": "svelte-5",
  "language": "typescript",
  "license": "MIT",
  "version": "1.0.0",
  "tags": ["stripe", "oauth-2.1", "rls", "multi-tenant", "audit-log"],
  "homepage": "https://impforge.dev/templates/fintech-saas",
  "repository": "https://github.com/AiImpDevelopment/impforge-templates",
  "parameters": {
    "project_name": {
      "type": "string",
      "description": "Kebab-case project name",
      "required": true,
      "pattern": "^[a-z][a-z0-9-]{1,62}$"
    },
    "stripe_secret_key": {
      "type": "secret",
      "description": "Stripe API secret key (starts with sk_)",
      "required": true,
      "secret_env": "STRIPE_SECRET_KEY"
    },
    "tenancy_model": {
      "type": "enum",
      "description": "How tenants are isolated",
      "enum": ["shared_rls", "schema_per_tenant", "db_per_tenant"],
      "default": "shared_rls"
    }
  },
  "compliance": ["FINRA", "SOC2", "GDPR"],
  "safety_class": "t1_filesystem",
  "preview": {
    "command": "bun run dev",
    "ready_url": "http://localhost:5173",
    "ready_timeout": 30000
  },
  "production": {
    "build_command": "bun run build",
    "output_path": "build",
    "deploy_targets": ["vercel", "fly", "self-hosted"]
  },
  "tests": {
    "command": "bun test",
    "coverage_floor_pct": 80
  },
  "ai_metadata": {
    "summary_for_llm": "Multi-tenant fintech SaaS with Stripe Subscriptions, FINRA-aware audit log, OAuth 2.1 + Lucia auth, and PostgreSQL RLS. Pick this when the user wants a SaaS that handles money or financial data.",
    "trigger_keywords": ["fintech", "saas", "multi-tenant", "stripe", "subscriptions", "audit", "finra", "compliance"],
    "estimated_complexity": "large",
    "estimated_minutes_to_run": 8
  }
}
```

---

## Validation — what makes a template conformant

A `template.json` is **conformant with v1** when:

1. ✅ It validates against the JSON schema above (`spec_version === "1"`)
2. ✅ The `id` is unique within its target registry (e.g. `impforge-templates`)
3. ✅ Every parameter marked `required: true` is documented in the README
4. ✅ Every compliance regime claimed has a corresponding [compliance-rules.json](compliance-rules.json.v1.md) reference
5. ✅ Every secret parameter has a `secret_env` field (so AI tools know which env-var to populate)
6. ✅ The `preview.command` and `production.build_command` actually run on a fresh clone

The [conformance test suite](https://github.com/AiImpDevelopment/impforge-templates/tree/main/conformance) exercises 1-5 deterministically; 6 is exercised by CI.

---

## Why this spec is intentionally small

Every additional field is a fork-point for the ecosystem. v1 ships **only** the fields that every tool needs to discover, validate, customise, and run a template.

- No "author" or "team" fields — git history covers that
- No "rating" or "downloads" — registry-side concerns
- No "tags taxonomy" — `tags` is a free string array, registries can normalise
- No "i18n" — every spec field is English; descriptions can include markdown for nuance

Future minor versions (v1.1, v1.2) MAY add backwards-compatible fields. v2 SHALL be a separate spec file in this repo with a clear migration path.

---

## License

This specification: **MIT** — adopt it, extend it, redistribute it, no permission needed.

When you ship templates conforming to this spec, you may use the badge:

```markdown
[![template.json v1](https://img.shields.io/badge/template.json-v1-blueviolet)](https://github.com/AiImpDevelopment/impforge-mcp-manifests/blob/main/spec/template.json.v1.md)
```
