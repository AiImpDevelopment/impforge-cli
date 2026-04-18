<p align="center">
  <img src="https://raw.githubusercontent.com/AiImpDevelopment/impforge/main/assets/logo-256.png" width="128" height="128" alt="ImpForge Templates" />
</p>

<h1 align="center">impforge / templates</h1>

<h3 align="center">78 production-grade industry × category app templates · MIT licensed · works with every AI tool</h3>

<p align="center">
  <em>Battle-tested scaffolds for fintech, legal, healthcare, retail and 22 more industries —<br/>
  each shipped with compliance rules, multi-tenant guardrails, and a working preview.</em>
</p>

<p align="center">
  <a href="https://github.com/AiImpDevelopment/impforge-templates/stargazers">
    <img src="https://img.shields.io/github/stars/AiImpDevelopment/impforge-templates?style=social" alt="Stars" />
  </a>
  <img src="https://img.shields.io/badge/license-MIT-green" alt="MIT" />
  <img src="https://img.shields.io/badge/templates-78%20%2F%2078%20shipped-brightgreen" alt="78 of 78 templates shipped" />
  <img src="https://img.shields.io/badge/compliance--rules-2600-brightgreen" alt="2600 compliance rules" />
  <img src="https://img.shields.io/badge/spec-template.json%20v1-blueviolet" alt="template.json v1" />
  <img src="https://img.shields.io/badge/MCP-ready-orange" alt="MCP ready" />
  <img src="https://img.shields.io/badge/works%20with-Cursor%20%C2%B7%20Bolt%20%C2%B7%20Lovable%20%C2%B7%20Claude%20Code-black" alt="Works with everything" />
  <img src="https://img.shields.io/badge/Made%20in-Germany%20%F0%9F%87%A9%F0%9F%87%AA-black" alt="Made in Germany" />
</p>

---

## What this is

**The first complete, MIT-licensed template gallery for every AI coding tool.**

When ChatGPT, Cursor, Bolt, Lovable, Claude Code, Windsurf or JetBrains AI generates a fintech SaaS app, what pattern should it use? Today: whatever its training data scraped from random GitHub repos. Tomorrow: a curated `impforge/templates` reference shipped under MIT — auditable, compliance-checked, and adoptable in one line.

This repo is the canonical source for **78 production-grade scaffolds** spanning 26 industries × 3 categories (Web · SaaS · Backend). Every template carries:

- ✅ A working starter — clone, install, run, see it
- ✅ Industry-specific compliance rules (HIPAA / FINRA / GDPR / PCI / GoBD where applicable)
- ✅ Multi-tenant isolation patterns audited by [tenant-guard][tg] semantics
- ✅ A `template.json` manifest conforming to the [v1 spec][spec]
- ✅ MIT license — fork it, ship it, sell it, no permission needed

[tg]: https://github.com/AiImpDevelopment/impforge
[spec]: https://github.com/AiImpDevelopment/impforge-mcp-manifests

## Status — All 5 Waves SHIPPED · 100 % complete (78 / 78)

**78 of 78 templates landed 2026-04-18** — 26 industries × 3 categories (web · saas · backend). Every template carries 100 industry-specific compliance rules and a `template.json` v1 manifest, totaling **2 600 production-grade rules** across five chunks.

| Wave | Industries | Status |
|---|---|---|
| **W16-1** | fintech · healthcare · legal · professional · retail | ✅ **Shipped 2026-04-19** (15 templates · 500 rules) |
| **W16-2** | education · insurance · construction · manufacturing · media | ✅ **Shipped 2026-04-19** (15 templates · 500 rules) |
| **W16-3** | agriculture · food · hospitality · logistics · nonprofit | ✅ **Shipped 2026-04-19** (15 templates · 500 rules) |
| **W16-4** | automotive · cybersecurity · aerospace · energy · engines | ✅ **Shipped 2026-04-18** (15 templates · 500 rules) |
| **W16-5** | telecom · pharma · mining · maritime · government · proptech | ✅ **Shipped 2026-04-18** (18 templates · 600 rules) |

### What landed in W16 Chunk 1 (5 industries)

