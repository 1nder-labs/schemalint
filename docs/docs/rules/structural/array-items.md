# array-items

> Category: **Structural** — overall schema structure triggers this rule

## Error Codes

| Profile | Code |
|---------|------|
| openai.so.2026-04-30 | `OAI-S-array-items` |

## Description

Array schemas must declare an items schema

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
