# all-properties-required

> Category: **Structural** — overall schema structure triggers this rule

## Error Codes

| Profile | Code | Severity |
|---------|------|----------|
| openai.so.2026-04-30 | `OAI-S-all-properties-required` | Forbid |

## Description

Every property must be listed in the required array

## Provider Evidence

| Profile | Status | Source | Basis |
|---|---|---|---|
| openai.so.2026-04-30 | `documented` | [OpenAI required fields](https://developers.openai.com/api/docs/guides/structured-outputs#all-fields-must-be-required) | — |

## Rationale

Some providers reject schemas with optional object properties.

## Bad Example

```json
{ "type": "object", "properties": { "name": { "type": "string" }, "age": { "type": "number" } }, "required": ["name"] }
```

## Good Example

```json
{ "type": "object", "properties": { "name": { "type": "string" }, "age": { "type": "number" } }, "required": ["name", "age"] }
```
