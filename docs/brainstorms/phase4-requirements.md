---
date: 2026-05-01
topic: phase-4-zod-ingestion
---

# Phase 4 Requirements — Zod Ingestion

## Summary

Add a TypeScript-side discovery helper so `schemalint` can lint Zod projects directly — finding schemas via AST walking, converting them to JSON Schema, and attributing diagnostics to TypeScript source lines. Mirrors Phase 3's Pydantic ingestion architecture, adapted to the fundamentally different discovery surface of Zod (AST-based, not class-introspection).

---

## Assumptions

*This requirements doc was authored without synchronous user confirmation. The items below are agent inferences that fill gaps in the input — un-validated bets that should be reviewed before planning proceeds.*

- A single Node subprocess is sufficient for batch mode, mirroring Phase 3's single-Python-process design. The SOW and phases doc mention a "process pool" but Phase 3 validated that batch discovery runs once per CLI invocation — a pool adds lifecycle complexity with no benefit.
- The `@schemalint/zod` `lint()` function delegates checking to an installed `schemalint` CLI binary in Phase 4. Self-contained napi-rs integration arrives in Phase 5.
- TypeScript compiler API (either raw `typescript` package or `ts-morph` wrapper) is the right AST tooling. The specific library choice is deferred to planning.
- The helper package lives co-located in the repo at `typescript/schemalint-zod/`, mirroring `python/schemalint-pydantic/`.
- The default discoverable pattern is: exported `z.object(...)` calls at the top level of source files matched by the include globs. Inline/non-exported schemas, schemas composed from imported factory functions, and schemas returned from function calls are not discoverable via AST walking — this matches the stated SOW scope ("locates z.object(...) call expressions") and is consistent with the practical limits of static analysis.

---

## Problem Frame

Today, a TypeScript developer using Zod with OpenAI or Anthropic structured outputs must: (1) manually call `zod-to-json-schema` on each schema or maintain a separate generation script, (2) save the output to JSON files, (3) run `schemalint check` against those files, and (4) mentally map schema-path diagnostics (`/properties/email`) back to TypeScript source (`schemas/order.ts:42`). The JSON files are ephemeral artifacts that drift from source; regenerating them is manual, easily forgotten, and untethered from the developer's actual workflow.

Phase 4 removes this gap by giving `schemalint` first-class awareness of Zod projects. A TypeScript discovery helper uses the compiler API to walk the project's AST, find Zod schema declarations with their source locations, evaluate each schema at runtime, and convert to JSON Schema. The Rust CLI manages the helper process and feeds discovered schemas through the existing linting pipeline. Diagnostics land on TypeScript source where the developer works.

Unlike Pydantic (where schemas are class-based and introspectable at runtime via `inspect`), Zod schemas are runtime values constructed from builder patterns — the helper must walk the AST to find them and resolve source locations, then evaluate them at runtime to extract the JSON Schema. This dual approach (AST for discovery + runtime for extraction) is the defining architectural difference from Phase 3.

---

## Actors

- A1. **TypeScript developer** — Authors Zod schemas, runs `schemalint check-node`, reads diagnostics pointing at their `.ts` files.
- A2. **Rust CLI (schemalint)** — Orchestrates discovery, normalization, rule checking, and output emission.
- A3. **Node helper (schemalint-zod)** — A subprocess managed by the Rust CLI. Walks the TypeScript project AST to find Zod schemas, evaluates them at runtime, converts via `zod-to-json-schema`, and returns schemas with per-property source locations.
- A4. **Programmatic consumer** — Calls `lint(schemas[], options)` from `@schemalint/zod` in application code (CI scripts, pre-commit hooks, or test suites).

---

## Key Flows

- F1. **Batch check of a Zod project via CLI**
  - **Trigger:** Developer runs `schemalint check-node --entrypoint ./src/schemas/**/*.ts`
  - **Actors:** A1, A2, A3
  - **Steps:**
    1. Rust CLI spawns the Node helper subprocess
    2. CLI sends a `discover` JSON-RPC request with the entrypoint globs
    3. Helper reads tsconfig.json, resolves the project, walks the AST of matched files for `z.object(...)` call expressions
    4. Helper evaluates each found schema at runtime, converts via `zod-to-json-schema`, and records AST source locations per property
    5. Helper returns discovered schemas with per-property source maps
    6. CLI feeds each schema through the existing normalize → check pipeline
    7. CLI emits diagnostics with TypeScript source spans in the requested output format
  - **Outcome:** Developer sees rustc-style diagnostics pointing at their `.ts` files, with error codes and schema paths.
  - **Failure path:** If Node or the helper is not installed, the CLI emits a single clear error message and exits with code 1.
  - **Covered by:** R1, R2, R3, R4, R5, R6, R7, R8

