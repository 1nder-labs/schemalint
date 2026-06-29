# external-refs

> Category: **Structural** — overall schema structure triggers this rule

## Error Codes

| Profile | Code |
|---------|------|
| openai.so.2026-04-30 | `OAI-S-external-refs` |
| anthropic.so.2026-04-30 | `ANT-S-external-refs` |

## Description

External $ref values are not supported

## Rationale

Providers require references to be internal to the submitted schema.

## Bad Example

```json
{ "type": "object", "properties": { "address": { "$ref": "https://example.com/address.json" } } }
```

## Good Example

```json
{ "type": "object", "$defs": { "Address": { "type": "object" } }, "properties": { "address": { "$ref": "#/$defs/Address" } } }
```
