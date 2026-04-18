# Contributing to impforge-templates

Thank you for your interest in contributing! Templates are how `impforge/` becomes the standard library every AI coding tool reaches for first.

## What we accept

- **Industry × category templates** that conform to [`template.json` v1](https://github.com/AiImpDevelopment/impforge-mcp-manifests/blob/main/spec/template.json.v1.md)
- **Compliance rule additions** (HIPAA / FINRA / GDPR / PCI / SOC2 / GoBD)
- **Bug fixes** in existing templates
- **Documentation improvements**
- **Test coverage improvements**

## What we don't accept (yet)

- Templates without `template.json` manifests
- Templates with non-MIT-compatible dependencies
- Templates that depend on the proprietary ImpForge engine

## Submission process

1. **Open an issue first** — describe the industry × category combination + your use case.  This avoids duplicate work.
2. **Fork + branch** — `feat/industry-category-template`
3. **Add your template** under `templates/<industry>-<category>/`
4. **Validate** — run `npm run validate templates/<industry>-<category>` (conformance test suite, lands W16-launch)
5. **Open PR** — link to your issue, describe what's novel

## Bounty program

We pay **EUR 100-500 per accepted template** that meets the conformance test suite + has been used by ≥ 50 unique installs in the first 60 days.

Bounty tiers:
- **EUR 100** — accepted template, basic conformance
- **EUR 250** — accepted template + comprehensive tests + documentation
- **EUR 500** — accepted template + comprehensive tests + documentation + ≥ 50 unique installs in 60 days

Open a separate `bounty/` issue with your accepted template ID + payment preferences (PayPal / SEPA / Stripe Connect) once the install milestone hits.

## Code of Conduct

Be excellent to each other. We follow the [Contributor Covenant 2.1](https://www.contributor-covenant.org/version/2/1/code_of_conduct/). Harassment, discrimination, doxxing, and bad-faith engagement are grounds for immediate ban.

## Maintainers

- **Sven Cramer** ([@GobbinGamz](https://github.com/GobbinGamz)) — AiImp Technology, Germany

## License

By contributing, you agree your contributions are licensed under MIT (see [LICENSE](LICENSE)).
