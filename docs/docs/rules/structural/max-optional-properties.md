# max-optional-properties

> Category: **Structural** — overall schema structure triggers this rule

## Error Codes

| Profile | Code |
|---------|------|
| anthropic.so.2026-04-30 | `ANT-S-max-optional-properties` |

## Description

Optional properties must not exceed 24

## Rationale

anthropic.so.2026-04-30 limits optional parameters across strict schemas.

## Bad Example

```json
{ "type": "object", "properties": { "optional": { "type": "string" } } }
```

## Good Example

```json
{ "type": "object", "properties": { "required": { "type": "string" } }, "required": ["required"], "additionalProperties": false }
```
