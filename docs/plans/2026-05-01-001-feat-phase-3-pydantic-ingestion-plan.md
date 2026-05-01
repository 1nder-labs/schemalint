---
title: "feat: Add Pydantic ingestion with source-span attribution"
type: feat
status: active
date: 2026-05-01
origin: docs/brainstorms/phase3-requirements.md
---

# feat: Add Pydantic ingestion with source-span attribution

## Summary

Extend schemalint with a `check-python` subcommand that discovers Pydantic models via a Python subprocess, feeds discovered schemas through the existing linting pipeline, and attributes diagnostics to Python source lines. Replaces the `Diagnostic.source` placeholder with a real `SourceSpan` type, adds a subprocess management module and a co-located Python helper package, and extracts the shared check pipeline before adding the third handler.

---

## Problem Frame

The two-step workflow (manual `model_json_schema()` → save → `schemalint check`) adds friction and makes diagnostics anonymous — file references point at ephemeral JSON files, not the `.py` source the developer edits. Phase 3 collapses this by giving schemalint native awareness of Pydantic projects. (Full context in origin: `docs/brainstorms/phase3-requirements.md`.)

---

## Requirements

**Origin actors:** A1 (Python developer), A2 (Rust CLI), A3 (Python helper)
**Origin flows:** F1 (batch check), F2 (pyproject.toml-driven check)
**Origin acceptance examples:** AE1 (end-to-end check-python), AE2 (field-level source spans), AE3 (Pydantic v1 warning), AE4 (helper-not-installed error), AE5 (CLI-overrides-config)

**Python helper package**
- R1. `schemalint-pydantic` is a pip-installable Python package with minimal dependencies.
- R2. It runs as a JSON-RPC 2.0 server over stdin/stdout with `discover` and `shutdown` methods.
- R3. The `discover` method discovers BaseModel subclasses and returns model name, module path, schema JSON, and a per-field source map.
- R4. Standard field declarations resolve to the correct file and line; complex cases fall back gracefully.
- R5. Pydantic v2 is fully supported; Pydantic v1 models are detected with a warning.

**Rust subprocess management**
- R6. The Rust CLI manages a single Python subprocess lifecycle (spawn, communicate, shutdown) with timeout and error guards.

**Pipeline integration**
- R7. The existing normalize → check → emit pipeline is reused unchanged for discovered schemas.
- R11. Extract a shared `process_schemas` function before adding the third orchestration handler to prevent tripling the known duplication between `run_check` and `handle_check`.

**Diagnostics and source attribution**
- R8. Source spans from Python discovery land on diagnostics. Human output shows `file.py:line` (column resolution is deferred; Python-discovered models carry `col: None`). SARIF, GHA, and JUnit formats include source locations when available.

**CLI and configuration**
- R9. A new `check-python` subcommand accepts `--package`, `--config`, `--profile`, and all existing `--format` options.
- R10. `pyproject.toml` `[tool.schemalint]` parses profiles and packages (functional). `exclude` and `severity` keys are parsed into the config struct but their matching/override engines are deferred to follow-up work. CLI flags override config.

---

## Scope Boundaries

- Process pool for parallel Python discovery — single subprocess per invocation.
- Full Pydantic v1 support — v1 is best-effort with a warning.
- Zod / TypeScript ingestion.
- IDE or LSP integration.
- Watch mode with incremental re-discovery.
- Auto-fix or schema rewriting.
- Package distribution to PyPI, npm, crates.io.
- Request-level Anthropic budget validation.
- Adding source spans to IR nodes — the brainstorm decided on separate JSON Pointer lookup.

### Deferred to Follow-Up Work

- Extracting a shared emitter trait — all five emitters share a common function signature pattern. A trait extraction would reduce boilerplate but is not required for Phase 3.
- Response pagination for large discovery payloads — the 10 MB limit inherited from the existing server mode is a known Phase 3 limitation.
- `[tool.schemalint.severity]` per-rule override implementation — the config schema supports it, but the Rust-side override engine is deferred.
- `exclude` glob pattern matching for model filtering — the config schema supports it, but the matching engine is deferred to a follow-up.

---

## Context & Research

### Relevant Code and Patterns

- **CLI dispatch:** `crates/schemalint/src/cli/mod.rs:24-35` — pattern match on `Commands::Check` / `Commands::Server`. New `CheckPython` variant follows the same derive-based clap pattern (`cli/args.rs:23-42`).
- **Check pipeline:** `cli/mod.rs:83-292` — `run_check()` does load profiles → discover files → `into_par_iter()` per file → normalize → check → aggregate → emit. The per-file closure at lines 155-193 is the target for extraction.
- **Server handler:** `cli/server.rs:143-341` — `handle_check()` duplicates ~30 lines of the pipeline. The extraction in U1 eliminates this.
- **Diagnostic struct:** `rules/registry.rs:12-24` — `source: Option<()>` with an explicit TODO for Phase 3.
- **JSON emitter SourceSpan:** `cli/emit_json.rs:46-52` — private `SourceSpan { file, line, col }` ready for promotion.
- **Server test subprocess pattern:** `tests/server_tests.rs:4-43` — spawns `schemalint server` with `Stdio::piped()`, writes line-delimited JSON to stdin, reads from stdout. Directly applicable to Python helper testing.
- **Integration test pattern:** `tests/integration_tests.rs:30-56` — `Command::cargo_bin()`, `tempfile::tempdir()`, `assert_cmd`, `predicates`.
- **Cache:** `cache.rs:17-175` — `DiskCache` with `NormalizedSchema { arena, root_id, defs, dialect }`. Source maps are per-model metadata held outside the cache.
- **Normalizer JSON Pointers:** `normalize/traverse.rs:46-215` — pointers built from parent prefix (e.g., `format!("{}/properties/{}", ptr, key)`). Stable through `$ref` resolution (no inlining).

