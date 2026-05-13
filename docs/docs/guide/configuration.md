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
