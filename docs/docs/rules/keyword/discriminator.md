# discriminator

> Category: **Keyword** — presence of a specific JSON Schema keyword triggers this rule

## Error Codes

| Profile | Code | Severity |
|---------|------|----------|
| openai.so.2026-04-30 | `OAI-K-discriminator` | Warn |
| anthropic.so.2026-04-30 | `ANT-K-discriminator` | Forbid |

## Description

Flag usage of the 'discriminator' keyword according to the active provider profile.

## Provider Evidence

| Profile | Status | Source | Basis |
|---|---|---|---|
| anthropic.so.2026-04-30 | `unknown` | — | Anthropic does not document this OpenAPI keyword for Structured Outputs. |
| openai.so.2026-04-30 | `inferred` | [OpenAI supported schemas](https://developers.openai.com/api/docs/guides/structured-outputs#supported-schemas) | OpenAPI-only keyword absent from OpenAI's supported JSON Schema subset. |

## Rationale

Provider profiles may enforce different behavior and severity for this rule; use the table below for the selected profile.

## Bad Example

```json
{ "type": "object", "discriminator": true, "properties": {} }
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
