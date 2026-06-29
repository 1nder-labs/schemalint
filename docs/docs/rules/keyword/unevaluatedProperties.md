# unevaluatedProperties

> Category: **Keyword** — presence of a specific JSON Schema keyword triggers this rule

## Error Codes

| Profile | Code |
|---------|------|
| openai.so.2026-04-30 | `OAI-K-unevaluatedProperties` |

## Description

Flag usage of the 'unevaluatedProperties' keyword, which is not supported by openai.so.2026-04-30

## Rationale

The openai.so.2026-04-30 structured-output provider rejects the 'unevaluatedProperties' keyword. Schemas using this keyword may be rejected or silently altered.

## Bad Example

```json
{ "type": "object", "unevaluatedProperties": true, "properties": {} }
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
