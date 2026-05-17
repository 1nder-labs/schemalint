---
date: 2026-05-02
topic: phase-6-release
---

# Phase 6 Requirements — v1.0 Release

## Summary

Tag v1.0, smoke-test every distribution channel on all target platforms, close the coverage gap to reach the `phases.md` thresholds (69.66% → 90% line, branch coverage on rules), lock performance benchmarks into CI gates, write release notes, and announce. Phase 6 verifies the Phase 5 distribution infrastructure, closes the quality gap, and ships.

**Verified 2026-05-02:** Performance benchmarks pass all thresholds with wide margins (481µs / 7.2ms / 1.7ms vs 1ms / 500ms / 5ms targets). Every rule implementation (all 11 static + Class A dynamic) has positive and negative tests. Line coverage on the core crate is at 69.66% — ~21 percentage points below the 90% target. The gap is concentrated in CLI emitters, cache, server, subprocess management, profile parser error paths, and normalizer edge paths — not in rule logic (rule files average 85%+). Closing this gap is a Phase 6 deliverable, not just a verification gate.

---

## Problem Frame

Phase 5 built the distribution machinery (cargo-dist binaries, maturin PyPI wheel, npm auto-download wrapper, Docusaurus docs, conformance mock). Phase 6 uses that machinery to ship v1.0. Four items must be completed:

1. **The release workflow does not exist** — no `release.yml`, no tag-driven pipeline, no cross-platform smoke testing.
2. **Coverage is below the 90% threshold.** Current line coverage on the core crate (`crates/schemalint/src/`) is 69.66% (measured 2026-05-02 via `cargo llvm-cov`). The gap of ~21 percentage points is concentrated in CLI emitters (`emit_gha.rs` 71%, `emit_human.rs` 84%, `emit_junit.rs` 77%, `emit_sarif.rs` 72%), cache (61%), server (70%), subprocess management (`node/mod.rs` 62%, `python/mod.rs` 55%), profile parser error paths (81%), and normalizer edge paths. Rule logic files average 85%+ and need minor gap closure. Branch coverage on rule implementations requires nightly Rust (`-Zcoverage-options=branch`) and has not been measured yet.
3. **Performance benchmarks are within all thresholds** (verified 2026-05-02) but are not codified as CI gates.
4. **Every rule has positive and negative tests** (verified 2026-05-02 via manual audit). This requirement is met.

Without Phase 6, schemalint is distributable in theory but never shipped — no tag, no release, no announcement, no verified quality baseline.

---

## Actors

- A1. **CI system** — Builds, smoke-tests, publishes, and gates on coverage/perf via GitHub Actions.
- A2. **schemalint maintainer** — Pushes the v1.0 tag, writes release notes and the announcement blog post, confirms coverage/perf gates.
- A3. **End user** (Python, TypeScript, Rust developer) — Installs v1.0 from their channel of choice and gets a working CLI on PATH.

---

## Key Flows

- F1. **Release workflow ships all channels**
  - **Trigger:** Maintainer pushes a `v*` tag (e.g., `v1.0.0`).
  - **Actors:** CI system (A1)
  - **Steps:**
    1. Workflow builds standalone binaries for Linux (x86_64, aarch64), macOS (x86_64, aarch64), Windows (x86_64) via cargo-dist.
    2. Workflow builds the Python wheel via maturin (from the `schemalint-python` crate).
    3. Workflow smoke-tests each built artifact: install + `schemalint --version` + basic `schemalint check` against a known-good schema. Runs on a matrix of Ubuntu, macOS, and Windows runners.
    4. Workflow publishes to crates.io (after all smoke tests pass).
    5. Workflow publishes the Python wheel to PyPI.
    6. Workflow creates the GitHub Release with binaries attached and auto-generated changelog from the tag range.
    7. Workflow smoke-tests the npm wrapper against the just-created GitHub Release (the wrapper auto-downloads from the release; this must run after step 6).
    8. Workflow publishes `@schemalint/cli`, `@schemalint/core`, and `@schemalint/zod` to npm.
  - **Outcome:** All four distribution channels carry the same version within minutes of each other. Every channel was smoke-tested before publish.
  - **Failure path:** If any smoke test fails or any publish step fails, the workflow stops and reports which channel failed. Full atomic rollback is impractical across PyPI/crates.io/npm (none of them support true rollback); the contract is "all smoke tests pass before any publish begins, and publish failures are reported clearly."
  - **Covered by:** R1, R2, R3, R4, R5, R6

