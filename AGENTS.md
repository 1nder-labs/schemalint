# Agent Instructions

## Workspace

Five-crate Rust workspace plus npm and TypeScript packages:
- `crates/schemalint` — core engine, CLI, and all tests
- `crates/schemalint-profiles` — built-in TOML profiles and conformance truth files (zero deps)
- `crates/schemalint-docgen` — rule documentation generator (`publish = false`)
- `crates/schemalint-conformance` — conformance mock server and truth engine (`publish = false`)
- `crates/schemalint-python` — maturin PyPI packaging crate (`publish = false`)
- `npm/cli/` — npm auto-download binary wrapper (`@schemalint/cli`)
- `npm/core/` — npm programmatic API placeholder (`@schemalint/core`, requires CLI on PATH)
- `typescript/schemalint-zod/` — Zod ingestion helper (`@schemalint/zod`)

Entrypoints:
- CLI binary: `crates/schemalint/src/main.rs` → `schemalint::cli::run()`
- Library root: `crates/schemalint/src/lib.rs` re-exports `cache`, `cli`, `ir`, `normalize`, `profile`, `rules`
- Docgen binary: `crates/schemalint-docgen/src/main.rs`
- Conformance server: `crates/schemalint-conformance/src/main.rs`
- PyPI wrapper: `crates/schemalint-python/src/main.rs` → `schemalint::cli::run()`

## Documentation

- Docusaurus v3.10 site at `docs/` — `npm run start` in `docs/` for local dev
- Auto-generated rule reference pages at `docs/docs/rules/` — run `cargo run --bin schemalint-docgen` to regenerate
- Docs deployed to GitHub Pages via `.github/workflows/docs.yml` on push to main

## Build & Verify

```bash
cargo build --workspace
cargo test --workspace
cargo clippy --workspace -- -D warnings
cargo fmt --all -- --check
cargo bench --no-run --workspace
```

MSRV: 1.80

## Test Commands

- `cargo test --workspace` — all tests
- `cargo test --test ir_tests` — IR and parser tests
- `cargo test --test profile_tests` — profile loader tests
- `cargo test --test normalizer_tests` — normalizer pipeline tests
- `cargo test --test rules_tests` — rule registry and Class A tests
- `cargo test --test structural_tests` — Class B structural rule tests
- `cargo test --test cli_tests` — CLI argument and output tests
- `cargo test --test integration_tests` — end-to-end CLI tests (uses `assert_cmd` + `predicates`)
- `cargo test --test snapshot_tests` — human/JSON output snapshots (`insta`)
- `cargo test --test property_tests` — normalizer round-trip properties (`proptest`)
- `cargo test --test corpus_tests` — regression corpus validation

Snapshot updates: set `INSTA_UPDATE=always` or review `.snapshots/new` before accepting.

## Conventions

- Arena-allocated IR via `Vec<Node>` indexed by `NodeId(u32)`.
- `IndexMap` for keyword order preservation.
- `linkme` distributed slice `RULES` for compile-time rule auto-registration. Add new rules to the slice; `RuleSet::from_profile` generates dynamic Class A / Class B rules from a loaded profile.
- Error codes: `{code_prefix}-K-*` for keyword rules, `{code_prefix}-K-<keyword>-restricted` for value restrictions, `{code_prefix}-S-*` for structural rules. The `code_prefix` field in the profile TOML drives the prefix (e.g. `OAI` for OpenAI, `ANT` for Anthropic).
- Exit code: `0` if no errors (warnings are OK), `1` on any error or fatal parse/IO error.
- Documented solutions in `docs/solutions/` — organized by category with YAML frontmatter (`module`, `tags`, `problem_type`). Relevant when implementing or debugging in documented areas.

## Regression Corpus

`crates/schemalint/tests/corpus/` contains synthetic schemas and `.expected` JSON files. Do not silently update `.expected` files; any change must be explicitly reviewed.

## Benchmarks

Criterion benchmarks live in `crates/schemalint/benches/`. Smoke-test with `cargo bench --workspace`.

## Coding Principles

- **Think before coding.** State assumptions. If unclear, ask. Surface tradeoffs; don't pick silently.
- **Simplicity first.** Minimum code that solves the problem. No speculative abstractions. No features beyond the ask. If it feels overcomplicated, simplify.
- **Surgical changes only.** Touch what the request requires. Match existing style. Clean up orphans your changes create; don't refactor unrelated code.
- **Goal-driven execution.** Define verifiable success criteria before implementing. Loop until verified.
- **Max 400 LOC per file.** Split early.
- **Prefer explicit over DRY.** A little duplication is fine if it aids clarity and modularity.
- **Extremely typed.** Leverage the type system; avoid `String` passthroughs, raw integers, or untyped maps where domain types fit.
- **Best practices only.** Follow idiomatic Rust, standard patterns, and existing repo conventions.
