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

Exit `0` means every discovered in-scope target was evaluated and checked and
there were no error diagnostics. Empty discovery, import/evaluation/conversion
failures, unresolved required SDK metadata, and partial batches exit `1`.
`--continue-on-discovery-error` collects later targets but never hides partial
coverage. JSON output uses schema version `1.1` and reports the authoritative
status in `report.coverage`.

## SDK and runtime compatibility

SchemaLint matches canonical, aliased, and namespace imports by their exact
module/export surface; it does not guess from a package version string. The
registry-tarball endpoints below were verified on 2026-07-09 and are retained
in `src/__tests__/fixtures/sdk-version-matrix.json` so future updates are
explicit rather than silently redefining "current".

| SDK surface | Verified versions | Recognized schema call |
| --- | --- | --- |
| AI SDK | Floor `ai@6.0.0`; current `ai@7.0.19` | `Output.object({ schema })`, `Output.array({ element })`, `dynamicTool({ inputSchema })` |
| AI SDK legacy | Deprecated in AI SDK 6; retained by SchemaLint 1.x | `generateObject({ schema })`, `streamObject({ schema })`, `tool({ inputSchema \| parameters })` |
| OpenAI JavaScript SDK | `zodTextFormat` floor `openai@4.87.0`; current `6.46.0` | `zodTextFormat(schema, name)` from `openai/helpers/zod` |
| OpenAI JavaScript SDK | `zodResponseFormat` floor `openai@4.55.0`; current `6.46.0` | `zodResponseFormat(schema, name)` from `openai/helpers/zod` |
| OpenAI JavaScript SDK legacy | `zodFunction` floor `openai@4.55.0`; retained by SchemaLint 1.x | `zodFunction({ name, parameters })` from `openai/helpers/zod` |
| Anthropic TypeScript SDK | `zodOutputFormat` floor `0.72.0`; current `0.110.0` | `zodOutputFormat(schema)` from `@anthropic-ai/sdk/helpers/zod` |
| Anthropic TypeScript SDK legacy | `betaZodTool` floor `0.63.0`; retained by SchemaLint 1.x | `betaZodTool({ name, inputSchema })` from `@anthropic-ai/sdk/helpers/beta/zod` |

Legacy rows are deprecated in SchemaLint and will be removed in SchemaLint 2.0.

The runtime supports Node 18, 20, and 22 and bundles its TypeScript loader.
Zod `>=3.20` is supported across v3, v4 from `4.0.1` through current `4.4.3`, and
`zod/mini`; no global `tsx`, TypeScript, or repository dev dependency is used.
Generic AI SDK targets are provider-ambiguous, so select the intended profile
explicitly when package/source evidence cannot do so.

## Providers

| Provider | Profile |
| --- | --- |
| OpenAI Structured Outputs | `openai.so.2026-04-30` |
| Anthropic Structured Outputs | `anthropic.so.2026-04-30` |

schemalint exits non-zero on errors, so it fails the build before a broken schema ships. Output formats: `human` (default), `json`, `sarif`, `gha`.

Ingestion and JSON-RPC caches are bounded and process-local. SchemaLint does
not persist source schemas or normalized schema data to disk.

## Documentation

Full guide, profile reference, and CI recipes: **https://1nder-labs.github.io/schemalint**

## License

Dual-licensed under MIT or Apache-2.0, at your option.