- F2. **Coverage gate blocks release if thresholds unmet**
  - **Trigger:** CI run on any PR or push to main.
  - **Actors:** CI system (A1)
  - **Steps:**
    1. CI runs `cargo llvm-cov --workspace --lcov --output-path lcov.info`.
    2. Coverage report is parsed for the core crate (`schemalint`): expected ≥ 90% line coverage.
    3. Coverage report is parsed for rule implementation files (`rules/` directory): expected ≥ 95% branch coverage.
    4. If either threshold is unmet, the CI job fails with a clear message showing current vs target.
  - **Outcome:** Coverage regressions block merge. The v1.0 tag cannot be pushed until the gate passes on main.
  - **Covered by:** R7, R8

- F3. **Performance gate confirms thresholds**
  - **Trigger:** CI run on push to main (not every PR — benchmarks are noisy on shared runners and gating per-PR would cause flaky failures).
  - **Actors:** CI system (A1)
  - **Steps:**
    1. CI runs `cargo bench --bench schemalint_benchmarks` with the `bench` profile.
    2. Results are parsed: `bench_single_schema` median time, `bench_cold_start` median time, `bench_incremental` median time.
    3. Each is compared against the threshold from `phases.md`.
    4. If any threshold is exceeded, the job fails and emits the benchmark name, measured time, and threshold.
  - **Outcome:** A perf regression on main surfaces immediately. The v1.0 tag requires a passing main build.
  - **Covered by:** R9

- F4. **Release notes and announcement**
  - **Trigger:** After the release workflow succeeds (manual step).
  - **Actors:** Maintainer (A2)
  - **Steps:**
    1. Release notes are auto-generated from conventional commits via `git-cliff` and attached to the GitHub Release.
    2. Maintainer writes a blog post (or announcement post) summarizing what schemalint is, what v1.0 delivers, and how to get started. This is published to the project's announcement channel (blog, dev.to, or similar).
    3. `CHANGELOG.md` is updated with the v1.0.0 section from the generated notes.
  - **Outcome:** Users discover v1.0, understand what's new, and know how to install.
  - **Covered by:** R10, R11

---

## Requirements

### Release Workflow

- R1. A `release.yml` GitHub Actions workflow triggers on `v*` tag push (e.g., `v1.0.0`). It builds all distribution artifacts, smoke-tests them, publishes them, and creates the GitHub Release.

- R2. The workflow builds standalone binaries via cargo-dist for: `x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu`, `x86_64-apple-darwin`, `aarch64-apple-darwin`, `x86_64-pc-windows-msvc`. Binaries are unsigned (code signing is out of scope for v1).

- R3. The workflow builds the Python wheel via maturin from the `schemalint-python` crate. `pip install schemalint` installs the CLI binary on PATH. The wheel is a platform-specific binary wheel (not source distribution) for the same target triples as R2.

- R4. The workflow publishes three npm packages: `@schemalint/cli` (auto-download wrapper — the JS file at `npm/cli/index.js` already exists), `@schemalint/core` (programmatic API placeholder at `npm/core/index.js`), and `@schemalint/zod` (the existing TypeScript Zod helper at `typescript/schemalint-zod/`). The `@schemalint/cli` wrapper downloads the correct platform binary from the GitHub Release at runtime, so the npm publish must happen after the GitHub Release is created.

- R5. The workflow publishes `schemalint` and `schemalint-profiles` to crates.io. Only these two crates are published; `schemalint-conformance`, `schemalint-docgen`, and `schemalint-python` have `publish = false` in their `Cargo.toml` files and are internal-only.

