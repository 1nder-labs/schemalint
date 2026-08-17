# patternProperties

> Category: **Keyword** — presence of a specific JSON Schema keyword triggers this rule

## Error Codes

| Profile | Code |
|---------|------|
| openai.so.2026-04-30 | `OAI-K-patternProperties` |

## Description

Flag usage of the 'patternProperties' keyword, which is discouraged by openai.so.2026-04-30

## Rationale

The openai.so.2026-04-30 structured-output provider discourages use of the 'patternProperties' keyword. Schemas using this keyword may be rejected or silently altered.

## Bad Example

```json
{ "type": "object", "patternProperties": true, "properties": {} }
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
