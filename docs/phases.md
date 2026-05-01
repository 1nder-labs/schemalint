# schemalint — Engineering Phases

This document breaks the v1 implementation into discrete engineering phases. Each phase has a defined boundary and completion criteria. No effort estimates or scheduling are included here; this is a purely technical ordering of what gets built and when.

---

## Phase 0 — Validation Spike

Before any production code is written, validate the riskiest architectural assumptions.

### What gets built

- **Child-process latency benchmark.** A throwaway harness that spawns long-lived Python and Node processes communicating via JSON-RPC over stdin/stdout. Measures end-to-end latency for Pydantic v2 model discovery and Zod schema extraction on a representative codebase (≥ 50 models). Specifically measures cold-import cost of Pydantic v2 (which routinely hits 800 ms–2 s due to `pydantic_core`, `typing_extensions`, and user model graphs) and process pool amortization across multiple schemas.
- **Regression corpus.** A curated collection of 50+ real-world JSON Schemas scraped from public bug reports, provider forums, and SDK issues (sources documented in the SOW §20). Each schema is annotated with expected diagnostics. This corpus becomes the acceptance test for all subsequent phases.

### Completion criteria

- Latency benchmark produces a go/no-go decision on the JSON-RPC process pool design against the 500 ms cold-start budget. If the budget is breached, a pivot plan is documented before Phase 1 begins.
- Regression corpus exists in `tests/corpus/` with at least 50 schemas, each with a deterministic expected diagnostic set.

---

## Phase 1 — Foundation

Build the minimal end-to-end pipeline: parse a schema, load a profile, detect mismatches, emit output. No hand-written semantic rules yet; only what the profile TOML can drive automatically.

### What gets built

- **Repository structure.** Two crates: `schemalint` (monolithic engine containing IR, normalizer, rules, and CLI) and `schemalint-profiles` (data-only; independent release cadence).
- **Internal Representation (IR).** Arena-allocated schema graph with `NodeId(u32)` indexing, stable JSON Pointers, parent links, source spans, and reference-aware `$ref` edges.
- **Schema normalizer.** Dialect detection, `$ref` graph resolution (no inline expansion), Tarjan SCC cycle detection, type-array desugaring, parent/depth/JSON Pointer computation, stable content-hash for caching.
- **Profile loader.** TOML parser for the five-state severity model (`allow`, `warn`, `strip`, `forbid`, `unknown`). Profiles compile to `HashMap<&'static str, Severity>` once per process.
- **Auto-registered rule registry.** Class A rules (profile-derived keyword rules) auto-generate from the loaded profile. Class B rule infrastructure is in place using `inventory` or `linkme` distributed slices — no hand-written Class B rules yet, but the registry is ready for them.
- **Schema-to-profile diff tool.** A minimal CLI command that reads a JSON Schema and a profile, then emits raw structural mismatches (e.g., "schema uses `minimum` at `/properties/x`; profile marks `minimum` as `forbid`"). No severity overrides, hints, SARIF, or configuration — just typed inventory of keyword-profile mismatches.
- **CLI with human and JSON output only.** Batch mode only.
- **OpenAI Structured Outputs profile.** Complete TOML profile covering all keywords emitted by Pydantic v2 and `zod-to-json-schema`. Grounded in live OpenAI docs (`developers.openai.com/api/docs/guides/structured-outputs`) as of 2026-04-30. Less common keywords may remain `unknown` with explicit scope notes.

### Completion criteria

- The diff tool runs end-to-end against every schema in the regression corpus and produces deterministic output.
- All 50 corpus schemas from Phase 0 produce expected diagnostics when run through the diff tool.
- The OpenAI profile has zero `unknown` states for keywords in the Pydantic/Zod emission surface.

---

## Phase 2 — Rules and Multi-Profile

Add hand-written semantic rules, the second provider profile, multi-profile composition, and additional output formats.

### What gets built

- **Class B semantic rules.** Hand-written rules for checks that are not single-keyword presence tests: cycle depth bounds, total enum cardinality, "all properties in required," empty-object detection, `additionalProperties: {}` detection, discriminator hints for `anyOf` over objects.
- **Anthropic Structured Outputs profile.** Complete TOML profile with the same coverage standards as the OpenAI profile.
- **Multi-profile composition.** The engine can run with multiple active profiles (`--profile openai --profile anthropic`) and emits the union of forbid/strip/warn rules. Each diagnostic identifies which profile produced it. The canonical use case — "write one schema that works on both providers" — is the primary framing for all examples and documentation.
- **SARIF v2.1.0 output.** Required for GitHub code scanning, Azure DevOps Advanced Security, and enterprise CI dashboards.
- **GitHub Actions annotation output.** Native `::error` and `::warning` workflow commands.
- **JUnit XML output.** For CI systems that surface JUnit results in PR checks (GitLab, Jenkins, CircleCI).
- **Anthropic-specific regression corpus.** 25 additional schemas covering Anthropic-specific failure modes (SDK stripping, optional parameter budgets, union budgets, strict tool budgets).
- **Server mode (`--watch`).** Streaming JSON-RPC server over stdin/stdout. Reuses the same engine, normalizer, and rule registry as the batch CLI; only the emitter changes.

### Completion criteria