- R6. The workflow includes a smoke-test job that runs after all builds complete but before any publish. Smoke tests verify on a matrix of `ubuntu-latest`, `macos-latest`, `windows-latest`:
  - **Binary:** Run the built binary with `--version` → prints the correct version and exits 0. Run `check` against a minimal valid schema → exits 0.
  - **Python wheel:** `pip install` the built wheel, run `schemalint --version` → prints the correct version and exits 0.
  - **npm:** `npm install -g` from the packed tarball, run `schemalint --version` → wrapper downloads the binary (from the just-created release) and prints the version. The npm smoke test runs as a separate job that depends on the GitHub Release creation step.

### Coverage

- R7. Coverage improvement work closes the gap from 69.66% to ≥ 90% line coverage on `crates/schemalint/src/`. Priority order: (1) CLI emitters — `emit_human.rs`, `emit_sarif.rs`, `emit_junit.rs`, `emit_gha.rs` (fastest ROI, straightforward to test); (2) profile parser — error paths for invalid TOML, missing fields, unknown severities; (3) normalizer — `traverse.rs` and `mod.rs` edge paths; (4) cache — eviction, disk I/O, hash collisions; (5) server — error paths, timeout handling; (6) subprocess management — `node/mod.rs` and `python/mod.rs` error paths. Each file's improvement is a test-authoring task: existing logic is correct; only test coverage is missing.

- R8. A `coverage` job in `ci.yml` runs `cargo llvm-cov --workspace --exclude schemalint-python` on every PR and push to main. The job fails if line coverage for `crates/schemalint/src/` drops below 90%. This is a regression gate — once the threshold is reached, it prevents erosion. The coverage report is also published as a CI artifact for review.

