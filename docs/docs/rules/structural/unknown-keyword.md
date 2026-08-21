# unknown-keyword

> Category: **Structural** — overall schema structure triggers this rule

## Error Codes

| Profile | Code | Severity |
|---------|------|----------|
| openai.so.2026-04-30 | `OAI-S-unknown-keyword` | Warn |
| anthropic.so.2026-04-30 | `ANT-S-unknown-keyword` | Warn |

## Description

Flag a keyword the engine does not recognize at all

## Provider Evidence

| Profile | Status | Source | Basis |
|---|---|---|---|
| anthropic.so.2026-04-30 | `inferred` | [Anthropic JSON Schema limitations](https://platform.claude.com/docs/en/build-with-claude/structured-outputs#json-schema-limitations) | Unknown keywords are outside the documented supported subset. |
| openai.so.2026-04-30 | `inferred` | [OpenAI supported schemas](https://developers.openai.com/api/docs/guides/structured-outputs#supported-schemas) | Unknown keywords are not covered by the documented supported subset. |

## Rationale

schemalint knows a fixed set of JSON Schema keywords. A keyword outside that set was never checked against provider behavior, so schemalint cannot say whether the provider accepts, ignores, or rejects it.

## Bad Example

```json
{ "type": "string", "contentEncoding": "base64" }
```

## Good Example

```json
{ "type": "string" }
```
