# minItems-restricted

> Category: **Restriction** — a keyword value outside the allowed set triggers this rule

## Error Codes

| Profile | Code | Severity |
|---------|------|----------|
| anthropic.so.2026-04-30 | `ANT-K-minItems-restricted` | Forbid |

## Description

Restrict values of the 'minItems' keyword to those accepted by anthropic.so.2026-04-30

## Provider Evidence

| Profile | Status | Source | Basis |
|---|---|---|---|
| anthropic.so.2026-04-30 | `documented` | [Anthropic JSON Schema limitations](https://platform.claude.com/docs/en/build-with-claude/structured-outputs#json-schema-limitations) | — |

## Rationale

anthropic.so.2026-04-30 only supports specific values for the 'minItems' keyword. Using unsupported values will cause validation errors at the API level.

## Bad Example

```json
{ "type": "object", "minItems": "invalid-value", "properties": {} }
```

## Good Example

```json
{ "type": "object", "minItems": 0, "properties": {} }
```
