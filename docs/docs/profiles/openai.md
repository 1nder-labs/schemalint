# OpenAI Structured Outputs

- **Profile**: `openai.so.2026-04-30`
- **API**: Responses API `text.format.type = "json_schema"` and Chat Completions `response_format.type = "json_schema"` with `strict: true`
- **Model behavior**: Strict — rejects schemas with unsupported keywords

## Supported Keywords

OpenAI supports a subset of JSON Schema. The following keywords are rejected for `gpt-4o-2024-08-06` / `gpt-4o-mini` Structured Outputs:

- `allOf`, `oneOf`, and root-level `anyOf`
- `not`, `if`/`then`/`else`
- `dependentRequired`, `dependentSchemas`
- `propertyNames`, `maxProperties`, `minProperties`, `uniqueItems`, `contains`, `unevaluatedProperties`

Nested `anyOf` remains supported when each branch is valid for the Structured Outputs subset.
`patternProperties` works for the base Structured Outputs model surface, but OpenAI documents additional restrictions for fine-tuned models.

## Value Restrictions

- `additionalProperties`: Must be `false` (no object schemas)
- `format`: Only `["date-time", "time", "date", "duration", "email", "hostname", "ipv4", "ipv6", "uuid"]`

## Structural Limits

| Limit | Value |
|-------|-------|
| Root schema type | Must be `object` |
| Root composition | Must not use `anyOf`, `oneOf`, `allOf`, `enum`, or `not` |
| `additionalProperties` | Must be `false` on all objects |
| `required` | Must include every property on every object |
| Array schemas | Must declare `items` |
| Max nesting depth | 10 |
| Max total properties | 5,000 |
| Max total enum values | 1,000 |
| Max string length budget | 120,000 |
| External `$ref` | Not supported |

## Reference

[OpenAI Structured Outputs documentation](https://developers.openai.com/api/docs/guides/structured-outputs)
