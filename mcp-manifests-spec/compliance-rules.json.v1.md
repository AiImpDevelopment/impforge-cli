# compliance-rules.json — v1 specification

**Status:** Pre-launch (v1.0.0-rc.1)
**License:** MIT
**Companion specs:** [template.json v1](template.json.v1.md) · [skill.md v1](skill.md.v1.md)

---

## Summary

`compliance-rules.json` lets a template declare which compliance regimes it conforms with — and lets an auditor / AI tool / CI pipeline machine-check that conformance.

The spec answers:

1. **Which compliance regime?** (HIPAA / GDPR / SOC2 / FINRA / PCI-DSS / GoBD / ISO-27001)
2. **Which specific rules of that regime?** (rule IDs, e.g. `HIPAA-164.312(a)(1)`)
3. **What pattern in the code satisfies the rule?** (matchers, file paths, test references)
4. **What's the audit evidence?** (test command, expected output, audit log location)

---

## Schema (JSON Schema draft-2020-12)

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://impforge.dev/spec/compliance-rules.v1.json",
  "type": "object",
  "required": ["spec_version", "regime", "rules"],
  "properties": {
    "spec_version": { "const": "1" },
    "regime":       { "enum": ["HIPAA", "GDPR", "SOC2", "FINRA", "PCI-DSS", "GoBD", "ISO-27001", "CCPA", "NIST-800-53"] },
    "regime_version": { "type": "string", "examples": ["2024", "2026-Q1"] },
    "rules": {
      "type": "array",
      "items": {
        "type": "object",
        "required": ["id", "title", "description", "satisfied_by"],
        "properties": {
          "id":          { "type": "string", "description": "Stable rule ID, e.g. HIPAA-164.312(a)(1)" },
          "title":       { "type": "string" },
          "description": { "type": "string" },
          "severity":    { "enum": ["informational", "low", "medium", "high", "critical"], "default": "medium" },
          "satisfied_by": {
            "type": "object",
            "required": ["pattern_kind"],
            "properties": {
              "pattern_kind": { "enum": ["file_exists", "code_grep", "test_passes", "schema_constraint", "manual_attestation"] },
              "value":        { "type": "string" },
              "test_command": { "type": "string", "description": "Required when pattern_kind is test_passes" },
              "expected_exit_code": { "type": "integer", "default": 0 }
            }
          },
          "evidence_path": { "type": "string", "description": "Optional: where the audit log evidence is written" }
        }
      }
    }
  }
}
```

---

## Example — minimal HIPAA conformance

```json
{
  "spec_version": "1",
  "regime": "HIPAA",
  "regime_version": "2024",
  "rules": [
    {
      "id": "HIPAA-164.312(a)(1)",
      "title": "Access control — unique user identification",
      "description": "Each user gets a unique identifier; no shared accounts.",
      "severity": "high",
      "satisfied_by": {
        "pattern_kind": "schema_constraint",
        "value": "users.id is UUID PRIMARY KEY UNIQUE NOT NULL"
      }
    },
    {
      "id": "HIPAA-164.312(a)(2)(iv)",
      "title": "Encryption + decryption",
      "description": "PHI at rest must be encrypted (AES-256-GCM minimum).",
      "severity": "critical",
      "satisfied_by": {
        "pattern_kind": "test_passes",
        "test_command": "bun test tests/encryption.test.ts",
        "expected_exit_code": 0
      },
      "evidence_path": "audit/encryption_evidence.log"
    }
  ]
}
```

---

## How templates reference compliance rules

A `template.json` v1 manifest's `compliance` array references compliance regimes by short name:

```json
{
  "compliance": ["HIPAA", "GDPR"]
}
```

For each regime listed, the template MUST ship a `compliance/<regime>.json` file conforming to this v1 spec.

The CI / AI auditor can then:

1. Read `template.json` → sees `compliance: ["HIPAA"]`
2. Read `compliance/HIPAA.json` → enumerate the rules
3. For each rule, run the `satisfied_by` matcher
4. Aggregate results into a pass/fail audit report

---

## Why machine-readable compliance matters

Today, "HIPAA-ready" is a marketing claim. Tomorrow, with this spec, "HIPAA-ready" is a deterministic, machine-checkable property — and the audit report is a `compliance-rules.json` walk away.

ImpForge templates ship with conformance to declared regimes pre-tested. Other tools that adopt this spec inherit the same predictability.

---

## License

This specification: **MIT** — adopt it, extend it, redistribute it, no permission needed.
