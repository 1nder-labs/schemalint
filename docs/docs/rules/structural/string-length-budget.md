# string-length-budget

> Category: **Structural** — overall schema structure triggers this rule

## Error Codes

| Profile | Code |
|---------|------|
| openai.so.2026-04-30 | `OAI-S-string-length-budget` |

## Description

Total property and enum string length must not exceed 120000

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
