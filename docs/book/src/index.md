# Schemalint

Schemalint is a static analysis tool that checks your JSON Schemas for compatibility with LLM structured-output providers like OpenAI and Anthropic. It catches keywords, patterns, and structural choices that will be rejected or silently altered by a provider's API before you submit them.

## Features

- **Provider-aware**: Built-in profiles for OpenAI Structured Outputs and Anthropic Structured Outputs
- **Multi-format output**: Human-readable, JSON, SARIF, GitHub Actions annotations, JUnit XML
- **CI-ready**: Zero-config integration with GitHub Actions, pre-commit hooks, and any CI pipeline
- **Language integrations**: First-class helpers for Python (Pydantic) and Node.js (Zod)
- **JSON-RPC server**: Headless mode for editor and tool integration

## Quick Start

```bash
# Install via cargo
cargo install schemalint

# Check a schema against OpenAI
schemalint check --profile openai.so.2026-04-30 schema.json

# Check against Anthropic
schemalint check --profile anthropic.so.2026-04-30 schema.json
```

## Supported Providers

| Provider | Profile | Version |
|----------|---------|---------|
| OpenAI Structured Outputs | `openai.so.2026-04-30` | 2026-04-30 |
| Anthropic Structured Outputs | `anthropic.so.2026-04-30` | 2026-04-30 |

## License

Schemalint is licensed under MIT OR Apache-2.0.
