# Contributing to schemalint

schemalint is a JSON Schema / Zod / Pydantic linter that catches provider-incompatible
schemas before OpenAI and Anthropic structured-output APIs reject them. We welcome bug
reports, feature requests, documentation improvements, and code contributions of all sizes.
Thank you for taking the time to help make schemalint better.

## Code of Conduct

Please read and follow our [Code of Conduct](./CODE_OF_CONDUCT.md).

## Reporting Bugs and Requesting Features

Open an issue at <https://github.com/1nder-labs/schemalint/issues>. The repository has
issue templates — please fill them out so maintainers can reproduce or evaluate your
report quickly.

**Security vulnerabilities must not be reported as public issues.** Follow the process
described in [SECURITY.md](./SECURITY.md) to report them privately via GitHub Security
Advisories.

## Project Layout

```
crates/
  schemalint/           # core engine, CLI, built-in profiles, and all tests
  schemalint-docgen/    # rule-documentation generator (publish = false)
  schemalint-conformance/ # conformance mock server (publish = false)
  schemalint-python/    # maturin crate for PyPI packaging (publish = false)
npm/schemalint/         # @1nder-labs/schemalint: native-binary launcher + Zod sidecar
docs/                   # Docusaurus v3 site
```

The CLI entry point is `crates/schemalint/src/main.rs`; the library root is
`crates/schemalint/src/lib.rs`.

## Development Setup

### Rust

Install Rust via [rustup](https://rustup.rs/). MSRV is **1.80**; the
`rust-toolchain.toml` in the repo root pins the toolchain automatically.

```bash
rustup update
cargo build --workspace
```

### Node / Zod sidecar

Node **18 or later** is required for the npm package and its bundled Zod ingestor.

```bash
cd npm/schemalint
npm ci
npm run build
```

## Build, Test, and Lint

### Rust workspace

```bash
# Run all tests
cargo test --workspace

# Lint (exclude the maturin crate — it requires a Python build environment)
cargo clippy --workspace --exclude schemalint-python -- -D warnings

# Check formatting
cargo fmt --all -- --check
```

A [lefthook](https://github.com/evilmartians/lefthook) pre-commit hook runs `fmt`,
`clippy`, and `cargo test --workspace` automatically on every commit. Install it once
with:

```bash
lefthook install
```

### npm / Zod

```bash
cd npm/schemalint
npm test
```

## Commit Messages

This project uses [Conventional Commits](https://www.conventionalcommits.org/). The
generated `CHANGELOG.md` is derived from commit history, so please use the right prefix:

| Prefix | When to use |
| ----------- | ------------------------------------ |
| `feat:` | New user-visible feature |
| `fix:` | Bug fix |
| `docs:` | Documentation only |
| `refactor:` | Code change with no behaviour change |
| `test:` | Tests only |
| `ci:` | CI / workflow changes |
| `chore:` | Tooling, dependencies, housekeeping |

Breaking changes: append `!` after the type (e.g. `feat!:`) and add a
`BREAKING CHANGE:` footer.

## Pull Requests

1. **Fork** the repository and create a branch off `main`.
2. Keep PRs **focused** — one logical change per PR.
3. Ensure **CI is green** before requesting review.
4. **Reference the related issue** in the PR description (e.g. `Closes #42`).
5. PRs are merged squash-style; the PR title becomes the commit message, so make it a
   valid Conventional Commit subject line.

## License

By contributing to schemalint you agree that your contributions will be dual-licensed
under **MIT OR Apache-2.0**, matching the project's existing license.
