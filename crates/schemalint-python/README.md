<p align="center">
  <br>
  <h1 align="center">schemalint</h1>
  <p align="center">Static analysis for JSON Schema compatibility with LLM structured-output providers.</p>
  <p align="center">
    <a href="https://github.com/1nder-labs/schemalint/actions/workflows/ci.yml"><img src="https://github.com/1nder-labs/schemalint/actions/workflows/ci.yml/badge.svg?event=push&branch=main" alt="CI"></a>
    <a href="https://crates.io/crates/schemalint"><img src="https://img.shields.io/crates/v/schemalint" alt="Crates.io"></a>
    <a href="https://docs.rs/schemalint"><img src="https://img.shields.io/docsrs/schemalint" alt="Docs.rs"></a>
    <a href="https://github.com/1nder-labs/schemalint/blob/main/LICENSE"><img src="https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue" alt="License"></a>
    <img src="https://img.shields.io/badge/MSRV-1.80-red" alt="MSRV 1.80">
  </p>
</p>

---

![schemalint demo](assets/Schemalint.gif)

Lint your JSON Schemas against provider capability profiles **before** you send them to the LLM API. Catches unsupported keywords, structural violations, and restricted values at build time instead of at runtime.

```bash
schemalint check --profile openai.so.2026-04-30 schema.json
```

## Supported Providers

| Provider | Profile ID |
|----------|-----------|
| OpenAI Structured Outputs | `openai.so.2026-04-30` |
| Anthropic Structured Outputs | `anthropic.so.2026-04-30` |

Check multiple profiles simultaneously — each produces its own tagged diagnostics:

```bash
schemalint check \
  --profile openai.so.2026-04-30 \
  --profile anthropic.so.2026-04-30 \
  schema.json
```

## Install

```bash
cargo install schemalint
# or build from source
cargo build --workspace --release
```

## Features

### Rule engine

- **Class A keyword rules** — auto-generated from the provider profile TOML. Forbid, warn, or allow any JSON Schema keyword per provider.
- **Class B structural rules** — configurable limits on nesting depth, total properties, enum cardinality, string-length budgets, root type requirements, `additionalProperties: false`, and required-field completeness.
- **Semantic rules** — provider-aware heuristics that catch structural anti-patterns:
  - Empty objects with `additionalProperties: false`
  - `additionalProperties` specified as an object schema instead of `false`
  - `anyOf` over object-typed branches
  - `allOf` combined with `$ref` (Anthropic-specific)
- **Value restrictions** — restrict keyword values to provider-approved sets (e.g., `format` limited to `date-time`, `email`, `uuid`).

### Multi-profile orchestration

- Run N profiles in a single invocation with `--profile` repeated.
- Each diagnostic is tagged with its originating profile.
- Profiles are deduplicated by name.
- Built-in profile IDs resolve without filesystem paths.

### Output formats

| Format | Flag | Use case |
|--------|------|----------|
| Human-readable | `--format human` (default in TTY) | Terminal inspection |
| JSON | `--format json` | Scripting, programmatic consumers |
| SARIF v2.1.0 | `--format sarif` | GitHub Code Scanning, Azure DevOps |
| GitHub Actions | `--format gha` | CI workflow annotations (`::error` / `::warning`) |
| JUnit XML | `--format junit` | GitLab, Jenkins, CircleCI test reports |

### JSON-RPC server

Long-running server mode over stdin/stdout with line-delimited JSON-RPC 2.0:

```bash
schemalint server
```

```json
{"jsonrpc":"2.0","method":"check","params":{"schema":{...},"profiles":["openai.so.2026-04-30"],"format":"json"},"id":1}
{"jsonrpc":"2.0","method":"shutdown","id":2}
```

- **Persistent disk cache** — normalized schemas survive process restarts via `~/.cache/schemalint-<pid>/`.
- **Hardened** — 10 MB payload limit, 100k node limit, path traversal protection, broken pipe detection.

### Performance

Measured on Apple M3:

| Scenario | Time |
|----------|------|
| Single 200-property schema | < 1 ms |
| 500 schemas, cold start | < 50 ms |
| Incremental (cache hit) | < 5 ms |

- **Content-hash caching** — identical schemas across files are normalized once.
- **Rayon parallelism** — file processing scales across all cores.
- **Arena-allocated IR** — `NodeId(u32)` indexing with zero pointer indirection.

## Usage

```bash
# Check a single schema
schemalint check --profile openai.so.2026-04-30 schema.json

# Check all .json files in a directory
schemalint check --profile openai.so.2026-04-30 ./schemas/

# Output structured JSON
schemalint check --profile openai.so.2026-04-30 --format json schema.json

# CI-friendly output
schemalint check --profile openai.so.2026-04-30 --format gha schema.json

# Multi-profile check
schemalint check \
  --profile openai.so.2026-04-30 \
  --profile anthropic.so.2026-04-30 \
  --format sarif \
  ./schemas/

# Write output to file
schemalint check --profile openai.so.2026-04-30 --output results.json schema.json

# Start JSON-RPC server
schemalint server
```

## Output

### Human

```
error[OAI-K-allOf]: keyword 'allOf' is not supported by openai.so.2026-04-30
   --> schema.json
     |
     = profile: openai.so.2026-04-30
     = schema path: /
     = see: https://schemalint.dev/rules/OAI-K-allOf

1 issue found (1 error, 0 warnings) across 1 schema
```

### SARIF

```json
{
  "version": "2.1.0",
  "runs": [{
    "tool": { "driver": { "name": "schemalint", "rules": [...] } },
    "results": [{
      "ruleId": "OAI-K-allOf",
      "level": "error",
      "message": { "text": "keyword 'allOf' is not supported..." },
      "locations": [{ "physicalLocation": { "artifactLocation": { "uri": "schema.json" } } }]
    }]
  }]
}
```

### GitHub Actions

```
::error file=schema.json,title=OAI-K-allOf::keyword 'allOf' is not supported...
```

### JUnit

```xml
<testsuites>
  <testsuite name="schema.json" tests="1" failures="1" skipped="0" errors="0" time="0">
    <testcase name="OAI-K-allOf - keyword 'allOf'...">
      <failure type="error" message="keyword 'allOf'...">keyword 'allOf'...</failure>
    </testcase>
  </testsuite>
</testsuites>
```

## Exit codes

| Code | Meaning |
|------|---------|
| `0` | No errors (warnings alone are OK) |
| `1` | At least one error, or a fatal parse/IO error |
| `2` | I/O error writing output file |

## Architecture

```
crates/
├── schemalint/              # Core engine + CLI
│   ├── src/cli/             # Args, file discovery, server, output formatters
│   ├── src/ir/              # Arena-allocated IR (Node, NodeId, Arena)
│   ├── src/normalize/       # Normalizer pipeline (dialect, $ref resolution, desugar)
│   ├── src/profile/         # TOML profile loader
│   ├── src/rules/           # Rule trait, registry, Class A/B + semantic rules
│   ├── tests/               # 128 tests across 12 test files
│   └── benches/             # Criterion benchmarks
└── schemalint-profiles/     # Bundled provider profiles (zero deps)
    └── profiles/
        ├── openai.so.2026-04-30.toml
        └── anthropic.so.2026-04-30.toml
```

## Contribute

```bash
cargo build --workspace
cargo test --workspace
cargo clippy --workspace -- -D warnings
cargo fmt --all -- --check
cargo bench --no-run --workspace
```

Pre-commit hooks via [lefthook](https://github.com/evilmartians/lefthook) run fmt, clippy, and tests on staged `.rs` files.

MSRV: **1.80**.

## License

MIT OR Apache-2.0
