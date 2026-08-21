# max-depth

> Category: **Structural** — overall schema structure triggers this rule

## Error Codes

| Profile | Code | Severity |
|---------|------|----------|
| openai.so.2026-04-30 | `OAI-S-max-depth` | Forbid |

## Description

Object nesting depth must not exceed 10 levels

## Provider Evidence

| Profile | Status | Source | Basis |
|---|---|---|---|
| openai.so.2026-04-30 | `live_verified` | [OpenAI schema limits](https://developers.openai.com/api/docs/guides/structured-outputs#objects-have-limitations-on-nesting-depth-and-size) | Observed from the OpenAI API; the published nesting limit is stale for the tested target. |

## Rationale

openai.so.2026-04-30 limits object nesting depth to 10 levels.

## Bad Example

```json
{ "type": "object", "properties": { "nested": { "type": "object", "properties": { "too_deep": { "type": "object" } } } } }
```

## Good Example

```json
{ "type": "object", "properties": { "name": { "type": "string" } } }
```
