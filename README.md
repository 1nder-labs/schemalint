<p align="center">
  <img src="assets/schemalint-header.png" alt="schemalint validates schemas before provider APIs reject them" width="100%">
</p>

<h1 align="center">schemalint</h1>

<p align="center">
  Lint schemas before OpenAI or Anthropic structured-output APIs reject them.
</p>

<p align="center">
  <a href="https://github.com/1nder-labs/schemalint/actions/workflows/ci.yml"><img src="https://github.com/1nder-labs/schemalint/actions/workflows/ci.yml/badge.svg?event=push&branch=main" alt="CI"></a>
  <a href="https://crates.io/crates/schemalint"><img src="https://img.shields.io/crates/v/schemalint" alt="Crates.io"></a>
  <a href="https://docs.rs/schemalint"><img src="https://img.shields.io/docsrs/schemalint" alt="Docs.rs"></a>
  <a href="https://github.com/1nder-labs/schemalint/blob/main/LICENSE"><img src="https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue" alt="License"></a>
</p>

schemalint catches provider-incompatible JSON Schema at build time: unsupported keywords, missing `required` entries, invalid root shapes, `additionalProperties` mistakes, size limits, and provider-specific restrictions.

New install? Use `@1nder-labs/cli`. The installed binary is `schemalint`.

## Install

```bash
# Bun / npm projects
bun add -d @1nder-labs/cli
# or: npm install -D @1nder-labs/cli

# Rust
cargo install schemalint

# Python
pip install schemalint
```

## Quick Start

Add a package script:

```json
{
  "scripts": {
    "schema": "schemalint check --profile openai.so.2026-04-30 schemas/"
  }
}
```

Run it:

```bash
bun schema
```

Lint for both OpenAI and Anthropic:

```json
{
  "scripts": {
    "schema": "schemalint check --profile openai.so.2026-04-30 --profile anthropic.so.2026-04-30 schemas/"
  }
}
```

## Zod

Install the Zod helper when schemas live in TypeScript:

```bash
bun add -d @1nder-labs/cli @1nder-labs/zod
```

Configure discovery in `package.json`:

```json
{
  "scripts": {
    "schema": "schemalint check-node"
  },
  "schemalint": {
    "profiles": ["openai.so.2026-04-30"],
    "include": ["src/**/*.ts"],
    "exclude": ["**/*.test.ts"]
  }
}
```

schemalint can auto-detect OpenAI or Anthropic imports, but explicit profiles are better for CI.

## CLI

```bash
schemalint check --profile openai.so.2026-04-30 schema.json
schemalint check --profile openai.so.2026-04-30 schemas/

# Run the local binary directly with Bun
bun schemalint check --profile openai.so.2026-04-30 schemas/
```

Supported profiles:

| Provider | Profile |
| --- | --- |
| OpenAI Structured Outputs | `openai.so.2026-04-30` |
| Anthropic Structured Outputs | `anthropic.so.2026-04-30` |

## Output

```bash
schemalint check --profile openai.so.2026-04-30 schema.json
schemalint check --format json --profile openai.so.2026-04-30 schema.json
schemalint check --format gha --profile openai.so.2026-04-30 schema.json
schemalint check --format sarif --profile openai.so.2026-04-30 schema.json
```

Exit codes:

| Code | Meaning |
| --- | --- |
| `0` | No errors |
| `1` | Schema errors, parse errors, or read errors |
| `2` | Could not write the output file |

## Example

```bash
$ schemalint check --profile openai.so.2026-04-30 schema.json
error[OAI-K-allOf]: keyword 'allOf' is not supported by openai.so.2026-04-30
  --> schema.json

1 issue found (1 error, 0 warnings) across 1 schema
```

## Links

- [Installation](https://1nder-labs.github.io/schemalint/guide/installation)
- [Quick start](https://1nder-labs.github.io/schemalint/guide/quick-start)
- [OpenAI profile](https://1nder-labs.github.io/schemalint/profiles/openai)
- [Anthropic profile](https://1nder-labs.github.io/schemalint/profiles/anthropic)
- [Rule reference](https://1nder-labs.github.io/schemalint/rules)

## Development

```bash
cargo test --workspace
cargo clippy --workspace -- -D warnings
cargo fmt --all -- --check
```

MSRV: 1.80. License: MIT OR Apache-2.0.
