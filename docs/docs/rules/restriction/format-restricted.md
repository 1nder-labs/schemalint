# format-restricted

> Category: **Restriction** — a keyword value outside the allowed set triggers this rule

## Error Codes

| Profile | Code |
|---------|------|
| openai.so.2026-04-30 | `OAI-K-format-restricted` |
| anthropic.so.2026-04-30 | `ANT-K-format-restricted` |

## Description

Restrict values of the 'format' keyword to those accepted by openai.so.2026-04-30

## Rationale

openai.so.2026-04-30 only supports specific values for the 'format' keyword. Using unsupported values will cause validation errors at the API level.

## Bad Example

```json
{ "type": "object", "format": "invalid-value", "properties": {} }
```

## Good Example

```json
{ "type": "object", "format": "date-time", "properties": {} }
```
