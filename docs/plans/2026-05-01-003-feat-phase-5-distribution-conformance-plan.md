---
title: "feat: Phase 5 — Distribution and Conformance Infrastructure"
type: feat
status: active
date: 2026-05-01
origin: docs/brainstorms/phase5-requirements.md
---

# feat: Phase 5 — Distribution and Conformance Infrastructure

## Summary

Deliver schemalint as a multi-platform installable tool (GitHub Releases binaries, PyPI wheel, npm packages, crates.io), build a mdBook documentation site with auto-generated rule reference pages, and create a three-tier conformance verification system — a local synthetic mock server driven by declarative truth files (co-located in `schemalint-profiles`), weekly live-API burn windows, and a monthly full corpus harness — all centralized into a provider-pluggable architecture. A unified GitHub Actions release workflow publishes all channels atomically from a single tag push.

---

## Problem Frame

schemalint cannot reach its primary users. Python and TypeScript developers — the two target audiences for the Pydantic and Zod ingestion helpers — can't install it without a Rust toolchain. There is no documentation beyond the README. There is no automated verification that the linter's rules match what the real provider APIs actually do: the only conformance check is ad-hoc Python scripts requiring manual API key invocation. The repository has no LICENSE file, a wrong Cargo.toml URL, and no CHANGELOG — all blockers for any distribution channel.

---

## Requirements

### Pre-Distribution
- R1. LICENSE file (MIT OR Apache-2.0) at repository root
- R2. `Cargo.toml` workspace `repository` field corrected to `https://github.com/1nder-labs/schemalint`
- R3. `CHANGELOG.md` following Keep a Changelog conventions

### Packaging
- R4. `cargo install schemalint` from crates.io
- R5. Standalone binaries for Linux (x86_64, aarch64), macOS (x86_64, aarch64), Windows (x86_64) on GitHub Releases
- R6. Python wheel on PyPI — `pip install schemalint` installs CLI on PATH, no Rust toolchain required
- R7. npm packages — `@schemalint/cli` (CLI on PATH), `@schemalint/core` (programmatic API), `@schemalint/zod` (existing helper)

### Documentation Site
- R8. mdBook site with getting-started guides, configuration reference, and per-rule reference pages
- R9. Rule reference pages auto-generated from compile-time rule registry metadata
- R10. Docs site published to stable URL, rebuilt on every push to main

### Conformance: Synthetic Mock
- R11. Mock server reads truth files from `schemalint-profiles/profiles/truth/`, returns Tier 3 fidelity (accept/reject + errors + transformation)
- R12. Truth files are declarative TOML artifacts authored from provider documentation, covering every keyword
- R13. Mock engine is provider-agnostic — new provider = new truth file, zero code changes
- R14. CI job on every PR/push runs mock server + linter consistency check; false positives and false negatives both fail

### Conformance: Live-API
- R15. Weekly burn windows — scheduled workflow submits keyword-exercise schemas to live APIs
- R16. Burn window report compares live responses against truth declarations, classifies divergence (merges origin R16+R17)
- R17. Monthly full harness — complete corpus through live APIs, compared against mock + `.expected` files (origin R18)
- R18. Full harness auto-files GitHub issues for detected provider drift (origin R19)

### Release Pipeline
- R19. Release workflow builds and publishes all artifacts from a single tag push
- R20. Pre-publish smoke tests on each artifact: install + `schemalint --version` + basic check invocation

**Origin actors:** A1 (Python developer), A2 (TypeScript developer), A3 (Rust developer), A4 (CI system), A5 (conformance mock server), A6 (schemalint maintainer)
**Origin flows:** F1 (install and run), F2 (browse docs), F3 (mock validates linter in CI), F4 (weekly burn window), F5 (monthly full harness), F6 (release workflow)
**Origin acceptance examples:** AE1 (cargo install / binary download), AE2 (pip install), AE3 (npm install), AE4 (mock + linter consistency), AE5 (add third provider), AE6 (burn window drift report), AE7 (full harness auto-issue), AE8 (atomic release)

---

## Scope Boundaries

- Docker image distribution — GitHub Releases, PyPI, npm, crates.io only
- Homebrew formula
- Conformance for providers beyond OpenAI and Anthropic in v1
- Provider-side conformance (validating user integration code)
- Auto-generation of linter profiles from truth files
- Auto-fix / schema rewriting (out of scope for all of v1)
- IDE / LSP integration

### Deferred to Follow-Up Work

- `schemalint-conformance` as an independently published crate — currently internal-use only; publication deferred until the truth file format stabilizes across multiple provider revisions
- napi-rs self-containment for `lint()` — Phase 4 `lint()` function requires `schemalint` CLI on PATH. napi-rs-native operation (no external binary dependency) is deferred to post-v1

---

## Context & Research

### Relevant Code and Patterns

- **Workspace structure**: `crates/schemalint` (engine + CLI binary), `crates/schemalint-profiles` (zero-deps TOML data via `include_str!`)
- **Rule registry** (`crates/schemalint/src/rules/registry.rs`): `Rule` trait with `fn check()`, `Diagnostic` struct, `RULES` distributed slice via `linkme`, `RuleSet::from_profile()` for dynamic rule generation
- **CLI** (`crates/schemalint/src/cli/mod.rs`): Four subcommands (check, check-python, check-node, server), five output formats (human, json, sarif, gha, junit)
- **Profile loader** (`crates/schemalint/src/profile/parser.rs`): TOML → `Profile` struct, 40 known keywords, `StructuralLimits` (8 fields), severity enum
- **Test patterns**: `assert_cmd::Command::cargo_bin("schemalint")` + `predicates` for CLI integration (472-line `integration_tests.rs`); subprocess spawn via `std::process::Command` for server tests (`server_tests.rs`); `.expected` JSON files for corpus validation (`corpus_tests.rs`); `tempfile::tempdir()` for isolated test environments
- **Subprocess helper pattern** (`crates/schemalint/src/node/mod.rs`, `crates/schemalint/src/python/mod.rs`): JSON-RPC 2.0 over stdin/stdout, configurable timeout, clear error messages for missing runtimes
- **Output emitter pattern**: Each format in a separate `cli/emit_*.rs` file, shared signature `emit_X_to_string(diagnostics, ...) -> String`
- **Workspace conventions**: `[workspace.package]` for shared version/edition/authors/license/repository/rust-version; `resolver = "2"`; `[workspace.dependencies]` for centralized dependency versions
- **Release profile**: `opt-level = 3`, `lto = true`, `codegen-units = 1`

### Institutional Learnings

- **GHA workflow command escaping** (`docs/solutions/best-practices/schemalint-phase2-learnings.md`): Any new scheduled workflow output must use `percent_encoding` for `::error`/`::warning` commands. The reference implementation is `cli/emit_gha.rs` `encode_gha_value()`.
- **Test field name matching**: Tests asserting on mock server or structured output must use the emitter's JSON field names (e.g., `pointer`), not internal Rust struct field names (e.g., `schema_path`). 80+ corpus tests were silently passing due to this mismatch in Phase 2.
- **I/O error logging**: In non-critical paths (cache writes, log output, doc generation), use `if let Err(e) = ... { eprintln!("warning: {e}"); }` instead of `let _ =`. Silent I/O discard made stale-cache debugging impossible in Phase 2.
- **Three rule registration paths**: Rule docs generator must handle all three: static `linkme` slice, dynamic `RuleSet::from_profile()`, and profile-gated dynamic rules. The `code_prefix` field is profile-driven — doc generator must be provider-agnostic.
- **Pipeline dedup**: `run_check` and `handle_check` in `cli/mod.rs` duplicate ~30 lines of pipeline orchestration. Phase 5 should not add a third copy — invoke existing functions or extract the pipeline.
- **serde_json over binary formats**: Default to `serde_json` for truth files, mock responses, and persistent artifacts unless a benchmark proves otherwise.

### External References

