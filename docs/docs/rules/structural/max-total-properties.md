# max-total-properties

> Category: **Structural** — overall schema structure triggers this rule

## Error Codes

| Profile | Code |
|---------|------|
| openai.so.2026-04-30 | `OAI-S-max-total-properties` |

## Description

Total object properties must not exceed 5000

## Rationale

openai.so.2026-04-30 limits the total number of object properties.

## Bad Example

```json
{ "type": "object", "properties": { "...many": {} } }
```

## Good Example

```json
{ "type": "object", "properties": { "name": { "type": "string" } } }
```
