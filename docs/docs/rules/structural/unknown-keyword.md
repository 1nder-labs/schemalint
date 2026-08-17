# unknown-keyword

> Category: **Structural** — overall schema structure triggers this rule

## Error Codes

| Profile | Code |
|---------|------|
| openai.so.2026-04-30 | `OAI-S-unknown-keyword` |
| anthropic.so.2026-04-30 | `ANT-S-unknown-keyword` |

## Description

Flag a keyword the engine does not recognize at all

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
