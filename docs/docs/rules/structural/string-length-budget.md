# string-length-budget

> Category: **Structural** — overall schema structure triggers this rule

## Error Codes

| Profile | Code | Severity |
|---------|------|----------|
| openai.so.2026-04-30 | `OAI-S-string-length-budget` | Forbid |

## Description

Total property and enum string length must not exceed 120000

## Provider Evidence

| Profile | Status | Source | Basis |
|---|---|---|---|
| openai.so.2026-04-30 | `documented` | [OpenAI schema limits](https://developers.openai.com/api/docs/guides/structured-outputs#objects-have-limitations-on-nesting-depth-and-size) | — |

## Rationale

openai.so.2026-04-30 enforces a schema string-length budget.

## Bad Example

```json
{ "type": "object", "properties": { "very_long_property_name": { "type": "string" } } }
```

## Good Example

```json
{ "type": "object", "properties": { "name": { "type": "string" } } }
```
