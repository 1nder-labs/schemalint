# anyof-objects

> Category: **Semantic** — schema semantics trigger this rule

## Error Codes

| Profile | Code |
|---------|------|
| openai.so.2026-04-30 | `OAI-S-anyof-objects` |
| anthropic.so.2026-04-30 | `ANT-S-anyof-objects` |

## Description

anyOf with only object-typed branches may not be fully supported

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
