---
date: 2026-05-01
topic: phase-5-distribution-and-conformance
---

# Phase 5 Requirements — Distribution and Conformance Infrastructure

## Summary

Phase 5 delivers schemalint as a multi-platform installable tool (cargo-dist binaries, PyPI wheel, npm packages), a mdBook documentation site with auto-generated rule reference, and a three-tier conformance verification system: a local synthetic mock driven by declarative truth files co-located in `schemalint-profiles`, weekly live-API burn windows, and a monthly full corpus harness — all centralized into a provider-pluggable architecture where adding a provider means adding a truth file and a linter profile.

---

## Problem Frame

Today schemalint can only be installed via `cargo install` — it is inaccessible to Python and TypeScript teams, the primary target users. There is no documentation site beyond the README. There is no automated way to verify that the linter's rules match what the real provider APIs actually do — the only conformance check is ad-hoc Python scripts in `scripts/validation/` that require manual invocation with API keys. The repository also has pre-distribution gaps: no LICENSE file, a wrong repository URL in `Cargo.toml`, and no CHANGELOG.

Without Phase 5, schemalint has zero distribution, no conformance verification, and no documentation site — a tool that cannot reach its users and cannot prove its own correctness.

---

## Pre-Work

Before distribution can begin, three blockers must be resolved:

- **Missing LICENSE file.** The workspace declares `license = "MIT OR Apache-2.0"` in `Cargo.toml` but no `LICENSE` file exists at the repository root. crates.io, PyPI, and npm all require a license file or will flag its absence.
- **Wrong repository URL.** `Cargo.toml` workspace `repository` field is `https://github.com/lahfir/schemalint`. The actual remote is `https://github.com/1nder-labs/schemalint`. This mismatch would cause crates.io to link to a nonexistent repository.
- **No CHANGELOG.** A `CHANGELOG.md` following Keep a Changelog conventions must exist before the first release.

---

## Actors

- A1. **Python developer** — Installs schemalint via `pip`, runs linting against Pydantic projects.
- A2. **TypeScript developer** — Installs schemalint via `npm`, runs linting against Zod projects.
- A3. **Rust developer** — Installs schemalint via `cargo install` or downloads a standalone binary.
- A4. **CI system** — Runs schemalint in automated pipelines, consumes SARIF/GHA/JUnit output, triggers conformance jobs.
- A5. **Conformance mock server** — Simulates provider API behavior from truth files for offline testing.
- A6. **schemalint maintainer** — Authors truth files, monitors burn window reports, responds to provider drift.

---

## Key Flows

- F1. **Install and run schemalint (any developer)**
  - **Trigger:** Developer runs `pip install schemalint`, `npm install -g @schemalint/cli`, or `cargo install schemalint`
  - **Actors:** A1, A2, or A3
  - **Steps:**
    1. Package manager fetches and installs the package
    2. CLI binary lands on PATH
    3. Developer runs `schemalint check`, `check-python`, or `check-node` with their project
    4. Diagnostics are emitted in the requested output format
  - **Outcome:** schemalint CLI is available and behaves identically regardless of install channel.
  - **Covered by:** R1, R2, R3, R4

- F2. **Browse documentation site**
  - **Trigger:** Developer navigates to the docs site
  - **Actors:** A1, A2, A3
  - **Steps:**
    1. Site loads with getting-started guides for Python and TypeScript
    2. Developer browses the rule reference — every registered rule has a dedicated page with code, severity, description, rationale, and example violations
    3. Developer reads the configuration reference to set up profiles and severity overrides
  - **Outcome:** Developer self-serves answers without reading source code or the README.
  - **Covered by:** R5, R6, R7

