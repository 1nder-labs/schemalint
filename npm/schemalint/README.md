# @1nder-labs/schemalint

Lint JSON Schema and Zod schemas before OpenAI or Anthropic structured-output APIs reject them.

This is the schemalint CLI as a single npm package. Installing it adds a `schemalint` command that downloads the native binary for your platform on first use. The Zod ingestor (TypeScript AST discovery + JSON-RPC server) is bundled in `bin/` and `dist/` so no extra package or install step is needed.

## Install

```bash
npm install -g @1nder-labs/schemalint
```

Or as a project dev dependency:

```bash
npm install -D @1nder-labs/schemalint
# bun add -d @1nder-labs/schemalint
```

## Quick Start

```bash
schemalint check --profile openai.so.2026-04-30 schemas/
```

Lint for both OpenAI and Anthropic:

```bash
schemalint check \
  --profile openai.so.2026-04-30 \
  --profile anthropic.so.2026-04-30 \
  schemas/
```

Add it as a package script so CI can run it:

```json
{
  "scripts": {
    "schema": "schemalint check --profile openai.so.2026-04-30 schemas/"
  }
}
```

## Documentation

Full docs, profile reference, and Zod configuration guide at
https://1nder-labs.github.io/schemalint

## License

MIT OR Apache-2.0