### Institutional Learnings

From `docs/solutions/best-practices/schemalint-phase2-learnings.md`:
- **Orchestration duplication:** `run_check` and `handle_check` duplicate ~30 lines. Adding `run_check_python` without extraction would triple it. Extract first (see U1).
- **Test field name matching:** Corpus test assertions must match emitter JSON keys, not Rust struct field names. When updating emitters (U6), verify corpus test keys align.
- **Content escaping in delimiter formats:** GHA and JUnit emitters must escape new source span values containing `\n`, `:`, `<`, `>`, `&`.
- **Mutex scope minimization:** If subprocess state is shared across rayon threads, extract data and drop the guard before expensive work.
- **I/O error logging:** Never discard subprocess I/O errors with `let _ =`. Every `read`/`write`/`spawn` must be handled or logged.
- **Three rule registration patterns:** No new rules needed for Phase 3 — existing rules operate on normalized schemas regardless of source.

### External References

No external research was used — the codebase has strong local patterns for CLI extension, subprocess management (via server_tests.rs), and diagnostic emission. The Python helper follows standard `pydantic` and `inspect` module patterns.

---

## Key Technical Decisions

| Decision | Rationale |
|---|---|
| Extract shared pipeline before adding third handler | Phase 2 learnings flag the `run_check`/`handle_check` duplication. Adding `run_check_python` without extraction would triple the debt. Extraction is scoped to a single refactor unit (U1) with no behavioral change. |
| SourceSpan as shared type in `rules/registry.rs` | The JSON emitter already defines a private `SourceSpan`. Promoting it to the rules module makes it canonical for all emitters and diagnostics. Follows the existing pattern where `Diagnostic` and `DiagnosticSeverity` live in `registry.rs`. |
| Source map as separate per-model metadata, not cached | The normalizer cache stores `NormalizedSchema { arena, root_id, defs, dialect }`. Adding source maps to `NormalizedSchema` would couple discovery metadata to normalization. Instead, source maps are held per-model in the check orchestrator and looked up after rule checking. Cache hit/correctness is unaffected — two models producing identical schemas each get their own source map. |
| Subprocess spawn: `python3` → `python` fallback + `--python-path` flag | Industry standard (matches ruff, mypy, black). The `--python-path` flag gives explicit override for CI and venv setups. |
| Synthetic IR nodes omit source spans | When a diagnostic fires on a node created during normalization (type-array desugaring, ref resolution), the node's JSON Pointer has no entry in the source map. Inheriting an ancestor's span could misattribute the diagnostic. Omitting the span is honest and conservative. |
| No JSON-RPC handshake; `discover` is the first request | The `shutdown` method is the only lifecycle method beyond `discover`. A handshake adds protocol complexity with no benefit — the subprocess starts ready. Any startup error (import failure, wrong Python version) surfaces on the first `discover` call. |
| `python/schemalint-pydantic/` co-located at repo root | Not a Cargo crate; lives outside the Rust workspace. Co-location simplifies development (single repo, cross-language integration tests) while remaining independently pip-installable. Mirrors how `ruff` ships its Python extension. |
| `PathBuf` as diagnostic grouping key retained; emitters use `diagnostic.source` for file attribution | Python module paths convert to `PathBuf` for grouping. When a diagnostic carries a source span, emitters use `source.file` for file attribution. When absent (synthetic nodes, raw JSON files), emitters fall back to the grouping key's path. No emitter signature changes needed — only internal logic. |

---

## Output Structure

New directories created by this plan:

```
python/
  schemalint-pydantic/
    pyproject.toml
    src/
      schemalint_pydantic/
        __init__.py
        __main__.py
        server.py          # JSON-RPC 2.0 stdin/stdout server loop
        discover.py        # BaseModel discovery + source span resolution

crates/schemalint/src/
  python/
    mod.rs                # PythonHelper: spawn, send_request, read_response, shutdown
  cli/
    pyproject.rs          # parse_pyproject_config(): reads [tool.schemalint] from TOML
```

Files modified:

```
crates/schemalint/src/
  cli/mod.rs              # +Commands::CheckPython, +run_check_python, -duplicated pipeline
  cli/args.rs             # +CheckPythonArgs struct
  cli/emit_human.rs       # render diagnostic.source file/line/col
  cli/emit_json.rs        # use canonical SourceSpan, populate real values
  cli/emit_sarif.rs       # +region.startLine/startColumn in physicalLocation
  cli/emit_gha.rs         # +line=X,col=Y in workflow command
  cli/emit_junit.rs       # +file/line on testcase
  rules/registry.rs       # +SourceSpan, Diagnostic.source: Option<()> → Option<SourceSpan>
  rules/class_a.rs        # source: None (unchanged, type updated)
  rules/class_b.rs        # source: None (unchanged, type updated)
  rules/semantic.rs       # source: None (unchanged, type updated)
  lib.rs                  # +pub mod python

crates/schemalint/tests/
  python_tests.rs         # new: integration tests for check-python
  corpus/                 # +Python-specific regression schemas
```

