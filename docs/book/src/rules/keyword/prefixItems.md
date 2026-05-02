# prefixItems

> Category: **Keyword** — presence of a specific JSON Schema keyword triggers this rule

## Error Codes

| Profile | Code |
|---------|------|
| openai.so.2026-04-30 | `OAI-K-prefixItems` |
| anthropic.so.2026-04-30 | `ANT-K-prefixItems` |

## Description

Flag usage of the 'prefixItems' keyword, which is discouraged by openai.so.2026-04-30

## Rationale

The openai.so.2026-04-30 structured-output provider discourages use of the 'prefixItems' keyword. Schemas using this keyword may be rejected or silently altered.

## Bad Example

```json
{ "type": "object", "prefixItems": true, "properties": {} }
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
