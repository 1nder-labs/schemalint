# OpenAI Structured Outputs

- **Profile**: `openai.so.2026-04-30`
- **API**: `gpt-4o-2024-08-06` with `response_format.type = "json_schema"` and `strict: true`
- **Model behavior**: Strict — rejects schemas with unsupported keywords

## Supported Keywords

OpenAI supports most standard JSON Schema keywords with a few notable exceptions. The following keywords are rejected:

- `allOf`, `oneOf`, `anyOf` (partial — `anyOf` with object branches gets a warning)
- `not`, `if`/`then`/`else`
- `discriminator`, `dependentRequired`, `dependentSchemas`

## Value Restrictions

- `additionalProperties`: Must be `false` (no object schemas)
- `format`: Only `["date-time", "time", "date", "duration", "email", "hostname", "ipv4", "ipv6", "uuid"]`

## Structural Limits

| Limit | Value |
|-------|-------|
| Root schema type | Must be `object` |
| `additionalProperties` | Must be `false` on all objects |
| Max nesting depth | 10 |
| Max total properties | 5,000 |
| Max total enum values | 1,000 |
| Max string length budget | 120,000 |
| External `$ref` | Allowed |

## Reference

[OpenAI Structured Outputs documentation](https://platform.openai.com/docs/guides/structured-outputs)