---

## High-Level Technical Design

> *This illustrates the intended approach and is directional guidance for review, not implementation specification. The implementing agent should treat it as context, not code to reproduce.*

### Rust ↔ Python communication sequence

```mermaid
sequenceDiagram
    participant CLI as Rust CLI (run_check_python)
    participant PH as PythonHelper (std::process::Child)
    participant PY as schemalint-pydantic (Python)
    participant User as User's Pydantic Models

    CLI->>PH: spawn("python3", ["-m", "schemalint_pydantic"])
    PH->>PY: import schemalint_pydantic.server; main()
    PY-->>PH: ready (waiting on stdin)

    CLI->>PH: write stdin: {"jsonrpc":"2.0","method":"discover","params":{"package":"myapp.models"},"id":1}
    PH->>PY: read stdin line
    PY->>User: import myapp.models
    User-->>PY: module loaded
    PY->>PY: walk for BaseModel subclasses
    PY->>User: model.model_json_schema()
    User-->>PY: JSON Schema dict
    PY->>PY: inspect.getsourcefile / getsourcelines for each field
    PY-->>PH: write stdout: {"jsonrpc":"2.0","result":{...},"id":1}
    PH-->>CLI: read stdout line, parse JSON

    CLI->>CLI: normalize each discovered schema
    CLI->>CLI: run rules against normalized arenas
    CLI->>CLI: look up diagnostic JSON Pointers in source map
    CLI->>CLI: attach SourceSpan to each Diagnostic
    CLI->>CLI: emit diagnostics in requested format

    CLI->>PH: write stdin: {"jsonrpc":"2.0","method":"shutdown","id":2}
    PY-->>PH: write stdout: {"jsonrpc":"2.0","result":"ok","id":2}
    PH->>PH: process exits
```

### Source span data flow

```
Python helper                    Rust CLI                         Emitters
─────────────                    ────────                         ────────
discover.py                      mod.rs                           emit_*.rs
    │                                │                                │
    ├─ model_json_schema() ──►  schema JSON  ──► normalize ──►  arena
    │                                                               │
    ├─ inspect sources ──►  source_map: Map<Pointer, Span>          │
    │                           │                                    │
    │                           │   rules check_all(arena) ──►  Vec<Diagnostic>
    │                           │   for each diag:                   │
    │                           │     span = source_map[diag.pointer] │
    │                           │     diag.source = Some(span) ──────┤
    │                           │                                    │
    │                           │                          human:  file.py:42:8
    │                           │                          json:   {"source":{"file":"...","line":42,"col":8}}
    │                           │                          sarif:  region.startLine=42, region.startColumn=8
    │                           │                          gha:    ::error file=file.py,line=42,col=8::msg
    │                           │                          junit:  testcase file="file.py" line="42"
```

Source spans flow **alongside** the schema (not through the IR). The normalizer and rules engine operate on the arena as before. The source map lookup happens after diagnostics are produced, immediately before emission. Synthetic nodes created during normalization lack source map entries — their diagnostics carry `source: None`.

---

## Implementation Units

### U1. Extract shared check pipeline

**Goal:** Refactor the duplicated `load → normalize → check → report` logic from `run_check` and `handle_check` into a single shared function, preventing tripling when `run_check_python` is added.

**Requirements:** R11

**Dependencies:** None

**Files:**
- Modify: `crates/schemalint/src/cli/mod.rs`
- Modify: `crates/schemalint/src/cli/server.rs`

**Approach:**
- Extract a function `process_schemas(schemas: Vec<(SourceKey, serde_json::Value)>, ...) -> Vec<(SourceKey, Vec<Diagnostic>)>` that takes pre-parsed schema JSON values and runs them through the normalize → check pipeline.
- `SourceKey` is a type alias for `PathBuf` in this unit; it carries the schema's identity for diagnostic grouping. The concrete type may evolve in U5 but the function's interface abstracts over it.
- `run_check` and `handle_check` become thin wrappers: parse files/params → call `process_schemas` → emit. Both `cli/mod.rs` and `cli/server.rs` are modified to call the extracted function.
- No behavioral change. All existing tests must pass identically.

**Patterns to follow:**
- The per-file closure at `cli/mod.rs:155-193` is the canonical normalization + checking sequence.
- The `server.rs:252-270` cache pattern shows how to integrate `DiskCache` into the shared function.
- Mutex scope minimization (Phase 2 learning D): extract data from `cache.lock()` before entering `check_all`.

**Test scenarios:**
- **Happy path:** `cargo test --workspace` — all existing tests pass with identical output.
- **Edge case:** Extract the pipeline without changing any function signatures visible to tests.
- **Error path:** Error propagation (normalize error, empty schema) preserved exactly.

**Verification:**
- Existing Phase 1 + Phase 2 corpus passes deterministically.
- Server mode integration tests (`server_tests.rs`) pass identically.
- No snapshot drift in `snapshot_tests.rs`.

