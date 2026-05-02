# allof-with-ref

> Category: **Structural** — overall schema structure triggers this rule

## Error Codes

| Profile | Code |
|---------|------|
| anthropic.so.2026-04-30 | `ANT-S-allof-with-ref` |

## Description

allOf combined with $ref is not supported by Anthropic

## Rationale

Anthropic Structured Outputs does not support combining allOf with $ref references. Schemas using this pattern will be rejected by the Anthropic API.

## Bad Example

```json
{
  "type": "object",
  "allOf": [
    { "$ref": "#/$defs/Base" },
    { "properties": { "extra": { "type": "string" } } }
  ],
  "$defs": {
    "Base": {
      "type": "object",
      "properties": { "id": { "type": "string" } }
    }
  }
}
```

## Good Example

```json
{
  "type": "object",
  "properties": {
    "id": { "type": "string" },
    "extra": { "type": "string" }
  }
}
```
