# Installation

## Cargo (Rust)

```bash
cargo install schemalint
```

This installs the `schemalint` binary to your Cargo bin directory (typically `~/.cargo/bin`). Requires Rust 1.80+.

## GitHub Releases

Pre-built binaries are available for:

- Linux (x86_64, aarch64)
- macOS (x86_64, aarch64)
- Windows (x86_64)

Download the latest release from [GitHub Releases](https://github.com/1nder-labs/schemalint/releases).

## PyPI

```bash
python -m pip install schemalint
```

The wheel supports Python 3.9+ and installs the actual `schemalint` command plus
the bundled `schemalint_pydantic` discovery sidecar — no Rust toolchain or
second SchemaLint package is required. `check-python` uses the Pydantic already
installed in your environment; Pydantic 1.10 and 2.x are supported, and the
wheel never installs or upgrades Pydantic at runtime.

## npm

```bash
npm install -g @1nder-labs/schemalint
```

For a project-local CI install, prefer:

```bash
npm install --save-dev @1nder-labs/schemalint
npx schemalint --version
```

The npm package supports Node 18, 20, and 22. It downloads and verifies the
platform binary on first use and bundles the TypeScript loader and Zod ingestor.
Supported schemas include Zod v3 (`>=3.20`), Zod v4 from `4.0.1` through
current `4.4.3`, and `zod/mini`; no global `tsx` or TypeScript installation is
required.

## SDK source compatibility

The endpoints below were verified from official npm registry tarballs on
2026-07-09. The dated test fixture in the npm package records the same evidence;
"current" is a captured endpoint, not a floating compatibility claim.

| SDK helper | First verified package | Current verified package |
| --- | --- | --- |
| AI SDK `Output.object`, `Output.array`, `dynamicTool` | `ai@6.0.0` | `ai@7.0.19` |
| OpenAI `zodTextFormat` | `openai@4.87.0` | `openai@6.46.0` |
| OpenAI `zodResponseFormat` | `openai@4.55.0` | `openai@6.46.0` |
| Anthropic `zodOutputFormat` | `@anthropic-ai/sdk@0.72.0` | `@anthropic-ai/sdk@0.110.0` |

SchemaLint also recognizes AI SDK `generateObject`, `streamObject`, and `tool`,
OpenAI `zodFunction` (floor `openai@4.55.0`), and Anthropic `betaZodTool` from
`@anthropic-ai/sdk/helpers/beta/zod` (floor `0.63.0`). Those surfaces are
deprecated in SchemaLint 1.x and will be removed in SchemaLint 2.0. Canonical,
aliased, and namespace imports are supported.

## Verify Installation

```bash
schemalint --version
```

All ingestion and JSON-RPC caches are bounded and process-local. SchemaLint
does not persist source schemas or normalized schema data to disk.
