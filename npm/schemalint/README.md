<p align="center">
  <img src="https://raw.githubusercontent.com/1nder-labs/schemalint/main/assets/schemalint-header.png" alt="schemalint" width="100%">
</p>

<h1 align="center">@1nder-labs/schemalint</h1>

<p align="center">
  <b>Catch provider-incompatible schemas before OpenAI or Anthropic reject them at runtime.</b>
</p>

<p align="center">
  <img src="https://raw.githubusercontent.com/1nder-labs/schemalint/main/assets/Schemalint.gif" alt="schemalint catching a provider-incompatible schema" width="90%">
</p>

OpenAI and Anthropic structured-output APIs accept only a strict subset of JSON Schema. Ship one with an unsupported keyword, a missing `required` entry, or the wrong `additionalProperties`, and the API rejects it in production as a `400`. **schemalint catches those errors at build time** — so a bad schema fails your CI instead of your users' requests.

One package, one `schemalint` command — JSON Schema, Zod, and Pydantic all in the box.

## Install

```bash
npm install -D @1nder-labs/schemalint
# or globally:  npm install -g @1nder-labs/schemalint
# or with bun:  bun add -d @1nder-labs/schemalint
```

## Quick start

```bash
schemalint check --profile openai.so.2026-04-30 schema.json
```

```text
error[OAI-K-allOf]: keyword 'allOf' is not supported by openai.so.2026-04-30
  --> schema.json

1 issue found (1 error, 0 warnings) across 1 schema
```

Check a directory for both providers at once:

```bash
schemalint check \
  --profile openai.so.2026-04-30 \
  --profile anthropic.so.2026-04-30 \
  schemas/
```

## Lint Zod directly

Schemas live in TypeScript? schemalint reads them straight from your Zod definitions — no JSON Schema export needed:

```jsonc
// package.json
{
  "scripts": { "lint:schemas": "schemalint check-node" },
  "schemalint": {
    "profiles": ["openai.so.2026-04-30"],
    "include": ["src/**/*.ts"]
  }
}
```

```bash
npm run lint:schemas
```

## Providers

| Provider | Profile |
| --- | --- |
| OpenAI Structured Outputs | `openai.so.2026-04-30` |
| Anthropic Structured Outputs | `anthropic.so.2026-04-30` |

schemalint exits non-zero on errors, so it fails the build before a broken schema ships. Output formats: `human` (default), `json`, `sarif`, `gha`.

## Documentation

Full guide, profile reference, and CI recipes: **https://1nder-labs.github.io/schemalint**

## License

Dual-licensed under MIT or Apache-2.0, at your option.
