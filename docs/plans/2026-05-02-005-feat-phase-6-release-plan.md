---
title: "feat: Phase 6 — v1.0 Release"
type: feat
status: active
date: 2026-05-02
origin: docs/brainstorms/phase6-requirements.md
---

# feat: Phase 6 — v1.0 Release

## Summary

Ship schemalint v1.0 by closing the 21-point line-coverage gap (69.66% → 90%), building a tag-driven multi-channel release workflow (cargo-dist + maturin + npm), codifying coverage and performance CI gates, bumping all artifacts to 1.0.0, and publishing release notes. The plan delivers 15 requirements across 11 implementation units.

---

## Problem Frame

Phase 5 delivered the distribution machinery: five-crate workspace, conformance mock, Docusaurus docs, maturin PyPI crate, npm auto-download wrapper. Phase 6 activates that machinery. Four gaps exist: (1) no release workflow exists — no `release.yml`, no tag-driven pipeline, no cross-platform smoke testing; (2) line coverage on the core crate is at 69.66%, ~21 points below the 90% threshold; (3) performance benchmarks pass all thresholds but are not codified as CI gates; (4) versions are at 0.1.0 across all artifacts.

---

## Requirements

- R1. `release.yml` triggers on `v*` tag push and builds all distribution artifacts.
- R2. Standalone binaries for 5 target triples via cargo-dist.
- R3. Python wheel via maturin from `schemalint-python` crate.
- R4. Three npm packages published (`@schemalint/cli`, `@schemalint/core`, `@schemalint/zod`), with npm publish after GitHub Release creation.
- R5. `schemalint` and `schemalint-profiles` published to crates.io.
- R6. Smoke tests on ubuntu/macos/windows matrix before any publish.
- R7. Coverage improved from 69.66% to ≥ 90% line on `crates/schemalint/src/`.
- R8. Coverage CI gate on every PR — fails if line coverage drops below 90%.
- R9. Branch coverage on rule files deferred to post-v1.0 (nightly tooling is unstable; line coverage gate in R8 is sufficient for v1.0 quality).
- R10. Bench-gate CI job on main asserts all 3 benchmarks under thresholds.
- R11. GitHub Release body auto-generated from conventional commits via git-cliff.
- R12. CHANGELOG.md updated with v1.0.0 section.
- R13. Every rule has +/- tests (verified met 2026-05-02; gate prevents erosion).
- R14. Binary name collision resolved by excluding `schemalint-python` from workspace CI commands.
- R15. All versions bumped lockstep to 1.0.0.
---

## Scope Boundaries

- **Code signing** — out of scope. Binaries are unsigned. macOS users need right-click → Open or `xattr -d com.apple.quarantine`.
- **Docker image, Homebrew, Chocolatey, Snap** — out of scope.
- **napi-rs native bindings** — out of scope. npm packages are auto-download wrappers (built in Phase 5).
- **Atomic rollback across channels** — PyPI/crates.io/npm do not support true rollback. Contract: all smoke tests pass before any publish begins.
- **Upgrading from 0.x** — v1.0 is the first public release; no migration path needed.
- **Post-v1 independent profile versioning** — mechanism designed; engine and profiles may version independently after v1.0.

### Deferred to Follow-Up Work

- **Signed binaries** — requires paid certificates and notarization infrastructure (post-v1).
- **napi-rs self-containment for `lint()`** — deferred per Phase 5 scope.
- **Automated rule-to-test mapping** — manual audit backed by coverage data is sufficient for v1.0.
- **Blog post distribution strategy** — the artifact is the post; where it's published is out of scope for engineering planning.

---

## Context & Research

### Relevant Code and Patterns

- **CI workflow** (`.github/workflows/ci.yml`): Single `test` job runs `cargo test --workspace`, `cargo clippy --workspace -- -D warnings`, `cargo fmt --all -- --check`, `cargo bench --no-run --workspace`, `cargo bench --workspace`. Uses `dtolnay/rust-toolchain@stable`, `Swatinem/rust-cache@v2`, `actions/setup-node@v4` (Node 22), `actions/checkout@v4`. Env: `CARGO_INCREMENTAL: 0`, `CARGO_TERM_COLOR: always`.
- **Docs deployment** (`.github/workflows/docs.yml`): Triggered on push to main, path-filtered to docs and rule sources. Deploys Docusaurus via `upload-pages-artifact@v4` + `deploy-pages@v5`. Auto-generates rule docs via `cargo run --bin schemalint-docgen`, checks staleness in `docgen-check` job.
- **Workspace** (`Cargo.toml`): 5 crates, resolver 2, shared version 0.1.0, MSRV 1.80. Release profile: `opt-level = 3`, `lto = true`, `codegen-units = 1`. Bench profile: `opt-level = 3`, `lto = false`, `codegen-units = 1`.
- **npm wrapper** (`npm/cli/index.js`): Hardcoded `VERSION = '0.1.0'` at line 11. Downloads from `https://github.com/1nder-labs/schemalint/releases/download/v{VERSION}/schemalint-{target}.{ext}`. Target mapping: `darwin-x64` → `x86_64-apple-darwin`, etc. Archive: `.tar.gz` (unix), `.zip` (windows).
- **Benchmarks** (`crates/schemalint/benches/schemalint_benchmarks.rs`, fixtures at workspace-root `benches/fixtures/`): Criterion 0.8, `harness = false`. Fixtures accessed via `PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..").join("benches/fixtures")`. 3 benchmark groups: `single_schema` (481µs measured), `cold_start` (7.2ms), `incremental` (1.7ms).
- **Test patterns**: `assert_cmd::Command::cargo_bin("schemalint")` + `predicates` for CLI integration; `std::process::Command` with piped stdin/stdout for server tests; `.expected` JSON for corpus; `tempfile::tempdir()` for isolation; `insta::assert_snapshot!()` for output snapshots; JSON-RPC 2.0 line protocol for server/subprocess tests.
- **Emit signatures**: `emit_human_to_string()`, `emit_json_to_string()`, `emit_sarif_to_string()`, `emit_gha_to_string()`, `emit_junit_to_string()` — all in `crates/schemalint/src/cli/emit_*.rs`.
- **Rule registration**: Three paths — static `linkme` slice, dynamic `RuleSet::from_profile()`, profile-gated. `code_prefix` is profile-driven.

### Institutional Learnings