- F2. **package.json-driven check**
  - **Trigger:** Developer runs `schemalint check-node` in a directory containing `package.json` with a `"schemalint"` field
  - **Actors:** A1, A2, A3
  - **Steps:**
    1. CLI reads `package.json` from cwd or explicit `--config` path
    2. CLI resolves profiles and include/exclude patterns from config
    3. CLI merges CLI flags on top of config (CLI wins)
    4. Discovery and linting proceed as in F1 for matched source files
  - **Outcome:** Developer configures once, then runs with no arguments. CI reads the same config.
  - **Covered by:** R8, R9

- F3. **Programmatic API call**
  - **Trigger:** Application code calls `lint([OrderSchema, UserSchema], { profile: "openai.so.2026-04-30" })`
  - **Actors:** A4, A2
  - **Steps:**
    1. `@schemalint/zod` converts each Zod schema to JSON Schema via `zod-to-json-schema` in-process
    2. If no `schemalint` CLI is on PATH, the call fails with a descriptive error
    3. The function sends JSON-RPC `check` requests to a `schemalint server` subprocess
    4. Diagnostics are collected, parsed, and returned to the caller as structured objects
  - **Outcome:** Caller receives typed diagnostic results without shelling out or parsing output manually.
  - **Covered by:** R10

---

## Requirements

