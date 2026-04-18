<p align="center">
  <img src="https://raw.githubusercontent.com/AiImpDevelopment/impforge/main/assets/logo-256.png" width="128" height="128" alt="ImpForge Skills" />
</p>

<h1 align="center">impforge / skills</h1>

<h3 align="center">Reusable AI agent skills · MIT licensed · drop-in for Claude Code · Cursor · Codex · Gemini CLI</h3>

<p align="center">
  <em>Small, sharp, single-purpose helpers that any AI agent can pick up<br/>
  in one line — search, validate, compose, audit, scaffold.</em>
</p>

<p align="center">
  <a href="https://github.com/AiImpDevelopment/impforge-skills/stargazers">
    <img src="https://img.shields.io/github/stars/AiImpDevelopment/impforge-skills?style=social" alt="Stars" />
  </a>
  <img src="https://img.shields.io/badge/license-MIT-green" alt="MIT" />
  <img src="https://img.shields.io/badge/works%20with-Claude%20Code%20%C2%B7%20Cursor%20%C2%B7%20Codex%20%C2%B7%20Gemini%20CLI-black" alt="Multi-tool" />
  <img src="https://img.shields.io/badge/skill.md-spec%20conformant-blueviolet" alt="SKILL.md conformant" />
  <img src="https://img.shields.io/badge/Made%20in-Germany%20%F0%9F%87%A9%F0%9F%87%AA-black" alt="Made in Germany" />
</p>

---

## What this is

**A growing collection of MIT-licensed agent skills** built to the [SKILL.md specification][skillspec] (arXiv:2602.12430), shipped alongside the [ImpForge AI workstation][main].

Each skill is:
- **Single-purpose** — one job, done well, no kitchen-sink
- **Tool-agnostic** — works with Claude Code, Cursor, Codex, Gemini CLI, JetBrains AI, Copilot, Windsurf, and any future AI tool that respects the SKILL.md convention
- **Trust-tiered** — explicit Tier 0 (sandbox-only) → Tier 3 (system-level) classification per skill
- **Co-evolving** — feedback loops improve each skill over time (EvoSkills pattern, arXiv:2604.01687)

[skillspec]: https://github.com/AiImpDevelopment/impforge-mcp-manifests
[main]: https://github.com/AiImpDevelopment/impforge

## Status — pre-launch

**Skills extract from the [main ImpForge repo][main] across W16-W20 alongside templates. This repo locks the namespace + the standard.**

⭐ **Star this repo** to be notified when skills land.

## Planned initial skills (drop-1)

| Skill | Tier | What it does |
|---|---|---|
| `template_search` | T0 | Semantic search over `impforge/templates` registry |
| `template_validate` | T0 | Validate any scaffold against the `template.json` v1 spec |
| `compliance_check` | T0 | Cross-check generated code against industry compliance rules |
| `tenant_audit` | T0 | Detect missing `tenant_id` filters in generated SaaS schemas |
| `preview_dispatch` | T1 | Build + serve a generated app for browser preview |
| `mcp_introspect` | T0 | List the tools any MCP server exposes |
| `template_compose` | T0 | Stitch multiple ImpForge templates into a single app |
| `skill_search` | T0 | Search this skill catalog (recursive) |

Each follows the [3-tier hierarchy][hier] — Atomic / Functional / Strategic — pioneered by SkillX (arXiv:2604.04804).

[hier]: https://github.com/AiImpDevelopment/impforge-mcp-manifests

## How agents discover these skills

```bash
# Claude Code — load the skill catalog once
claude mcp add impforge-skills -- npx -y @impforge/mcp-skills

# Cursor / Codex / Gemini — same pattern, MCP-first
```

Every skill ships with a SKILL.md describing trigger keywords, input schema, output schema, and trust tier. Your agent of choice loads the catalog, picks the relevant skill on demand, and runs it.

## Why this exists

The wider AI agent ecosystem has dozens of competing "skill" specifications — Claude Skills, OpenAI Functions, Anthropic Tools, Gemini Tools, every framework has its own. We're not adding to the noise; we're **shipping conformant skills under the open SKILL.md spec** so they work everywhere.

ImpForge's [main product][main] uses these exact skills internally with a proprietary skill router (Tantivy BM25 + embedding rerank, full-body indexing per arXiv:2603.22455). **The skills are open. The router stays ours.** Use the skills standalone or with our engine — your call.

## Contributing

Once initial skills land (W16+), we'll open issues for community-contributed skills. Bounty per accepted skill: EUR 50-200 depending on scope.

## License

All skills: **MIT** — adopt them, modify them, redistribute them, no permission needed.

## Made in Germany 🇩🇪

> *"Small skills compound into capabilities. We build the small ones in the open."*

---

<p align="center">
  <a href="https://github.com/AiImpDevelopment/impforge">ImpForge (main product)</a> &bull;
  <a href="https://github.com/AiImpDevelopment/impforge-templates">impforge-templates</a> &bull;
  <a href="https://github.com/AiImpDevelopment/impforge-mcp-manifests">impforge-mcp-manifests</a>
</p>
