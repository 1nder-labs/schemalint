# max-enum-values

> Category: **Structural** — overall schema structure triggers this rule

## Error Codes

| Profile | Code |
|---------|------|
| openai.so.2026-04-30 | `OAI-S-max-enum-values` |

## Description

Total enum values must not exceed 1000

## Rationale

openai.so.2026-04-30 limits total enum values.

## Bad Example

```json
{ "type": "string", "enum": ["...1000+ values"] }
```

## Good Example

```json
{ "type": "string", "enum": ["red", "green", "blue"] }
```
