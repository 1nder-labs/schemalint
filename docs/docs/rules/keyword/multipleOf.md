# multipleOf

> Category: **Keyword** — presence of a specific JSON Schema keyword triggers this rule

## Error Codes

| Profile | Code |
|---------|------|
| anthropic.so.2026-04-30 | `ANT-K-multipleOf` |

## Description

Flag usage of the 'multipleOf' keyword, which is not supported by anthropic.so.2026-04-30

## Rationale

The anthropic.so.2026-04-30 structured-output provider rejects the 'multipleOf' keyword. Schemas using this keyword may be rejected or silently altered.

## Bad Example

```json
{ "type": "object", "multipleOf": true, "properties": {} }
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
