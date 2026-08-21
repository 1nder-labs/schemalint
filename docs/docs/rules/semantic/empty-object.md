# empty-object

> Category: **Semantic** — schema semantics trigger this rule

## Error Codes

| Profile | Code | Severity |
|---------|------|----------|
| openai.so.2026-04-30 | `OAI-S-empty-object` | Warn |
| anthropic.so.2026-04-30 | `ANT-S-empty-object` | Warn |

## Description

Object schema with additionalProperties: false but no properties

## Provider Evidence

| Profile | Status | Source | Basis |
|---|---|---|---|
| anthropic.so.2026-04-30 | `sdk_transform` | [Anthropic SDK transformation](https://platform.claude.com/docs/en/build-with-claude/structured-outputs#how-sdk-transformation-works) | Official SDK transformation operates on declared object properties. |
| openai.so.2026-04-30 | `live_verified` | [OpenAI object restrictions](https://developers.openai.com/api/docs/guides/structured-outputs#additionalproperties-false-must-always-be-set-in-objects) | Observed rejection of an object schema without properties. |

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
