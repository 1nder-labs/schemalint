# array-items

> Category: **Structural** — overall schema structure triggers this rule

## Error Codes

| Profile | Code | Severity |
|---------|------|----------|
| openai.so.2026-04-30 | `OAI-S-array-items` | Forbid |

## Description

Array schemas must declare an items schema

## Provider Evidence

| Profile | Status | Source | Basis |
|---|---|---|---|
| openai.so.2026-04-30 | `documented` | [OpenAI supported schemas](https://developers.openai.com/api/docs/guides/structured-outputs#supported-schemas) | — |

## Rationale

openai.so.2026-04-30 rejects array schemas that omit the items keyword.

## Bad Example

```json
{ "type": "array" }
```

## Good Example

```json
{ "type": "array", "items": { "type": "string" } }
```
