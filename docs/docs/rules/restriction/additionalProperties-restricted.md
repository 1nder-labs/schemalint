# additionalProperties-restricted

> Category: **Restriction** — a keyword value outside the allowed set triggers this rule

## Error Codes

| Profile | Code | Severity |
|---------|------|----------|
| openai.so.2026-04-30 | `OAI-K-additionalProperties-restricted` | Forbid |
| anthropic.so.2026-04-30 | `ANT-K-additionalProperties-restricted` | Forbid |

## Description

Restrict 'additionalProperties' values according to the active provider profile.

## Provider Evidence

| Profile | Status | Source | Basis |
|---|---|---|---|
| anthropic.so.2026-04-30 | `documented` | [Anthropic JSON Schema limitations](https://platform.claude.com/docs/en/build-with-claude/structured-outputs#json-schema-limitations) | — |
| openai.so.2026-04-30 | `documented` | [OpenAI additionalProperties](https://developers.openai.com/api/docs/guides/structured-outputs#additionalproperties-false-must-always-be-set-in-objects) | — |

## Rationale

Provider profiles may enforce different behavior and severity for this rule; use the table below for the selected profile.

## Bad Example

```json
{ "type": "object", "additionalProperties": "invalid-value", "properties": {} }
```

## Good Example

```json
{ "type": "object", "additionalProperties": false, "properties": {} }
```
