# Agent Instructions

## Build

```bash
cargo build --workspace
cargo test --workspace
cargo clippy --workspace
```

## Test Commands

- `cargo test --workspace` — all tests
- `cargo test --test ir_tests` — IR and parser tests
- `cargo test --test profile_tests` — profile loader tests
- `cargo test --test normalizer_tests` — normalizer pipeline tests
- `cargo test --test rules_tests` — rule registry and Class A tests
- `cargo test --test structural_tests` — Class B structural rule tests
- `cargo test --test cli_tests` — CLI argument and output tests
- `cargo test --test integration_tests` — end-to-end CLI tests
- `cargo test --test snapshot_tests` — human/JSON output snapshots
- `cargo test --test property_tests` — normalizer round-trip properties
- `cargo test --test corpus_tests` — regression corpus validation

## Conventions

- Arena-allocated IR via `Vec<Node>` indexed by `NodeId(u32)`.
- `IndexMap` for keyword order preservation.
- `linkme` distributed slices for rule auto-registration.
- Error codes: `OAI-K-*` for keyword rules, `OAI-S-*` for structural rules.
- Exit code: 0 if no errors, 1 if any error or fatal parse error.