---

### U2. Replace Diagnostic.source placeholder with SourceSpan

**Goal:** Implement the `SourceSpan` type and replace `Diagnostic.source: Option<()>` with `Option<SourceSpan>`. This is a type-level change — all rule sites still pass `source: None`.

**Requirements:** R8

**Dependencies:** None

**Files:**
- Modify: `crates/schemalint/src/rules/registry.rs` — add `SourceSpan` struct, update `Diagnostic.source` field
- Modify: `crates/schemalint/src/rules/class_a.rs` — update `source: None` sites (~2)
- Modify: `crates/schemalint/src/rules/class_b.rs` — update `source: None` sites (~10)
- Modify: `crates/schemalint/src/rules/semantic.rs` — update `source: None` sites (~3)
- Modify: `crates/schemalint/src/cli/emit_json.rs` — remove private `SourceSpan`, import canonical one from `rules`

**Approach:**
- Define `SourceSpan` in `rules/registry.rs` with fields: `file: String`, `line: Option<u32>`, `col: Option<u32>`.
- Change `Diagnostic.source` from `Option<()>` to `Option<SourceSpan>`.
- Update all `source: None` sites — the value literal doesn't change, only the type annotation.
- Remove the private `SourceSpan` from `emit_json.rs` and replace with `use crate::rules::SourceSpan`.
- The JSON emitter's existing `SourceSpan` consumers (`line: None, col: None`) are updated to reference the canonical type path.

**Patterns to follow:**
- `DiagnosticSeverity` is defined in `registry.rs` alongside `Diagnostic` — `SourceSpan` follows the same co-location pattern.
- The private JSON emitter `SourceSpan` at `emit_json.rs:46-52` is the template.

**Test scenarios:**
- **Happy path:** `cargo build --workspace` compiles cleanly. `cargo test --workspace` passes with no snapshot drift.
- **Edge case:** `SourceSpan` derives or implements `Serialize`, `Deserialize`, `Clone`, `Debug` — all needed for JSON output and test assertions.

**Verification:**
- Compilation succeeds (type change affects ~15 `source: None` sites across the 5 modified files).
- All existing tests pass identically — this is a pure type replacement, no behavior changes.
- CI clippy passes with `-D warnings`.

---

### U3. Python subprocess management module

**Goal:** Create a `src/python/` module with a `PythonHelper` struct that spawns a Python subprocess, sends JSON-RPC requests, reads JSON-RPC responses, and shuts down cleanly.

**Requirements:** R6

**Dependencies:** U2 (DiscoverResponse uses SourceSpan type)

**Files:**
- Create: `crates/schemalint/src/python/mod.rs`
- Modify: `crates/schemalint/src/lib.rs` — add `pub mod python;`

**Approach:**
- `PythonHelper::spawn(python_path: Option<&str>) -> Result<Self, PythonError>` — resolves `python3` then `python` fallback, or uses explicit path. Spawns `python -m schemalint_pydantic` with `Stdio::piped()` for all three channels (stdin, stdout, stderr). A dedicated reader thread drains stderr continuously to prevent pipe-buffer deadlock; stderr contents are logged at debug level and included in error messages when the helper fails.
- `PythonHelper::discover(&mut self, package: &str) -> Result<DiscoverResponse, PythonError>` — writes a JSON-RPC `discover` request to stdin, reads one line-delimited JSON response from stdout. Applies a configurable timeout (default: 60s). Timeout is implemented via `std::sync::mpsc::recv_timeout` on a reader thread, since `std::process::ChildStdout` has no native timeout on sync I/O.
- `PythonHelper::shutdown(&mut self)` — sends `shutdown` request, waits for child to exit with a brief timeout, then kills if unresponsive.
- `Drop` implementation: if the child process is still running, logs a warning and attempts shutdown.
- `PythonError` enum: `SpawnFailed`, `NotInstalled`, `RequestFailed`, `Timeout`, `InvalidResponse`, `DiscoverFailed(String)`.
- `DiscoverResponse` struct: carries `Vec<DiscoveredModel>` where each model has `name: String`, `module_path: String`, `schema: serde_json::Value`, `source_map: HashMap<String, SourceSpan>`.
- Thread-safe: `PythonHelper` is not `Send` (holds a `Child` with piped I/O). The orchestrator (U5) uses it sequentially before the rayon parallel phase.

**Patterns to follow:**
- `server_tests.rs:36-43` — `Stdio::piped()` spawn pattern.
- `server_tests.rs:17-28` — line-delimited JSON write/read on stdin/stdout.
- `server.rs:23-141` — JSON-RPC 2.0 request structure (`{ jsonrpc, method, params, id }`).
- Phase 2 learnings C and D: log all I/O errors, never `let _ =`.

**Test scenarios:**
- **Happy path:** Unit test with a mock Python script that echoes a valid `discover` response to stdout.
- **Edge case:** Timeout — helper hangs on import. Test with a script that sleeps, verify `PythonError::Timeout` is returned.
- **Error path:** Python not installed — spawn fails with `PythonError::NotInstalled`, message includes the attempted command.
- **Error path:** Helper crashes mid-response — partial/broken JSON read from stdout produces `PythonError::InvalidResponse`.
- **Error path:** Helper returns a JSON-RPC error response — `PythonError::DiscoverFailed` carries the error message.
- **Integration:** Spawn real `python3 -c "import json, sys; ..."` as a lightweight helper for IO protocol validation.

