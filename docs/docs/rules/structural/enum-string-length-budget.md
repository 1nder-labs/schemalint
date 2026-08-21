# enum-string-length-budget

> Category: **Structural** — overall schema structure triggers this rule

## Error Codes

| Profile | Code | Severity |
|---------|------|----------|
| openai.so.2026-04-30 | `OAI-S-enum-string-length-budget` | Forbid |

## Description

Enum strings must not exceed 15000 characters after 250 values

## Provider Evidence

| Profile | Status | Source | Basis |
|---|---|---|---|
| openai.so.2026-04-30 | `documented` | [OpenAI schema limits](https://developers.openai.com/api/docs/guides/structured-outputs#objects-have-limitations-on-nesting-depth-and-size) | — |

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