- **Phase 2 test field name mismatch** (`docs/solutions/best-practices/schemalint-phase2-learnings.md`): Assertions on structured output must use emitter JSON field names (e.g., `pointer`), not Rust struct field names (e.g., `schemaPath`). 80+ tests once silently passed due to this mismatch. Coverage tests for emitters must follow this rule.
- **GHA workflow command escaping**: Use `encode_gha_value()` for `::error`/`::warning` commands — percent-encoding for `%`, `\r`, `\n`, `:`. Applies to smoke-test job annotations in `release.yml`.
- **Three rule registration paths**: Docgen and test coverage must handle all three. Coverage work should verify the docgen binary output is tested (currently at 0% — covered by U2-U4 targeting emitter tests).
- **Pipeline dedup**: `run_check` and `handle_check` duplicate ~30 lines. Phase 6 should not add a third copy. Coverage improvement should not require refactoring this — test the existing code as-is.
- **Conventional commits**: Already the project's style — `feat`, `fix`, `docs`, `refactor`, `test`, `ci`, `chore`, `fix(review)`, `feat(packaging)`, `feat(docs)`. git-cliff parsers should handle all observed prefixes.

### External References

- `cargo-llvm-cov` v0.8.5 (installed locally 2026-05-02): `--workspace --summary-only` produces per-file region/function/line/cover percentages. `--lcov --output-path lcov.info` produces parseable LCOV. Install in CI via `taiki-e/install-action@v2` with `tool: cargo-llvm-cov`.
- `cargo-dist` v0.25+: Use `dist init` to generate baseline config. Target triples must match `TARGET_MAP` in `npm/cli/index.js`. Archive naming must match `schemalint-{target}.{ext}` convention.
- `maturin` v1+: Build wheel from `schemalint-python` crate. Use `PyO3/maturin-action@v1` in CI with commit-SHA pinning.
- `git-cliff`: Single-binary changelog generator. Configure via `cliff.toml` at repo root. Supports conventional commits, Keep a Changelog, and GitHub Release body templates.
- `gh` CLI: Available in GitHub Actions runners. Use `gh release create` with `--generate-notes` or `--notes-file` for GitHub Release creation.

---

## Key Technical Decisions

| Decision | Rationale |
|---|---|
| Coverage gap closed by test-authoring, not refactoring | Existing logic is correct — only test coverage is missing. Adding tests to untested paths is lower risk and faster than restructuring code for testability. |
| Coverage gate on every PR (not just main) | Prevents erosion after the 90% threshold is reached. If the gate ran only on main, a PR could merge uncovered code that goes unnoticed until the next tag. |
| Bench gate on main only (not PRs) | Shared-runner benchmark noise makes per-PR gating unreliable. Main-only catches regressions before the next tag without blocking development on flaky comparisons. |
| npm publish after GitHub Release | `@schemalint/cli` downloads binaries from releases at runtime — the release must exist before npm users can install. |
| `schemalint-python` excluded from workspace CI, built via maturin separately | Zero code change. The binary name collision is a Cargo lint, not a correctness issue. Maturin handles the binary name correctly for `pip install schemalint`. |
| git-cliff for release notes | Single-binary, conventional-commits-native, no runtime dependency. Already used in the project's commit style. |
| All artifacts ship at 1.0.0 lockstep | Profiles are designed for independent versioning (date-encoded IDs), but for the first public release a single version number avoids confusion. |
| Unsigned binaries | Code signing requires paid certificates and ongoing maintenance. The UX cost is a one-time right-click → Open on macOS. |
| Coverage improvement ordered by ROI | CLI emitters first (fastest % gain, simplest to test with snapshot testing), then parser/normalizer, then cache, finally server/subprocess (requires subprocess spawning, slowest). |

---

## Open Questions

### Resolved During Planning

- **cargo-dist config:** Run `dist init` to generate baseline `release.yml` with cross-compilation. Adapt for npm-after-release chain. See U7, U8.
- **Release workflow `needs` chain:** `dist-build` → `build-wheel` → `smoke-test-matrix` → `publish-crates` + `publish-pypi` → `create-github-release` → `smoke-npm` (ubuntu + macos + windows) → `publish-npm`. See U8.
- **cargo-llvm-cov invocation:** `cargo llvm-cov --workspace --exclude schemalint-python --lcov --output-path lcov.info` then parse with `lcov --summary` or regex. See U5.
- **Benchmark result parsing:** Use `cargo bench --bench schemalint_benchmarks -- --output-format bencher` (stable machine-readable output). Parse `test <name> ... bench: <ns> ns/iter` lines. See U6.
- **git-cliff configuration:** `cliff.toml` at repo root with conventional-commit parsers. See U10.
- **npm wrapper VERSION:** Extract from `package.json` at runtime (`require('./package.json').version`). This works with all npm-compatible package managers (npm, yarn, pnpm) because `require` resolves relative to the script file regardless of package manager. See U9.
- **Rehearsal tag:** Not shipped — zero-value ceremony. The release workflow is CI configuration; if the first release fails, push a patch tag (v1.0.1).
- **npm post-publish registry install test:** Not included. The local tarball smoke test confirms the package is well-formed; the auto-download smoke test confirms the binary download chain works. Registry propagation delays are npm infrastructure, not a schemalint concern.
- **Branch coverage nightly CI:** Deferred to post-v1.0. Stable Rust lacks branch-level `llvm-cov` instrumentation; nightly tooling is unstable and the per-PR line coverage gate (R8) is sufficient for v1.0 quality.

### Deferred to Implementation

- **[Implementation]** Exact GitHub Actions secret names for PyPI, crates.io, and npm tokens — confirmed when setting up the release environment.
- **[Implementation]** Whether macOS quarantine blocks unsigned binaries during smoke tests — test during first release workflow run; add `xattr -d com.apple.quarantine` if needed.
- **[Implementation]** Exact blog post content and publishing location — written after release artifacts are verified.
- **[Implementation]** Whether the 5000-schema monorepo benchmark (< 5s target) passes — measured manually before tagging; documented in release notes.

---

## Implementation Units

### U1. Fix CI binary name collision

**Goal:** Exclude `schemalint-python` from workspace CI commands to eliminate the binary name collision warning, unblocking the subsequent CI changes (coverage and bench-gate jobs that use `--workspace`).

**Requirements:** R14

**Dependencies:** None

**Files:**
- Modify: `.github/workflows/ci.yml`

