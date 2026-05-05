# contains

> Category: **Keyword** — presence of a specific JSON Schema keyword triggers this rule

## Error Codes

| Profile | Code |
|---------|------|
| openai.so.2026-04-30 | `OAI-K-contains` |
| anthropic.so.2026-04-30 | `ANT-K-contains` |

## Description

Flag usage of the 'contains' keyword, which is not supported by openai.so.2026-04-30

## Rationale

The openai.so.2026-04-30 structured-output provider rejects the 'contains' keyword. Schemas using this keyword may be rejected or silently altered.

## Bad Example

```json
{ "type": "object", "contains": true, "properties": {} }
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
