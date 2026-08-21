# root-anyof

> Category: **Structural** — overall schema structure triggers this rule

## Error Codes

| Profile | Code | Severity |
|---------|------|----------|
| openai.so.2026-04-30 | `OAI-S-root-anyof` | Forbid |

## Description

The root schema must not use anyOf

## Provider Evidence

| Profile | Status | Source | Basis |
|---|---|---|---|
| openai.so.2026-04-30 | `documented` | [OpenAI root object](https://developers.openai.com/api/docs/guides/structured-outputs#root-objects-must-not-be-anyof-and-must-be-an-object) | — |

## Rationale

openai.so.2026-04-30 requires a plain object root; anyOf is only valid below the root.

## Bad Example

```json
{ "type": "object", "anyOf": [{ "type": "object" }] }
```

## Good Example

```json
{ "type": "object", "properties": { "value": { "anyOf": [{ "type": "string" }, { "type": "number" }] } }, "required": ["value"], "additionalProperties": false }
```
