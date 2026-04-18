# Contributing to impforge-mcp-manifests

Specifications shape the ecosystem. Every contribution needs careful review because changes here ripple to every template + every skill + every AI tool that consumes them.

## What we accept

- **Backwards-compatible additions** to v1 specs (becomes v1.1, v1.2, ...)
- **Clarifications** in spec docs (typos, ambiguity, examples)
- **JSON Schema improvements** (tighter constraints, better error messages)
- **Conformance test additions**
- **Adopter additions** (your tool ships v1 conformance — open a PR)

## What we don't accept (yet)

- Breaking changes to v1 (those ship as v2 — open a discussion first)
- Schema changes that would invalidate existing templates
- Maximalist additions (keep specs minimum-viable)

## Submission process

1. **Open a discussion first** for any non-trivial change
2. **Fork + branch** — `spec/v1.x-<topic>`
3. **Update both** the markdown spec AND the JSON Schema in `schemas/`
4. **Add at least one example** showing the new behaviour
5. **Open PR** — link to discussion, summarise impact

## License

By contributing, you agree your contributions are licensed under MIT (see [LICENSE](LICENSE)).
