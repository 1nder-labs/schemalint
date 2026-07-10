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
| Enum string budget | 15,000 characters when an enum has more than 250 values |
| External `$ref` | Not supported |

The depth limit is evidence-based: the retained
[`probe_limits_2026-06-16.json`](https://github.com/1nder-labs/schemalint/blob/main/scripts/validation/results/probe_limits_2026-06-16.json)
records inline object depth 10 as accepted and depth 11 as rejected by the
Responses API. That probe does not contain local `$ref` chains, so SchemaLint
does not reinterpret it as proof of a separate semantic reference-depth rule.
Other budgets in this table follow the linked provider contract and are covered
by offline profile/conformance tests; live provider probes remain manual and
credential-gated.

## Reference

[OpenAI Structured Outputs documentation](https://developers.openai.com/api/docs/guides/structured-outputs)
