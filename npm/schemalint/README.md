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

The install script puts the verified native binary at the `schemalint` entry,
so the command starts in about 2 ms on macOS and Linux. pnpm and bun block that
script by default; the command still works through a Node launcher (about
40 ms per run). Allow it with `pnpm add -g --allow-build=@1nder-labs/schemalint`,
`pnpm approve-builds` in a project, or `bun add --trust`.

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
there were no error diagnostics. Import/evaluation/conversion
failures, and partial batches exit `1`; empty
discovery (nothing checked) exits `3`.
`--continue-on-discovery-error` collects later targets but never hides partial
coverage. JSON output uses schema version `1.1` and reports the authoritative
status in `report.coverage`.

## Runtime compatibility

The runtime supports Node 18, 20, and 22 and bundles its TypeScript loader.
Zod `>=3.20` is supported across v3, v4 from `4.0.1` through current `4.4.3`, and
`zod/mini`; no global `tsx`, TypeScript, or repository dev dependency is used.

`check-node` lints every Zod object schema the files in `--source` can build
without running a function, whether exported or not. Schemas built from
run-time input are skipped, and a schema Zod cannot convert to JSON Schema is
reported as a warning. Without `--profile`, providers come from `package.json`
dependencies (`openai`, `@anthropic-ai/sdk`, or both), defaulting to OpenAI.

## Providers

| Provider | Profile |
| --- | --- |
| OpenAI Structured Outputs | `openai.so.2026-04-30` |
| Anthropic Structured Outputs | `anthropic.so.2026-04-30` |

schemalint exits non-zero on errors, so it fails the build before a broken schema ships. Output formats: `human` (default), `json`, `sarif`, `gha`.

Ingestion and JSON-RPC caches are bounded and process-local. SchemaLint does
not persist source schemas or normalized schema data to disk.

## For AI agents

Coding agents (Claude Code, Codex, Cursor, ...) should start with the usage guide that ships inside the binary, so it always matches the installed version:

```bash
schemalint skills get core          # workflow, monorepo usage, exit codes, common fixes
schemalint skills get core --full   # plus the full command reference
```

## Documentation

Full guide, profile reference, and CI recipes: **https://1nder-labs.github.io/schemalint**

## License

Dual-licensed under MIT or Apache-2.0, at your option.
