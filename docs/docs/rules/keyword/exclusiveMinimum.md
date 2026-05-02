# exclusiveMinimum

> Category: **Keyword** — presence of a specific JSON Schema keyword triggers this rule

## Error Codes

| Profile | Code |
|---------|------|
| anthropic.so.2026-04-30 | `ANT-K-exclusiveMinimum` |

## Description

Flag usage of the 'exclusiveMinimum' keyword, which is not supported by anthropic.so.2026-04-30

## Rationale

The anthropic.so.2026-04-30 structured-output provider rejects the 'exclusiveMinimum' keyword. Schemas using this keyword may be rejected or silently altered.

## Bad Example

```json
{ "type": "object", "exclusiveMinimum": true, "properties": {} }
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