**Verification:**
- `PythonHelper::spawn()` produces a working subprocess.
- `discover()` round-trips a JSON-RPC request/response correctly.
- `shutdown()` terminates the child process cleanly.
- Timeout and crash scenarios produce clear, actionable errors.

---

### U4. Python helper package (schemalint-pydantic)

**Goal:** Create the `schemalint-pydantic` Python package — a JSON-RPC 2.0 stdin/stdout server that discovers Pydantic BaseModel subclasses in a target package, extracts their JSON Schemas, and resolves per-field source locations.

**Requirements:** R1, R2, R3, R4, R5

**Dependencies:** None (independent Python code, tested separately from Rust)

**Files:**
- Create: `python/schemalint-pydantic/pyproject.toml`
- Create: `python/schemalint-pydantic/src/schemalint_pydantic/__init__.py`
- Create: `python/schemalint-pydantic/src/schemalint_pydantic/__main__.py`
- Create: `python/schemalint-pydantic/src/schemalint_pydantic/server.py`
- Create: `python/schemalint-pydantic/src/schemalint_pydantic/discover.py`
- Create: `python/schemalint-pydantic/tests/__init__.py`
- Create: `python/schemalint-pydantic/tests/test_discover.py`
- Create: `python/schemalint-pydantic/tests/test_server.py`

**Approach:**

