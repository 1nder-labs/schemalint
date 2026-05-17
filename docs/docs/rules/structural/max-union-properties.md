# max-union-properties

> Category: **Structural** — overall schema structure triggers this rule

## Error Codes

| Profile | Code |
|---------|------|
| anthropic.so.2026-04-30 | `ANT-S-max-union-properties` |

## Description

Union parameters must not exceed 16

## Rationale

anthropic.so.2026-04-30 limits parameters that use anyOf or type arrays across strict schemas.

## Bad Example

```json
{ "type": "object", "properties": { "value": { "anyOf": [{ "type": "string" }, { "type": "number" }] } } }
```

## Good Example

```json
{ "type": "object", "properties": { "value": { "type": "string" } }, "required": ["value"], "additionalProperties": false }
```
