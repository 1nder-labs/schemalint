# allof-with-ref

> Category: **Structural** — overall schema structure triggers this rule

## Error Codes

| Profile | Code |
|---------|------|
| anthropic.so.2026-04-30 | `ANT-S-allof-with-ref` |

## Description

allOf combined with $ref is not supported by Anthropic

## Rationale

Anthropic rejects schemas that combine allOf with $ref references.

## Bad Example

```json
{ "type": "object", "allOf": [{ "$ref": "#/$defs/Base" }] }
```

## Good Example

```json
{ "type": "object", "properties": { "id": { "type": "string" } } }
```
