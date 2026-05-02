# max-depth

> Category: **Structural** — overall schema structure triggers this rule

## Error Codes

| Profile | Code |
|---------|------|
| openai.so.2026-04-30 | `OAI-S-max-depth` |

## Description

Object nesting depth must not exceed 10 levels

## Rationale

openai.so.2026-04-30 limits object nesting depth to 10 levels. Exceeding this causes API rejection.

## Bad Example

```json
{ "type": "object", "properties": { "a": { "type": "object", "properties": { "b": { "type": "object", "properties": { "c": { "type": "object", "properties": { "d": { "type": "object", "properties": { "e": { "type": "object", "properties": { "f": { "type": "object", "properties": { "g": { "type": "object", "properties": { "h": { "type": "object", "properties": { "i": { "type": "object", "properties": { "j": { "type": "object", "properties": { "k": { "type": "object", "properties": {} } } } } } } } } } } } } } } } } } } } } } } }
```

## Good Example

```json
{
  "type": "object",
  "properties": {
    "name": { "type": "string" }
  }
}
```
