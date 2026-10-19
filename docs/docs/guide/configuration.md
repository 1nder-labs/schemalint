# Configuration

Schemalint can be configured through CLI arguments, language-specific config files, or both.

## CLI Arguments

```bash
schemalint check [OPTIONS] [PATHS]...

OPTIONS:
  -p, --profile <PROFILE>    Provider profile ID or TOML path (can be specified multiple times)
  -f, --format <FORMAT>      Output format: human, json, sarif, gha, junit
  -o, --output <OUTPUT>      Write output to a file instead of stdout
```

## For AI agents

Agents should read the bundled, version-matched usage guide before running the CLI:

```bash
schemalint skills get core          # core loop, monorepo usage, exit codes, common fixes
schemalint skills get core --full   # plus the full command reference
schemalint skills list              # list bundled skills
```

`schemalint --help` points to the same guide at the top of its output.

## Python (pyproject.toml)

```toml
[tool.schemalint]
profiles = ["openai.so.2026-04-30"]
include = ["src/models/"]
```

## Node.js (package.json)

```json
{
  "schemalint": {
    "profiles": ["openai.so.2026-04-30"],
    "include": ["src/models/"]
  }
}
```

## How Zod schemas are selected (`check-node`)

`check-node` lints every Zod object schema the files in `--source` can build
without running a function. It does not matter whether the schema is declared
or inline, exported or not, at module scope or inside a function body, or how
it is spelled (`z.object`, `z.strictObject`, `z.looseObject`, a namespace
import, `zod/mini`, a factory call). Barrel re-exports are followed to the
definition, and a schema reached twice is reported once.

Two kinds of schema are not checked:

- Schemas built from run-time input (they use a parameter, a local variable, or
  `this`) are skipped. No static tool can check them.
- A schema Zod cannot convert to JSON Schema (`Date`, transforms, custom types)
  is reported as a warning and skipped. It does not fail the run.

Globs, files, and directories all follow the same rule. Use `--source` and
`--exclude` to set the scope.

Every schema is checked against every selected profile. Without `--profile`,
providers come from `package.json` dependencies: `openai` or `@ai-sdk/openai`
selects OpenAI, `@anthropic-ai/sdk` or `@ai-sdk/anthropic` selects Anthropic,
and both together select both. With neither, OpenAI is used.

Each schema is evaluated in isolation: from its own module, only the
declarations and imports it depends on are loaded, so a module that also
imports runtime-only bindings (`cloudflare:workers`, a database client, an
HTTP app) evaluates cleanly. A local module the schema imports from is still
loaded whole.

