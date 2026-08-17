# recursive-schema

> Category: **Structural** — overall schema structure triggers this rule

## Error Codes

| Profile | Code |
|---------|------|
| anthropic.so.2026-04-30 | `ANT-S-recursive-schema` |

## Description

A schema must not contain a $ref cycle

## Rationale

Anthropic's structured-outputs documentation lists "Recursive schemas" among the unsupported schema features.

## Bad Example

```json
{ "type": "object", "properties": { "root": { "$ref": "#/$defs/Node" } }, "$defs": { "Node": { "type": "object", "properties": { "next": { "$ref": "#/$defs/Node" } } } } }
```

## Good Example

```json
{ "type": "object", "properties": { "root": { "$ref": "#/$defs/Node" } }, "$defs": { "Node": { "type": "object", "properties": { "value": { "type": "string" } } } } }
```