- F3. **Synthetic mock validates linter profiles in CI**
  - **Trigger:** PR is opened or pushed to main
  - **Actors:** CI system (A4), mock server (A5)
  - **Steps:**
    1. CI builds the mock server binary
    2. Mock server starts and loads truth files for each provider
    3. CI runs schemalint against the test schemas embedded in each truth file
    4. CI compares linter diagnostics against mock predictions — both false positives (linter rejects, API accepts) and false negatives (linter accepts, API rejects)
    5. Any mismatch fails the CI job
  - **Outcome:** Linter profiles are continuously verified against documented API behavior. No silent drift.
  - **Covered by:** R10, R11, R12, R13

- F4. **Weekly burn window detects provider drift**
  - **Trigger:** Scheduled GitHub Actions workflow (weekly)
  - **Actors:** CI system (A4)
  - **Steps:**
    1. Workflow submits a curated set of minimal "keyword-exercise" schemas to live OpenAI and Anthropic APIs
    2. Workflow compares live API responses against truth file declarations
    3. Divergence is captured in a structured report artifact
    4. Report distinguishes: provider drift, truth file errors, and transient API failures
  - **Outcome:** Provider behavior changes are detected within one week. Reports are reviewable without parsing raw API responses.
  - **Covered by:** R14, R15, R16

- F5. **Monthly full corpus harness**
  - **Trigger:** Scheduled GitHub Actions workflow (monthly)
  - **Actors:** CI system (A4)
  - **Steps:**
    1. Workflow runs the complete regression corpus (all 75+ schemas) through live OpenAI and Anthropic APIs
    2. Results are compared against synthetic mock predictions and existing `.expected` corpus files
    3. Any drift automatically files a GitHub issue with: affected schemas, expected vs actual behavior, and provider version/date
    4. Maintainer reviews, confirms whether drift is intentional, and updates truth files and profiles accordingly
  - **Outcome:** Authoritative monthly snapshot of provider behavior. Drift is documented and actionable.
  - **Covered by:** R17, R18

- F6. **Release workflow publishes all artifacts**
  - **Trigger:** Maintainer pushes a version tag (e.g., `v0.1.0`)
  - **Actors:** CI system (A4)
  - **Steps:**
    1. Workflow builds binaries for all target platforms via cargo-dist
    2. Workflow builds the Python wheel via maturin
    3. Workflow builds npm packages via napi-rs
    4. Smoke tests run against each artifact: install + `schemalint --version` + basic `schemalint check`
    5. All artifacts are published atomically — if any publish fails, none are published
    6. GitHub Release is created with binaries attached and auto-generated release notes from conventional commits
  - **Outcome:** All distribution channels updated in one operation. Smoke-tested before publish. No partial releases.
  - **Covered by:** R19, R20

---

## Requirements

### Pre-Distribution

- R1. A `LICENSE` file (MIT OR Apache-2.0) exists at the repository root, matching the workspace declaration.
- R2. The `Cargo.toml` workspace `repository` field is set to `https://github.com/1nder-labs/schemalint`, matching the actual git remote.
- R3. A `CHANGELOG.md` following Keep a Changelog conventions exists at the repository root with an initial `[Unreleased]` section.

### Packaging

- R4. schemalint CLI binary is installable via `cargo install schemalint` from crates.io with zero additional setup.
- R5. Standalone binaries for Linux (x86_64, aarch64), macOS (x86_64, aarch64), and Windows (x86_64) are published to GitHub Releases — downloadable and runnable with no Rust toolchain.
- R6. A Python wheel is published to PyPI that bundles the schemalint CLI binary. `pip install schemalint` installs the CLI on PATH without requiring a Rust toolchain.
- R7. npm packages are published: `@schemalint/cli` (CLI binary on PATH), `@schemalint/core` (programmatic API for Node), and `@schemalint/zod` (existing TypeScript helper). `npm install -g @schemalint/cli` puts `schemalint` on PATH without requiring a Rust toolchain.

### Documentation Site

- R8. A mdBook-based documentation site is built and published with: getting-started guides for Python and TypeScript projects, a complete configuration reference, and a per-rule reference page for every registered rule.
- R9. Rule reference pages are auto-generated from the compile-time rule registry metadata — rule name, error code, severity, description, rationale, and example violations. Adding a new rule to the registry automatically surfaces it in the docs without a separate docs edit.
- R10. The docs site is published to a stable URL and rebuilt on every push to main.

