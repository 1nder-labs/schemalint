# max-total-properties

> Category: **Structural** — overall schema structure triggers this rule

## Error Codes

| Profile | Code | Severity |
|---------|------|----------|
| openai.so.2026-04-30 | `OAI-S-max-total-properties` | Forbid |

## Description

Total object properties must not exceed 5000

## Provider Evidence

| Profile | Status | Source | Basis |
|---|---|---|---|
| openai.so.2026-04-30 | `documented` | [OpenAI schema limits](https://developers.openai.com/api/docs/guides/structured-outputs#objects-have-limitations-on-nesting-depth-and-size) | — |

## Rationale

openai.so.2026-04-30 limits the total number of object properties.

## Bad Example

```json
{ "type": "object", "properties": { "...many": {} } }
```

## Good Example

```json
{ "type": "object", "properties": { "name": { "type": "string" } } }
```
