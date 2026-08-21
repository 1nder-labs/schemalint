# additional-properties-false

> Category: **Structural** — overall schema structure triggers this rule

## Error Codes

| Profile | Code | Severity |
|---------|------|----------|
| openai.so.2026-04-30 | `OAI-S-additional-properties-false` | Forbid |
| anthropic.so.2026-04-30 | `ANT-S-additional-properties-false` | Forbid |

## Description

Every object schema must declare additionalProperties: false

## Provider Evidence

| Profile | Status | Source | Basis |
|---|---|---|---|
| anthropic.so.2026-04-30 | `documented` | [Anthropic JSON Schema limitations](https://platform.claude.com/docs/en/build-with-claude/structured-outputs#json-schema-limitations) | — |
| openai.so.2026-04-30 | `documented` | [OpenAI additionalProperties](https://developers.openai.com/api/docs/guides/structured-outputs#additionalproperties-false-must-always-be-set-in-objects) | — |

## Rationale

Providers require object nodes to explicitly reject extra properties.

## Bad Example

```json
{ "type": "object", "properties": { "name": { "type": "string" } } }
```

## Good Example

```json
{ "type": "object", "additionalProperties": false, "properties": { "name": { "type": "string" } } }
```
