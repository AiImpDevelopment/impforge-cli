# Contributing to impforge-skills

Skills are how AI agents become composable. The smaller and more focused, the more valuable.

## What we accept

- **Single-purpose skills** that conform to [`skill.md` v1](https://github.com/AiImpDevelopment/impforge-mcp-manifests/blob/main/spec/skill.md.v1.md)
- **Test additions** to existing skills
- **Documentation improvements**

## What we don't accept

- Skills without a `SKILL.md` manifest
- Multi-purpose "kitchen-sink" skills (split them up)
- Skills with non-MIT-compatible dependencies

## Submission process

1. **Open an issue first** — describe the skill purpose + use case
2. **Fork + branch** — `feat/skill-<name>`
3. **Add your skill** under `skills/<name>/`
4. **Add tests** — every skill needs at least 3 test cases
5. **Open PR** — link to your issue

## Bounty program

EUR 50-200 per accepted skill that ships with comprehensive tests.

## Code of Conduct

[Contributor Covenant 2.1](https://www.contributor-covenant.org/version/2/1/code_of_conduct/). Be excellent.

## License

By contributing, you agree your contributions are licensed under MIT (see [LICENSE](LICENSE)).
