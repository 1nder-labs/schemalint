# max-enum-values

> Category: **Structural** — overall schema structure triggers this rule

## Error Codes

| Profile | Code |
|---------|------|
| openai.so.2026-04-30 | `OAI-S-max-enum-values` |

## Description

Total number of enum values across the schema must not exceed 1000

## Rationale

openai.so.2026-04-30 imposes a limit of 1000 total enum values across the entire schema.

## Bad Example

```json
{ "type": "object", "properties": { "color": { "enum": [...1000+ values...] } } }
```

## Good Example

```json
{
  "type": "object",
  "properties": {
    "color": { "enum": ["red", "green", "blue"] }
  }
}
```