### Conformance: Synthetic Mock

- R11. A synthetic conformance mock server reads per-provider truth files from `schemalint-profiles/profiles/truth/` and exposes an endpoint that, given a JSON Schema, returns: whether the API accepts or rejects it, structured error messages with JSON Pointer paths where violations occur, and the transformed schema as the API would see it after stripping and default-filling (Tier 3 fidelity).
- R12. Truth files are declarative artifacts authored from official provider documentation. Each truth file covers every keyword in the provider's documented JSON Schema surface with: keyword name, expected behavior (accept/reject/strip), and at least one test schema that exercises the keyword in isolation.
- R13. The mock server engine is provider-agnostic — it reads any valid truth file and replays its declared behavior. Adding a new provider requires only a new truth file and a new linter profile; no mock server code changes.
- R14. A CI job runs on every PR and push to main: it starts the mock server, runs schemalint against the test schemas embedded in each truth file, and asserts that the linter's diagnostics are consistent with what the mock declares the API does. False positives (linter rejects, API accepts) and false negatives (linter accepts, API rejects) both fail the job.

### Conformance: Live-API Burn Windows

- R15. A scheduled GitHub Actions workflow (weekly) submits a curated set of minimal keyword-exercise schemas to the live OpenAI and Anthropic APIs using secrets-stored API keys. Each schema tests one keyword in isolation.
- R16. The burn window workflow compares live API responses against truth file declarations. Any divergence produces a structured report artifact attached to the workflow run.
- R17. The burn window report classifies divergence into three categories: provider drift (API behavior changed since the truth file was authored), truth file error (truth file declared wrong behavior), and API transient failure (rate limits, timeouts).

### Conformance: Full Monthly Harness

- R18. A scheduled GitHub Actions workflow (monthly) runs the complete regression corpus through live OpenAI and Anthropic APIs and compares results against both synthetic mock predictions and existing `.expected` files.
- R19. When the full harness detects provider drift, it automatically files a GitHub issue with: the affected schemas, expected vs actual API behavior, and the provider and date. When drift is confirmed intentional (provider policy change), the maintainer updates truth files and profiles accordingly.

### Release Pipeline

- R20. A `release.yml` GitHub Actions workflow builds and publishes all distribution artifacts — GitHub Release binaries, crates.io, PyPI wheel, and npm packages — from a single tag push. Artifacts are published atomically: if any channel fails, none are published.
- R21. The release workflow includes pre-publish smoke tests: install the CLI from the built artifact for each channel and verify `schemalint --version` prints the expected version and exits 0, plus a basic `schemalint check` invocation against a known schema succeeds.

---

## Acceptance Examples

- AE1. **Covers R4, R5.** `cargo install schemalint` on a machine with Rust 1.80+ installs the CLI. `schemalint --version` prints the version and exits 0. Same behavior when downloading and running the GitHub Release binary on a machine with no Rust toolchain.

- AE2. **Covers R6.** `pip install schemalint` on a machine with Python 3.9+ and no Rust toolchain installs the CLI on PATH. `schemalint --version` prints the version and exits 0.

- AE3. **Covers R7.** `npm install -g @schemalint/cli` on a machine with Node 18+ and no Rust toolchain installs the CLI on PATH. `schemalint --version` prints the version and exits 0.

- AE4. **Covers R11, R14.** Given a truth file declaring that OpenAI rejects `minimum` on strings but accepts `minimum` on numbers, and a linter profile that flags `minimum` on strings as forbidden: the CI job submits a schema `{ "type": "string", "minimum": 5 }` to the mock, the mock returns a rejection, schemalint flags the violation — CI passes (linter and mock agree). If the linter profile were misconfigured to allow `minimum` on strings, the CI job fails because the linter accepted what the API would reject.

