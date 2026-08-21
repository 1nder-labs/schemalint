# patternProperties

> Category: **Keyword** — presence of a specific JSON Schema keyword triggers this rule

## Error Codes

| Profile | Code | Severity |
|---------|------|----------|
| openai.so.2026-04-30 | `OAI-K-patternProperties` | Warn |

## Description

Flag usage of the 'patternProperties' keyword, which is discouraged by openai.so.2026-04-30

## Provider Evidence

| Profile | Status | Source | Basis |
|---|---|---|---|
| openai.so.2026-04-30 | `inferred` | [OpenAI type-specific keywords](https://developers.openai.com/api/docs/guides/structured-outputs#some-type-specific-keywords-are-not-yet-supported) | Mentioned only as unavailable for fine-tuned models, not as supported for the standard subset. |

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
