# maxProperties

> Category: **Keyword** — presence of a specific JSON Schema keyword triggers this rule

## Error Codes

| Profile | Code | Severity |
|---------|------|----------|
| openai.so.2026-04-30 | `OAI-K-maxProperties` | Forbid |

## Description

Flag usage of the 'maxProperties' keyword, which is not supported by openai.so.2026-04-30

## Provider Evidence

| Profile | Status | Source | Basis |
|---|---|---|---|
| openai.so.2026-04-30 | `inferred` | [OpenAI type-specific keywords](https://developers.openai.com/api/docs/guides/structured-outputs#some-type-specific-keywords-are-not-yet-supported) | Absent from the documented supported object constraints. |

## Rationale

The openai.so.2026-04-30 structured-output provider rejects the 'maxProperties' keyword. Schemas using this keyword may be rejected or silently altered.

## Bad Example

```json
{ "type": "object", "maxProperties": true, "properties": {} }
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