- AE5. **Covers R13.** Adding a third provider requires: one truth file in `profiles/truth/`, one linter profile in `profiles/`, and zero code changes to the mock server or CLI engine. The mock automatically surfaces the new provider's behavior, and the CI consistency check covers it.

- AE6. **Covers R16, R17.** A weekly burn window runs against the OpenAI API. OpenAI suddenly starts accepting a keyword that was previously rejected (and the truth file declares it rejected). The burn window report captures: the keyword, the schema that triggered it, the live API response, the truth file declaration, and the classification "provider drift."

- AE7. **Covers R18, R19.** A monthly full harness run reveals that Anthropic now strips an additional keyword. The workflow files a GitHub issue titled "Provider drift (Anthropic): keyword `exclusiveMinimum` now stripped" with affected corpus schemas and a link to the workflow run artifacts.

- AE8. **Covers R20, R21.** Pushing tag `v0.1.0` triggers the release workflow. All four channels build and smoke-test. If the PyPI publish fails due to a network error, crates.io and npm publishes are also rolled back — no partial release. If all succeed, all four channels carry `0.1.0` within minutes of each other.

---

## Success Criteria

- Clean install from PyPI, npm, GitHub Releases, and crates.io on Linux, macOS, and Windows with no Rust toolchain required.
- Documentation site is live with complete rule reference (every registered rule has a page), configuration reference, and getting-started guides for both Python and TypeScript.
- Synthetic mock runs in CI on every PR and produces deterministic results — no flaky passes. False positives and false negatives both fail the job.
- Weekly burn windows execute without manual intervention and produce reviewable reports for both OpenAI and Anthropic.
- Monthly full harness completes within the GitHub Actions time limit and files issues when provider drift is detected.
- Release workflow publishes all artifacts from a single tag push with zero manual steps between channels.
- Existing Phase 1–4 regression corpus continues to pass — no regressions from packaging or conformance infrastructure changes.

---

## Scope Boundaries