*`pyproject.toml`:* Package name `schemalint-pydantic`, Python >= 3.9, no install dependencies (stdlib only — pydantic is a peer dependency satisfied by the user's project). Entry point: `schemalint-pydantic = "schemalint_pydantic.__main__:main"`.

*`server.py`:* Reads stdin line-by-line. Each line is a JSON-RPC 2.0 request. Dispatches to method handlers (`discover`, `shutdown`). Writes one line of JSON response to stdout. Follows the same protocol as the Rust `server.rs` — id matching, error codes, no batching.

*`discover.py`:* `discover_models(package: str) -> dict` — redirects `sys.stdout` to `sys.stderr` during import to prevent user-code `print()` calls from corrupting the JSON-RPC protocol channel, then imports the package via `importlib.import_module()`. Walks (recursively) for `pydantic.BaseModel` subclasses using `inspect.getmembers()` with `isinstance` check. For each model:
1. Call `model.model_json_schema()` (v2). If `AttributeError`, detect v1 via `model.schema()` and emit a warning.
2. For each field in `model.model_fields`, resolve source location via `inspect.getsourcefile()` on the model class and `inspect.getsourcelines()` for line mapping.
3. Build a source map: `{json_pointer: {file, line}}` where JSON Pointer is `/properties/{field_name}`.
4. Return model entry: `{name, module_path, schema, source_map}`.

*V1 detection:* Check for presence of `model.model_json_schema` (v2) vs `model.schema` only (v1). V1 models produce a warning diagnostic (returned as part of the discovery response) and use `model.schema()` for extraction. The Rust side handles v1 schemas through the existing normalizer (which already supports both `$defs` and `definitions`).

*Source span resolution strategy:*
- Primary: `inspect.getsourcelines(model_class)` to get line numbers, then match field name against source lines to find declaration line.
- Fallback for complex cases (e.g., `Annotated[T, Field(...)]`, generics, inherited fields): use the model class declaration line.
- Source map keys are JSON Pointers matching the schema structure produced by `model_json_schema()`. For nested models, pointers follow the standard `/properties/field_name` or `/properties/field_name/items` pattern.

**Test scenarios (Python-side):**
- **Happy path:** Discover a test package with one Pydantic v2 model; verify returned schema and source map.
- **Happy path:** Discover a package with multiple models across submodules; verify all models found.
- **Happy path:** Source map entries have correct file paths and line numbers matching the test fixture's source.
- **Happy path:** JSON-RPC server round-trips a `discover` request and response via stdin/stdout.
- **Edge case:** Model with no fields — returns empty source map, valid schema.
- **Edge case:** `Annotated` field types — falls back to model class line.
- **Edge case:** Inherited fields from a parent BaseModel — attributed to the parent's source location.
- **Error path:** Invalid package name — returns JSON-RPC error response.
- **Error path:** Pydantic v1 model — schema extracted, warning diagnostic included in response.
- **Error path:** Non-Pydantic class in the package — skipped silently.

**Verification:**
- `pip install -e python/schemalint-pydantic && python -m schemalint_pydantic` starts the server.
- Python test suite passes (`python -m pytest python/schemalint-pydantic/tests/`).
- Manual verification: server responds to a hand-crafted `discover` request piped to stdin.

---

### U5. check-python CLI subcommand and pyproject.toml parsing

**Goal:** Add the `check-python` subcommand with argument parsing, Python subprocess orchestration, pyproject.toml config loading, and integration with the shared check pipeline from U1.

**Requirements:** R6, R7, R9, R10; flows F1, F2; AE1, AE2, AE3, AE4, AE5

**Dependencies:** U1 (shared pipeline), U2 (SourceSpan), U3 (PythonHelper), U4 (Python package)

**Files:**
- Modify: `crates/schemalint/src/cli/args.rs` — add `CheckPythonArgs`, `Commands::CheckPython` variant
- Modify: `crates/schemalint/src/cli/mod.rs` — add `run_check_python()`, dispatch to `Commands::CheckPython`
- Create: `crates/schemalint/src/cli/pyproject.rs` — `PyProjectConfig` struct, `load_pyproject_config()` function

**Approach:**

*`CheckPythonArgs` (args.rs):*
- `--package` / `-p`: `Vec<String>`, optional, repeatable. Target package names like `myapp.models`.
- `--profile`: `Vec<String>`, repeatable. Profile IDs or paths. Merges with pyproject.toml config.
- `--config`: `Option<PathBuf>`. Explicit pyproject.toml path; defaults to `./pyproject.toml`.
- `--python-path`: `Option<String>`. Explicit Python executable path.
- `--format`: `Option<OutputFormat>`. Same as `CheckArgs`.
- `--output`: `Option<PathBuf>`. Same as `CheckArgs`.

*`run_check_python()` (mod.rs):*
1. Load pyproject.toml config if available (see pyproject.rs below).
2. Merge CLI flags on top of config: CLI `--profile` replaces config `profiles`; CLI `--package` appends to config `packages`; CLI `--format` overrides config.
3. If no packages are configured (neither CLI nor config), emit a user-friendly error directing to `--package` or `[tool.schemalint]`.
4. Load profiles (same `resolve_profile()` logic as `run_check`).
5. Spawn `PythonHelper` (U3).
6. For each package, send `discover` request. Collect all `DiscoveredModel` results. Per-model import failures are surfaced as stderr messages but don't abort. Pydantic v1 warnings are returned as diagnostic entries in the discovery response and flow through the normal diagnostic pipeline to emitter output.
7. Build a `Vec<(PathBuf, serde_json::Value, HashMap<String, SourceSpan>)>` — model module path → schema JSON → source map.
8. Feed schemas through the shared `process_schemas()` pipeline from U1. The pipeline returns `Vec<(PathBuf, Vec<Diagnostic>)>`.
9. For each diagnostic, look up its `pointer` in the model's source map via `source_map.get(&diag.pointer)`. If found, set `diag.source = Some(span)`. If not found (synthetic node), leave as `None`.
10. Aggregate and emit using the existing emitter functions (U6 updated them for source spans).

*`PyProjectConfig` (pyproject.rs):*
```toml
[tool.schemalint]
profiles = ["openai.so.2026-04-30"]   # list of profile IDs
packages = ["myapp.models"]             # list of package names
exclude = ["myapp.models.internal.*"]  # glob patterns (schema only; matching deferred)

[tool.schemalint.severity]
"OAI-K-pattern" = "warn"               # per-rule severity overrides (schema only; engine deferred)
```
- Parse using the existing `toml` crate (already a dependency).
- `PyProjectConfig` has `profiles: Vec<String>`, `packages: Vec<String>`, `exclude: Vec<String>`, `severity: HashMap<String, String>`.
- `load_pyproject_config(path: &Path) -> Result<Option<PyProjectConfig>>` — returns `None` if no `[tool.schemalint]` section exists. Returns error for invalid TOML but not for missing file.
- Config merge: CLI `--profile` replaces (not appends to) pyproject.toml `profiles`. CLI `--package` appends to pyproject.toml `packages`.

**Patterns to follow:**
- `cli/args.rs:23-39` — `CheckArgs` derive pattern for clap.
- `cli/mod.rs:83-292` — `run_check()` orchestrator structure.
- `profile/parser.rs:30-40` — TOML parsing via `toml` crate.
- Phase 2 learnings D and G: mutex minimization, no new orchestration duplication.

**Test scenarios:**
- **Happy path — Covers F1, AE1:** Run `check-python` against a test fixture with real Pydantic models; verify diagnostics with correct source spans in human output.
- **Happy path — Covers F2:** `check-python` with no args reads pyproject.toml and discovers configured packages.
- **Happy path — Covers AE5:** CLI `--profile` overrides pyproject.toml profiles.
- **Happy path — Covers AE2:** Diagnostic for a field keyword points at the field's declaration line, not the model class line.
- **Edge case — Covers AE4:** Python helper not installed → clear error message, exit code 1.
- **Edge case:** Empty package (no BaseModel subclasses) → 0 schemas checked, success exit (no error).
- **Edge case:** Missing pyproject.toml + no `--package` → actionable error message.
- **Error path — Covers AE3:** Pydantic v1 model → warning diagnostic included in output.
- **Error path:** One package fails to import, another succeeds → per-package errors on stderr, successful package diagnostics on stdout.
- **Error path:** Subprocess timeout → clear error, exit code 1.
- **Error path:** Invalid pyproject.toml syntax → parse error with line reference.

**Verification:**
- `schemalint check-python --help` shows subcommand usage.
- Integration test end-to-end: `assert_cmd` runs `check-python` against a tempdir with Python fixtures and pyproject.toml.
- All existing `check` and `server` functionality is unaffected.

---

### U6. Update all emitters for source spans

**Goal:** Update human, JSON, SARIF, GHA, and JUnit emitters to render `diagnostic.source` when present, falling back to the schema file path when absent.

**Requirements:** R8; AE1, AE2

**Dependencies:** U2 (SourceSpan type available)

**Files:**
- Modify: `crates/schemalint/src/cli/emit_human.rs`
- Modify: `crates/schemalint/src/cli/emit_json.rs`
- Modify: `crates/schemalint/src/cli/emit_sarif.rs`
- Modify: `crates/schemalint/src/cli/emit_gha.rs`
- Modify: `crates/schemalint/src/cli/emit_junit.rs`

**Approach:**
- Each emitter already receives `&[(PathBuf, Vec<Diagnostic>)]`. The `PathBuf` is the schema file path (for JSON files) or the module path (for Python models).
- Each emitter's per-diagnostic renderer checks `diagnostic.source`:
  - `Some(span)` → use `span.file` as the file, `span.line`/`span.col` for location.
  - `None` → use `path.display()` as the file, no line/col (current behavior).
- **Human:** When source is present, format the location line as `--> {span.file}:{line}:{col}` instead of `--> {path}`.
- **JSON:** Replace `SourceSpan { file, line: None, col: None }` with the actual `diagnostic.source` values. The struct was promoted in U2 and the placeholder nulls become real values.
- **SARIF:** Add `region: { "startLine": line, "startColumn": col }` inside `physicalLocation` when source is present.
- **GHA:** Append `,line=X,col=Y` to the `::error file=...::` workflow command when source is present. Apply `encode_gha_value()` escaping to the file path.
- **JUnit:** Add `file="span.file"` and `line="span.line"` attributes to `<testcase>` elements when source is present. XML-escape the file path (`<`, `>`, `&`).

**Patterns to follow:**
- Phase 2 learning B: escape content values in delimiter-based formats. The GHA emitter already has `encode_gha_value()` — extend to file paths from source spans. JUnit XML needs `encode_xml_value()` for file paths containing special characters.
- Phase 2 learning A: corpus test assertions must match emitter JSON keys. The JSON emitter's `"source"` field name must align with corpus test comparisons.

**Test scenarios:**
- **Happy path:** Human output shows `--> file.py:42:8` when source is present.
- **Happy path:** Human output shows `--> schema.json` when source is absent (backward compatible).
- **Happy path:** JSON output populates `source.file`, `source.line`, `source.col` with real values.
- **Happy path:** JSON output has `"source": null` when source is absent (backward compatible).
- **Happy path:** SARIF output includes `region.startLine` and `region.startColumn`.
- **Happy path:** GHA output includes `,line=42,col=8` in workflow commands.
- **Edge case:** Source span with `line: Some(42), col: None` — emitters degrade gracefully (show line without col).
- **Edge case:** Source span file path contains GHA delimiter characters (`:`, `%`, `\n`) — properly escaped.

**Verification:**
- Snapshot tests (`snapshot_tests.rs`) for all five formats with source spans present.
- Snapshot tests for all five formats with no source spans (existing snapshots unchanged or trivially updated).
- Manual visual check: human output with Python source spans matches rustc diagnostic style.

---

### U7. Integration tests and regression corpus

**Goal:** Add integration tests for the `check-python` flow and Python-specific regression corpus entries that validate source span attribution end-to-end.

**Requirements:** R4, R8; success criteria "10+ known issues from the regression corpus with correct Python source file and line attribution"; AE1, AE2, AE3, AE4, AE5

**Dependencies:** U5 (check-python functional), U6 (emitters render source spans)

**Files:**
- Create: `crates/schemalint/tests/python_tests.rs` — integration tests for check-python CLI
- Create: `crates/schemalint/tests/corpus/python/` — Python model fixtures
- Create: `crates/schemalint/tests/corpus/python/*.expected` — expected diagnostic sets for Python fixtures
- Modify: `crates/schemalint/tests/snapshot_tests.rs` — add snapshot tests for check-python output formats

**Approach:**

*`python_tests.rs`:*
- Follow the `integration_tests.rs` pattern (`Command::cargo_bin("schemalint")`, `tempfile::tempdir()`, `assert_cmd`, `predicates`).
- Each test creates a tempdir with a minimal Python project structure: `pyproject.toml` with `[tool.schemalint]`, and a `.py` file with a Pydantic model.
- The Python model fixtures are self-contained (they import pydantic, define a model class, and are executable). Fixtures target specific keyword violations known to fire from the profile.
- Install the `schemalint-pydantic` package in the test venv before running (or use `PYTHONPATH` to point at `python/schemalint-pydantic/src/`).
- Test categories:
  - Single model with a forbidden keyword → human output shows `error` with Python file:line.
  - Multiple models → all discovered.
  - Pydantic v1 model → warning diagnostic.
  - Helper not installed → clear error.
  - pyproject.toml config vs CLI flag merging.
  - Source span correctness: verify the line number in the output matches the fixture's source.

*`corpus/python/`:*
- 10+ Python fixtures, each a minimal `.py` file with one Pydantic model exercising a distinct profile violation.
- Each fixture has a `.expected` JSON file following the existing corpus format (`corpus_tests.rs:13-28`).
- The corpus test framework is extended to support Python fixtures: the `check-python` subcommand is invoked instead of `check`, models are discovered from the package rather than loaded from a `.json` file.
- Fixture coverage:
  - `forbid_allof.py` — model with `allOf` (OpenAI forbid)
  - `forbid_minimum.py` — model with `minimum` constraint (Anthropic forbid)
  - `warn_unique_items.py` — model with `Set[T]` (emits uniqueItems)
  - `additional_props_missing.py` — model missing `model_config = ConfigDict(extra="forbid")`
  - `nested_model.py` — nested model with violations at both levels
  - `enum_oversize.py` — Literal with too many values
  - `external_ref.py` — model referencing an external $ref
  - `deep_nesting.py` — deeply nested models exceeding max_object_depth
  - `v1_model.py` — Pydantic v1 model with `class Config`
  - `empty_model.py` — model with no fields (edge case)

**Patterns to follow:**
- `integration_tests.rs:38-56` — `tempdir()`, `fs::write()`, `cmd().arg().arg().assert()`.
- `server_tests.rs:4-15` — binary discovery (`Command::cargo_bin()` or sibling binary path).
- `corpus_tests.rs:13-28` — `diagnostics_match()` comparison function.

**Test scenarios:**
- **Covers AE1:** End-to-end `check-python` with a forbidden keyword → human output includes error code + Python file:line.
- **Covers AE2:** Verify source span line number matches the actual Python fixture source.
- **Covers AE3:** Pydantic v1 fixture → warning diagnostic about limited support.
- **Covers AE4:** Helper package not installed → single clear error, not a crash or traceback.
- **Covers AE5:** pyproject.toml profiles overridden by CLI `--profile`.
- **Edge case:** Nested models — diagnostics for different nesting levels carry correct source spans for each level.
- **Edge case:** Model with no violations → success, "0 issues found".
- **Regression:** All existing Phase 1 + Phase 2 corpus tests continue to pass via `check` (no regression).

**Verification:**
- `cargo test --test python_tests` passes.
- `cargo test --test corpus_tests` passes including new Python fixtures.
- `cargo test --test snapshot_tests` passes with new `check-python` output snapshots.
- Manual verification: run `schemalint check-python` against a real Pydantic project and inspect output.

---

## System-Wide Impact

- **Interaction graph:** New `Commands::CheckPython` variant in the CLI dispatch (`cli/mod.rs`). No changes to `check` or `server` dispatch paths. The new `src/python/` module is only invoked from `run_check_python`.
- **Error propagation:** Subprocess errors (`PythonError`) are surfaced as stderr messages and reflected in the exit code (1 on any error). Per-model import errors don't abort — consistent with per-file error handling in `run_check`.
- **State lifecycle risks:** `PythonHelper` holds a `std::process::Child` which must be shut down even on panic or early return. The `Drop` implementation is the safety net.
- **API surface parity:** `check` (JSON files) and `check-python` (Pydantic models) share the same output formats and exit code semantics. The same CLI `--format` flag works for both.
- **Integration coverage:** The JSON-RPC stdin/stdout protocol is the hardest integration seam. Server test pattern (`server_tests.rs`) validates this layer. Python helper tests validate the Python side independently.
- **Unchanged invariants:** The existing `check` and `server` subcommands are untouched. The normalizer, rule engine, profile loader, and cache have zero behavioral changes. `Diagnostic.source` type changes are mechanical and backward-compatible (all sites still pass `None`).

---

## Risks & Dependencies

| Risk | Mitigation |
|------|------------|
| Pydantic v2 `model_json_schema()` output differs in keyword naming from normalized IR expectations, breaking source map lookups | U4 tests validate source map accuracy against known Pydantic output shapes. If keyword naming divergence is found, U4 applies normalization on the Python side before building the source map. |
| `inspect` module fails on namespace packages, frozen modules, or `__init__.py` re-exports | U4 implements graceful fallback: if `inspect.getsourcefile()` returns `None`, the model's source span is omitted rather than crashing. The deferred-to-planning research question is resolved at implementation time. |
| Subprocess I/O deadlocks — stdout fills up while Rust is still writing to stdin, or vice versa | U3 uses sequential write-then-read for each request (synchronous, no concurrent I/O). The `discover` request is sent, then the Rust side blocks on reading the response. No interleaved read/write that could deadlock. |
| Python helper import of user code triggers side effects (network calls, database connections) that hang or fail | U3's configurable timeout guards against hangs. Side-effect failures are user-code errors, not schemalint bugs — surfaced as per-model errors consistent with the "one model doesn't abort the run" principle. |
| Large Pydantic codebase produces a `discover` response exceeding the 10 MB JSON-RPC payload limit | Known limitation for Phase 3. Surface a clear error message if the response exceeds the limit. Response pagination is deferred to follow-up work. |

---

## Sources & References

- **Origin document:** [docs/brainstorms/phase3-requirements.md](../brainstorms/phase3-requirements.md)
- Related code: `crates/schemalint/src/cli/mod.rs`, `crates/schemalint/src/cli/args.rs`, `crates/schemalint/src/rules/registry.rs`, `crates/schemalint/src/cli/emit_json.rs`
- Related tests: `crates/schemalint/tests/server_tests.rs`, `crates/schemalint/tests/integration_tests.rs`
- Institutional learnings: [docs/solutions/best-practices/schemalint-phase2-learnings.md](../solutions/best-practices/schemalint-phase2-learnings.md)
- Phase reference: [docs/phases.md](../phases.md)