| Industry | Web framework | SaaS auth | Backend DB | Compliance regimes |
|---|---|---|---|---|
| **fintech** | Next 15 + shadcn/ui v2 | OAuth 2.1 + PKCE / Stripe | Postgres 17 + read-replica | BSA · PCI-DSS · GDPR · SOC2 · ISO-27001 · FINRA · MiFID-II · SOX · PSD2 · GLBA · Basel-III · Reg-Z · Reg-E · CCPA · ... |
| **healthcare** | Svelte 5 + bits-ui | OAuth 2.1 + PKCE / Stripe (schema-per-tenant) | Postgres 17 + PHI audit log | HIPAA · HITRUST · GDPR · FDA-21-CFR-Part-11 · ISO-13485 · DEA · ONC · MDR · IVDR · GxP · ... |
| **legal** | Vue 3.5 + ark-ui | OAuth 2.1 + PKCE / Stripe | Postgres 17 + matter / conflict-check | ABA-MRPC · GDPR · ISO-27001 · SOC2 · FRCP · FOIA · GoBD · BDSG · BRAO · FATF · ... |
| **professional** | Astro 5 + shadcn/ui | OAuth 2.1 + PKCE / Stripe | Postgres 17 + project / time-entry / invoice | GDPR · SOC2 · ISO-27001 · ePrivacy · GOBD · CCPA · CPRA · AICPA · PCAOB · SOX · ... |
| **retail** | Remix 3 + shadcn/ui | OAuth 2.1 + PKCE / Stripe | Postgres 17 + product / inventory / order-line | PCI-DSS · GDPR · CCPA · CPRA · SOC2 · GOBD · WCAG · TCPA · CAN-SPAM · DSA · DMA · ... |

### What landed in W16 Chunk 2 (5 industries)

| Industry | Web framework | SaaS auth | Backend DB | Compliance regimes |
|---|---|---|---|---|
| **education** | Svelte 5 + bits-ui | OAuth 2.1 + PKCE / Stripe (schema-per-tenant) | Postgres 17 + enrollment / grade / transcript | FERPA · COPPA · GDPR · ADA · IDEA · Title-IX · Clery-Act · HEA · GLBA · ESEA · WCAG · BIPA · ... |
| **insurance** | Next 15 + shadcn/ui v2 | OAuth 2.1 + PKCE / Stripe | Postgres 17 + policy / claim / reserve | NAIC · GLBA · HIPAA · IFRS-17 · SOLVENCY-II · ERISA · ACA · ESG · BIPA · ADA · FATCA · CRS · ... |
| **construction** | Astro 5 + shadcn/ui | OAuth 2.1 + PKCE / Stripe | Postgres 17 + project / bid / change-order | OSHA · ISO-45001 · EPA · ADA · ABA · BIM-ISO-19650 · Davis-Bacon · BABA · FAR · ICC · NFPA · NEC · ASCE-7 · ... |
| **manufacturing** | Remix 3 + shadcn/ui | OAuth 2.1 + PKCE / Stripe | Postgres 17 + work-order / BOM / inspection | ISO-9001 · ISO-14001 · ISO-45001 · ITAR · EAR · REACH · RoHS · WEEE · TSCA · DFARS · CMMC · NIST-800-171 · IEC-62443 · UFLPA · ... |
| **media** | Next 15 + shadcn/ui v2 | OAuth 2.1 + PKCE / Stripe | Postgres 17 + content-asset / ad-placement / royalty | DMCA · GDPR · DSA · DMA · COPPA · CCPA · ePrivacy · Section-230 · FCC · Audio-License · BIPA · VPPA · CSAM · FOSTA-SESTA · ... |

### What landed in W16 Chunk 5 (6 industries)

