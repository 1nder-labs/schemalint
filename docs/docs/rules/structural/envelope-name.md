# envelope-name

> Category: **Structural** — overall schema structure triggers this rule

## Error Codes

| Profile | Code | Severity |
|---------|------|----------|
| openai.so.2026-04-30 | `OAI-S-envelope-name` | Forbid |
| anthropic.so.2026-04-30 | `ANT-S-envelope-name` | Forbid |

## Description

Provider request names must use the supported format and length.

## Provider Evidence

| Profile | Status | Source | Basis |
|---|---|---|---|
| anthropic.so.2026-04-30 | `inferred` | — | Schemalint applies Anthropic's documented tool-name character and length constraints to Structured Outputs request names. |
| openai.so.2026-04-30 | `inferred` | — | Schemalint applies the documented function-name character and length constraints to Structured Outputs request names. |

## Rationale

Provider SDK request envelopes impose constraints outside JSON Schema.

## Bad Example

```json
{"name":"bad name"}
```

## Good Example

```json
{"name":"safe_name"}
```
