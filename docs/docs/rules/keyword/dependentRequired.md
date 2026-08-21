# dependentRequired

> Category: **Keyword** — presence of a specific JSON Schema keyword triggers this rule

## Error Codes

| Profile | Code | Severity |
|---------|------|----------|
| openai.so.2026-04-30 | `OAI-K-dependentRequired` | Forbid |
| anthropic.so.2026-04-30 | `ANT-K-dependentRequired` | Forbid |

## Description

Flag usage of the 'dependentRequired' keyword according to the active provider profile.

## Provider Evidence

| Profile | Status | Source | Basis |
|---|---|---|---|
| anthropic.so.2026-04-30 | `inferred` | [Anthropic JSON Schema limitations](https://platform.claude.com/docs/en/build-with-claude/structured-outputs#json-schema-limitations) | Cross-field applicator absent from the documented supported subset. |
| openai.so.2026-04-30 | `documented` | [OpenAI type-specific keywords](https://developers.openai.com/api/docs/guides/structured-outputs#some-type-specific-keywords-are-not-yet-supported) | — |

## Rationale

Provider profiles may enforce different behavior and severity for this rule; use the table below for the selected profile.

## Bad Example

```json
{ "type": "object", "dependentRequired": true, "properties": {} }
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