- R9. Branch coverage on rule implementations (`crates/schemalint/src/rules/`) is measured using `cargo +nightly llvm-cov --branch` in a scheduled (weekly) CI job. The target is ≥ 95% branch coverage. This gate is advisory (not required to merge) until nightly coverage tooling stabilizes; it surfaces regressions for maintainer review. Line coverage on rule files (measured via R8's stable job) serves as the blocking gate for rule code.

### Performance

- R10. A `bench-gate` job in `ci.yml` runs on pushes to main (not PRs, to avoid flaky gating on noisy shared-runner benchmarks). It runs `cargo bench --bench schemalint_benchmarks` and asserts:
  - `bench_single_schema` median < 1 ms. Current: ~481 µs (verified 2026-05-02).
  - `bench_cold_start` (500 schemas, 1 profile) median < 500 ms. Current: ~7.2 ms.
  - `bench_incremental` (one schema changed in 500) median < 5 ms. Current: ~1.7 ms.
  - The 5000-schema monorepo target (< 5 s) is verified manually and documented in release notes — it is impractically large for CI bench timeouts and resource limits.

### Release Documentation

- R11. The GitHub Release body is auto-generated from conventional commits using `git-cliff` (or equivalent) from the previous tag to the new tag. The generated notes are grouped by type (feat, fix, docs, etc.) following Keep a Changelog conventions. The maintainer may prepend a manual summary paragraph.

- R12. `CHANGELOG.md` is updated with the v1.0.0 section matching the generated release notes. The `[Unreleased]` section is cleared and a new empty one is started for post-v1 work.

### Quality Verification

- R13. Every rule already has positive and negative tests (verified 2026-05-02). As coverage improves to 90%, the CI coverage gate (R8) will prevent future erosion. No additional per-rule test work is required.

- R14. Before merging any further changes, the binary name collision warning between `schemalint` and `schemalint-python` (both produce `schemalint` binary) is resolved. The fix: `schemalint-python` keeps its `[[bin]] name = "schemalint"` (required for `pip install schemalint` to put the right name on PATH) but is excluded from `cargo build --workspace` default builds via CI configuration — the CI job builds it separately via maturin. In CI, `cargo build --workspace --exclude schemalint-python` and `cargo test --workspace --exclude schemalint-python` avoid the collision. The maturin build is tested separately. This is a CI hygiene fix, not a code change.

### Versioning

- R15. The workspace version in `Cargo.toml` is bumped to `1.0.0`. All crates in the workspace share this version for the v1.0 release. The npm package versions (`npm/cli/package.json`, `npm/core/package.json`, `typescript/schemalint-zod/package.json`) are bumped to `1.0.0`. The Python package version (`python/schemalint-pydantic/pyproject.toml`) is bumped to `1.0.0`. All versions ship in lockstep for v1.0.

- R16. Post-v1.0, engine and profiles may version independently under SemVer (as stated in `phases.md`). This is a future concern; v1.0 ships with a single version number across all artifacts.

---

## Acceptance Examples

- AE1. **Covers R1, R2, R3, R4, R5, R6.** Maintainer pushes tag `v1.0.0`. The release workflow builds binaries for 5 targets, the Python wheel, and npm tarballs. Smoke tests pass on Ubuntu, macOS, and Windows. crates.io, PyPI, npm, and GitHub Releases all carry `1.0.0`. A user on any platform can install from any channel and run `schemalint --version` → `schemalint 1.0.0`.

- AE2. **Covers R7, R8.** A PR introduces uncovered code in `rules/class_a.rs`. The coverage CI job fails with: "Branch coverage for crates/schemalint/src/rules/: 93.2% (threshold: 95%). Uncovered: class_a.rs:142-148." The PR cannot merge.

- AE3. **Covers R9.** A refactor accidentally introduces a regression in the parse-normalize-lint hot path, pushing `bench_single_schema` to 1.8 ms. The bench-gate job on main fails with: "bench_single_schema: 1.8 ms exceeds threshold 1.0 ms." The maintainer reverts or fixes before tagging.

- AE4. **Covers R4.** After GitHub Release creation, the npm smoke-test jobs run. On Ubuntu, `npm install -g @schemalint/cli` installs the wrapper. Running `schemalint --version` downloads the Linux x86_64 binary from the release and prints `schemalint 1.0.0`. Same on macOS and Windows with their respective binaries.

- AE5. **Covers R12.** The coverage report shows 100% branch coverage on `rules/class_a.rs`, `rules/class_b.rs`, and `rules/semantic.rs`. Manual inspection confirms every rule has both a positive test (fires on a violating schema) and a negative test (does not fire on a valid schema). The 29 static rules are covered; dynamic Class A rules are covered transitively through profile tests.

---

## Success Criteria

- Pushing a `v1.0.0` tag triggers the release workflow, which completes end-to-end without manual intervention (except PyPI/crates.io/npm tokens, which are pre-configured as GitHub Secrets).
- `pip install schemalint`, `npm install -g @schemalint/cli`, `cargo install schemalint`, and downloading the GitHub Release binary all install a working `schemalint` CLI that prints `1.0.0` on `--version` and lints a valid schema successfully.
- Coverage gate passes on main: ≥ 90% line coverage on the core crate, ≥ 95% branch coverage on rule implementations.
- Performance benchmarks on main are within all thresholds from `phases.md`.
- GitHub Release includes auto-generated release notes, attached binaries for 5 targets, and links to PyPI, npm, and crates.io.
- `CHANGELOG.md` has a complete `[1.0.0]` section.
- The announcement blog post is published and discoverable.

---

## Scope Boundaries

- **Code signing** for binaries (Apple notarization, Windows Authenticode) — out of scope. Binaries are unsigned. Users on macOS will need to right-click → Open or run `xattr -d com.apple.quarantine` on first launch. This is a documented limitation.
- **Docker image** — out of scope for v1 (per `phases.md` Phase 5 scope).
- **Homebrew formula, Chocolatey, Snap, or other package managers** — out of scope.
- **npm packages via napi-rs** (native Node bindings) — out of scope. The `@schemalint/cli` npm package is an auto-download wrapper as built in Phase 5. napi-rs-native `lint()` is deferred to post-v1 (per Phase 5 scope).
- **Release workflow atomic rollback** — PyPI, crates.io, and npm do not support true rollback. The contract is: all smoke tests pass before any publish begins. Publish failures are reported clearly; manual cleanup is the fallback.
- **Upgrading users from 0.x to 1.0** — v1.0 is the first public release. No migration path exists because no prior version was publicly distributed.
- **Post-v1 version independence for profiles** — the mechanism is designed (profile IDs include dates, profiles crate is separate), but profiles ship at 1.0.0 in lockstep with the engine for v1.0. Independent versioning activates post-v1.
- **Blog post distribution / promotion strategy** — the artifact is the post. Where it's published, how it's promoted, and social media strategy are out of scope for engineering requirements.

---

## Key Decisions

| Decision | Rationale |
|---|---|
| `cargo-llvm-cov` for coverage (not tarpaulin) | LLVM source-based coverage is fast, accurate, and works natively on all platforms without Docker. tarpaulin is slow, Docker-dependent, and has known accuracy issues on macOS. `cargo-llvm-cov` integrates with the `-C instrument-coverage` flag already in stable Rust. |
| Coverage gate on every PR (not just main) | Branch coverage regressions need to block merge, not just get noticed later. The core crate's line coverage threshold is a floor, not a target — it prevents erosion. |
| Performance gate on main only (not PRs) | Benchmark noise on GitHub's shared runners makes per-PR gating unreliable. Running on main after merge catches regressions before the next tag without blocking development. A separate manual `cargo bench` before tagging serves as the final check. |
| npm publish after GitHub Release (not before) | The `@schemalint/cli` wrapper downloads binaries from the GitHub Release at runtime. The release must exist before npm smoke tests can pass and before users can install. |
| `schemalint-python` excluded from `cargo build --workspace` in CI | The binary name collision is a Cargo lint, not a correctness issue. The simplest fix with zero code changes: build `schemalint-python` via maturin separately in CI and exclude it from `--workspace` commands. The collision warning is cosmetic in dev; maturin handles the binary name correctly for `pip install schemalint`. |
| All artifacts ship at `1.0.0` in lockstep | Profiles are designed for independent versioning (date-encoded IDs), but for the first public release, a single version number across all artifacts is simpler for users and avoids confusion. Independent versioning activates post-v1. |
| `git-cliff` for auto-generated release notes | Conventional commits are already the project's commit style. `git-cliff` is a single-binary tool that generates Keep a Changelog-compliant notes from commit history. No runtime dependency, no config drift. |
| Unsigned binaries for v1 | Code signing (Apple notarization, Windows Authenticode) requires paid certificates, notarization infrastructure, and ongoing maintenance. The UX cost is a one-time right-click → Open on macOS. The engineering cost is zero. Users who need signed binaries can build from source or use a package manager. |
| Manual verification of "every rule has +/- tests" | Automated verification requires a rule-to-test mapping that doesn't exist in the current test structure. The coverage report provides the signal (uncovered branches in rule files). Manual confirmation backed by coverage data is sufficient for v1.0; automated mapping can be added post-v1. |
| Workspace-wide version bump to `1.0.0` | All crates, npm packages, and Python packages ship at 1.0.0. The `npm/cli/index.js` hardcoded `VERSION = '0.1.0'` must be updated to `1.0.0` (and ideally should read from `package.json` at runtime to prevent future drift). |

---

## Dependencies / Assumptions

- `cargo-dist` v0.25+ is configured and working for cross-compilation of standalone binaries. The tool must support the target triples listed in R2.
- `maturin` v1+ is configured and working for building a platform-specific Python wheel from the `schemalint-python` crate.
- `cargo-llvm-cov` is installed in CI (via `cargo install cargo-llvm-cov` or the `taiki-e/install-action` GitHub Action).
- `git-cliff` is installed in the release workflow for changelog generation.
- GitHub Secrets are configured with: `CRATES_IO_TOKEN`, `PYPI_TOKEN`, `NPM_TOKEN`, and `GITHUB_TOKEN` (the latter is auto-provided by Actions).
- The `npm/cli/index.js` wrapper correctly downloads the binary from `https://github.com/1nder-labs/schemalint/releases/download/v{VERSION}/schemalint-{target}.{ext}` where `{ext}` is `.tar.gz` (Linux/macOS) or `.zip` (Windows). Cargo-dist's default archive naming must match this pattern, or the wrapper must be updated to match.
- The `npm/core/index.js` placeholder is published as-is at v1.0.0 with a note that the programmatic API requires the CLI binary on PATH (matching Phase 4's documented limitation for `@schemalint/zod`).
- `typescript/schemalint-zod/package.json` is bumped to `1.0.0` and the `@schemalint/zod` package is published alongside the CLI packages.
- The `python/schemalint-pydantic/pyproject.toml` version is bumped to `1.0.0` and the `schemalint-pydantic` package is published (or prepared for publish — the package is pure Python and can be published independently from the Rust release cycle).
- Existing CI jobs (test, clippy, fmt, benchmark-compile) continue to run on every PR and push to main. Phase 6 adds coverage and perf-gate jobs alongside them; it does not replace or weaken existing gates.
- The Docusaurus docs site (deployed via `docs.yml`) is already live at `https://1nder-labs.github.io/schemalint/`. No Phase 6 changes needed for docs beyond verifying the site is live and reflects v1.0 content.
- The `schemalint-conformance` and `schemalint-docgen` crates have `publish = false` and are excluded from the crates.io publish step.

---

## Outstanding Questions

### Resolved (verified 2026-05-02)

- **Coverage baseline:** 69.66% line on core crate, 0% on `metadata.rs` (dead code), 0% on docgen and python crate binaries (internal tools). Rule files: semantic.rs 97.4%, class_a.rs 90.9%, class_b.rs 66.7%, registry.rs 80.4%. Coverage improvement is R7.
- **Benchmark baseline:** All three benchmarks pass thresholds with wide margins (481µs, 7.2ms, 1.7ms vs 1ms, 500ms, 5ms). Bench gate is R10.
- **Per-rule tests:** All 11 static rule implementations have positive and negative tests. Class A dynamic rules (KeywordRule, RestrictionRule) covered transitively via profile tests. Requirement met.

### Deferred to Planning

- **[Technical]** Exact `cargo-dist` configuration — `dist.toml` or `[package.metadata.dist]` in Cargo.toml, target triples, archive format, install-path layout.
- **[Technical]** Release workflow job dependency graph — the exact `needs` chain that enforces: build → smoke-test → publish (with npm publish after GitHub Release).
- **[Technical]** `cargo-llvm-cov` exact invocation and output parsing — whether to use `--lcov` and parse with a tool, or use `--json` and extract from the summary.
- **[Technical]** Benchmark result parsing in CI — whether to parse criterion's `--verbose` output, use `criterion-table`, or write a small post-processing script.
- **[Technical]** How `git-cliff` is configured — `cliff.toml` at repo root with commit parsers for conventional commits, grouping rules, and template for GitHub Release body and CHANGELOG.md.
- **[Technical]** Whether to extract `VERSION` from `package.json` at runtime in `npm/cli/index.js` instead of the hardcoded `const VERSION = '0.1.0'` to prevent version drift between the npm package and the download URL.
- **[Process]** Which secrets manager or GitHub Environment is used for PyPI, crates.io, and npm tokens, and whether they require maintainer approval for publish workflows.
- **[Process]** Where the announcement blog post is published (project blog, dev.to, maintainer's personal blog) and whether it's reviewed before publishing.
- **[Needs research]** Whether GitHub Actions macOS runners support running unsigned binaries for smoke tests without user interaction. If the quarantine gate blocks the binary, the smoke test must `xattr -d com.apple.quarantine` the binary before invoking it.
