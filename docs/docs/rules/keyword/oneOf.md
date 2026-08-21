# oneOf

> Category: **Keyword** — presence of a specific JSON Schema keyword triggers this rule

## Error Codes

| Profile | Code | Severity |
|---------|------|----------|
| openai.so.2026-04-30 | `OAI-K-oneOf` | Forbid |

## Description

Flag usage of the 'oneOf' keyword, which is not supported by openai.so.2026-04-30

## Provider Evidence

| Profile | Status | Source | Basis |
|---|---|---|---|
| openai.so.2026-04-30 | `inferred` | [OpenAI supported schemas](https://developers.openai.com/api/docs/guides/structured-outputs#supported-schemas) | Absent from the documented supported schema subset. |

## Rationale

The openai.so.2026-04-30 structured-output provider rejects the 'oneOf' keyword. Schemas using this keyword may be rejected or silently altered.

## Bad Example

```json
{ "type": "object", "oneOf": true, "properties": {} }
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
