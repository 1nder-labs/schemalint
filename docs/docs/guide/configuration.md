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

Only schemas that reach a model are worth linting, so `check-node` traces
each schema from a provider call site (`ai`, `openai/helpers/zod`,
`@anthropic-ai/sdk/helpers/zod`) back to where it is defined, through any
number of wrapper functions that forward it as a parameter, a destructured
binding, or a spread object.

What a source entry names decides how much else is linted:

| Source | Scope |
| --- | --- |
| A glob (`src/**/*.ts`) | Schemas traced to a provider call site. If none of the matched files import a provider SDK, every exported `z.object` is linted instead. If they do but nothing could be traced, the run checks nothing and says so. |
| A file or directory path (`src/models/`, `src/schemas.ts`) | Everything above, plus every exported `z.object` in those files. Use this to lint a schema module directly. |

Each schema is evaluated in isolation: from its own module, only the
declarations and imports it depends on are loaded, so a module that also
imports runtime-only bindings (`cloudflare:workers`, a database client, an
HTTP app) evaluates cleanly. A local module the schema imports from is still
loaded whole. Tracing and the SDK check see only the files the source entry
selects, so a scope that leaves out the wrapper module finds only exports.

