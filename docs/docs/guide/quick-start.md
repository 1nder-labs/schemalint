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
  }
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

## Check All JSON Schemas in a Directory

```bash
schemalint check --profile openai.so.2026-04-30 schemas/
```

## Check with Multiple Profiles

```bash
schemalint check --profile openai.so.2026-04-30 --profile anthropic.so.2026-04-30 schema.json
```
