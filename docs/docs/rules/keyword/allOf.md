# allOf

> Category: **Keyword** — presence of a specific JSON Schema keyword triggers this rule

## Error Codes

| Profile | Code | Severity |
|---------|------|----------|
| openai.so.2026-04-30 | `OAI-K-allOf` | Forbid |

## Description

Flag usage of the 'allOf' keyword, which is not supported by openai.so.2026-04-30

## Provider Evidence

| Profile | Status | Source | Basis |
|---|---|---|---|
| openai.so.2026-04-30 | `documented` | [OpenAI type-specific keywords](https://developers.openai.com/api/docs/guides/structured-outputs#some-type-specific-keywords-are-not-yet-supported) | — |

## Rationale

The openai.so.2026-04-30 structured-output provider rejects the 'allOf' keyword. Schemas using this keyword may be rejected or silently altered.

## Bad Example

```json
{ "type": "object", "allOf": true, "properties": {} }
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
