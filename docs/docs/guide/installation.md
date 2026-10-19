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

The npm package supports Node 18, 20, and 22. On macOS and Linux its install
script downloads and verifies the platform binary and puts it at the package's
`bin/schemalint` entry, so the command starts as fast as the native binary
(about 2 ms) with no Node process in front of it. It also bundles the
TypeScript loader and Zod ingestor.

pnpm and bun block dependency install scripts by default. schemalint still
works then: a small Node launcher downloads and verifies the binary on first
run, at the cost of Node start-up (about 40 ms) on every command. Allow the
script to get the native start-up:

```bash
pnpm add -g --allow-build=@1nder-labs/schemalint @1nder-labs/schemalint
pnpm approve-builds                      # in a project
bun add -g --trust @1nder-labs/schemalint
bun pm -g trust @1nder-labs/schemalint   # already installed
```

Windows always uses the Node launcher.
Supported schemas include Zod v3 (`>=3.20`), Zod v4 from `4.0.1` through
current `4.4.3`, and `zod/mini`; no global `tsx` or TypeScript installation is
required.

## Verify Installation

```bash
schemalint --version
```

All ingestion and JSON-RPC caches are bounded and process-local. SchemaLint
does not persist source schemas or normalized schema data to disk.
