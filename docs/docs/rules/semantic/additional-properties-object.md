# additional-properties-object

> Category: **Semantic** — schema semantics trigger this rule

## Error Codes

| Profile | Code | Severity |
|---------|------|----------|
| openai.so.2026-04-30 | `OAI-S-additional-properties-object` | Forbid |
| anthropic.so.2026-04-30 | `ANT-S-additional-properties-object` | Forbid |

## Description

additionalProperties must be set to false, not an object schema

## Provider Evidence

| Profile | Status | Source | Basis |
|---|---|---|---|
| anthropic.so.2026-04-30 | `sdk_transform` | [Anthropic SDK transformation](https://platform.claude.com/docs/en/build-with-claude/structured-outputs#how-sdk-transformation-works) | Official SDK transformation sets additionalProperties to false for object schemas. |
| openai.so.2026-04-30 | `inferred` | [OpenAI additionalProperties](https://developers.openai.com/api/docs/guides/structured-outputs#additionalproperties-false-must-always-be-set-in-objects) | The documented subset requires additionalProperties to be false. |

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
