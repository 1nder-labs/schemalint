# additional-properties-object

> Category: **Semantic** — schema semantics trigger this rule

## Error Codes

| Profile | Code |
|---------|------|
| openai.so.2026-04-30 | `OAI-S-additional-properties-object` |
| anthropic.so.2026-04-30 | `ANT-S-additional-properties-object` |

## Description

additionalProperties must be set to false, not an object schema

## Rationale

LLM structured-output providers require additionalProperties: false to guarantee schema compliance. An object value indicates intent to define allowed extras, which most providers do not support.

## Bad Example

```json
{
  "type": "object",
  "additionalProperties": { "type": "string" },
  "properties": {}
}
```

## Good Example

```json
{
  "type": "object",
  "additionalProperties": false,
  "properties": {}
}
```
