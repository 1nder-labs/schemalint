# format-restricted

> Category: **Restriction** — a keyword value outside the allowed set triggers this rule

## Error Codes

| Profile | Code | Severity |
|---------|------|----------|
| openai.so.2026-04-30 | `OAI-K-format-restricted` | Forbid |
| anthropic.so.2026-04-30 | `ANT-K-format-restricted` | Forbid |

## Description

Restrict 'format' values according to the active provider profile.

## Provider Evidence

| Profile | Status | Source | Basis |
|---|---|---|---|
| anthropic.so.2026-04-30 | `documented` | [Anthropic JSON Schema limitations](https://platform.claude.com/docs/en/build-with-claude/structured-outputs#json-schema-limitations) | — |
| openai.so.2026-04-30 | `documented` | [OpenAI supported formats](https://developers.openai.com/api/docs/guides/structured-outputs#supported-schemas) | — |

## Rationale

Provider profiles may enforce different behavior and severity for this rule; use the table below for the selected profile.

## Bad Example

```json
{ "type": "object", "format": "invalid-value", "properties": {} }
```

## Good Example

```json
{ "type": "object", "format": "date-time", "properties": {} }
```
