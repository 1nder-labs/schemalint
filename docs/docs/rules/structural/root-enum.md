# root-enum

> Category: **Structural** — overall schema structure triggers this rule

## Error Codes

| Profile | Code | Severity |
|---------|------|----------|
| openai.so.2026-04-30 | `OAI-S-root-enum` | Forbid |

## Description

The root schema must not use enum

## Provider Evidence

| Profile | Status | Source | Basis |
|---|---|---|---|
| openai.so.2026-04-30 | `inferred` | [OpenAI root object](https://developers.openai.com/api/docs/guides/structured-outputs#root-objects-must-not-be-anyof-and-must-be-an-object) | The root schema is documented as an object; a root enum is therefore outside the subset. |

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