- Docker image distribution — GitHub Releases, PyPI, npm, and crates.io only per SOW scope.
- Homebrew formula or other package managers beyond the four channels.
- Conformance for providers beyond OpenAI and Anthropic in v1 — the architecture is pluggable but only two providers ship.
- Provider-side conformance (validating that a user's provider integration code is correct) — schemalint validates schemas, not integration code.
- Auto-generation of linter profiles from truth files — profiles and truth files are maintained independently; consistency is verified by CI, not enforced by derivation.
- Auto-fix and schema rewriting — out of scope for all of v1.
- IDE / LSP integration for conformance results.
- A single universal truth file format — per-provider variation is allowed where provider behavior genuinely differs (e.g., Anthropic's silent stripping vs OpenAI's hard rejection).

---

## Key Decisions

| Decision | Rationale |
|---|---|
| Declarative truth files (TOML), not programmatic provider drivers | Truth files are diffable, reviewable, and authorable by non-Rust contributors. The mock engine is generic — one implementation reads any truth file. Adding a provider requires a data artifact, not code. |
| Tier 3 conformance fidelity (accept/reject + errors + transformation) | Anthropic silently strips keywords — without transformation output, the mock cannot validate that the linter's "strip" behavior matches reality. Tier 3 covers both providers with one model. |
| Truth files co-located in `schemalint-profiles` crate | Single crate for all provider data (linter config + conformance truth). Zero-deps crate keeps it lightweight. Truth files and profiles are versioned together, making cross-referencing trivial. |
| Standalone mock server binary, not embedded in the CLI | Conformance testing is a separate concern from schema linting. A standalone server is callable from CI, test suites, and external tools without CLI coupling. |
| GitHub Actions scheduled workflows for burn windows and full harness | Native CI integration requires no external scheduler. Secrets management is built into GitHub Actions. Reports are artifacts attached to workflow runs. |
| cargo-dist + maturin + napi-rs for packaging | Industry-standard tools for each target. cargo-dist handles cross-compilation and release artifacts. maturin produces auditable Python wheels. napi-rs gives native Node performance. |
| Truth files authored from provider documentation, validated by live API | Truth files capture documented behavior; live API testing verifies correctness. This decouples truth creation from API key availability and makes truth files reviewable like any configuration. |
| Atomic release — all channels or none | Prevents the situation where crates.io has v0.1.0 but PyPI still has v0.0.9. Users on any channel see a consistent version. |

---

## Dependencies / Assumptions

- `cargo-dist` v0.25+ supports the target triples: Linux (x86_64-unknown-linux-gnu, aarch64-unknown-linux-gnu), macOS (x86_64-apple-darwin, aarch64-apple-darwin), Windows (x86_64-pc-windows-msvc).
- `maturin` v1+ supports building Python wheels from a Rust workspace crate with a bundled binary.
- `napi-rs` v2+ supports building npm packages from a Rust workspace crate and cross-compiling to the target platforms.
- `mdbook` and a code-generation mechanism (build script or preprocessor) are sufficient for auto-generating rule reference pages from the rule registry metadata.
- Existing regression corpus schemas cover the documented keyword surface sufficiently to bootstrap truth file content.
- GitHub Actions supports scheduled workflows at weekly and monthly cadences with the required runtime (up to 6 hours for the full harness).
- GitHub Actions secrets are configured with valid OpenAI and Anthropic API keys with appropriate rate limits.
- The `schemalint-profiles` crate remains zero-dependencies — truth files are embedded via `include_str!()` like existing profiles; parsing happens in the mock server binary, not in the profiles crate.
- The `schemalint-zod` TypeScript package (at `typescript/schemalint-zod/`) and `schemalint-pydantic` Python package (at `python/schemalint-pydantic/`) are ready for their first publish — they have valid `package.json` and `pyproject.toml`, correct version numbers, and working builds.
- The mock server is a new binary within the existing `schemalint` crate or a new crate in the workspace — no new language runtime is introduced.

---

## Outstanding Questions

### Resolve Before Planning

*(none)*

### Deferred to Planning

- [Affects R5][Technical] Exact `cargo-dist` configuration — target triples, install-path layout, installer format per platform.
- [Affects R6][Technical] maturin project layout — whether the PyPI wheel uses a separate `pyproject.toml` at the workspace root or embeds Python metadata within the existing `schemalint` crate.
- [Affects R7][Technical] napi-rs project layout — whether npm packages live in a separate directory or are built from the workspace crate via a build script.
- [Affects R8–R10][Technical] mdBook auto-generation mechanism — build-time code generation (a Rust binary that writes markdown) vs a mdBook preprocessor plugin that reads rule metadata at build time.
- [Affects R11, R12][Needs research] Exact TOML schema for truth files — how keywords, test schemas, expected acceptance, expected errors, and expected transformation are structured. Must be expressive enough to model both OpenAI's rejection semantics and Anthropic's stripping semantics in one format.
- [Affects R11][Technical] Mock server HTTP framework and API surface design.
- [Affects R14][Needs research] Consistency check logic — how the CI job determines whether a linter diagnostic and a mock response agree or disagree. False positive/negative classification needs a precise definition that handles partial matches.
- [Affects R15][Technical] API key injection — whether keys come from GitHub Actions Secrets directly or from an external secrets manager.
- [Affects R19][Needs research] Auto-issue filing mechanism — `gh issue create` in the workflow vs GitHub API direct call vs a dedicated bot account.
- [Affects R20][Technical] Release workflow sequencing — job dependency graph to enforce atomic publish across all four channels.
- [Affects R8][Needs research] Docs site hosting — GitHub Pages vs custom domain with a static host.
- [Affects R9][Technical] Rule metadata completeness — whether the existing `Rule` trait carries enough metadata (description, examples, rationale) for auto-generated docs pages, or if metadata needs to be enriched.
- [Affects R20][Needs research] Release note generation — whether to use a conventional-commits parser, manually authored notes, or a hybrid approach.
