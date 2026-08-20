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
        run: npm install --global @1nder-labs/schemalint
      - name: Check schemas
        run: schemalint check --profile openai.so.2026-04-30 --format gha schemas/
```

For GHA annotations to appear inline in pull requests, use `--format gha`.

The gate succeeds only when coverage is `complete` and no error diagnostic was
produced. Empty globs, import failures, schema conversion failures, unresolved
required SDK metadata, and partial batches exit `1`. For machine processing,
`--format json` emits schema version `1.1` and exposes the authoritative status
at `report.coverage.status`.

For source schemas, invoke the same installed command:

```yaml
- run: schemalint check-node --profile openai.so.2026-04-30
- run: schemalint check-python --package my_app.models --profile openai.so.2026-04-30
```

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

The server uses newline-delimited JSON-RPC 2.0 over stdin/stdout; it does not
open a network port. Responses include the same strict `success` and additive
coverage report as CLI JSON output. Requests and caches are bounded and
process-local, and schema data is never persisted to disk.
