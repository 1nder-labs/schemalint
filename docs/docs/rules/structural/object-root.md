# object-root

> Category: **Structural** — overall schema structure triggers this rule

## Error Codes

| Profile | Code |
|---------|------|
| openai.so.2026-04-30 | `OAI-S-object-root` |

## Description

The root schema must be of type object

## Rationale

Structured-output providers require the top-level schema to be an object. Array, string, or primitive root schemas are rejected at the API level.

## Bad Example

```json
{
  "type": "array",
  "items": { "type": "string" }
}
```

## Good Example

```json
{
  "type": "object",
  "properties": {
    "name": { "type": "string" }
  }
}
```
