# all-properties-required

> Category: **Structural** — overall schema structure triggers this rule

## Error Codes

| Profile | Code |
|---------|------|
| openai.so.2026-04-30 | `OAI-S-all-properties-required` |

## Description

Every property must be listed in the required array

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
