# additional-properties-false

> Category: **Structural** — overall schema structure triggers this rule

## Error Codes

| Profile | Code |
|---------|------|
| openai.so.2026-04-30 | `OAI-S-additional-properties-false` |
| anthropic.so.2026-04-30 | `ANT-S-additional-properties-false` |

## Description

Every object schema must declare additionalProperties: false

## Rationale

Providers require all object nodes to explicitly set additionalProperties: false to guarantee no unexpected properties appear in responses.

## Bad Example

```json
{
  "type": "object",
  "properties": {
    "name": { "type": "string" }
  }
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
