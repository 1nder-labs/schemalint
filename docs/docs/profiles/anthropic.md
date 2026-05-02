# Anthropic Structured Outputs

- **Profile**: `anthropic.so.2026-04-30`
- **API**: Latest Claude models with `tools` / structured output configuration
- **Model behavior**: Stripping — strips unsupported keywords from the schema

## Supported Keywords

Anthropic supports a smaller subset of JSON Schema keywords. Many numeric and array-validation keywords are rejected.

## Value Restrictions

- `additionalProperties`: Must be `false`
- `format`: `["date-time", "time", "date", "duration", "email", "hostname", "uri", "ipv4", "ipv6", "uuid"]`
- `minItems`: Only `0` or `1`

## Structural Limits

| Limit | Value |
|-------|-------|
| Root schema type | Object not required, but recommended |
| `additionalProperties` | Must be `false` on all objects |
| Max nesting depth | Unlimited |
| Max total properties | Unlimited |
| Max total enum values | Unlimited |
| Max string length budget | Unlimited |
| External `$ref` | Allowed |
| `allOf` with `$ref` | Not supported |

## Reference

[Anthropic Structured Outputs documentation](https://docs.anthropic.com/en/docs/build-with-claude/structured-outputs)
