# enum-string-length-budget

> Category: **Structural** — overall schema structure triggers this rule

## Error Codes

| Profile | Code |
|---------|------|
| openai.so.2026-04-30 | `OAI-S-enum-string-length-budget` |

## Description

Enum strings must not exceed 15000 characters after 250 values

## Rationale

openai.so.2026-04-30 enforces a conditional enum budget.

## Bad Example

```json
{ "type": "string", "enum": ["...many long values"] }
```

## Good Example

```json
{ "type": "string", "enum": ["red", "green"] }
```
