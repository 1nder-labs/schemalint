# string-length-budget

> Category: **Structural** — overall schema structure triggers this rule

## Error Codes

| Profile | Code |
|---------|------|
| openai.so.2026-04-30 | `OAI-S-string-length-budget` |

## Description

Total string length (property names + enum values) must not exceed 120000

## Rationale

openai.so.2026-04-30 imposes a string length budget of 120000 across all property names and enum values.

## Bad Example

```json
{ "type": "object", "properties": { "very_long_property_name": { "type": "string" } } }
```

## Good Example

```json
{
  "type": "object",
  "properties": {
    "name": { "type": "string" }
  }
}
```
