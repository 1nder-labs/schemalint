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

Providers require object nodes to explicitly reject extra properties.

## Bad Example

```json
{ "type": "object", "properties": { "name": { "type": "string" } } }
```

## Good Example

```json
{ "type": "object", "additionalProperties": false, "properties": { "name": { "type": "string" } } }
```