| Industry | Web framework | SaaS auth | Backend DB | Compliance regimes |
|---|---|---|---|---|
| **telecom** | Svelte 5 + bits-ui | OAuth 2.1 + PKCE / Stripe | Postgres 17 + subscriber / cdr / rating | FCC · CALEA · CPNI · TCPA · STIR-SHAKEN · 3GPP · Brazil-LGPD · ETSI · GDPR · SOC2 · ISO-27001 · ... |
| **pharma** | Next 15 + shadcn/ui v2 | WorkOS SSO / Stripe (schema-per-tenant) | Postgres 17 + study / subject / adverse-event | FDA-21-CFR-Part-11 · FDA-cGMP · ICH · EMA · GAMP-5 · DSCSA · GxP · HIPAA · PhRMA-Code · Sunshine-Act · ... |
| **mining** | Vue 3.5 + ark-ui | OAuth 2.1 + PKCE / Stripe | Postgres 17 + site / shift / tailings_dam / incident | MSHA · ICMM · GISTM · SMCRA · EITI · S-K-1300 · NI-43-101 · JORC-Code · IRMA · OECD-Due-Diligence · ... |
| **maritime** | Astro 5 + shadcn/ui | OAuth 2.1 + PKCE / Stripe | Postgres 17 + vessel / voyage / port-call / cargo | IMO-SOLAS · MARPOL · MLC-2006 · STCW · ISM-Code · ISPS-Code · USCG · BIMCO-Cyber · EU-ETS-Maritime · FuelEU-Maritime · ... |
| **government** | Remix 3 + headless-ui | WorkOS SSO + PIV / no billing (db-per-tenant) | Postgres 17 + program / grant / case / public_record | FOIA · FISMA · FedRAMP · NIST-800-171 · NIST-800-53 · FAR · DFARS · CMMC · Section-508 · 21st-Century-IDEA · Privacy-Act · ... |
| **proptech** | Svelte 5 + bits-ui | OAuth 2.1 + PKCE / Stripe | Postgres 17 + portfolio / unit / lease / rent_ledger | Fair-Housing · FCRA · ECOA · TILA · RESPA · REIT-Tax · ADA-Title-III · 1031-Exchange · GRESB · NAR-COE · ... |

⭐ **Star this repo** — all 78 templates now live, plus the upcoming Quarantine Pro Mesh (signed-snapshot delivery, SLSA L3 provenance, OWASP LLM01 sanitizer).

## How AI tools consume these templates

```bash
# Cursor / Claude Code / Windsurf — register the MCP server once
claude mcp add impforge-templates -- npx -y @impforge/mcp-templates

# Then ask in plain English
"Scaffold me a fintech SaaS using the ImpForge fintech-saas template"
```

The MCP server announces every available template via the [`template.json` v1 spec][spec]. Your AI tool of choice picks the right one, customises it for your prompt, and you're running.

**No vendor lock-in.** The templates are MIT. The MCP manifest is MIT. Use them with any tool, any way you like.

## Why this exists

Bolt, Lovable, v0 and Cursor all generate SaaS apps. None of them ship a curated, auditable template gallery you can read before you trust it. Every output is a fresh roll of the dice — works most of the time, but you have no idea which security pattern it picked or whether the multi-tenancy is sound.

ImpForge changes that. Our [main product][main] (source-available, EUR 25/mo) generates apps using these exact templates with a proprietary engine that handles preview windows, ATF trust graduation, live tenant-guard auditing, and Pop-out Tauri windows. **The templates themselves are open. The engine that makes them feel magical stays ours.**

This repo is our gift to the wider AI coding ecosystem — and our way of becoming the standard everyone defaults to.

[main]: https://github.com/AiImpDevelopment/impforge

## Roadmap (executable)

1. **Now** — namespace + spec locked, MCP-server stub published
2. **W16** — first 15 templates land (fintech / professional / retail / legal / healthcare)
3. **W17-W20** — remaining 63 templates ship in coordinated waves
4. **Launch day** — coordinated drop on Hacker News + Reddit + Dev.to + Twitter
5. **Post-launch** — community bounty program (EUR 100-500 per accepted template)
6. **Q3 2026** — premium template marketplace (EUR 5-15 advanced packs) on top of MIT base

## Contributing

Once templates start landing (W16+), we'll open issues for community-contributed industries. **Bounty: EUR 100-500 per accepted template** that meets the conformance test suite.

The `template.json` v1 spec lives in [`impforge-mcp-manifests`][spec] — start there if you want to build a conformant template ahead of launch.

## License

All templates: **MIT** — fork it, ship it, sell it, no permission needed.

The proprietary [ImpForge engine][main] that generates / customises / runs these templates is licensed ELv2 + BUSL (source-available, anti-fork). Templates work standalone with any tool; the engine adds preview windows, ATF trust graduation, multi-tenant guard, and personal AI training.

## Made in Germany 🇩🇪

ImpForge is built by AiImp Technology in Germany. We believe AI tooling should respect users — privacy-first, no surveillance, no data harvesting, no dark patterns.

> *"The best AI tooling is the one you can read before you run."*

---

<p align="center">
  <a href="https://github.com/AiImpDevelopment/impforge">ImpForge (main product)</a> &bull;
  <a href="https://github.com/AiImpDevelopment/impforge-skills">impforge-skills</a> &bull;
  <a href="https://github.com/AiImpDevelopment/impforge-mcp-manifests">impforge-mcp-manifests</a>
</p>
