# minItems-restricted

> Category: **Restriction** — a keyword value outside the allowed set triggers this rule

## Error Codes

| Profile | Code |
|---------|------|
| anthropic.so.2026-04-30 | `ANT-K-minItems-restricted` |

## Description

Restrict values of the 'minItems' keyword to those accepted by anthropic.so.2026-04-30

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
