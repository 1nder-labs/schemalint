# anyof-objects

> Category: **Semantic** — schema semantics trigger this rule

## Error Codes

| Profile | Code | Severity |
|---------|------|----------|
| openai.so.2026-04-30 | `OAI-S-anyof-objects` | Warn |
| anthropic.so.2026-04-30 | `ANT-S-anyof-objects` | Warn |

## Description

anyOf with only object-typed branches may not be fully supported

## Provider Evidence

| Profile | Status | Source | Basis |
|---|---|---|---|
| anthropic.so.2026-04-30 | `inferred` | [Anthropic JSON Schema limitations](https://platform.claude.com/docs/en/build-with-claude/structured-outputs#json-schema-limitations) | Each anyOf branch must remain within Anthropic's supported subset. |
| openai.so.2026-04-30 | `documented` | [OpenAI anyOf requirements](https://developers.openai.com/api/docs/guides/structured-outputs#for-anyof-the-nested-schemas-must-each-be-a-valid-json-schema-per-this-subset) | — |

## Rationale

When all anyOf branches are object-typed, some providers may not correctly resolve the union. Merging branches into a single object schema when appropriate improves compatibility across providers.

## Bad Example

```json
{
  "type": "object",
  "anyOf": [
    { "type": "object", "properties": { "x": { "type": "string" } } },
    { "type": "object", "properties": { "y": { "type": "number" } } }
  ]
}
```

## Good Example

```json
{
  "type": "object",
  "properties": {
    "x": { "type": "string" },
    "y": { "type": "number" }
  }
}
```