**Approach:**
- In the `test` job, add `--exclude schemalint-python` to: `cargo test --workspace`, `cargo clippy --workspace`, `cargo build --workspace` (if present). Add a separate step that builds `schemalint-python` via `cargo build -p schemalint-python` to confirm it compiles (the actual maturin build happens in U8's release workflow).
- In the `msrv` job, add `--exclude schemalint-python` to `cargo build --workspace`.
- The `cargo bench --workspace` already excludes non-benchmark crates automatically; no change needed unless it picks up `schemalint-python`.
- No code changes to any Rust file. The collision warning is a Cargo lint, not a correctness issue. Maturin handles the binary name correctly for `pip install schemalint`.

**Patterns to follow:**
- Existing CI conventions in `.github/workflows/ci.yml` (job names, step ordering, actions versions).

**Test scenarios:**
- Happy path: `cargo build --workspace --exclude schemalint-python` produces zero warnings about output filename collisions.
- Happy path: `cargo test --workspace --exclude schemalint-python` runs all tests (195+) and passes.
- Happy path: `cargo clippy --workspace --exclude schemalint-python -- -D warnings` passes.
- Edge case: `cargo build -p schemalint-python` compiles the maturin crate independently without workspace-level collision.

**Verification:**
- CI `test` job passes with zero warnings about binary name collision.
- All existing tests continue to pass (proof: `cargo test --workspace --exclude schemalint-python` exits 0).

---

### U2. Coverage improvement — CLI emitters

**Goal:** Add test coverage to the five CLI output emitters to close the largest coverage gap with the fastest ROI. Emitters currently at 71–100% line; target ≥ 95% each to contribute to the overall 90% threshold.

**Requirements:** R7

**Dependencies:** U1 (CI must be clean before adding tests)

**Files:**
- Modify: `crates/schemalint/tests/snapshot_tests.rs` (add emitter edge-case snapshots)
- Create or review existing: coverage for `crates/schemalint/src/cli/emit_json.rs` (already 100% — verify), `crates/schemalint/src/cli/emit_human.rs` (84%), `crates/schemalint/src/cli/emit_sarif.rs` (72%), `crates/schemalint/src/cli/emit_junit.rs` (77%), `crates/schemalint/src/cli/emit_gha.rs` (71%)

**Approach:**
- Each emitter has a `emit_X_to_string()` function. Tests exercise:
  1. **Multi-diagnostic output:** 2+ diagnostics of varying severity (error, warning, info) — verify correct formatting, ordering, grouping.
  2. **Empty diagnostic list:** verify each emitter produces valid empty output (no crashes, no malformed JSON/XML/GHA commands).
  3. **Edge-case diagnostic content:** diagnostics with empty messages, messages containing special characters (`%`, `:`, `\n`, `<`, `>`, `&` — relevant for SARIF XML-escaping, GHA percent-encoding), very long messages, Unicode content.
  4. **Source span presence/absence:** diagnostics with and without file/line/col source spans — verify each emitter handles both cases.
- Use `insta::assert_snapshot!()` for format-sensitive emitters (human, SARIF, JUnit, GHA) so regressions in output formatting are caught immediately.
- For JSON, validate structural correctness with `serde_json::from_str` and assert on key fields (not full-diff snapshots since JSON output is already at 100%).
- Follow the Phase 2 learnings: assert on emitter field names (e.g., JSON keys like `pointer`, `message`, `code`), not internal Rust struct field names.

**Patterns to follow:**
- Existing snapshot test pattern in `crates/schemalint/tests/snapshot_tests.rs`: `insta::assert_snapshot!()` after normalizing temp paths and durations with regex.
- Emitter function signatures (verified in codebase): `emit_human_to_string(diagnostics: &[(PathBuf, Vec<Diagnostic>)], total_errors: usize, total_warnings: usize, duration_ms: Option<u64>) -> String`. JSON, SARIF, JUnit, and GHA emitters also take `profile_names: &[String]` parameter. No emitter has a `sources: &HashMap<String, String>` parameter.

**Test scenarios:**
- Happy path: Two error diagnostics with source spans → human output shows `file.ts:line` format, JSON has `source` objects, SARIF has `locations`, JUnit has `file` + `line`, GHA has `::error file=file.ts,line=42::`.
- Happy path: One warning, one error → correct severity ordering per format (errors first in human, correct `level` in SARIF, correct `severity` in JUnit).
- Edge case: Empty diagnostic list → each emitter returns valid output (empty JSON array, empty SARIF `results`, empty JUnit `<testsuite>` with zero test cases, GHA produces no output, human prints summary with zero counts).
- Edge case: Message containing `%` and `:` → GHA output uses `encode_gha_value()` (percent-encoding), SARIF correctly escapes for JSON, JUnit correctly escapes for XML.
- Edge case: Message containing `<`, `>`, `&` → JUnit and SARIF XML output is properly escaped.
- Edge case: Diagnostic with no source span → human shows `(no source)`, JSON omits `source`, SARIF/JUnit handle gracefully.
- Edge case: Very long message (> 200 chars) → no truncation or formatting breakage.

**Verification:**
- `cargo llvm-cov --workspace --exclude schemalint-python -- --include 'cli/emit_*'` shows ≥ 95% line coverage on each emit file.
- `cargo test --test snapshot_tests` passes with new snapshots.
- Manual inspection of new snapshot files confirms correct formatting.

---

### U3. Coverage improvement — profile parser, normalizer, cache

**Goal:** Add test coverage for profile parser error paths, normalizer edge cases, and cache operations to close the second-tier coverage gaps.

**Requirements:** R7

**Dependencies:** U1

**Files:**
- Modify: `crates/schemalint/tests/profile_tests.rs` (add error-path tests)
- Modify: `crates/schemalint/tests/normalizer_tests.rs` (add edge-case tests)
- Modify or create: `crates/schemalint/tests/cache_tests.rs` (new file for cache-specific tests)
- Target source files: `crates/schemalint/src/profile/parser.rs` (81%), `crates/schemalint/src/normalize/traverse.rs` (66%), `crates/schemalint/src/normalize/mod.rs` (70%), `crates/schemalint/src/normalize/dialect.rs` (72%), `crates/schemalint/src/cache.rs` (61%)

**Approach:**
- **Profile parser** (target: 81% → 95%): Existing tests cover valid profiles, openai/anthropic profiles, code prefix overrides, restriction formats, structural limits, and basic error paths (invalid TOML, missing name, unknown severity). Missing coverage: additional TOML syntax errors (unclosed tables, invalid values, duplicate keys in restrictions), all `unknown` keyword severities exercised, structural limit boundary values (zero, max u32), empty profile (minimal valid), profile with only `name` field.
- **Normalizer** (target: 70% → 90%): Add tests for `traverse.rs` edge paths — schemas with deeply nested `$ref` chains, schemas with `$ref` pointing to non-existent definitions, schemas with `if/then/else` + `$ref` interactions, schemas with `allOf` containing multiple `$ref`s, boolean schemas inside compound keywords, type-array desugaring with additional keywords present.
- **Cache** (target: 61% → 85%): Add tests for cache insertion and retrieval (content-hash match), cache miss, cache eviction (when capacity is reached), disk cache round-trip (write to temp dir, read back), disk cache corruption (invalid version header, truncated file), concurrent access (basic multi-threaded insert/read — no race condition tests needed). Hash collisions are statistically impossible with the content hash used — no test needed.

**Patterns to follow:**
- Existing profile test pattern: `load(bytes).unwrap()` / `load(bytes).unwrap_err()`, then assert on `Profile` fields or error messages.
- Existing normalizer test pattern: `normalize(value).unwrap()` then assert on `Arena` state, `NodeId` fields, and `IndexMap` contents.
- Cache test pattern: `Cache::new()`, `cache.insert(hash, normalized)`, `cache.get(hash)`. Use `tempfile::tempdir()` for disk cache tests (existing `prop_disk_cache_roundtrip` in property_tests.rs provides the pattern).

**Test scenarios:**
- *Profile parser:* Invalid TOML with duplicate restriction keys → `Err`. Empty string profile → `Err`. Structural limit value zero → parses successfully with zero limit. Profile with only `name` field → parses, all keywords `unknown`. Unknown severity string → `Err`. Missing `[profile]` header → `Err`.
- *Normalizer:* Schema with `$ref` chain depth 10 → resolves correctly, no stack overflow. Schema with `$ref: "#/definitions/nonexistent"` → `Err` (or gracefully unresolved). Boolean `true` schema inside `allOf` → normalizes correctly. Schema with `if/then/else` + `$ref` in each branch → all branches resolved. `type: ["object", "null"]` with `properties` → desugared, properties attached to object variant. Empty object schema `{}` → normalizes to empty Arena with root node. Schema with 100+ `$ref`s → completes without timeout.
- *Cache:* Insert → get returns `Some`. Different content → cache miss returns `None`. Insert N+1 items when capacity is N → oldest item evicted, newest item present. Disk cache: write to temp dir → read back → content matches. Disk cache: truncated file → read returns `None` (not panic). Disk cache: invalid version header → read returns `None`.

**Verification:**
- `cargo llvm-cov --workspace --exclude schemalint-python -- --include 'profile/parser.rs' --include 'normalize/' --include 'cache.rs'` shows ≥ 90% line coverage on each file.
- `cargo test --test profile_tests`, `cargo test --test normalizer_tests` pass with new tests.
- No existing tests regress.

---

### U4. Coverage improvement — server and subprocess management

**Goal:** Add test coverage for the JSON-RPC server error paths and subprocess management error paths (Node and Python helpers).

**Requirements:** R7

**Dependencies:** U1, U2 (emit tests may reveal server output patterns to reuse)

**Files:**
- Modify: `crates/schemalint/tests/server_tests.rs` (add error-path tests)
- Modify: `crates/schemalint/tests/node_tests.rs` (add error-path coverage)
- Modify: `crates/schemalint/tests/python_tests.rs` (add error-path coverage)
- Target source files: `crates/schemalint/src/cli/server.rs` (70%), `crates/schemalint/src/node/mod.rs` (62%), `crates/schemalint/src/python/mod.rs` (55%)

**Approach:**
- **Server** (target: 70% → 85%): Add tests for malformed JSON-RPC messages (missing `jsonrpc`, missing `method`, non-JSON input), unknown method names, `check` with missing `params.schemas`, `check` with invalid schema JSON, concurrent requests (two requests in rapid succession without waiting for response), large schema payload (near the message size limit), and server shutdown via `shutdown` method. The existing tests cover happy-path single/multi-profile checks. Server tests use `std::process::Command` to spawn `schemalint server` child process with piped stdin/stdout.
- **Node subprocess** (target: 62% → 80%): The existing node_tests.rs has 19 tests including 7 end-to-end tests with real Node helper. Missing coverage: helper timeout (helper takes too long to respond), helper crash mid-response (simulate by sending `shutdown` then another request), invalid JSON-RPC responses from helper (malformed JSON), helper stderr output during normal operation, empty discover response (no schemas found), discover with large number of schemas (100+), JSON-RPC protocol violations from helper (wrong `id`, extra fields). Some of these require modifying the test helper fixture — the e2e tests already spawn a real Node subprocess; extend the fixture schemas to cover edge cases.
- **Python subprocess** (target: 55% → 80%): Mirror the Node subprocess error-path tests. Same categories: timeout, crash recovery, invalid responses, empty discover, stderr handling. The existing python_tests.rs has 12 tests; extend with Python-specific error paths: Python import error (package not found), Pydantic v1 model detection (already covered by R5 in Phase 3 — verify), helper crash during model extraction.

**Patterns to follow:**
- Server test: `std::process::Command::cargo_bin("schemalint").arg("server").stdin(Stdio::piped()).stdout(Stdio::piped()).spawn()`, then write JSON-RPC requests to stdin, read responses from stdout via `BufReader::read_line()`.
- Node/Python test: `assert_cmd::Command::cargo_bin("schemalint").args(["check-node", "--entrypoint", ...]).assert()` for CLI integration; `std::process::Command` for direct helper spawning.
- JSON-RPC error codes: `-32700` (parse error), `-32600` (invalid request), `-32601` (method not found), `-32602` (invalid params), `-32603` (internal error).

**Test scenarios:**
- *Server:* Malformed JSON input → server returns JSON-RPC error `-32700` and stays alive for next request. Unknown method → server returns `-32601` and stays alive. Missing `params` field → server returns `-32602`. Invalid schema JSON in `check` → server returns error with meaningful message. Two concurrent requests → both get correct responses with matching `id` fields. Large schema (100KB+) → server processes successfully or returns descriptive error without crashing. `shutdown` method → server exits cleanly.
- *Node:* Helper timeout → CLI emits clear timeout error (not raw spawn error). Helper returns malformed JSON → CLI emits parse error, does not crash. Empty discover → CLI reports zero schemas found. Stderr from helper during normal operation → captured and surfaced if CLI exits with error.
- *Python:* Same categories as Node. Additionally: Python package not found → CLI emits "package not found" error. Pydantic not installed in target env → CLI emits clear dependency error.

**Verification:**
- `cargo llvm-cov --workspace --exclude schemalint-python -- --include 'cli/server.rs' --include 'node/' --include 'python/'` shows ≥ 80% line coverage on each file.
- `cargo test --test server_tests`, `cargo test --test node_tests`, `cargo test --test python_tests` pass with new tests.
- No existing tests regress.
- Node tests pass on CI (Node 22 available).

---

### U5. Add coverage CI gate

**Goal:** Add a `coverage` job to `ci.yml` that runs `cargo-llvm-cov` on every PR and push to main, fails if line coverage on the core crate drops below 90%, and publishes the coverage report as a CI artifact.

**Requirements:** R8

**Dependencies:** U1 (CI clean), U2–U4 (coverage ≥ 90% — the gate must pass before this job can be useful)

**Files:**
- Modify: `.github/workflows/ci.yml`

**Approach:**
- Install `cargo-llvm-cov` via `taiki-e/install-action@v2` with `tool: cargo-llvm-cov`.
- Run `cargo llvm-cov --workspace --exclude schemalint-python --lcov --output-path lcov.info` (the `--exclude` flag needed per R14 resolution from U1).
- Parse the LCOV file to extract line coverage percentage for `crates/schemalint/src/`. Option A: use `lcov --summary lcov.info` and extract the `lines......: XX.X%` line. Option B: write a small inline script (bash + grep/awk) to sum `LF` (lines found) and `LH` (lines hit) for all source files under `crates/schemalint/src/` and compute `LH/LF * 100`.
- Assert that the computed percentage ≥ 90.0. If below, fail with: `"Line coverage: XX.X% (threshold: 90.0%). See coverage artifact for details."`
- Upload `lcov.info` as a CI artifact via `actions/upload-artifact@v4` for review.
- Branch coverage (R9) is shipped as a deferred commitment: stable Rust does not currently support branch-level `llvm-cov` instrumentation. The per-PR line coverage gate (R8) gate-keeps all coverage requirements. When stable Rust adds branch instrumentation, it activates without a separate plan.

**Patterns to follow:**
- Existing CI job conventions: `runs-on: ubuntu-latest`, `actions/checkout@v4`, `Swatinem/rust-cache@v2`.
- `CARGO_INCREMENTAL: 0` env (already in ci.yml) improves coverage accuracy.
- `taiki-e/install-action@v2` is the recommended pattern for installing Rust tools in CI.

**Test scenarios:**
- Happy path: Coverage ≥ 90% → job passes, artifact uploaded.
- Error path: Coverage < 90% → job fails with clear message showing current % vs threshold.
- Edge case: `lcov.info` is empty or malformed → job fails with parse error, not a silent pass.
- Edge case: `schemalint-python` crate not present → `--exclude schemalint-python` succeeds (already excluded by U1).
- Integration: On PR merge to main, coverage gate runs and must pass for branch protection.

**Verification:**
- Push a branch with known coverage below 90% → CI coverage job fails.
- Push a branch with coverage ≥ 90% → CI coverage job passes.
- Coverage artifact is downloadable from the GitHub Actions run summary.
- Weekly coverage-nightly workflow runs on schedule and produces artifact.

---

### U6. Add benchmark CI gate

**Goal:** Add a `bench-gate` job to `ci.yml` that runs the Criterion benchmarks on pushes to main and asserts all three benchmark groups are within their `phases.md` thresholds.

**Requirements:** R10

**Dependencies:** U1

**Files:**
- Modify: `.github/workflows/ci.yml`

**Approach:**
- Add a new job `bench-gate` that runs only on `push` events (not `pull_request`). This avoids flaky per-PR failures from shared-runner benchmark noise.
- Run `cargo bench --bench schemalint_benchmarks -- --output-format bencher` to get Criterion's stable machine-readable output. This produces lines like `test single_schema/parse_normalize_and_lint ... bench: <ns> ns/iter`.
- Parse each `test <name> ... bench: <ns> ns/iter` line. Extract the `ns/iter` value, convert to the appropriate unit (ms/µs), and compare against the threshold.
- Thresholds are advisory — the current fixture schemas are intentionally small and a 2x regression floor is the real detection signal. Accept the phases.md thresholds as ceilings; the actual gate is informed by current measured values. Document this in the job output.
  - `single_schema` median < 1.0 ms (current: ~0.48 ms)
  - `cold_start` (500 schemas) median < 500.0 ms (current: ~7.2 ms)
  - `incremental` (one changed in 500) median < 5.0 ms (current: ~1.7 ms)
- If any threshold is exceeded, fail with: `"bench_X: Y ms exceeds threshold Z ms"`.
- Document in the job output that the 5000-schema monorepo target (< 5 s) is verified as part of the maintainer's pre-tag checklist.

**Patterns to follow:**
- Criterion's `--output-format bencher` is the stable interface for CI parsing. It produces deterministic output unlike `--verbose` (which is debug-only and may change format between Criterion versions).
- Bench profile in `Cargo.toml`: `opt-level = 3`, `lto = false`, `codegen-units = 1`.

**Test scenarios:**
- Happy path: All benchmarks under thresholds → job passes.
- Error path: One benchmark exceeds threshold → job fails with clear message specifying which benchmark, measured time, and threshold.
- Edge case: Benchmark compilation fails → job fails (inherits `set -e` or explicit error handling).
- Edge case: Criterion output format changes → parse failure causes job failure (safe default).

**Verification:**
- Push to main after a change that doesn't affect performance → bench-gate passes.
- Temporarily lower a threshold below current performance → bench-gate fails with correct error message.
- PR does not trigger bench-gate job.

---

### U7. Configure cargo-dist and generate release CI

**Goal:** Configure `cargo-dist` for cross-compilation of standalone binaries across 5 target triples, use `dist init` to generate the baseline release CI, then adapt the workflow for the npm-after-release dependency chain.

**Requirements:** R1, R2

**Dependencies:** None (pure configuration)

**Files:**
- Modify: `Cargo.toml` (workspace root — add `[workspace.metadata.dist]`)
- Create: `.github/workflows/release.yml` (generated by `dist init`, then adapted)

**Approach:**
- Add `[workspace.metadata.dist]` to workspace `Cargo.toml`. Run `dist init` to generate the baseline `release.yml` with cross-compilation matrix, archive creation, and installer scripts. Cargo-dist handles: target toolchain installation, cross-compilation via `cross` or QEMU, `.tar.gz`/`.zip` archive creation, shell/powershell installers.
- Adapt the generated workflow: insert the smoke-test gate between build and publish, reorder jobs so npm publish runs after GitHub Release creation, remove any auto-generated crates.io publish for non-publishable crates.
- Only the `schemalint` crate produces binaries. `schemalint-python` is built via maturin (separate job, not cargo-dist).
- Verify archive naming: cargo-dist default is `schemalint-{target}.tar.gz` (unix) / `.zip` (windows). Must match `npm/cli/index.js` URL pattern. If it diverges, configure `[workspace.metadata.dist]` `archive-prefix` or update the npm wrapper (our code — one-line fix).
- Target triples: `x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu`, `x86_64-apple-darwin`, `aarch64-apple-darwin`, `x86_64-pc-windows-msvc`. Match `TARGET_MAP` in `npm/cli/index.js`.
- Installers: `shell`, `powershell`.

**Patterns to follow:**
- Phase 5 plan (U11) documented cargo-dist as the intended distribution mechanism. This unit activates that decision.
- The generated workflow uses `dist build` and `dist plan` — adapt, don't replace. The npm-after-release constraint is the only structural change.

**Test scenarios:**
- Happy path: `cargo dist plan` confirms 5 target triples, archive naming matches npm wrapper expectations.
- Happy path: `cargo dist build --artifacts=local` produces `.tar.gz`/`.zip` archives with correct naming.

**Verification:**
- `Cargo.toml` contains valid `[workspace.metadata.dist]` section.
- `release.yml` exists, uses `dist build` for binary artifacts, includes cross-compilation.

---

### U8. Build release workflow

**Goal:** Create `release.yml` — a GitHub Actions workflow triggered by `v*` tag push that builds all distribution artifacts, smoke-tests them on ubuntu/macos/windows, publishes to all four channels, and creates the GitHub Release with auto-generated notes.

**Requirements:** R1, R2, R3, R4, R5, R6

**Dependencies:** U7 (cargo-dist config), U5 (coverage gate must pass on main before tag), U6 (bench gate)

**Files:**
- Create: `.github/workflows/release.yml`

**Approach:**
U7's `dist init` generates the baseline `release.yml`. Adapt it with the following job dependency chain (the critical constraint is npm publish after GitHub Release because the npm wrapper auto-downloads from the release):

```
dist-build (cargo-dist matrix: cross-compilation + archive creation)
  ↓
build-wheel (maturin, linux only)
  ↓
smoke-test (matrix: ubuntu, macos, windows) — needs all build artifacts
  ↓
publish-crates (crates.io: schemalint + schemalint-profiles)
publish-pypi (PyPI: schemalint wheel)
  ↓
create-github-release (upload dist artifacts, generate notes)
  ↓
smoke-npm (test @schemalint/cli auto-download from the release)
  ↓
publish-npm (@schemalint/cli, @schemalint/core, @schemalint/zod)
```

Job details (adapted from dist-generated workflow):

1. **dist-build**: Cargo-dist's generated matrix handles cross-compilation (`cross` for Linux aarch64, native builds otherwise), archive creation (`.tar.gz` / `.zip`), and installer generation. No manual `cargo build` needed — cargo-dist owns the build, archive, and artifact upload.

2. **build-wheel**: Unchanged. `ubuntu-latest`, `PyO3/maturin-action@v1`, `maturin build --release --out dist`.

3. **smoke-test**: Matrix over `ubuntu-latest`, `macos-latest`, `windows-latest`. Downloads the platform-appropriate binary from dist artifacts. For binary: `./schemalint --version` (assert matches tag), `./schemalint check` minimal schema (assert exit 0). For wheel (ubuntu only): `pip install dist/*.whl && schemalint --version`. macOS: may need `xattr -d com.apple.quarantine`.

4. **publish-crates**: `cargo publish -p schemalint-profiles --no-verify` then `cargo publish -p schemalint --no-verify` (dependency first). Uses `CRATES_IO_TOKEN`. Retry-on-failure for transient registry propagation delay.

5. **publish-pypi**: `PyO3/maturin-action@v1` with `command: publish`, `args: --no-sdist`. Uses `PYPI_TOKEN`.

6. **create-github-release**: `softprops/action-gh-release@v2`. Uploads all dist artifacts. Generates notes from git-cliff (U10) or GitHub auto-notes. Uses `GITHUB_TOKEN`.

7. **smoke-npm**: Matrix over `ubuntu-latest`, `macos-latest`, `windows-latest`. Packs `npm/cli/` into tarball. Runs `npm install -g ./schemalint-cli-1.0.0.tgz && schemalint --version`. The wrapper downloads from the release just created — exercises the platform-specific download, extraction, and permissions paths (`.zip` + Expand-Archive on Windows, `.tar.gz` + chmod on Unix).

8. **publish-npm**: Publishes `npm/cli/` (`@schemalint/cli`), `npm/core/` (`@schemalint/core`), `typescript/schemalint-zod/` (`@schemalint/zod`). Uses `NPM_TOKEN`. Post-publish registry install is not tested — npm registry propagation takes minutes to hours and is transient infrastructure; the local tarball smoke test (step 7) plus the binary download test together cover the critical paths.

**Edge cases and failure handling:**
- If any build job fails, all dependent jobs are skipped.
- If smoke-test fails on any platform, no publish jobs run.
- If crates.io publish succeeds but PyPI publish fails, GitHub Release is created (binary-only). Maintainer retries PyPI manually — true atomic rollback is impossible across channels.
- Re-running the workflow on the same tag has idempotency issues. Use `needs` for the dependency chain. If a publish fails, push a new patch version — do not re-push the same tag.
- npm post-publish registry install failure is an accepted risk: npm CDN propagation is outside our control. The local tarball test confirms the package is well-formed; the auto-download test (smoke-npm) confirms the binary download chain works.

### U9. Version bumps to 1.0.0

**Goal:** Bump all version numbers from 0.1.0 to 1.0.0 across the workspace, npm packages, Python package, and the npm wrapper's hardcoded version constant.

**Requirements:** R15

**Dependencies:** None (can be done independently; must be committed before the release tag)

**Files:**
- Modify: `Cargo.toml` (workspace `version = "1.0.0"`)
- Modify: `crates/schemalint/Cargo.toml` (bump `schemalint-profiles` dependency from `version = "0.1.0"` to `version = "1.0.0"` — this is a hardcoded field, not workspace-inherited)
- Modify: `npm/cli/package.json` (`"version": "1.0.0"`)
- Modify: `npm/cli/index.js` (line 11: change `const VERSION = '0.1.0'` to extract from `package.json` at runtime)
- Modify: `npm/core/package.json` (`"version": "1.0.0"`)
- Modify: `typescript/schemalint-zod/package.json` (`"version": "1.0.0"`)
- Modify: `crates/schemalint-python/pyproject.toml` (`version = "1.0.0"`)

**Approach:**
- **Cargo.toml**: Change `[workspace.package] version = "0.1.0"` to `"1.0.0"`. All 5 crates inherit from workspace.
- **npm packages**: Bump `"version"` field in each `package.json`.
- **npm wrapper VERSION fix**: Replace the hardcoded `const VERSION = '0.1.0'` with `const VERSION = require('./package.json').version`. This ensures the download URL always matches the installed package version. The `./package.json` relative path works because `index.js` is in the same directory as `package.json` (confirmed by `npm/cli/package.json` `"files": ["index.js"]` and `"bin": { "schemalint": "./index.js" }`).
- **Python package**: Bump `version` in `[project]` section.
- **Verification**: `cargo metadata --no-deps --format-version 1 | jq '.packages[].version'` shows `1.0.0` for all crates. `npm pkg get version` in each npm directory shows `1.0.0`. The npm wrapper's `require('./package.json').version` is verified by running the wrapper with a test package.json.

**Patterns to follow:**
- Workspace version inheritance is already used (`version.workspace = true` in all crate `Cargo.toml` files).
- The Phase 6 requirements doc (R15) explicitly calls for lockstep version bumps.

**Test scenarios:**
- Happy path: All version files read `1.0.0`.
- Happy path: `cargo metadata` confirms all workspace crates at 1.0.0.
- Happy path: `npm/cli/index.js` correctly extracts `version` from `package.json` at runtime (test: create a temp dir with `package.json` `{"version": "1.0.0"}` and `index.js`, run `node -e "require('./index.js')"` — no crash, correct download URL constructed).
- Edge case: `package.json` is missing the `version` field → wrapper fails with clear error (not silently constructing a URL with `undefined`).

**Verification:**
- `grep -r "0.1.0" --include="*.toml" --include="*.json" --include="*.js"` returns no matches for version fields.
- `cargo build --workspace --exclude schemalint-python` succeeds.
- `node -e "console.log(require('./npm/cli/package.json').version)"` prints `1.0.0`.

---

### U10. Release notes infrastructure

**Goal:** Create a `cliff.toml` configuration for `git-cliff` to auto-generate Keep a Changelog-compliant release notes from conventional commits, and verify it works against the commit history.

**Requirements:** R11, R12

**Dependencies:** None

**Files:**
- Create: `cliff.toml` (repo root)
- Modify: `CHANGELOG.md` (updated with v1.0.0 section after release)

**Approach:**
- Create `cliff.toml` with:
  - Conventional commit parsers for `feat`, `fix`, `docs`, `refactor`, `test`, `ci`, `chore`, `perf`, `style`.
  - Grouping: `### Added` (feat), `### Changed` (refactor, perf, style), `### Fixed` (fix), `### Documentation` (docs), `### CI & Tooling` (ci, chore, test).
  - Template: Keep a Changelog format matching the existing `CHANGELOG.md` structure.
  - GitHub Release body template: Markdown with links to PRs and commit hashes.
- Configure `cliff.toml` to handle the project's observed commit scopes: `(packaging)`, `(docs)`, `(ci)`, `(review)`, `(docgen)`, `(conformance)`, `(rules)`, `(meta)`.
- Dry-run: `git cliff --unreleased --prepend CHANGELOG.md` (in dry-run mode or to a temp file) to verify the generated output looks correct.
- The actual CHANGELOG.md update happens in the release workflow (U8) when the `v1.0.0` tag is pushed. For pre-release verification, generate the notes to a temp file and review.
- After the release workflow succeeds, run `git cliff --tag v1.0.0 --output CHANGELOG.md` (or prepend) to update the file.

**Patterns to follow:**
- Existing `CHANGELOG.md` format: `## [Unreleased]` with `### Added`, `### Changed`, `### Fixed`, `### Removed`.
- Conventional commits: the project uses `feat`, `fix`, `docs`, `refactor`, `test`, `ci`, `chore` with optional scopes like `(packaging)`, `(docs)`, `(ci)`, `(review)`.

**Test scenarios:**
- Happy path: `git cliff --unreleased` generates Markdown output with all commits since the last tag grouped correctly.
- Happy path: Generated output includes links to GitHub commits/PRs.
- Edge case: Empty commit range → generates empty changelog section (not an error).
- Edge case: Commit with unrecognized type → falls into a catch-all group or is listed under `### Other`.

**Verification:**
- `cliff.toml` exists at repo root with valid TOML syntax.
- `git cliff --unreleased` produces output that matches Keep a Changelog conventions.
- Generated notes for the full commit history (first tag will include all commits since repo creation) are reviewable.

---

### U11. Verify docs site and prepare announcement

**Goal:** Verify the Docusaurus docs site reflects v1.0 content, write the release blog post, and confirm all quality gates are met before tagging.

**Requirements:** R13 (verification that every rule has tests — already met), R12 (CHANGELOG), docs readiness

**Dependencies:** U5 (coverage gate), U6 (bench gate), U9 (version bumps), U10 (release notes)

**Approach:**
- **Docs verification**: Confirm the Docusaurus site at `https://1nder-labs.github.io/schemalint/` is live and includes: all rule reference pages (keyword, restriction, semantic, structural), getting-started guides for Python and TypeScript, configuration reference, CI integration guide, and profile pages for OpenAI and Anthropic. Verify by visiting the site or checking the latest `docs.yml` deployment. No code changes needed — the site is deployed automatically on push to main.
- **Rule coverage verification**: Confirm every rule has positive and negative tests. Verified 2026-05-02 via manual audit. The coverage CI gate (U5) now enforces this going forward. No additional action needed.
- **Blog post**: Write a concise release announcement covering: what schemalint is (one sentence), what v1.0 delivers (multi-provider linting, Pydantic + Zod ingestion, SARIF/GHA/JUnit output, multi-channel distribution), and how to get started (one-liner install per channel). Publish location: TBD by maintainer — the artifact is the post content; where it's published is out of scope for engineering planning per the origin doc.
- **Pre-tag checklist**: Run the full test suite one final time, confirm coverage gate passes, confirm bench gate passes, confirm no warnings beyond the known `total_issues` dead-code warning in `node_tests.rs`.

**Patterns to follow:**
- The existing `README.md` serves as the base for the blog post's "what is schemalint" content.

**Test scenarios:**
- N/A — this is a verification and writing unit, not a code change.

**Verification:**
- Docs site is live and includes all v1.0 content.
- `cargo test --workspace --exclude schemalint-python` passes with zero failures.
- Coverage CI gate passes on main.
- Bench CI gate passes on main.
- Blog post is written and ready to publish.

---

## System-Wide Impact

- **Interaction graph:** The release workflow (`release.yml`) is additive — it does not modify the existing `ci.yml` or `docs.yml`. U1 modifies `ci.yml` to exclude `schemalint-python` from workspace commands. U5 and U6 add new jobs to `ci.yml` alongside existing ones. No other workflows are affected.
- **Error propagation:** Release workflow: if any build job fails, dependent jobs are skipped via `needs`. Publish failures are non-retryable within the workflow; maintainer manually retries the specific channel. Existing CI: coverage gate failure blocks PR merge; bench gate failure on main is informational (no automatic rollback).
- **State lifecycle risks:** The npm wrapper downloads binaries at runtime — there is no server-side state. The GitHub Release is the source of truth for binary availability. If a release is deleted, npm users who cached the binary are unaffected; new installs fail with a clear download error.
- **API surface parity:** No API changes. Version 1.0.0 is a SemVer signal, not a behavioral change. All existing CLI flags, JSON-RPC methods, and output formats remain identical.
- **Integration coverage:** The smoke-test jobs (U8) verify the end-to-end install → run chain for each channel. The npm smoke test verifies the auto-download → execute chain. These are the critical integration points.
- **Unchanged invariants:** CLI exit codes (0 clean, 1 error, 2 I/O error) unchanged. Rule error codes unchanged. Output format schemas (JSON, SARIF, JUnit, GHA) unchanged. Profile file format unchanged. JSON-RPC protocol unchanged. Corpus expected files unchanged.

---

## Risks & Dependencies

| Risk | Mitigation |
|------|------------|
| Coverage improvement is larger than estimated — reaching 90% requires touching code, not just adding tests | Coverage gap analysis (Phase 6 verification, 2026-05-02) confirmed the gap is in untested paths of correct code. Each unit is bounded to specific files with clear test scenarios. If a file resists test coverage (untestable error paths, platform-specific code), document as an explicit exclusion and adjust the threshold target. |
| cargo-dist archive naming does not match npm wrapper expectations | U7 explicitly verifies this. If cargo-dist defaults differ, configure `[workspace.metadata.dist]` with custom archive naming or update the npm wrapper URL pattern (the wrapper is our code — changing it is a one-line fix). |
| macOS smoke tests fail due to unsigned binary quarantine | U8 includes `xattr -d com.apple.quarantine` step if needed. If this doesn't work, document the right-click → Open workaround and move the macOS smoke test to a manual verification step. |
| crates.io first-publish validation fails | Pre-verify with `cargo publish --dry-run`. The Phase 5 plan (U1) fixed all metadata blockers (LICENSE, repository URL, description). |
| GitHub Actions macOS runner does not support `xattr` or blocks execution entirely | The smoke test can use `spctl --assess` to check notarization status. If the OS version on the runner enforces notarization, disable macOS smoke test for the first release and document the manual verification step. |
| `git-cliff` output doesn't match Keep a Changelog expectations | The `cliff.toml` is configured in U10 and verified with `--unreleased` dry run on the full commit history. If output is unsatisfactory, iterate on the template before the release. |
| Schemas in `benches/fixtures/project_500_schemas/` are too small to provide meaningful perf signal | The benchmarks already run in CI and produce stable results. The thresholds from phases.md are absolute. If the 500 small schemas don't exercise realistic workloads, larger fixtures ship as part of the benchmark suite. |

---

## Documentation / Operational Notes

- **Release process**: The maintainer pushes a `v1.0.0` tag. The release workflow runs automatically. After success: verify docs site, publish blog post, announce. If any channel fails: diagnose from workflow logs, fix, push a new patch tag (e.g., `v1.0.1`).
- **Post-release**: Update `CHANGELOG.md` `[Unreleased]` section to start fresh for post-1.0 work. Bump the workspace version in `Cargo.toml` to `1.0.1-dev` or similar.
- **npm wrapper drift**: The `require('./package.json').version` fix (U9) ensures the wrapper always downloads the correct binary. If the package is published with a mismatched binary, the error is clear: "Failed to download schemalint binary from {url}".
- **Coverage monitoring**: The coverage CI gate (U5) runs on every PR. A drop below 90% blocks merge. The bench-gate (U6) runs on push to main.
- **Benchmark monitoring**: The bench-gate (U6) runs on push to main. A regression after a merge is surfaced immediately. Investigate before the next tag.

---

## Sources & References

- **Origin document:** [docs/brainstorms/phase6-requirements.md](../brainstorms/phase6-requirements.md)
- **Phase specification:** [docs/phases.md](../phases.md#phase-6--release)
- **Phase 5 distribution plan:** [docs/plans/2026-05-01-003-feat-phase-5-distribution-conformance-plan.md](2026-05-01-003-feat-phase-5-distribution-conformance-plan.md)
- **Phase 5 Docusaurus plan:** [docs/plans/2026-05-02-004-feat-phase5-docusaurus-packaging-plan.md](2026-05-02-004-feat-phase5-docusaurus-packaging-plan.md)
- **Phase 2 learnings:** [docs/solutions/best-practices/schemalint-phase2-learnings.md](../solutions/best-practices/schemalint-phase2-learnings.md)
- **CI workflow:** [.github/workflows/ci.yml](../../.github/workflows/ci.yml)
- **Docs workflow:** [.github/workflows/docs.yml](../../.github/workflows/docs.yml)
- **Workspace manifest:** [Cargo.toml](../../Cargo.toml)
- **npm CLI wrapper:** [npm/cli/index.js](../../npm/cli/index.js)
- **Benchmarks:** [crates/schemalint/benches/schemalint_benchmarks.rs](../../crates/schemalint/benches/schemalint_benchmarks.rs)
- **Coverage measurement:** `cargo llvm-cov --workspace --summary-only` (run 2026-05-02)
- **Benchmark measurement:** `cargo bench --bench schemalint_benchmarks` (run 2026-05-02)
