# Anthropic Structured Outputs

- **Profile**: `anthropic.so.2026-04-30`
- **API**: Native Claude API JSON outputs (`output_config.format`) and strict tool use (`strict: true`)
- **Model behavior**: Strict structured outputs compile supported JSON Schema into a grammar. SDK helpers may remove unsupported constraints before sending schemas to Claude, then validate locally.

## Supported Keywords

Anthropic supports standard JSON Schema with documented limitations. Numeric and string constraints such as `minimum`, `maximum`, `minLength`, and `maxLength` are not sent directly by SDK helpers; they are moved into descriptions and enforced by local validation.

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
| Optional properties | 24 across strict schemas |
| Union parameters (`anyOf` / type arrays) | 16 across strict schemas |
| External `$ref` | Allowed |
| `allOf` with `$ref` | Not supported |

The native Claude profile is not the same as Anthropic's OpenAI SDK
compatibility layer. Anthropic documents that the compatibility layer ignores
OpenAI `response_format`, and ignores `strict` for function calling; use the
native Claude API for guaranteed schema conformance.

## Reference

[Anthropic Structured Outputs documentation](https://docs.anthropic.com/en/docs/build-with-claude/structured-outputs)
