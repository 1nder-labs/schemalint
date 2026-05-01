---
date: 2026-05-01
topic: phase-3-pydantic-ingestion
---

# Phase 3 Requirements — Pydantic Ingestion

## Summary

Add a Python-side discovery helper so `schemalint` can lint Pydantic projects directly — ingesting models, extracting schemas, and attributing diagnostics to Python source lines — without the manual two-step of generating and checking JSON files.

---

## Problem Frame

Today, a Python developer using Pydantic with OpenAI or Anthropic structured outputs must: (1) run `model_json_schema()` manually or via a script, (2) save the output to a file, (3) run `schemalint check` against that file, and (4) mentally map schema-path diagnostics (`/properties/email`) back to Python source (`models/user.py:42`). The schema file is an ephemeral artifact that drifts from the source; regenerating it is manual and easy to forget. Diagnostics carry a file reference to the JSON — not the `.py` file the developer actually edits. This friction makes the linter feel like a second-class tool for the primary authoring language.

Phase 3 removes this gap by giving `schemalint` first-class awareness of Pydantic projects. A Python discovery helper finds models, extracts their schemas, and resolves source locations. The Rust CLI manages the helper process and feeds discovered schemas through the existing linting pipeline. Diagnostics land on the Python source where the developer works.

---

## Actors

- A1. **Python developer** — Authors Pydantic models, runs `schemalint`, reads diagnostics pointing at their `.py` files.
- A2. **Rust CLI (schemalint)** — Orchestrates discovery, normalization, rule checking, and output emission.
- A3. **Python helper (schemalint-pydantic)** — A subprocess managed by the Rust CLI. Discovers Pydantic BaseModel subclasses, extracts JSON Schema, and resolves per-field source locations.

---

## Key Flows

- F1. **Batch check of a Python project**
  - **Trigger:** Developer runs `schemalint check-python --package myapp.models`
  - **Actors:** A1, A2, A3
  - **Steps:**
    1. Rust CLI spawns the Python helper subprocess
    2. CLI sends a `discover` JSON-RPC request for the target package
    3. Helper imports the package, walks for BaseModel subclasses, extracts schemas and source maps
    4. Helper returns discovered models with schemas and field-level source locations
    5. CLI feeds each discovered schema through the existing normalize → check pipeline
    6. CLI emits diagnostics with Python source spans in the requested output format
  - **Outcome:** Developer sees rustc-style diagnostics pointing at their `.py` files, with error codes, messages, and schema paths.
  - **Covered by:** R1, R2, R5, R6, R7, R8

- F2. **pyproject.toml-driven check**
  - **Trigger:** Developer runs `schemalint check-python` in a directory containing `pyproject.toml` with `[tool.schemalint]`
  - **Actors:** A1, A2, A3
  - **Steps:**
    1. CLI reads `pyproject.toml` from cwd or explicit `--config` path
    2. CLI resolves profiles, packages, exclude patterns, and severity overrides from config
    3. CLI merges CLI flags on top of config (CLI wins)
    4. Discovery and linting proceed as in F1 for each configured package
  - **Outcome:** Developer configures once, then runs with no arguments. CI reads the same config.
  - **Covered by:** R9, R10

---

## Requirements

