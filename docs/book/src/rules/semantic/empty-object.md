# empty-object

> Category: **Semantic** — schema semantics trigger this rule

## Error Codes

| Profile | Code |
|---------|------|
| openai.so.2026-04-30 | `OAI-S-empty-object` |
| anthropic.so.2026-04-30 | `ANT-S-empty-object` |

## Description

Object schema with additionalProperties: false but no properties

## Rationale

Some providers may reject or misbehave when a schema permits no properties while also forbidding all extras via additionalProperties: false. This pattern is semantically valid but rarely intentional.

## Bad Example

```json
{
  "type": "object",
  "additionalProperties": false,
  "properties": {}
}
```

## Good Example

```json
{
  "type": "object",
  "additionalProperties": false,
  "properties": {
    "name": { "type": "string" }
  }
}
```
