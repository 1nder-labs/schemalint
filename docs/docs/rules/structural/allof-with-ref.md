# allof-with-ref

> Category: **Structural** — overall schema structure triggers this rule

## Error Codes

| Profile | Code | Severity |
|---------|------|----------|
| anthropic.so.2026-04-30 | `ANT-S-allof-with-ref` | Forbid |

## Description

allOf combined with $ref is not supported by Anthropic

## Provider Evidence

| Profile | Status | Source | Basis |
|---|---|---|---|
| anthropic.so.2026-04-30 | `documented` | [Anthropic JSON Schema limitations](https://platform.claude.com/docs/en/build-with-claude/structured-outputs#json-schema-limitations) | — |

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