**Python helper package (schemalint-pydantic)**
- R1. `schemalint-pydantic` is a pip-installable Python package with minimal dependencies (stdlib + pydantic, which the user's project already provides).
- R2. It runs as a JSON-RPC 2.0 server over stdin/stdout, accepting `discover` and `shutdown` methods.
- R3. The `discover` method accepts a package name (e.g., `myapp.models`), imports the package, walks for all `pydantic.BaseModel` subclasses, and returns each model's name, module path, JSON Schema (via `model_json_schema()`), and a per-field source map.

**Source span resolution**
- R4. Standard field declarations (e.g., `email: str`, `age: int = Field(...)`) resolve to the correct file and line number. Complex cases (e.g., `Annotated[T, Field(...)]` inside generics) fall back gracefully to the model class declaration line.
- R5. Pydantic v2 is fully supported. Pydantic v1 models are detected automatically and produce a warning diagnostic noting limited support, but the schema is still extracted via `model.schema()` on a best-effort basis.

**Rust-side process management**
- R6. The Rust CLI manages the lifecycle of a single Python subprocess: spawn, send JSON-RPC requests, read responses, and shutdown. The subprocess has a configurable execution timeout and the CLI surfaces a clear error when the helper is not installed or crashes.
- R7. The existing normalization, rule-checking, and output pipeline is reused unchanged for discovered schemas.

**Source span propagation**
- R8. Source spans from Python discovery survive the linting pipeline and land on diagnostics. Human output shows `file.py:line:col` (e.g., `models/user.py:42:8`). JSON output populates the `source` field with `{ "file", "line", "col" }`. SARIF, GHA, and JUnit formats include source locations where their schemas support them.

**CLI integration**
- R9. A new `check-python` subcommand accepts `--package` (repeatable), `--config` (path to pyproject.toml), `--profile` (repeatable, overrides config), and all existing `--format` options (human, json, sarif, gha, junit). When no `--package` is given and a pyproject.toml exists, packages are read from config.

**pyproject.toml configuration**
- R10. The `[tool.schemalint]` section supports: `profiles` (list of profile IDs), `packages` (list of package names to discover), `exclude` (glob patterns for models to skip), and a `[tool.schemalint.severity]` sub-table for per-rule severity overrides. CLI flags override config values where both are provided.

---

## Acceptance Examples

- AE1. **Covers R1, R2, R8.** Given a project with a Pydantic model at `myapp/models.py:15` using a keyword the profile forbids, when the developer runs `schemalint check-python --package myapp.models --profile openai.so.2026-04-30`, the human output shows `error[OAI-K-...]: ... --> myapp/models.py:15:8`.

- AE2. **Covers R4.** Given a model with `email: str` declared at line 42 and a forbidden keyword on the email field's schema, the diagnostic points at line 42 — not a schema file.

- AE3. **Covers R5.** Given a project using Pydantic v1, when `check-python` runs, the output includes a warning diagnostic noting limited v1 support alongside any keyword diagnostics.

- AE4. **Covers R6.** Given the Python helper is not installed, when `check-python` runs, the CLI emits a single clear error message (not a stack trace) and exits with code 1.

- AE5. **Covers R10.** Given a `pyproject.toml` with `profiles = ["openai.so.2026-04-30"]` and `packages = ["myapp.models"]`, and a CLI invocation `schemalint check-python --profile anthropic.so.2026-04-30`, the CLI uses the Anthropic profile (CLI wins) but still discovers from `myapp.models`.

---

## Success Criteria

- A representative real-world Pydantic v2 codebase can be linted end-to-end in a single command.
- At least 10 known issues from the regression corpus produce correct diagnostics with Python source file and line attribution.
- Diagnostics for standard field declarations point at the correct `.py` file and line, verified manually against source.
- The same schema discovered via Pydantic and checked via `check-python` produces the same diagnostic codes as checking the raw JSON Schema file via `check`.
- The existing Phase 1 + Phase 2 regression corpus continues to pass when checked directly via `check` (no regression).
- Single-package discovery + check completes within 500 ms cold start (dominated by Pydantic import; discovery overhead is marginal).

---

## Scope Boundaries

- Process pool for parallel Python discovery — a single subprocess is sufficient for batch mode.
- Full Pydantic v1 support — v1 models are detected and schema-extracted on a best-effort basis, but v1-specific edge cases are not guaranteed.
- Zod / TypeScript ingestion — that is Phase 4.
- IDE or LSP integration — discovery is batch-only; no incremental or file-watching mode in this phase.
- Watch mode with incremental re-discovery — the `schemalint server` mode remains JSON-Schema-only.
- Auto-fix or schema rewriting — out of scope for v1 per the project SOW.
- Package distribution to PyPI, npm, and crates.io — that is Phase 5.
- Request-level Anthropic budget validation — requires multi-schema request context and belongs in ingestion helpers or a separate validator.

---

## Key Decisions

| Decision | Rationale |
|---|---|
| Single Python subprocess, not process pool | Batch mode needs only one discovery call per invocation. A pool adds lifecycle complexity (health checks, load balancing, cleanup) with no benefit until server-mode or multi-environment discovery arrives. |
| New `check-python` subcommand | File paths and Python package names have different semantics and argument validation. Overloading `check` with auto-detection would create ambiguity (is `myapp.models` a file or a package?) and surprise. |
| Source spans via JSON Pointer lookup map | The IR and normalizer remain unchanged. Spans survive `$ref` graph resolution (which does not inline) and type-array desugaring (synthetic nodes simply lack spans). |
| Pydantic v2 primary, v1 best-effort | v2 has been stable since 2023 and is the current standard. v1 is in maintenance mode. Detecting v1 and emitting a warning is honest; attempting full dual-version support in Phase 3 would balloon scope. |
| Python helper as JSON-RPC stdin/stdout server | Matches the existing `schemalint server` protocol pattern. Avoids filesystem temp files, network ports, and platform-specific IPC. The protocol is trivial to test (pipe stdin/stdout in integration tests). |
| pyproject.toml as the single config source | Standard Python project config location. Avoids inventing a new config format. Mirrors how ruff, mypy, and pytest configure themselves. |

---

## Dependencies / Assumptions

- Python 3.9+ available on the developer's machine (Pydantic v2 minimum).
- `pydantic` is installed in the Python environment where the target models live (the helper imports the user's modules, which import pydantic).
- The existing `serde_json` dependency is sufficient for JSON-RPC message serialization on the Rust side.
- No new Rust crate dependencies beyond what's already in `Cargo.toml` (`serde_json` for parsing discovery responses, `std::process::Command` for subprocess management).
- Pydantic v2's `model_json_schema()` output is stable enough that the JSON-RPC protocol does not need version negotiation in this phase.
- The `schemalint-pydantic` Python package is maintained in a separate repository or directory structure outside the Rust workspace (it is not a Cargo crate).

---

## Outstanding Questions

### Resolve Before Planning

*(none)*

### Deferred to Planning

- **[Needs research]** Exact JSON-RPC message schema for the `discover` response — model list shape, per-model schema embedding, source map key format.
- **[Needs research]** How the Python helper is packaged and installed — standalone repo with `pyproject.toml`, or co-located in the workspace under a `python/` directory.
- **[Needs research]** Pydantic v1 `model.schema()` output differences vs v2 `model_json_schema()` — what normalization the helper must apply for v1 schemas.
- **[Needs research]** How `inspect.getsource()` / `inspect.findsource()` performs on namespace packages, frozen modules, and `__init__.py` re-exports.
- **[Technical]** Subprocess spawn strategy — find the helper via `python -m schemalint_pydantic` (assumes it's on PATH), embed the helper script, or locate via `sys.executable` and a known package path.
- **[Technical]** Integration test strategy for the Python helper — spawn a real Python subprocess in Rust tests, or mock the helper with a Rust-side test double that replays pre-recorded discovery responses.
