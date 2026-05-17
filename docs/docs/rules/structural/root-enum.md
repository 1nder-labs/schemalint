# root-enum

> Category: **Structural** — overall schema structure triggers this rule

## Error Codes

| Profile | Code |
|---------|------|
| openai.so.2026-04-30 | `OAI-S-root-enum` |

## Description

The root schema must not use enum

## Rationale

openai.so.2026-04-30 requires a plain object root and rejects enum at the top level.

## Bad Example

```json
{ "type": "string", "enum": ["yes", "no"] }
```

## Good Example

```json
{ "type": "object", "properties": { "answer": { "type": "string", "enum": ["yes", "no"] } }, "required": ["answer"], "additionalProperties": false }
```