- All 29 rules from the v1 catalogue are implemented and registered.
- Existing Phase 1 corpus passes with the full rule engine (not just the diff tool).
- Anthropic-specific corpus of 25 schemas is added and produces expected diagnostics.
- Multi-profile runs produce correct unions of diagnostics.

---

## Phase 3 — Pydantic Ingestion

Add first-class Pydantic model discovery with source-span mapping and Python configuration support.

### What gets built

- **`schemalint-pydantic` helper.** A pip-installable Python package that runs as a long-lived JSON-RPC server over stdin/stdout. The Rust CLI maintains a process pool and sends discovery requests.
- **Model discovery.** Imports the target Python package, walks for `pydantic.BaseModel` subclasses, calls `Model.model_json_schema()` (v2) or `Model.schema()` (v1), resolves source locations via `inspect`, and returns JSONL records via JSON-RPC.
- **Heuristic source-span resolution.** Per-field line resolution via string match against `Model.model_fields[name]`. Falls back to the field declaration line for `Annotated[T, Field(...)]` inside generics.
- **Python configuration.** `pyproject.toml` support: `[tool.schemalint]` section with profile selection, severity overrides, include/exclude globs, and Pydantic discovery settings.

### Completion criteria

- Can lint a representative real-world Pydantic codebase end-to-end.
- Correct diagnostics on at least 10 known issues from the regression corpus.
- Source spans resolve to the correct file and line for standard field declarations.

---

## Phase 4 — Zod Ingestion

Add first-class Zod schema discovery with TypeScript source mapping and Node configuration support.

### What gets built

- **`schemalint-zod` helper.** An npm-installable TypeScript package that runs as a long-lived JSON-RPC server over stdin/stdout. The Rust CLI maintains a process pool.
- **Schema discovery.** Loads the TypeScript project, locates `z.object(...)` call expressions, evaluates each schema in a sandboxed context, converts via `zod-to-json-schema`, captures the original `CallExpression` location, and returns JSONL records via JSON-RPC.
- **Programmatic API.** `@schemalint/zod` exposes `lint(schemas[], options)` for direct use in Node code.
- **TypeScript configuration.** `package.json` support: `"schemalint"` field with profile selection, severity overrides, include/exclude globs, and Zod discovery settings.

### Completion criteria

- Can lint a representative real-world Zod codebase end-to-end.
- Correct diagnostics on at least 10 known issues from the regression corpus.
- Source spans resolve to the correct file and line for standard `z.object()` declarations.

---

## Phase 5 — Distribution and Conformance Infrastructure

Package the tool for all target platforms and build the conformance test harness.

### What gets built

- **Core packaging channels.** GitHub Releases (standalone binaries via `cargo-dist`), PyPI (`schemalint` CLI bundled + `schemalint-py` library via `maturin`), npm (`@schemalint/cli`, `@schemalint/core`, `@schemalint/zod` via `napi-rs`), and crates.io (`schemalint` + `schemalint-profiles`).
- **Documentation site.** mdBook-based site with auto-generated rule reference pages (built from the auto-registered rule registry metadata), configuration reference, and getting-started guides for Python and TypeScript projects.
- **Synthetic conformance mock.** A local server driven directly from profile TOML that simulates provider validation responses. Runs in daily CI to catch linter configuration regressions.
- **Live-API burn windows.** Weekly 15-minute sessions against real OpenAI and Anthropic APIs using minimal schemas that exercise one keyword at a time. Catches provider drift at a frequency that matters.
- **Full conformance harness.** Monthly complete corpus run against live APIs as the authoritative ground truth. Auto-files issues and draft PRs when drift is detected.

### Completion criteria

- Clean install from PyPI, npm, GitHub Releases, and crates.io on Linux, macOS, and Windows.
- Documentation site is live with complete rule reference, configuration reference, and getting-started guides.
- Synthetic mock runs in CI and produces deterministic results.
- Burn windows execute against live APIs and report results.
- Full conformance harness is operational and can run the complete corpus.

---

## Phase 6 — Release

Tag, verify, and announce v1.0.

### What gets built

- **Release artifacts.** Tagged release with binaries, wheels, npm packages, and crates published from a single GitHub Actions workflow.
- **Smoke tests.** Verified clean install and correct invocation from each entry point on all target platforms.
- **Release documentation.** Release notes, blog post, and announcement.

### Completion criteria

- All release artifacts pass smoke tests on Linux, macOS, and Windows.
- Documentation is complete and live.
- Coverage thresholds met: ≥ 90% line coverage on core crate, ≥ 95% branch coverage on rule implementations, every rule has positive and negative tests.
- Performance targets met on the reference benchmark corpus.

---

## Cross-Phase Constraints

These constraints apply to every phase and are not tied to any single boundary.

### Performance

- Single 200-property nested schema, single profile: < 1 ms.
- Project of 500 schemas, 3 profiles, cold start: < 500 ms.
- Incremental run after single-file edit: < 5 ms.
- Monorepo of 5000 schemas, CI cold start: < 5 s.

### Quality

- Every rule has at least one positive test (rule fires) and one negative test (rule does not fire on adjacent valid schemas).
- Snapshot testing for all diagnostic output formats.
- Property tests for normalizer round-trips.

### Versioning

- Engine and profiles version independently under SemVer.
- Profile updates are explicit user actions; CI does not silently pull newer profiles.
- Old profile revisions remain available; users opt in to new revisions.
