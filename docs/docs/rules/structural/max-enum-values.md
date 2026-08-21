# max-enum-values

> Category: **Structural** — overall schema structure triggers this rule

## Error Codes

| Profile | Code | Severity |
|---------|------|----------|
| openai.so.2026-04-30 | `OAI-S-max-enum-values` | Forbid |

## Description

Total enum values must not exceed 1000

## Provider Evidence

| Profile | Status | Source | Basis |
|---|---|---|---|
| openai.so.2026-04-30 | `documented` | [OpenAI schema limits](https://developers.openai.com/api/docs/guides/structured-outputs#objects-have-limitations-on-nesting-depth-and-size) | — |

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
