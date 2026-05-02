# then

> Category: **Keyword** — presence of a specific JSON Schema keyword triggers this rule

## Error Codes

| Profile | Code |
|---------|------|
| openai.so.2026-04-30 | `OAI-K-then` |
| anthropic.so.2026-04-30 | `ANT-K-then` |

## Description

Flag usage of the 'then' keyword, which is not supported by openai.so.2026-04-30

## Rationale

The openai.so.2026-04-30 structured-output provider rejects the 'then' keyword. Schemas using this keyword may be rejected or silently altered.

## Bad Example

```json
{ "type": "object", "then": true, "properties": {} }
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