- **cargo-dist** (`dist`): `dist init` generates GitHub Actions release CI; config in `[workspace.metadata.dist]` or `dist-workspace.toml`; targets matrix via `targets = [...]`; must pin `cargo-dist-version` for reproducible CI. Requires valid `repository` URL in `Cargo.toml`.
- **maturin** (v1+): PEP 621 `pyproject.toml`; `[tool.maturin] bindings = "bin"` for CLI-only crates; `PyO3/maturin-action@v1` for CI; `manylinux` compliance via Docker containers; `--compatibility pypi` for pre-upload validation.
- **napi-rs** (v2+): Shell package + platform-specific `optionalDependencies` pattern; `os`/`cpu` fields in `package.json` for npm auto-selection; cross-compilation from Linux CI via `cargo-xwin`/`cargo-zigbuild`/llvm.
- **mdBook** (0.5+): Configuration in `book.toml`; custom preprocessors; auto-generation via build-time binary that writes markdown (recommended over preprocessor for creating new files); `actions/deploy-pages@v5` for GitHub Pages deployment.
- **GitHub Actions scheduled workflows**: `schedule` + `workflow_dispatch` for manual triggers; `nick-fields/retry@v3` for API retry logic; `actions/upload-artifact@v4` with `retention-days` for report storage.
- **Ruff rule reference pattern**: Per-rule pages with code, severity, description, rationale, bad/good examples, see-also links. Rule metadata co-located with rule implementation via trait method.
- **crates.io first-publish**: Requires `cargo login`, valid `license`/`description`/`repository` in `Cargo.toml`; `cargo publish --dry-run` before real publish; 10MB size limit checked via `cargo package --list`.

---

## Key Technical Decisions

| Decision | Rationale |
|---|---|
| `metadata()` method on `Rule` trait, not separate registry | Co-locates metadata with rule logic. Works naturally with `linkme` slice iteration. Dynamic rules compute metadata from profile data at construction time. Follows Ruff's pattern. |
| Build-time binary for rule doc generation, not mdBook preprocessor | Preprocessors cannot create new files or add chapters — only modify existing content. A build-time binary writes plain `.md` files, works with any static site generator, and is debuggable independently. |
| New `schemalint-conformance` crate for mock server, not embedded in CLI | Conformance testing is a separate concern from schema linting. A standalone crate avoids bloating the CLI binary and allows independent testing and future independent publication. |
| Mock server over HTTP, not JSON-RPC over stdin/stdout | The mock is a standalone validation server, not a CLI subprocess. HTTP is simpler for CI consumption (curl, workflow steps) and supports concurrent requests naturally. JSON-RPC over stdin/stdout is reserved for language-specific ingestion helpers. |
| `dist` (cargo-dist) for Rust binary distribution | Industry standard. Auto-generates CI, handles cross-compilation matrix, produces installers (shell/powershell). Single `dist init` sets up the entire release pipeline. |
| `maturin` with `bindings = "bin"` for PyPI | Standard tool for Rust → Python wheels. `bindings = "bin"` mode bundles a CLI binary as a Python package entry point — no pyo3 library bindings needed for a CLI tool. |
| napi-rs shell + platform-packages pattern for npm | Shell package with `optionalDependencies` on platform-specific packages. npm's `os`/`cpu` fields auto-select the right binary. Industry standard for CLI tools distributed via npm. |
| Cargo.toml as canonical version source | `dist` and `maturin` read version from `Cargo.toml` natively. npm/pyproject versions are synced from the git tag in CI. Avoids out-of-sync versions across channels. |
| Generated rule doc pages committed to repo | Enables local `mdbook serve` without running docgen. CI checks for staleness. Reviewers see documentation changes in PRs. |
| GitHub Actions scheduled workflows for burn windows and full harness | Native CI integration — no external scheduler. Secrets management built-in. Artifacts attached to workflow runs. `workflow_dispatch` enables manual triggering. |
| API key secrets in GitHub Actions environment-level secrets | Environment secrets support required reviewers for key rotation. Separate from repository secrets — limited blast radius. OIDC trusted publishing used for PyPI/npm where supported. |

---

## Output Structure

New and modified directories relative to repo root:

```
crates/
├── schemalint-conformance/       # NEW: conformance mock server + truth file types
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs                # Truth file types, loader, mock engine
│       ├── main.rs               # Mock server HTTP binary
│       └── truth.rs              # Truth file TOML schema types
├── schemalint-docgen/            # NEW: rule documentation generator binary
│   ├── Cargo.toml
│   └── src/
│       └── main.rs               # Collects all rules → writes markdown pages
├── schemalint-python/            # NEW: maturin Python packaging crate
│   ├── Cargo.toml
│   ├── pyproject.toml
│   └── python/schemalint/
│       ├── __init__.py
│       └── cli.py
└── schemalint/                   # MODIFIED: existing crate
    ├── Cargo.toml                # Add schemalint-conformance as dev-dep
    └── src/
        └── rules/
            ├── registry.rs       # MODIFIED: add RuleMetadata, metadata() to trait
            ├── semantic.rs       # MODIFIED: impl metadata() on static rules
            ├── class_a.rs        # MODIFIED: impl metadata() on dynamic rules
            ├── class_b.rs        # MODIFIED: impl metadata() on structural rules
            └── metadata.rs       # NEW: RuleMetadata, RuleCategory types

docs/book/                        # NEW: mdBook site root
├── book.toml
└── src/
    ├── SUMMARY.md
    ├── index.md
    ├── guide/
    │   ├── installation.md
    │   ├── quick-start.md
    │   ├── configuration.md
    │   └── ci-integration.md
    ├── profiles/
    │   ├── index.md
    │   ├── openai.md
    │   └── anthropic.md
    └── rules/                    # AUTO-GENERATED by schemalint-docgen
        ├── index.md
        ├── keyword/
        ├── restriction/
        ├── structural/
        └── semantic/

crates/schemalint-profiles/
└── profiles/
    └── truth/                    # NEW: conformance truth files
        ├── openai.truth.toml
        └── anthropic.truth.toml

.github/workflows/
├── ci.yml                        # EXISTING: test + MSRV
├── docs.yml                      # NEW: mdBook build + deploy to GH Pages
├── conformance.yml               # NEW: synthetic mock + linter consistency on PR
├── burn-windows.yml              # NEW: weekly live-API keyword burn
├── full-harness.yml              # NEW: monthly full corpus live-API run
└── release.yml                   # NEW: tag-triggered multi-channel release

npm/
├── cli/
│   ├── package.json
│   └── index.js
└── core/
    ├── package.json
    └── index.js

LICENSE                           # NEW
CHANGELOG.md                      # NEW
```

---

## Implementation Units

### Phase 1: Foundation

- U1. **Pre-work: repo metadata and licensing**

**Goal:** Resolve three blockers that prevent any distribution: missing LICENSE file, wrong Cargo.toml repository URL, and absent CHANGELOG.

**Requirements:** R1, R2, R3

**Dependencies:** None

**Files:**
- Create: `LICENSE`
- Create: `CHANGELOG.md`
- Modify: `Cargo.toml`

**Approach:**
- Create `LICENSE` file with MIT OR Apache-2.0 dual-license text, matching the workspace declaration
- Fix `repository` field in workspace `Cargo.toml` from `lahfir/schemalint` to `1nder-labs/schemalint`
- Create `CHANGELOG.md` with Keep a Changelog structure: `[Unreleased]` section with `Added`, `Changed`, `Fixed`, `Removed` subsections

**Patterns to follow:** Standard dual-license format (Copyright line, MIT text, Apache-2.0 text). Keep a Changelog format from keepachangelog.com.

**Test scenarios:**
- Happy path: `cargo publish --dry-run` succeeds (validates `repository` and `license` fields)
- Happy path: LICENSE file exists at repo root and contains both license texts

**Verification:**
- `cargo publish --dry-run` passes without metadata errors
- `LICENSE` file content matches the workspace `license = "MIT OR Apache-2.0"` declaration

---

- U2. **Rule metadata enrichment**

**Goal:** Add descriptive metadata to every rule so the documentation generator can produce useful rule reference pages. Introduce `RuleMetadata` struct and `metadata()` method on the `Rule` trait with implementations for all three registration paths.

**Requirements:** R9

**Dependencies:** None

**Files:**
- Create: `crates/schemalint/src/rules/metadata.rs`
- Modify: `crates/schemalint/src/rules/registry.rs`
- Modify: `crates/schemalint/src/rules/semantic.rs`
- Modify: `crates/schemalint/src/rules/class_a.rs`
- Modify: `crates/schemalint/src/rules/class_b.rs`

