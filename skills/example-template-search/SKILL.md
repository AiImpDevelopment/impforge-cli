# template_search — example skill

**Trust tier:** T0 (sandbox-only, no side effects)
**Spec:** [`skill.md` v1](https://github.com/AiImpDevelopment/impforge-mcp-manifests/blob/main/spec/skill.md.v1.md)
**License:** MIT

## Purpose

Search the [`impforge/templates`](https://github.com/AiImpDevelopment/impforge-templates) registry by industry, category, framework, or free-text query. Returns ranked matches with their `template.json` manifests.

## When the AI agent should pick this

Trigger keywords: `find template`, `search scaffold`, `which template for`, `is there a template`, `template library`

Use when the user wants to discover an existing scaffold rather than generate one from scratch.

## Input schema

```json
{
  "query": { "type": "string", "description": "free-text search" },
  "industry": { "type": "string", "description": "optional industry filter (fintech / healthcare / ...)" },
  "category": { "type": "string", "description": "optional category filter (web / saas / backend / ...)" },
  "limit": { "type": "integer", "default": 10, "minimum": 1, "maximum": 50 }
}
```

## Output schema

```json
{
  "matches": [
    {
      "id": "string",
      "name": "string",
      "description": "string",
      "score": "number (0-1)",
      "manifest": "<full template.json content>"
    }
  ]
}
```

## Reference implementation

This skill is a reference implementation only. The actual searchable index lives in [`impforge-templates`](https://github.com/AiImpDevelopment/impforge-templates).

```python
# pseudo-code for the canonical implementation
def template_search(query, industry=None, category=None, limit=10):
    manifests = fetch_all_manifests("impforge-templates")
    candidates = filter(manifests, industry=industry, category=category)
    ranked = bm25_rank(candidates, query)
    return [{"manifest": m, "score": s} for m, s in ranked[:limit]]
```

## Tests (planned)

- ✅ Empty query returns top-N templates
- ✅ Industry filter narrows results correctly
- ✅ Category filter narrows results correctly
- ✅ Combined filter works
- ✅ Score is in [0, 1] range
- ✅ Limit cap is enforced

## License

MIT — copy this skill, modify it, redistribute it, no permission needed.
