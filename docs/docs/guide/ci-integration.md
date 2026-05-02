# CI Integration

## GitHub Actions

```yaml
name: Lint Schemas
on: [push, pull_request]

jobs:
  schemalint:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Install schemalint
        run: cargo install schemalint
      - name: Check schemas
        run: schemalint check --profile openai.so.2026-04-30 --format gha schemas/
```

For GHA annotations to appear inline in pull requests, use `--format gha`.

## Pre-commit

```yaml
repos:
  - repo: local
    hooks:
      - id: schemalint
        name: schemalint
        entry: schemalint check --profile openai.so.2026-04-30
        language: system
        files: \.json$
```

## JSON-RPC Server

For headless CI and editor integration:

```bash
schemalint server
```

The server listens on a local port and accepts JSON-RPC 2.0 `check` requests.