**Approach:**
- Define `RuleMetadata` struct in `rules/metadata.rs` with fields: `name` (slug), `code` (canonical format with `{prefix}` placeholder for profile-driven codes), `description`, `rationale`, `severity`, `category` (enum: `Keyword`, `Restriction`, `Structural`, `Semantic`), `bad_example`, `good_example`, `see_also`, `profile` (optional — `Some` for profile-specific rules, `None` for universal)
- Define `RuleCategory` enum with four variants matching the three registration paths + restriction sub-category
- Add `fn metadata(&self) -> Option<RuleMetadata> { None }` to the `Rule` trait with a default `None` return — backward compatible, no existing trait impls break
- Implement `metadata()` on the three static `linkme` rules in `semantic.rs` with hardcoded descriptions, rationales, examples
- Implement `metadata()` on `KeywordRule` and `RestrictionRule` in `class_a.rs` — dynamically computed from `self.keyword` and `self.severity` at construction time
- Implement `metadata()` on all eight structural rule structs in `class_b.rs` with descriptions derived from the structural limit they enforce
- Profile-gated dynamic rules (e.g., `AllOfWithRefRule`) get profile-specific metadata
- Use canonical `{prefix}` placeholder in static rule codes — the doc generator expands it per-profile

**Patterns to follow:** Ruff's per-rule metadata pattern. The `Diagnostic` struct in `registry.rs` for severity enum. `Box::leak` for `&'static str` metadata strings (profile loader already uses this pattern).

**Test scenarios:**
- Happy path: Every rule in the `RULES` distributed slice returns `Some(metadata)` — `name`, `description`, `rationale`, `category` are non-empty
- Happy path: `KeywordRule` metadata has `category = Keyword`, `name` matches the keyword name, `code` includes the profile prefix
- Happy path: `RestrictionRule` metadata has `category = Restriction`
- Happy path: Static `linkme` rules have `profile = None` (universal)
- Happy path: Profile-gated rules have `profile = Some(...)`
- Edge case: Rules added in the future without `metadata()` impls return `None` — doc generation skips them gracefully

**Verification:**
- `cargo test --workspace` passes (no regression in existing rule behavior)
- A quick smoke script iterates `RULES` slice and asserts all entries return non-None metadata
- Static rule metadata strings are reviewed against provider documentation for accuracy

---

- U3. **Docgen binary crate**

**Goal:** Create a new binary crate `schemalint-docgen` that collects metadata from all rules across all profiles and generates markdown rule reference pages for the mdBook site.

**Requirements:** R9, R10

**Dependencies:** U2

**Files:**
- Create: `crates/schemalint-docgen/Cargo.toml`
- Create: `crates/schemalint-docgen/src/main.rs`

**Approach:**
- New workspace member crate at `crates/schemalint-docgen/` — `publish = false` (internal tool)
- Depends on `schemalint` (for `Rule` trait, `RULES` slice, `Profile`, `RuleSet`) and `schemalint-profiles` (for built-in profiles)
- Main function flow:
  1. Load all built-in profiles from `schemalint-profiles` (currently OpenAI, Anthropic)
  2. Collect static rules from the `RULES` distributed slice — iterate, call `.metadata()`, skip `None`
  3. For each profile, build a `RuleSet` via `RuleSet::from_profile()`, collect dynamic rule metadata
  4. Deduplicate: same rule appearing in multiple profiles is grouped under a single page with a per-profile code table
  5. Group by `RuleCategory` (keyword, restriction, structural, semantic)
  6. Generate `docs/book/src/rules/index.md` — summary table with Rule Name, Category, Codes (per profile), Severity
  7. Generate `docs/book/src/rules/{category}/{rule-name}.md` — per-rule page with description, rationale, bad/good examples, see-also links
  8. Auto-append rules section entries to `docs/book/src/SUMMARY.md` (within an `<!-- AUTO-GENERATED RULES -->` marker block)
- Expand `{prefix}` placeholders in static rule codes to real profile code prefixes
- Generation is idempotent — running twice produces identical output
- Output directory is configurable via CLI argument (defaults to `docs/book/src/rules/`)

**Patterns to follow:** `schemalint-profiles` crate pattern (zero-deps data crate loaded via `include_str!`). `clap` derive for CLI args. `toml` 0.8 for profile parsing (reuse existing profile loader).

**Test scenarios:**
- Happy path: `cargo run --bin schemalint-docgen` generates `rules/index.md`, `rules/keyword/`, `rules/structural/`, `rules/semantic/` directories with per-rule `.md` files
- Happy path: Generated `index.md` table has one row per unique rule, with per-profile code columns
- Happy path: Generated per-rule page has all required sections (description, rationale, examples)
- Happy path: Running twice produces identical output (idempotent)
- Edge case: Rules with `metadata()` returning `None` are excluded without error
- Edge case: Profile with zero dynamic rules produces an empty category directory (or is skipped)
- Error path: Missing `docs/book/src/rules/` parent directory produces clear error message

**Verification:**
- `cargo build --bin schemalint-docgen` compiles without errors
- Generated markdown files pass basic markdown lint (valid links, consistent headings)
- All generated rule pages can be cross-referenced from the index
- In CI: check that generated files match committed files (staleness detection)

---

### Phase 2: Documentation Site

- U4. **mdBook site and deployment**

**Goal:** Create the mdBook documentation site with hand-written content (installation, quick-start, configuration, CI integration, profile descriptions) and integrate auto-generated rule reference pages. Deploy to GitHub Pages on every push to main.

**Requirements:** R8, R9, R10

**Dependencies:** U3

**Files:**
- Create: `docs/book/book.toml`
- Create: `docs/book/src/SUMMARY.md`
- Create: `docs/book/src/index.md`
- Create: `docs/book/src/guide/installation.md`
- Create: `docs/book/src/guide/quick-start.md`
- Create: `docs/book/src/guide/configuration.md`
- Create: `docs/book/src/guide/ci-integration.md`
- Create: `docs/book/src/profiles/index.md`
- Create: `docs/book/src/profiles/openai.md`
- Create: `docs/book/src/profiles/anthropic.md`
- Create: `.github/workflows/docs.yml`

**Approach:**
- `book.toml` configures: title "Schemalint", language, build directory, output.html with site-url (GitHub Pages path), git-repository-url, edit-url-template, custom CSS
- Hand-written pages cover: landing page (what schemalint is, quick one-liner), installation (all four channels), quick-start (first `schemalint check`), configuration (profiles, CLI args, pyproject.toml, package.json), CI integration (GitHub Actions example, pre-commit), profile overview (what profiles are, how to create custom), per-profile pages (keyword coverage, structural limits, known caveats)
- Rules section in SUMMARY.md uses an `<!-- AUTO-GENERATED RULES -->` marker where docgen appends entries — hand-written content above the marker is preserved
- `docs.yml` workflow: triggers on push to main (paths: `docs/book/**`, `crates/schemalint/src/rules/**`, `crates/schemalint-profiles/**`), runs docgen, builds mdBook, deploys via `actions/deploy-pages@v5`
- CI check in `docs.yml`: after docgen runs, `git diff --exit-code docs/book/src/rules/` — fails if generated files are stale (developer forgot to run docgen)
- Local development: `cargo run --bin schemalint-docgen && mdbook serve docs/book`

**Patterns to follow:** `peaceiris/actions-mdbook@v2` for mdBook installation in CI. `actions/upload-pages-artifact@v4` + `actions/deploy-pages@v5` for deployment. Existing CI patterns from `.github/workflows/ci.yml` (rust-toolchain, rust-cache).

**Test scenarios:**
- Happy path: `mdbook build docs/book` produces valid HTML in `docs/book/book/`
- Happy path: Documentation site loads with working navigation, all links resolve
- Happy path: Docs deployment workflow runs on push to main and deploys to GitHub Pages
- Happy path: `git diff --exit-code docs/book/src/rules/` passes when rules are up to date
- Edge case: First deployment — GitHub Pages source must be set to "GitHub Actions" in repo settings
- Error path: Stale generated docs cause CI failure with clear instruction to run docgen locally

**Verification:**
- Local `mdbook serve` shows complete site with working navigation
- All cross-references between pages resolve
- Docs deployment workflow succeeds on CI
- Site is accessible at the published URL

---

### Phase 3: Conformance Infrastructure

- U5. **Conformance crate: truth file types and loader**

**Goal:** Create the `schemalint-conformance` crate with TOML truth file schema types, a loader, and the provider-agnostic truth engine that the mock server will use.

**Requirements:** R11, R12, R13

**Dependencies:** None (new crate, depends on `serde`, `toml`, `serde_json`)

**Files:**
- Create: `crates/schemalint-conformance/Cargo.toml`
- Create: `crates/schemalint-conformance/src/lib.rs`
- Create: `crates/schemalint-conformance/src/truth.rs`

**Approach:**
- New workspace member crate at `crates/schemalint-conformance/` — `publish = false` (internal tool for v1)
- Dependencies: `serde` (with `derive`), `serde_json`, `toml` 0.8
- `truth.rs` defines the truth file TOML schema as Rust types with serde derives:
  - `ProviderTruth` — top-level table: `[provider]` with `name`, `version`, `endpoint`, `model`, `behavior` (enum: `Strict | Permissive | Stripping`)
  - `KeywordTruth` — per-keyword entry in `[[keywords]]` array: `name`, `behavior` (enum: `Accept | Reject | Strip`), `test_schema` (inline JSON), `expected_error` (optional, for reject), `expected_error_path` (optional, JSON Pointer), `expected_transformed` (optional, for strip)
  - Optional `[[structural_tests]]` array-of-tables for structural limit validation: `limit_name`, `test_schema`, `expected_behavior`, `expected_error_path`
- `lib.rs` exposes:
  - `load_truth(path: &Path) -> Result<ProviderTruth>` — parses a truth TOML file
  - `evaluate(truth: &ProviderTruth, schema: &Value) -> TruthResult` — evaluates a schema against the truth declarations, returns accept/reject + errors + transformed schema
  - `TruthResult` enum — `Accepted { transformed: Value }` or `Rejected { errors: Vec<TruthError> }` where `TruthError` has `message`, `pointer`, `keyword`
  - The evaluate function walks the schema, matches keywords against truth declarations, and applies the declared behavior
  - For `Strip` behavior: deep-clone the schema, recursively remove matching keywords, return the transformed schema
  - For `Reject` behavior: return errors at the declared paths with the declared messages
  - For `Accept` behavior: do nothing (keyword is allowed)
- The engine is provider-agnostic — it reads any valid `ProviderTruth` and replays its declarations
- No `schemalint` crate dependency — conformance is independently testable

**Patterns to follow:** `schemalint-profiles` pattern (zero or minimal external deps, serde-driven TOML parsing). `serde_json::Value` for schema representation (consistent with the existing codebase's JSON handling). `serde_json::from_str` for inline test schemas in truth files.

**Test scenarios:**
- Happy path: Loading a valid truth file returns `ProviderTruth` with all keywords deserialized
- Happy path: `evaluate()` with a schema containing a `Reject`-declared keyword returns `Rejected` with the expected error and pointer
- Happy path: `evaluate()` with a schema containing a `Strip`-declared keyword returns `Accepted` with the keyword removed from the transformed schema
- Happy path: `evaluate()` with a schema containing only `Accept`-declared keywords returns `Accepted` with an identical transformed schema
- Edge case: Schema with a keyword not declared in the truth file — engine logs a warning and passes (assumes unknown = allow, conservative)
- Edge case: Nested keyword (e.g., `properties.x.additionalProperties`) — engine recurses into object properties
- Error path: Invalid truth TOML produces a parse error with line/column
- Error path: Inline test schema in truth file is invalid JSON — parse error

**Verification:**
- Unit tests for each `evaluate()` branch (accept, reject, strip, nested, unknown keyword)
- `cargo test -p schemalint-conformance` passes
- Truth file round-trip: serialize a loaded truth back to TOML, re-parse, verify equality

---

- U6. **Author truth files from provider documentation**

**Goal:** Create declarative truth files for OpenAI and Anthropic by exhaustively reviewing the official Structured Outputs documentation for each provider. Each truth file covers every keyword in the documented JSON Schema surface with at least one test schema.

**Requirements:** R12, R13

**Dependencies:** U5

**Files:**
- Create: `crates/schemalint-profiles/profiles/truth/openai.truth.toml`
- Create: `crates/schemalint-profiles/profiles/truth/anthropic.truth.toml`

**Approach:**
- Author truth files by consulting:
  - OpenAI Structured Outputs docs (`platform.openai.com/docs/guides/structured-outputs`) — which keywords are supported, which are rejected, supported schema patterns
  - Anthropic Structured Outputs docs (`docs.anthropic.com/en/docs/build-with-claude/structured-outputs`) — which keywords are stripped, which are accepted, tool use budgets
  - Cross-reference with the existing linter profiles (`openai.so.2026-04-30.toml`, `anthropic.so.2026-04-30.toml`) to ensure the truth files cover every keyword the linter profiles classify
- Each truth file structure (TOML):
  ```toml
  [provider]
  name = "openai"
  version = "2026-04-30"
  endpoint = "https://api.openai.com/v1/chat/completions"
  model = "gpt-4o-2024-08-06"
  behavior = "strict"

  [[keywords]]
  name = "allOf"
  behavior = "reject"
  test_schema = '''
  { "type": "object", "allOf": [{"properties": {"x": {"type": "string"}}}], "properties": {} }
  '''
  expected_error = "`allOf` is not supported"
  expected_error_path = "/allOf"
  ```
- Coverage: every keyword in the profile's keyword_map gets a truth entry. Keywords marked `allow` in the profile get `behavior = "accept"` in truth. Keywords marked `forbid` get `behavior = "reject"`. Keywords marked `strip` (Anthropic) get `behavior = "strip"` with a `expected_transformed` value.
- At least one `test_schema` per keyword that exercises the keyword in isolation
- For restricted values (e.g., `format` only allows `["date-time", "date", "time", "duration"]`): multiple entries for the same keyword with different test schemas exercising allowed and disallowed values
- Include structural limit test cases in `[[structural_tests]]`: max depth, max properties, max enum values, string length budget
- The existing Python validation scripts (`scripts/validation/validate_openai.py`) serve as reference — their test schemas inform truth file entries
- Truth files are embedded via `include_str!()` in `schemalint-profiles/src/lib.rs` — add two new constants: `OPENAI_TRUTH` and `ANTHROPIC_TRUTH`

**Patterns to follow:** Existing profile TOML format (`keyword = "severity"`) for familiarity. `include_str!()` pattern from `schemalint-profiles/src/lib.rs` for embedding.

**Test scenarios:**
- Happy path: Loading `openai.truth.toml` via the truth loader produces a valid `ProviderTruth`
- Happy path: Every keyword in the corresponding linter profile has a matching truth entry
- Happy path: Each truth keyword's `test_schema` is valid JSON and exercises the keyword
- Edge case: Keyword appears in both profiles with different behaviors (e.g., `anyOf` is reject in OpenAI, accept in Anthropic) — each truth file correctly reflects its provider
- Integration: Running the mock server's `evaluate()` against each `test_schema` produces the declared expected behavior

**Verification:**
- Truth files parse without TOML errors
- Manual review against provider documentation confirms accuracy
- Every keyword in each linter profile is covered by a truth entry (automated check in U8)
- Truth file test schemas are valid JSON and exercise their keyword

---

- U7. **Mock server binary**

**Goal:** Build an HTTP server binary in the `schemalint-conformance` crate that loads truth files at startup and exposes an endpoint for Tier 3 conformance evaluation.

**Requirements:** R11, R13

**Dependencies:** U5, U6

**Files:**
- Create: `crates/schemalint-conformance/src/main.rs`
- Modify: `crates/schemalint-conformance/Cargo.toml` (add HTTP server dependency)

**Approach:**
- Binary entry point in `main.rs`:
  1. Parse CLI args: `--port` (default 0 for OS-assigned), `--truth-dir` (path to `profiles/truth/`, required)
  2. Load all truth files from the truth directory: scan for `*.truth.toml` files, parse each, build a `HashMap<String, ProviderTruth>` keyed by provider name
  3. Start HTTP server with single endpoint:
     - `POST /evaluate/{provider}` — body is a JSON Schema (JSON), returns `TruthResult` as JSON
     - Response `200`: `{ "status": "accepted", "transformed": {...} }` or `{ "status": "rejected", "errors": [{"message": "...", "pointer": "/keyword", "keyword": "..."}] }`
     - Response `404`: unknown provider
     - Response `400`: invalid JSON body
  4. Print bound address to stdout for CI consumption
- HTTP framework: `tiny_http` or `axum` — `tiny_http` preferred for zero-async, minimal dependency footprint (mock is internal-use, low throughput, single-threaded is fine)
- Concurrency: single-threaded, process one request at a time (mock server handles trivial CPU work — no I/O wait)
- Bind to `127.0.0.1` only (CI-internal use, not exposed to network)
- Enforce a configurable max request body size (default 1 MB) and a hard request timeout (default 5 seconds)
- Graceful shutdown on SIGTERM (CTRL-C)
- Startup failure: if truth directory is empty or no valid truth files found, exit with clear error message

**Patterns to follow:** `server.rs` in the existing CLI for server lifecycle (startup logging, graceful shutdown). `assert_cmd` integration test pattern for spawning and testing the binary. `cli/emit_json.rs` for structured JSON response formatting.

**Test scenarios:**
- Happy path: POST a schema with a rejected keyword → `200 { "status": "rejected", "errors": [...] }`
- Happy path: POST a valid schema → `200 { "status": "accepted", "transformed": {...} }`
- Happy path: POST a schema with a stripped keyword → `200 { "status": "accepted", "transformed": {...} }` with keyword removed
- Happy path: POST to `/evaluate/anthropic` uses the Anthropic truth file
- Edge case: POST a schema with no declared keywords → `200 { "status": "accepted", "transformed": {...} }` (unknown = accept)
- Edge case: POST to `/evaluate/unknown-provider` → `404 { "error": "unknown provider" }`
- Error path: POST invalid JSON body → `400 { "error": "invalid JSON" }`
- Error path: Server started with empty truth directory → exits with error message
- Integration: Spawn mock server, POST a test schema from a truth file, assert the response matches the declared expected behavior

**Verification:**
- `cargo build --bin schemalint-conformance` compiles
- Server starts, prints bound address, accepts requests
- All truth file test schemas produce the declared behavior when submitted to the server
- Server shuts down cleanly on SIGTERM

---

- U8. **Conformance CI: mock + linter consistency check**

**Goal:** Add a CI job that runs on every PR and push to main, starts the mock server, and verifies that schemalint's linter diagnostics are consistent with what the mock server declares the real API does.

**Requirements:** R14

**Dependencies:** U6, U7

**Files:**
- Create: `.github/workflows/conformance.yml`
- Modify: `crates/schemalint-conformance/src/main.rs`

**Approach:**
- New workflow `conformance.yml`: triggers on `push` and `pull_request` to `main`
- Single job with steps:
  1. Checkout + install Rust + cargo cache (same pattern as `ci.yml`)
  2. Build `schemalint` binary and `schemalint-conformance` binary
  3. Start mock server in background on OS-assigned port, capture port
  4. For each truth file: iterate over `[[keywords]]` entries
     a. Send the `test_schema` to the mock server → get expected behavior
     b. Run `schemalint check --profile {provider} --format json` on the same schema → get linter diagnostics
     c. Compare:
        - If mock says `reject` and linter emits an error at the expected path → consistent (pass)
        - If mock says `reject` and linter produces no diagnostic → false negative (fail)
        - If mock says `accept` and linter emits a forbid/warn diagnostic → false positive (fail)
        - If mock says `strip` and linter emits a strip diagnostic → consistent (pass)
  5. Report results: summary table of consistent/inconsistent per keyword, exit code 1 if any inconsistencies
  6. Shutdown mock server
- Implementation: a Rust test binary or a script that orchestrates this. A Rust binary (`schemalint-conformance` with a `ci-check` subcommand) is preferred — reuse the same crate, same truth types, same evaluate function.
- Add `ci-check` subcommand to the mock server binary:
  - `schemalint-conformance ci-check --truth-dir ... --schemalint-bin ...`
  - Loads truth files, spawns schemalint subprocess for each test schema, compares results
  - Prints human-readable report to stdout, exits 0 on full consistency, 1 on any mismatch

**Patterns to follow:** Existing CI workflow patterns from `.github/workflows/ci.yml` (rust-toolchain, rust-cache, Node setup). `assert_cmd` subprocess spawning pattern from integration tests. `server_tests.rs` pattern for spawning and communicating with a server process.

**Test scenarios:**
- Happy path: All truth entries consistent → CI passes, exit 0
- Happy path: Single false positive (linter rejects, mock accepts) → CI fails with specific keyword and schema
- Happy path: Single false negative (linter accepts, mock rejects) → CI fails with specific keyword and schema
- Edge case: Keyword not in the profile (no linter rule) but present in truth → consistency check skips (no linter rule to compare)
- Edge case: Keyword in profile but no truth entry → consistency check skips with warning (truth coverage gap)

**Verification:**
- `conformance.yml` workflow runs successfully on CI
- Introducing a deliberate mismatch between a linter profile and truth file causes CI failure
- Report output clearly identifies which keyword, which provider, and whether it's a false positive or false negative

---

- U9. **Weekly burn windows workflow**

**Goal:** Create a scheduled GitHub Actions workflow that submits minimal keyword-exercise schemas to live OpenAI and Anthropic APIs weekly and compares responses against truth file declarations.

**Requirements:** R15, R16

**Dependencies:** U6

**Files:**
- Create: `.github/workflows/burn-windows.yml`

**Approach:**
- Scheduled workflow: `schedule: cron: '0 6 * * 1'` (weekly Monday 6am UTC) + `workflow_dispatch` for manual runs
- Uses environment-level secrets: `OPENAI_API_KEY`, `ANTHROPIC_API_KEY` with required reviewers for rotation
- Job steps:
  1. Checkout code
  2. Write a Python script (inlined in the workflow or at `scripts/validation/burn_window.py`) that:
     a. Reads each truth file from `crates/schemalint-profiles/profiles/truth/`
     b. For each `[[keywords]]` entry, submits the `test_schema` to the appropriate API
     c. For OpenAI: uses `gpt-4o-2024-08-06` model with `response_format.type = "json_schema"` + `strict: true`
     d. For Anthropic: uses the latest Claude model with `tools` / structured output configuration
     e. Captures the API response (accept/reject, error messages, error paths, transformed schema if observable)
     f. Compares against truth file declarations
     g. Classifies divergence: `provider_drift` (API behavior changed), `truth_error` (truth file wrong), `api_transient` (rate limit/timeout)
  3. Write structured report as JSON artifact (sanitized per step 6)
  4. Upload report via `actions/upload-artifact@v4` with `retention-days: 30`
  5. If drift detected, the workflow step exits 1 (red) but does NOT fail the overall job — report is always uploaded
  6. Sanitize API responses before artifact storage: keep only comparison-relevant fields (status code, error message text, error pointer). Drop raw response bodies and headers.
- Rate limiting: 5 req/s (OpenAI standard tier limit). Inter-request delay via `time.sleep(0.2)`.
- The script reuses the API calling patterns from `scripts/validation/validate_openai.py` and extends to Anthropic

**Patterns to follow:** `scripts/validation/validate_openai.py` for API calling pattern. `scripts/validation/compare_with_openai.py` for comparison logic. `cli/emit_gha.rs` `encode_gha_value()` for workflow command escaping in script output. GitHub Actions `schedule` + `workflow_dispatch` pattern. `nick-fields/retry@v3` for retry logic on transient failures.

**Test scenarios:**
- Happy path: Burn window runs against both APIs, all keywords consistent, report shows zero drift
- Happy path: API returns a different behavior than truth declares — report tags it as provider_drift
- Edge case: API rate limit hit mid-run — script retries with backoff, classifies persistent failures as api_transient
- Edge case: API key expired or invalid — workflow fails fast with clear error
- Error path: One provider API is down — other provider results still captured and reported

**Verification:**
- `burn-windows.yml` workflow can be triggered manually via `workflow_dispatch`
- Report artifact is attached to the workflow run and contains per-keyword results for both providers
- Drift detection correctly classifies known divergence (can be tested by temporarily modifying a truth file)

---

- U10. **Monthly full harness workflow**

**Goal:** Create a scheduled GitHub Actions workflow that runs the complete regression corpus through live APIs monthly, compares against mock predictions and `.expected` files, and auto-files GitHub issues for detected provider drift.

**Requirements:** R17, R18

**Dependencies:** U6, U7

**Files:**
- Create: `.github/workflows/full-harness.yml`

**Approach:**
- Scheduled workflow: `schedule: cron: '0 4 1 * *'` (monthly 1st at 4am UTC) + `workflow_dispatch`
- Uses the same environment-level secrets as burn windows
- Job steps:
  1. Checkout code
  2. Build schemalint binary
  3. Write/run a Python script (`scripts/validation/full_harness.py`) that:
     a. Iterates all corpus schemas (50 OpenAI + 30 Anthropic from `tests/corpus/`)
     b. Submits each schema to the appropriate live API
     c. Captures response: accepted/rejected, error details, transformed schema (where observable)
     d. Compares live API response against:
        - Synthetic mock prediction (call evaluate from the truth engine)
        - Existing `.expected` file diagnostics
     e. Records per-schema results: consistent, drift_detected, api_error, timeout
  4. Generate a summary markdown report
  5. If drift detected on any schema: auto-file a GitHub issue via `gh issue create` using the `GITHUB_TOKEN`
     - Issue title: `Provider drift ({provider}): {month} {year} conformance run`
     - Issue body: affected schemas, expected vs actual behavior, provider version, link to workflow run artifacts
  6. Upload full JSON results as workflow artifact with `retention-days: 90` (sanitized per burn window sanitization rules)
- Rate limiting: careful pacing — 75+ schemas at 5 req/s ~15 seconds of API time. Use 1 req/s for safety margin (~75 seconds).
- Idempotency: if the monthly run re-runs (manual trigger), check for existing drift issue before filing a duplicate — search by title pattern
- Timeout: 15 minutes (well within the 6-hour GHA limit; corpus is 75 schemas at ~1 req/s)

**Patterns to follow:** `corpus_tests.rs` for corpus iteration pattern. `gh issue create` via `GITHUB_TOKEN` (built-in, no extra auth). Existing `.expected` file format for comparison baseline.

**Test scenarios:**
- Happy path: Full harness runs against both APIs, all schemas consistent with mock + `.expected`, zero drift issues filed
- Happy path: Single schema shows provider drift → one GitHub issue filed with correct title and body
- Edge case: Re-running the monthly workflow does not file duplicate issues (title-based dedup)
- Edge case: API rate limit hit with 75+ schemas — retry with backoff, mark schemas that exhaust retries as api_transient
- Error path: GitHub issue creation fails (token scope, rate limit) — report is still uploaded as artifact

**Verification:**
- `full-harness.yml` workflow can be triggered manually
- Complete report artifact is attached to workflow run
- Drift issue auto-filing produces correctly formatted GitHub issues
- Workflow completes within 15 minutes

---

### Phase 4: Multi-Platform Packaging

- U11. **cargo-dist setup: GitHub Releases + crates.io**

**Goal:** Configure cargo-dist for the workspace, generate the release CI pipeline, and set up crates.io publishing.

**Requirements:** R4, R5, R19

**Dependencies:** U1

**Files:**
- Modify: `Cargo.toml` (workspace root — add `[workspace.metadata.dist]`)
- Modify: `crates/schemalint/Cargo.toml` (add `description`, `homepage` if missing)
- Create/Modify: `.github/workflows/release.yml`

**Approach:**
- Run `dist init --yes` in the workspace to generate the initial config and CI template
- Configure in `[workspace.metadata.dist]`:
  - `cargo-dist-version = "0.25.0"` (pin for reproducible CI)
  - `targets = ["x86_64-apple-darwin", "aarch64-apple-darwin", "x86_64-unknown-linux-gnu", "aarch64-unknown-linux-gnu", "x86_64-pc-windows-msvc"]`
  - `installers = ["shell", "powershell"]`
  - `ci = ["github"]`
- Verify `crates/schemalint/Cargo.toml` has: `description = "Static analysis for JSON Schema compatibility with LLM structured-output providers"`, `homepage = "https://schemalint.dev"`, `readme = "README.md"`, `license = "MIT OR Apache-2.0"`, `repository = "https://github.com/1nder-labs/schemalint"`
- Exclude non-distributable crates: `schemalint-profiles` is already excluded (no `[[bin]]`), `schemalint-conformance` and `schemalint-docgen` are `publish = false`
- crates.io: set up API token in GitHub Secrets as `CARGO_REGISTRY_TOKEN`. Add publish step to the release workflow (U14).
- Test locally: `dist build` builds for the current platform. `dist plan` simulates what CI will do.

**Patterns to follow:** `dist init` generated template. Existing `Cargo.toml` `[workspace.package]` conventions.

**Test scenarios:**
- Happy path: `dist build` produces a binary for the current platform
- Happy path: `dist plan` shows the full release matrix without errors
- Happy path: `cargo publish --dry-run -p schemalint` succeeds
- Edge case: `dist init` errors on missing repository URL — already fixed in U1

**Verification:**
- `dist build` succeeds and produces a runnable binary
- `cargo publish --dry-run -p schemalint` passes without metadata errors
- Release workflow (U14) integration verified end-to-end from tag push

---

- U12. **PyPI packaging via maturin**

**Goal:** Configure maturin to build and publish a Python wheel that bundles the schemalint CLI binary, and integrate into the release pipeline.

**Requirements:** R6, R19

**Dependencies:** U11 (release pipeline foundation)

**Files:**
- Create: `crates/schemalint-python/Cargo.toml`
- Create: `crates/schemalint-python/pyproject.toml`
- Create: `crates/schemalint-python/python/schemalint/__init__.py`
- Create: `crates/schemalint-python/python/schemalint/cli.py`
- Modify: `Cargo.toml` (workspace root — add to `[workspace] members`)

**Approach:**
- Create `crates/schemalint-python/` crate:
  - `Cargo.toml`: depends on `schemalint` (path dep), single `[[bin]]` target that calls `schemalint::cli::run()` — a thin wrapper re-exporting the CLI entry point
  - `pyproject.toml`: PEP 621 metadata, `[tool.maturin] bindings = "bin"`, `python-source = "python"`, `module-name = "schemalint._core"` (optional — only if also shipping pyo3 lib, which we are not)
  - `python/schemalint/__init__.py` + `python/schemalint/cli.py`: thin Python entry point that invokes the bundled binary
- Register `schemalint` name on PyPI (if not already taken)
- maturin CI integration: add matrix job to release workflow (U14) using `PyO3/maturin-action@v1` with:
  - `target: ${{ matrix.platform.target }}`
  - `manylinux: ${{ matrix.platform.manylinux }}` (2_28 for aarch64, auto for x86_64)
  - `args: --release --locked --compatibility pypi`
  - Target matrix: macOS x86_64, macOS aarch64, Linux x86_64 (manylinux), Linux aarch64 (manylinux), Windows x86_64
- Publish via `maturin publish` with `MATURIN_PYPI_TOKEN` secret
- Version: maturin reads from `Cargo.toml` by default. The workspace version (0.1.0) is authoritative. No separate version in `pyproject.toml` — let maturin inherit.
- Smoke test after build: `pip install target/wheels/*.whl && schemalint --version`

**Patterns to follow:** Official maturin `bin` bindings example. `PyO3/maturin-action@v1` with commit-SHA pinning for supply-chain security. Existing workspace member conventions from `Cargo.toml`.

**Test scenarios:**
- Happy path: `maturin build --release` produces a `.whl` file
- Happy path: `pip install target/wheels/schemalint-*.whl` installs the CLI, `schemalint --version` works
- Happy path: Smoke test passes — `schemalint check` against a known schema produces expected output
- Edge case: Wheel installed on Python 3.8 (oldest supported) — CLI binary is compatible (Rust binary, not Python-version-dependent)
- Error path: Attempting to publish to PyPI with an already-taken version number → workflow fails clearly

**Verification:**
- `maturin build --release` succeeds on all target platforms in CI
- Local smoke test: install wheel, run basic check, verify output matches cargo-installed binary
- PyPI publish works from the release workflow

---

- U13. **npm packaging via napi-rs**

**Goal:** Configure napi-rs to build and publish npm packages: `@schemalint/cli` (CLI binary on PATH), `@schemalint/core` (programmatic API), and include the existing `@schemalint/zod` helper in the release pipeline.

**Requirements:** R7, R19

**Dependencies:** U11 (release pipeline foundation)

**Files:**
- Create: `npm/cli/package.json`
- Create: `npm/cli/index.js`
- Create: `npm/cli/Cargo.toml` (if using napi-rs native build; otherwise just a thin JS wrapper)
- Create: `npm/core/package.json`
- Create: `npm/core/index.js`
- Modify: `Cargo.toml` (workspace root — add to `[workspace] members` if napi-rs crate is in the workspace)

**Approach:**
- Two distribution strategies for `@schemalint/cli`:
  - **Recommended (simpler for v1):** A thin JS wrapper that downloads and invokes the GitHub Release binary. The `index.js` detects platform, downloads the appropriate cargo-dist binary on first use, and caches it. Similar to `@esbuild/darwin-x64` pattern but single-package with auto-download. Simpler than napi-rs for a CLI tool that doesn't need native Node integration.
  - **Alternative (napi-rs, more work):** Shell package + platform-specific packages pattern. Appropriate if we also want `@schemalint/core` as a native module.
- For v1, use the **auto-download approach** for `@schemalint/cli`:
  - `npm/cli/package.json`: `"name": "@schemalint/cli"`, `"bin": { "schemalint": "./index.js" }`
  - `npm/cli/index.js`: JS entry point that downloads the platform-appropriate binary from GitHub Releases on first run, caches in `~/.cache/schemalint-npm/`, then exec's the binary with forwarded args
  - Downloads are verified via SHA256 checksums published alongside the binaries
- For `@schemalint/core`: direct napi-rs native module (deferred to post-v1 — Phase 4 requirement explicitly defers self-contained napi-rs to Phase 5). For now, `@schemalint/core` is a placeholder that requires `schemalint` on PATH (same as Phase 4 `lint()` function).
- `@schemalint/zod`: the existing TypeScript helper at `typescript/schemalint-zod/` — publish from its existing `package.json` with `npm publish`
- npm CI integration in release workflow (U14):
  - Build/sync versions: `npm version ${{ github.ref_name }} --no-git-tag-version` in each package directory
  - Publish: `npm publish --access public` for each package
  - Token: `NPM_TOKEN` secret
- Scope `@schemalint` must be registered on npm (associated with the `1nder-labs` org or user)

**Patterns to follow:** `esbuild` npm distribution pattern (auto-download binary). `optionalDependencies` pattern if switching to napi-rs platform packages later. Existing npm package structure at `typescript/schemalint-zod/`.

**Test scenarios:**
- Happy path: `npm install -g @schemalint/cli` downloads and caches the binary, `schemalint --version` works
- Happy path: Second invocation uses cached binary (no download)
- Happy path: Binary SHA256 matches published checksum — mismatched checksum blocks execution with clear error
- Edge case: Platform not supported (e.g., FreeBSD) — clear error message with supported platforms list
- Edge case: Binary download fails (network error) — retry with backoff, clear error on exhaustion
- Error path: GitHub Releases binary not yet published for current version — clear error message

**Verification:**
- `npm pack` produces a valid `.tgz` for each package
- Local smoke test: `npm install -g ./npm/cli/schemalint-cli-*.tgz`, verify `schemalint --version`
- Three packages can be published to npm from the release workflow

---

- U14. **Unified release workflow**

**Goal:** Create a single GitHub Actions workflow triggered by a version tag push that builds all distribution artifacts, smoke-tests each, and publishes all channels atomically.

**Requirements:** R19, R20

**Dependencies:** U11, U12, U13

**Files:**
- Create/Modify: `.github/workflows/release.yml`

**Approach:**
- Trigger: `push: tags: ['v*']` + `workflow_dispatch` (manual fallback with tag input)
- Architecture: parallel matrix build jobs → single publish job with sequential channel releases
- Jobs:
  1. **github-release** (dist plan → build → host → publish → announce):
     - `dist plan` determines which packages to release based on the tag
     - `dist build` runs the platform matrix (GitHub-hosted runners for each target)
     - `dist host` uploads artifacts as GitHub Release assets
     - `dist publish` publishes to package managers configured in dist (crates.io via `cargo publish`)
  2. **pypi** (maturin matrix):
     - Builds wheels on macOS (x64, arm64), Linux (x64, arm64 via manylinux Docker), Windows (x64)
     - Uploads wheels as workflow artifacts
  3. **npm** (package build):
     - Syncs version from git tag: sets `version` in all `package.json` files
     - Runs `npm pack` for each package, uploads `.tgz` as artifacts
  4. **smoke-test** (runs after all builds, before publish):
     - Downloads each artifact, installs, runs `schemalint --version` + `schemalint check` on a known-good schema
     - Fails if any artifact doesn't produce expected output
  5. **publish** (runs after smoke-test passes):
     - Publishes to PyPI: downloads wheels, runs `maturin publish`
     - Publishes to npm: downloads `.tgz`, runs `npm publish` for each
     - `dist publish` handles GitHub Releases + crates.io
- Atomicity: the publish job runs only if ALL builds and smoke tests pass. If any channel publish fails, the remaining channels are not attempted (roll-forward, not roll-back — but sequential execution means failure stops before further publishes)
- Environment secrets: `CARGO_REGISTRY_TOKEN`, `MATURIN_PYPI_TOKEN`, `NPM_TOKEN`
- Release notes: auto-generated from conventional commits between the previous tag and the new tag (using `dist` or a `git cliff` step)

**Patterns to follow:** `dist` generated release workflow. `uv` release workflow pattern (astral-sh/uv) for multi-channel publish. Existing `ci.yml` patterns for Rust toolchain, caching, and Node setup.

**Test scenarios:**
- Happy path: Pushing `v0.1.0` tag triggers the workflow, all channels publish successfully
- Happy path: Smoke test catches a broken binary — publish step is skipped, workflow fails with clear error
- Edge case: PyPI publish fails (network error) → npm publish is not attempted, GitHub Release is already done (dist handles its own publish before this job)
- Edge case: Publishing a version that already exists on a channel → individual channel fails, reported clearly
- Error path: Missing secret (e.g., `NPM_TOKEN` not configured) → workflow fails fast in the build step with clear error

**Verification:**
- `release.yml` workflow runs on tag push (tested with a prerelease tag like `v0.1.0-rc.1`)
- All four channels receive the new version
- `schemalint --version` on each channel reports the correct version
- Smoke test results are visible in the workflow run summary

---

## System-Wide Impact

- **Interaction graph:** The new crates (`schemalint-conformance`, `schemalint-docgen`, `schemalint-python`) are workspace members but have no runtime interaction with the main `schemalint` CLI binary. The mock server communicates with the CLI only through CI orchestration (shelling out). The docgen binary reads rule metadata from the `schemalint` library crate but does not modify runtime behavior.
- **Error propagation:** Mock server errors (port binding, truth file parse failures) are surfaced at the CI level, not through the CLI. Docgen errors are surfaced at build time and in `docs.yml` CI checks.
- **State lifecycle risks:** npm binary download cache (`~/.cache/schemalint-npm/`) — stale cache risk if version pinning is incorrect. Mitigated by SHA256 verification on every download and version-keyed cache paths.
- **API surface parity:** The `Rule` trait gains a `metadata()` method with a default `None` return — backward compatible. No existing rule implementations break. The `schemalint` library's public API is unchanged.
- **Integration coverage:** Cross-layer scenarios — `conformance.yml` CI (U8) exercises mock server + schemalint CLI together; `full-harness.yml` (U10) exercises mock + CLI + corpus + `.expected` files; `release.yml` smoke tests exercise install + basic functionality per channel.
- **Unchanged invariants:** Existing CLI behavior, rule checking, output formats, and corpus tests are not modified. The `schemalint-profiles` crate remains zero-dependencies (truth files are embedded via `include_str!` as string constants, not parsed by the profiles crate itself — parsing happens in `schemalint-conformance`).

---

## Risks & Dependencies

| Risk | Mitigation |
|------|------------|
| `schemalint` package name already taken on PyPI or npm | Verify availability before U12/U13. If taken, use `schemalint-cli` or `schemalint-tool` as fallback. |
| npm @schemalint scope not available | Register under a different scope (e.g., `@1nder-labs/schemalint`) or publish unscoped. |
| cargo-dist CI template conflicts with existing `ci.yml` | dist generates `release.yml` — it does not modify existing workflow files. No conflict risk. |
| Truth file authored from documentation but documentation is incomplete or vague | Flag ambiguous keywords with `# UNVERIFIED: needs live API confirmation` comments. Burn windows (U9) serve as verification. |
| Live API rate limits prevent full harness completion | Monthly harness uses 1 req/s (75 seconds total) — well within any rate limit. Retry logic with exponential backoff for transient failures. |
| Cross-compilation failures for aarch64 or musl targets in CI | cargo-dist includes cross-compilation toolchains. Test `dist plan` early to catch missing toolchain dependencies. |
| GitHub Pages first-deploy requires manual Settings change | Add a note in the docs deployment README. This is a one-time setup step; the action documentation is clear. |
| `cargo publish` fails due to 10MB crate size limit | Check with `cargo package --list` early. Use `exclude` in `Cargo.toml` to strip large test data/corpus from the published crate. |

---

## Documentation / Operational Notes

- **One-time setup before first release:**
  1. Register `schemalint` on PyPI (reserve name)
  2. Register `@schemalint` npm scope (if using scoped packages)
  3. Set GitHub Pages source to "GitHub Actions" in repo Settings → Pages
  4. Configure environment secrets: `OPENAI_API_KEY`, `ANTHROPIC_API_KEY`, `CARGO_REGISTRY_TOKEN`, `MATURIN_PYPI_TOKEN`, `NPM_TOKEN`
  5. Enable OIDC trusted publishing on PyPI (preferred over long-lived tokens)
  6. Run `cargo login` with crates.io token (one-time per maintainer machine)
- **Secret rotation schedule:** Rotate API keys quarterly. Update via `gh secret set` in the `release` environment.
- **First release procedure:** Push a prerelease tag (`v0.1.0-rc.1`) to test the full release pipeline without publishing to primary channels. Verify all artifacts, smoke tests, and CI logs. Then push `v0.1.0`.
- **Provider drift response:** When a burn window or full harness detects drift, the maintainer reviews the filed issue, confirms against the provider's changelog, and either updates the truth file and linter profile or adjusts the `expected_error` if the behavior change is subtle.

---

## Sources & References

- **Origin document:** [docs/brainstorms/phase5-requirements.md](../../brainstorms/phase5-requirements.md)
- Related code: `crates/schemalint/src/rules/registry.rs` (Rule trait, Diagnostic, RuleSet)
- Related code: `crates/schemalint/src/cli/server.rs` (JSON-RPC server pattern)
- Related code: `crates/schemalint/src/cli/emit_gha.rs` (GHA workflow command escaping)
- Related code: `scripts/validation/validate_openai.py` (live API validation reference)
- Related code: `crates/schemalint/tests/server_tests.rs` (subprocess spawn test pattern)
- Related code: `crates/schemalint/tests/integration_tests.rs` (assert_cmd + predicates pattern)
- Inst. learnings: `docs/solutions/best-practices/schemalint-phase2-learnings.md`
- External: [cargo-dist docs](https://opensource.axo.dev/cargo-dist/)
- External: [maturin User Guide](https://www.maturin.rs/)
- External: [napi-rs docs](https://napi.rs/)
- External: [mdBook Guide](https://rust-lang.github.io/mdBook/)

---

## Deferred / Open Questions

### From 2026-05-01 review

The following decisions surfaced during document review and are deferred for maintainer judgment. Each entry includes the finding, its severity, and the reviewer's suggested resolution.

- **[P0] U14 release workflow: dist publish appears in both parallel and sequential jobs.** The `github-release` job (job 1) runs `dist plan → build → host → publish → announce` while the `publish` job (job 5) also says `dist publish handles GitHub Releases + crates.io`. Only one job should own the dist publish step. *Suggested: remove `publish` from job 1's flow (do only plan+build+host), let job 5 handle all publishes after smoke tests pass.*

- **[P0] R20 smoke test bypassed for GitHub Releases + crates.io.** Job 1 publishes to GitHub Releases (via `dist host`) and crates.io (via `dist publish`) before job 4's smoke-test runs. A broken binary could reach these channels without being smoke-tested. *Suggested: separate dist's host/publish from the build job. Build only in job 1; defer host+publish to job 5 after smoke tests pass.*

- **[P1] schemalint-profiles may be accidentally published to crates.io.** The plan claims it's "already excluded (no [[bin]])" but cargo publish publishes library crates regardless. The profiles crate has valid metadata. *Suggested: add `publish = false` to `crates/schemalint-profiles/Cargo.toml` or ensure release workflow uses `cargo publish --package schemalint`.*

- **[P1] npm package name mismatch: @schemalint/zod vs existing schemalint-zod.** `typescript/schemalint-zod/package.json` declares `name: "schemalint-zod"` (unscoped). Publishing from this file would produce `schemalint-zod`, not `@schemalint/zod`. *Suggested: either change the package.json name field, or keep the unscoped name and drop @schemalint/zod from R7.*

- **[P1] npm auto-download binary pattern creates first-run friction.** `@schemalint/cli` downloads the binary from GitHub Releases on first invocation, introducing network dependency for TypeScript users. The plan acknowledges napi-rs as better but defers to post-v1. *Suggested: consider napi-rs self-containment before v1, or provide an offline fallback.*

- **[P1] Packaging (Phase 4) sequenced last despite installation being the primary blocker.** The Problem Frame identifies that Python/TypeScript developers "can't install it without a Rust toolchain" but conformance (6 units) ships before packaging (4 units). *Suggested: reorder to Foundation → Packaging → Docs → Conformance, or at minimum prioritize PyPI to Phase 2.*

- **[P1] Truth files ship without pre-release live-API verification.** Authored from documentation only, verified by weekly burn windows that run after release. First users may get inaccurate rule guidance. *Suggested: run a one-time pre-release burn against live APIs to verify every truth entry before v0.1.0 ships.*

- **[P1] @schemalint/core delivered as placeholder contradicts R7 "programmatic API."** R7 promises a programmatic API but U13 delivers a placeholder requiring CLI on PATH. *Suggested: clarify in plan R7 that core ships as a placeholder, or defer core publishing until napi-rs containment lands.*

- **[P1] U8 comparison logic misses wrong-path diagnostics.** Linter error at an unexpected JSON Pointer path counts as "consistent (pass)" instead of a path_mismatch failure. *Suggested: add a fourth comparison case for path-mismatch failures.*

- **[P1] U8/U6 keyword coverage gaps silently tolerated.** Conformance CI skips keywords present in profile but missing from truth (and vice versa), allowing coverage to silently erode. *Suggested: add a coverage completeness check that fails CI on coverage gaps.*

- **[P1] U9 burn window green-washes provider drift.** Drift exits step code 1 but overall job succeeds (green checkmark), making drift invisible in the GitHub Actions dashboard. *Suggested: fail the overall job on drift AND upload the report artifact. Use `if: failure() \|\| success()` for artifact upload.*

- **[P1] U14 sequential channel publishes create partial-publish hazard.** The parallel github-release job publishes before the sequential PyPI/npm publish job validates. If PyPI succeeds but npm fails, GitHub Release is already live. *Suggested: use two-phase release — build all artifacts first, then single gated publish job. Or use a staging tag first.*

- **[P2] Three-tier conformance creates disproportionate maintenance surface.** 6 units, 3 scheduled workflows with indefinite maintenance tail. Zero cited incidents of rule inaccuracy to justify live-API tiers. *Suggested: ship synthetic mock (U5-U8) in v1, defer burn windows (U9) and full harness (U10) to v1.1 pending evidence of provider drift frequency.*