**TypeScript helper package (schemalint-zod)**
- R1. `schemalint-zod` is an npm-installable TypeScript package with minimal dependencies (`zod` and `zod-to-json-schema` are peer dependencies, assumed already present in the user's project).
- R2. It runs as a JSON-RPC 2.0 server over stdin/stdout, accepting `discover` and `shutdown` methods. The protocol is line-delimited JSON, identical in structure to the Phase 3 Python helper.
- R3. The `discover` method accepts entrypoint globs (e.g., `./src/schemas/**/*.ts`), reads the TypeScript project configuration (tsconfig.json), resolves matched source files, walks their ASTs for top-level exported `z.object(...)` call expressions, evaluates each schema at runtime, converts via `zod-to-json-schema`, and returns each discovered schema with a per-property source map.

**Source span resolution**
- R4. Each property key in a `z.object({ email: z.string(), ... })` literal resolves to the correct file and line number from the AST. Nested `z.object()` properties are recursively mapped (e.g., `/properties/address/properties/street` → correct nested line). Schemas spread via `...` or composed from other variables map properties to the nearest identifiable source location with graceful degradation (the enclosing `z.object()` call line when the property's origin cannot be traced). Schemas constructed from imported factory functions or returned from function calls are not resolved at the property level and fall back to the call-site line — this limitation is documented and consistent with what the SOW specifies ("locates z.object(...) call expressions").

**Rust-side process management**
- R5. The Rust CLI manages a single Node subprocess: spawn, send JSON-RPC requests, read responses, and shutdown. The subprocess has a configurable execution timeout (60s default for discovery, matching Phase 3). The CLI surfaces a clear error when `node` is not on PATH, when the helper is not installed, when the helper crashes, when the TypeScript project fails to load (e.g., invalid tsconfig.json), and when a schema fails evaluation at runtime.
- R6. The existing normalization, rule-checking, and output pipeline is reused unchanged for discovered schemas — `process_schemas()`, `check_rulesets()`, `attach_source_spans()`, and all five emitters (human, JSON, SARIF, GHA, JUnit).

**Source span propagation**
- R7. Source spans from Zod discovery survive the linting pipeline and land on diagnostics. Human output shows `file.ts:line` (e.g., `schemas/order.ts:42:8`). JSON output populates the `source` field with `{ "file", "line", "col" }`. SARIF, GHA, and JUnit formats include source locations where their schemas support them.

**CLI integration**
- R8. A new `check-node` subcommand accepts: `--entrypoint` (repeatable, file globs pointing at TypeScript source files containing Zod schemas), `--config` (path to package.json, defaults to `./package.json`), `--profile` (repeatable, overrides config), `--node-path` (path to Node executable, defaults to `node` on PATH), and all existing `--format` options (human, json, sarif, gha, junit). When no `--entrypoint` is given and a package.json with a `"schemalint"` field exists, include patterns are read from config. When neither entrypoints nor config provides include patterns, the CLI exits with a clear error.

**package.json configuration**
- R9. The `"schemalint"` field in package.json supports: `profiles` (list of profile IDs), `include` (file globs for TypeScript source files containing Zod schemas), `exclude` (glob patterns for files to skip), and a `"severity"` object for per-rule overrides (keyed by rule code). CLI flags override config values where both are provided. The config shape mirrors the Phase 3 `[tool.schemalint]` pyproject.toml structure, adapted to JSON.

**Programmatic API**
- R10. `@schemalint/zod` exports a `lint(schemas: ZodObject[], options: LintOptions)` function. It converts each Zod schema to JSON Schema via `zod-to-json-schema`, then communicates with an installed `schemalint` CLI to perform profile-based checking. The function returns a promise resolving to structured diagnostic objects (with code, severity, message, pointer, and source fields). Self-contained operation via napi-rs bindings is explicitly deferred to Phase 5; the Phase 4 implementation requires `schemalint` on PATH.

**Module system and project compatibility**
- R11. Discovered entrypoints support both ESM (`"type": "module"`) and CommonJS project formats. tsconfig.json path aliases (the `paths` compiler option) are resolved during AST walking. Monorepo workspace layouts (npm workspaces, yarn workspaces, pnpm workspaces) receive basic support — the helper reads the root package.json workspaces configuration and discovers schemas within workspace packages. Bun and Deno runtimes are not targeted in Phase 4.

---

## Acceptance Examples

- AE1. **Covers R1, R2, R3, R4, R7, R8.** Given a project with `src/schemas/order.ts:15` containing `export const OrderSchema = z.object({ email: z.string() })` and a profile that forbids `format` on strings, when the developer runs `schemalint check-node --entrypoint ./src/schemas/**/*.ts --profile openai.so.2026-04-30`, the human output shows `error[OAI-K-format-restricted]: ... --> src/schemas/order.ts:15:8`.

- AE2. **Covers R4.** Given a nested Zod schema at `address.ts:10` with `z.object({ street: z.string(), city: z.string() })` used inside another schema at `user.ts:5` with `z.object({ name: z.string(), address: AddressSchema })`, diagnostics for `/properties/address/properties/street` point at `address.ts:10` (the property's definition site), not `user.ts:5`.

- AE3. **Covers R8, R9.** Given a `package.json` with `"schemalint": { "profiles": ["openai.so.2026-04-30"], "include": ["./src/schemas/**/*.ts"] }` and a CLI invocation `schemalint check-node --profile anthropic.so.2026-04-30`, the CLI uses the Anthropic profile (CLI wins) but still discovers from `./src/schemas/**/*.ts` (config provides include patterns).

- AE4. **Covers R5.** Given Node.js is not installed on PATH, when `check-node` runs, the CLI emits a single clear error ("node executable not found") and exits with code 1 — no stack trace or raw spawn error.

- AE5. **Covers R10.** Given a Node script that calls `lint([OrderSchema], { profile: "openai.so.2026-04-30" })` and the schema contains a forbidden keyword, the function returns a resolved promise with a diagnostic array containing an `OAI-K-*` code and the schema path as the pointer.

- AE6. **Covers R11.** Given a TypeScript project with `"type": "module"` in package.json, a tsconfig.json with `"paths": { "@schemas/*": ["./src/schemas/*"] }`, and a schema file that uses `import { OrderSchema } from "@schemas/order"`, when `check-node` runs, the helper resolves the path alias, discovers the schema, and produces correct diagnostics.

---

## Success Criteria

- A representative real-world TypeScript/Zod codebase (50+ schemas across multiple files) can be linted end-to-end in a single `schemalint check-node` command.
- At least 10 known Zod issues from the regression corpus produce correct diagnostics with TypeScript source file and line attribution.
- Standard `z.object({ email: z.string() })` declarations resolve diagnostics to the correct `.ts` file and line, verified manually against source.
- The same schema discovered via Zod and checked via `check-node` produces the same diagnostic codes as checking the raw JSON Schema file via `check` (identical rule engine output).
- The existing Phase 1 + Phase 2 + Phase 3 regression corpus continues to pass when checked via their respective entrypoints (no regression).
- Programmatic `lint()` function works correctly in both ESM and CJS Node projects and produces diagnostic objects with the same codes as the CLI.
- Single-project discovery + check completes within 500 ms cold start for a typical project (dominated by TypeScript project loading; AST walking and schema conversion overhead is marginal).

---

## Scope Boundaries

- Process pool for parallel Node discovery — a single subprocess is sufficient for batch mode. Multi-process parallelization deferred.
- napi-rs self-containment for `lint()` — the Phase 4 `lint()` function requires `schemalint` CLI on PATH. Self-contained operation via napi-rs native bindings arrives in Phase 5.
- Non-Zod TypeScript schema libraries (valibot, arktype, typebox) — Zod only in v1, matching the SOW's explicit scope boundary ("Other schema generators... the answer is the JSON-file fallback").
- Schemas constructed from imported factory functions (e.g., `export const Foo = buildFooSchema()` where `buildFooSchema` returns a Zod object) — these are not discoverable via AST walking. This is a documented limitation, consistent with the SOW's "locates z.object(...) call expressions" scope.
- Auto-detection of schemas from the import/export graph — requires explicit entrypoint patterns. Full import-graph traversal for automatic schema discovery deferred.
- IDE / LSP integration (lens-like "check this schema on save") — the `schemalint server` mode remains JSON-Schema-only for Phase 4. Adding Node ingestion to server mode deferred.
- Bun and Deno runtime support — Node.js only.
- Full monorepo tooling integration (turborepo, nx caching) — basic workspace support only. Caching-aware invocations deferred.
- Auto-fix / schema rewriting — out of scope for v1 per the project SOW.
- Package distribution to npm, PyPI, and crates.io — Phase 5.

---

## Key Decisions

| Decision | Rationale |
|---|---|
| `check-node` subcommand (not `check-zod` or `check-ts`) | Mirrors `check-python` naming convention — named after the runtime, not the library. Leaves room for non-Zod Node/TS schemas in the future. |
| AST-based discovery (not pure runtime import) | Zod schemas are values, not classes — there is no equivalent to Python's `inspect.getmembers()`. The helper must walk the source AST to find `z.object()` calls and their property source locations, then evaluate them at runtime. This is what the SOW specifies. |
| Single Node subprocess, not pool | Batch discovery runs once per CLI invocation. Phase 3 validated this architecture. A pool adds lifecycle complexity (health checks, load balancing, cleanup) with no benefit until server-mode ingestion arrives. |
| `lint()` as wrapper around CLI subprocess | Full napi-rs integration comes in Phase 5. For Phase 4, `lint()` converts schemas to JSON Schema in-process (via `zod-to-json-schema`) and delegates checking to a `schemalint server` subprocess. Requires `schemalint` on PATH. |
| `include` (file globs) for config, not `packages` | Python discovery uses importable package names (`packages`). TypeScript/Zod discovery works with source file patterns (`include`) because schemas are found by walking source files, not by importing modules. |
| Source spans from AST positions, not heuristic string matching | TypeScript AST nodes carry precise source positions in the compiler API. This gives more accurate spans than Phase 3's heuristic `inspect.getsource()` approach. The cost is the Ast+Runtime dual-path architecture. |
| `entrypoint` CLI arg (not `package` or `file`) | Term signals the user is specifying where schemas are defined, not which Node module to require. Distinct from the Phase 3 `--package` to avoid confusing Python developers who use both. |

---

## Dependencies / Assumptions

- Node.js 18+ available on the developer's machine (required by the TypeScript compiler API and `zod-to-json-schema`).
- `zod` and `zod-to-json-schema` are installed in the project where the target schemas live (peer dependencies of the helper).
- `schemalint` CLI binary installed and on PATH for the programmatic `lint()` function.
- The TypeScript project has a valid `tsconfig.json` reachable from the entrypoint root (the helper reads it for module resolution, path aliases, and project configuration).
- `serde_json` is sufficient for JSON-RPC message serialization on the Rust side — no new Rust crate dependencies beyond what Phase 3 already uses.
- The `schemalint-zod` TypeScript package is maintained in the same repository as the Rust workspace (co-located at `typescript/schemalint-zod/`, mirroring `python/schemalint-pydantic/`).

---

## Outstanding Questions

### Resolve Before Planning

*(none)*

### Deferred to Planning

- **[Technical]** Which TypeScript AST library — `ts-morph` (higher-level, richer API) vs raw `typescript` compiler API (lighter, no extra dependency). The choice affects the helper's dependency footprint, bundle size, and AST traversal ergonomics.
- **[Needs research]** ESM dynamic import behavior — `import()` returns a promise, but the JSON-RPC `discover` handler must process schemas synchronously within a single request/response cycle. The helper must use top-level await, a synchronous `require()` fallback for CJS, or a batched async evaluation model.
- **[Needs research]** Runtime schema evaluation strategy — whether to use `tsx`/`ts-node` for JIT compilation or require the project to be pre-compiled before discovery.
- **[Technical]** Integration test strategy for the Node helper — spawn a real Node subprocess in Rust tests (mirrors Phase 3 approach), or mock with Rust-side test doubles that replay pre-recorded discovery responses.
- **[Needs research]** Exact JSON-RPC message schema for the `discover` response — discovered schema list shape, per-schema property name and module path conventions, source map key format (JSON Pointer keys matching the Phase 3 DiscoveredModel shape).
- **[Technical]** How `attach_source_spans()` generalizes to support both Python and Node discovered models — extract a shared trait vs maintain parallel implementations for each ingestion source.
- **[Needs research]** Best-effort monorepo workspace support — how to resolve schemas across workspace package boundaries, whether to discover from the root or per-package tsconfig.json.
