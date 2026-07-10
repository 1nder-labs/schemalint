# Quick Start

## Your First Check

Create a simple JSON Schema file:

```json
{
  "type": "object",
  "additionalProperties": false,
  "properties": {
    "name": { "type": "string" },
    "age": { "type": "number" }
  },
  "required": ["name", "age"]
}
```

Run schemalint against OpenAI's profile:

```bash
schemalint check --profile openai.so.2026-04-30 schema.json
```

This schema is clean for OpenAI. Now introduce a problem — add a forbidden keyword:

```json
{
  "type": "object",
  "allOf": [{ "properties": { "x": { "type": "string" } } }],
  "additionalProperties": false,
  "properties": {
    "name": { "type": "string" }
  },
  "required": ["name"]
}
```

Run again:

```bash
$ schemalint check --profile openai.so.2026-04-30 schema.json
error[OAI-K-allOf]: keyword 'allOf' is not supported by openai.so.2026-04-30
  --> schema.json
```

## Output Formats

```bash
# JSON output (useful for tooling)
schemalint check --format json --profile openai.so.2026-04-30 schema.json

# GitHub Actions annotations
schemalint check --format gha --profile openai.so.2026-04-30 schema.json

# SARIF (VS Code, GitHub Code Scanning)
schemalint check --format sarif --profile openai.so.2026-04-30 schema.json
```

JSON output uses schema version `1.1`. In addition to the stable 1.0 fields, the
`report.coverage` object records attempted, excluded, discovered, checked, and
failed targets plus a status of `complete`, `empty`, `partial`, or `failed`.

## Strict Completeness

Exit `0` means every discovered in-scope schema was evaluated and checked and
there were no error diagnostics. An empty match, import/evaluation/conversion
failure, unresolved required SDK envelope field, or partially checked batch
exits `1`. `--continue-on-discovery-error` only controls whether later sources
are attempted; it never hides incomplete coverage.

## Check All JSON Schemas in a Directory

```bash
schemalint check --profile openai.so.2026-04-30 schemas/
```

## Check with Multiple Profiles

```bash
schemalint check --profile openai.so.2026-04-30 --profile anthropic.so.2026-04-30 schema.json
```
