# minLength

> Category: **Keyword** — presence of a specific JSON Schema keyword triggers this rule

## Error Codes

| Profile | Code | Severity |
|---------|------|----------|
| openai.so.2026-04-30 | `OAI-K-minLength` | Warn |
| anthropic.so.2026-04-30 | `ANT-K-minLength` | Forbid |

## Description

Flag usage of the 'minLength' keyword according to the active provider profile.

## Provider Evidence

| Profile | Status | Source | Basis |
|---|---|---|---|
| anthropic.so.2026-04-30 | `documented` | [Anthropic JSON Schema limitations](https://platform.claude.com/docs/en/build-with-claude/structured-outputs#json-schema-limitations) | — |
| openai.so.2026-04-30 | `inferred` | [OpenAI type-specific keywords](https://developers.openai.com/api/docs/guides/structured-outputs#some-type-specific-keywords-are-not-yet-supported) | Absent from the documented supported string constraints. |

## Rationale

Provider profiles may enforce different behavior and severity for this rule; use the table below for the selected profile.

## Bad Example

```json
{ "type": "object", "minLength": true, "properties": {} }
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
