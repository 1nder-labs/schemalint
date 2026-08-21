# external-refs

> Category: **Structural** — overall schema structure triggers this rule

## Error Codes

| Profile | Code | Severity |
|---------|------|----------|
| openai.so.2026-04-30 | `OAI-S-external-refs` | Forbid |
| anthropic.so.2026-04-30 | `ANT-S-external-refs` | Forbid |

## Description

External $ref values are not supported

## Provider Evidence

| Profile | Status | Source | Basis |
|---|---|---|---|
| anthropic.so.2026-04-30 | `inferred` | [Anthropic JSON Schema limitations](https://platform.claude.com/docs/en/build-with-claude/structured-outputs#json-schema-limitations) | Anthropic documents local references but not retrieval of external references. |
| openai.so.2026-04-30 | `inferred` | [OpenAI definitions](https://developers.openai.com/api/docs/guides/structured-outputs#definitions-are-supported) | OpenAI documents local definitions, but not external reference retrieval. |

## Rationale

Providers require references to be internal to the submitted schema.

## Bad Example

```json
{ "type": "object", "properties": { "address": { "$ref": "https://example.com/address.json" } } }
```

## Good Example

```json
{ "type": "object", "$defs": { "Address": { "type": "object" } }, "properties": { "address": { "$ref": "#/$defs/Address" } } }
```
