# external-refs

> Category: **Structural** — overall schema structure triggers this rule

## Error Codes

| Profile | Code |
|---------|------|
| openai.so.2026-04-30 | `OAI-S-external-refs` |
| anthropic.so.2026-04-30 | `ANT-S-external-refs` |

## Description

External $ref values (URLs, absolute paths) are not supported

## Rationale

Providers require all $ref references to be internal to the schema (e.g., `#/$defs/Foo`). External references via URLs or file paths are rejected.

## Bad Example

```json
{
  "type": "object",
  "properties": {
    "address": { "$ref": "https://example.com/schemas/address.json" }
  }
}
```

## Good Example

```json
{
  "type": "object",
  "$defs": {
    "Address": {
      "type": "object",
      "properties": {
        "street": { "type": "string" }
      }
    }
  },
  "properties": {
    "address": { "$ref": "#/$defs/Address" }
  }
}
```
