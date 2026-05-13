# max-depth

> Category: **Structural** — overall schema structure triggers this rule

## Error Codes

| Profile | Code |
|---------|------|
| openai.so.2026-04-30 | `OAI-S-max-depth` |

## Description

Object nesting depth must not exceed 10 levels

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
