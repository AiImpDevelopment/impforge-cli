<p align="center">
  <img src="https://raw.githubusercontent.com/AiImpDevelopment/impforge/main/assets/logo-256.png" width="128" height="128" alt="ImpForge MCP Manifests" />
</p>

<h1 align="center">impforge / mcp-manifests</h1>

<h3 align="center">The canonical specs that make ImpForge templates and skills work everywhere</h3>

<p align="center">
  <em>template.json v1 · skill.md v1 · compliance-rules.json v1<br/>
  All MIT licensed. All MCP-native. All adoption-ready.</em>
</p>

<p align="center">
  <img src="https://img.shields.io/badge/license-MIT-green" alt="MIT" />
  <img src="https://img.shields.io/badge/template.json-v1-blueviolet" alt="template.json v1" />
  <img src="https://img.shields.io/badge/skill.md-v1-blueviolet" alt="skill.md v1" />
  <img src="https://img.shields.io/badge/compliance--rules.json-v1-blueviolet" alt="compliance-rules v1" />
  <img src="https://img.shields.io/badge/MCP-JSON--RPC%202.0-orange" alt="MCP" />
  <img src="https://img.shields.io/badge/Made%20in-Germany%20%F0%9F%87%A9%F0%9F%87%AA-black" alt="Made in Germany" />
</p>

---

## What this is

**The shared vocabulary that lets every AI tool consume ImpForge templates and skills consistently.**

Three small, focused specifications:

- [`template.json` v1](spec/template.json.v1.md) — manifests every scaffold in [`impforge-templates`][tpl] follows
- [`skill.md` v1](spec/skill.md.v1.md) — the SKILL.md convention (arXiv:2602.12430) every skill in [`impforge-skills`][skl] follows
- [`compliance-rules.json` v1](spec/compliance-rules.json.v1.md) — industry-specific compliance rule schema (HIPAA / FINRA / GDPR / PCI / GoBD)

[tpl]: https://github.com/AiImpDevelopment/impforge-templates
[skl]: https://github.com/AiImpDevelopment/impforge-skills

Plus the **MCP server config** for the `impforge-templates` and `impforge-skills` namespaces, so any AI tool (Claude Code, Cursor, Codex, Gemini CLI, JetBrains AI, Copilot, Windsurf) can register either with one line.

## Status — pre-launch

**Specs are stabilising in lockstep with [`impforge-templates`][tpl] and [`impforge-skills`][skl]. v1 freeze targeted for W16-W20 launch.**

⭐ **Star this repo** if you're building tooling that should consume ImpForge templates / skills natively.

## Why this exists

When ChatGPT, Cursor, Bolt, Lovable, Claude Code, Windsurf or JetBrains AI scaffolds a SaaS app, every tool reaches for a slightly different convention. The result: 14 partially-overlapping "best practices" that drift apart over time, reproducibility nightmares, no shared compliance vocabulary.

We're fixing that with three short, intentionally-minimal specs. Each spec answers one question:

- **template.json v1** — *"What does a scaffold look like that any AI tool can consume?"*
- **skill.md v1** — *"What does a reusable skill look like that any agent can pick up?"*
- **compliance-rules.json v1** — *"How do we describe industry compliance in a machine-checkable form?"*

These are not maximalist standards. They are minimum-viable contracts so the ecosystem can compose. Adopt them in your tool, adopt them in your template, adopt them in your skill — everything snaps together.

## Want a "Powered by ImpForge templates" badge?

Once your tool ships conformance with `template.json` v1 + `skill.md` v1, you're eligible for the badge. Open an issue with your conformance test results — we'll add you to the [adopters][ad] page.

[ad]: docs/adopters.md

## Adopters (target list — pre-launch)

We are building this with the adoption of these tools in mind. If you maintain one of these and want early input, open an issue:

- Claude Code (Anthropic) — Skill / MCP first-class citizen
- Cursor (Anysphere)
- Codex CLI (OpenAI)
- Gemini CLI (Google)
- JetBrains AI Assistant
- GitHub Copilot
- Windsurf (Codeium)
- Bolt.new (StackBlitz)
- Lovable.dev
- v0 (Vercel)
- Replit Agent
- Continue.dev
- Tabby

## License

All specifications: **MIT** — adopt them, extend them, redistribute them, no permission needed.

## Made in Germany 🇩🇪

> *"Standards become standards because the first mover shipped them, not because they were perfect."*

---

<p align="center">
  <a href="https://github.com/AiImpDevelopment/impforge">ImpForge (main product)</a> &bull;
  <a href="https://github.com/AiImpDevelopment/impforge-templates">impforge-templates</a> &bull;
  <a href="https://github.com/AiImpDevelopment/impforge-skills">impforge-skills</a>
</p>
