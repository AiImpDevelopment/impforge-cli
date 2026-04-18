# skill.md — v1 specification

**Status:** Pre-launch (v1.0.0-rc.1)
**License:** MIT
**Companion specs:** [template.json v1](template.json.v1.md) · [compliance-rules.json v1](compliance-rules.json.v1.md)

---

## Summary

`SKILL.md` is the manifest format that lets any AI agent (Claude Code, Cursor, Codex, Gemini CLI, JetBrains AI, Copilot, Windsurf, custom) discover, validate, and invoke a reusable skill — all without bespoke per-tool integration.

It builds on the **3-tier skill hierarchy** (Atomic → Functional → Strategic) from arXiv:2604.04804 and the **trust-tier system** from arXiv:2602.12430.

---

## File location

A conforming skill is a directory containing exactly one `SKILL.md` at the root. Everything else is skill content (source code, tests, docs).

```
skills/template_search/
├── SKILL.md          ← the manifest (this spec)
├── README.md         ← optional human-readable description
├── src/              ← skill implementation
└── tests/            ← test cases
```

## Structure

`SKILL.md` is a markdown file with required headings. Each heading becomes a queryable field for the agent's skill router.

### Required headings

```markdown
# <skill_name> — short tagline

**Trust tier:** T0 | T1 | T2 | T3
**License:** MIT | Apache-2.0 | ...

## Purpose
One-paragraph explanation of what the skill does.

## When the AI agent should pick this
Trigger keywords + scenarios (free-form, the router uses this for retrieval).

## Input schema
JSON Schema describing the skill's input.

## Output schema
JSON Schema describing the skill's output.
```

### Optional headings

- `## Examples` — usage examples
- `## Tests` — test coverage notes
- `## Reference implementation` — pseudo-code or canonical impl
- `## Trust tier rationale` — why this tier and not another

---

## Trust tiers (arXiv:2602.12430)

| Tier | Name | What's allowed |
|------|------|----------------|
| **T0** | Sandbox-only | Pure computation. No side effects. No filesystem, no network, no env-var access. |
| **T1** | Filesystem | Read + write under explicitly-listed paths. No network, no system calls. |
| **T2** | Network | T1 + outbound HTTP(S) to explicitly-listed hosts. No inbound listening. |
| **T3** | System | T2 + spawn subprocesses, modify env, install packages. **Requires explicit user approval per invocation.** |

The router MUST refuse to invoke a T3 skill without user approval, regardless of context.

---

## 3-tier skill hierarchy (arXiv:2604.04804)

Skills compose:

| Hierarchy tier | Example |
|---|---|
| **Atomic** | `run_cargo_check` — one shell call |
| **Functional** | `build_and_test_rust_module` — composes 3-5 Atomic skills |
| **Strategic** | `deploy_new_feature_end_to_end` — composes 5-15 Functional skills |

The hierarchy is implicit — a skill declares its tier informally in the `## Purpose` paragraph or `## When the AI agent should pick this` section. The router uses dependency analysis (what other skills does this skill call?) to confirm.

---

## Discovery via MCP

A skill registry exposes its catalog via the [Model Context Protocol](https://modelcontextprotocol.io). The MCP server announces:

- `list_skills` — enumerate all skills in the catalog
- `describe_skill(id)` — return the full SKILL.md content
- `invoke_skill(id, input)` — execute the skill (T0/T1 only — T2/T3 require user approval flow)

See [`mcp-server/`](../mcp-server/) for the reference MCP server config.

---

## Validation — what makes a skill conformant

A `SKILL.md` is **conformant with v1** when:

1. ✅ The required headings are present in the order shown above
2. ✅ The trust tier line is `**Trust tier:** T<0|1|2|3>` exactly
3. ✅ The license line is `**License:** <SPDX-id>`
4. ✅ Input/output schemas are valid JSON Schema draft-2020-12
5. ✅ For T2 skills: trust tier rationale section is present
6. ✅ For T3 skills: trust tier rationale section is present + at least one explicit safety guard documented

The conformance test suite ([planned](https://github.com/AiImpDevelopment/impforge-skills/tree/main/conformance)) exercises 1-6 deterministically.

---

## Why this spec is intentionally minimal

Every additional required field is a barrier to adoption. v1 ships only what's needed for an AI agent to:

1. Discover the skill (router lookup via `## When the AI agent should pick this`)
2. Validate the inputs (router checks against `## Input schema`)
3. Invoke the skill safely (router enforces trust tier)
4. Use the output (router parses against `## Output schema`)

Future minor versions MAY add backwards-compatible optional headings. v2 SHALL be a separate spec file with a clear migration path.

---

## License

This specification: **MIT** — adopt it, extend it, redistribute it, no permission needed.

Adopter badge:

```markdown
[![skill.md v1](https://img.shields.io/badge/skill.md-v1-blueviolet)](https://github.com/AiImpDevelopment/impforge-mcp-manifests/blob/main/spec/skill.md.v1.md)
```
