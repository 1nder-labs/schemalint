# schemalint

Static analysis tool for JSON Schema compatibility with LLM structured-output providers.

## Installation

```bash
cargo install schemalint
```

## Usage

```bash
schemalint check --profile openai.so.2026-04-30.toml schema.json
```

## Performance Targets

- Single 200-property nested schema, single profile: < 1 ms
- Project of 500 schemas, 1 profile, cold start: < 500 ms
- Incremental run within a single batch invocation (cache hit): < 5 ms

## License

MIT OR Apache-2.0
