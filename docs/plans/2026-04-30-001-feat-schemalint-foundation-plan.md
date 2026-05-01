---
title: schemalint Phase 1 — Foundation
type: feat
status: active
date: 2026-04-30
origin: docs/brainstorms/phase1-requirements.md
deepened: 2026-04-30
---

# schemalint Phase 1 — Foundation

## Summary

Build the minimal end-to-end diff tool: a Rust CLI that reads JSON Schemas and an OpenAI capability profile, normalizes schemas into an arena-allocated IR, auto-generates keyword rules from the profile, and emits deterministic diagnostics in rustc-style human and structured JSON formats. The output is two crates (`schemalint` monolithic engine + `schemalint-profiles` data), a complete OpenAI profile with zero `unknown` states for Pydantic/Zod-emitted keywords, and a 50-schema regression corpus that gates all acceptance.

---

## Problem Frame

Engineers write JSON Schemas for LLM structured-output endpoints, but providers accept only a strict subset. Failures surface at runtime as cryptic 400 errors, or worse, schemas are silently transformed by SDKs — dropping constraints the engineer expected to be enforced. There is no static tool that checks a schema against the provider's accepted subset before the API call. (see origin: docs/brainstorms/phase1-requirements.md §1)

---

## Requirements

- R1. `schemalint check --profile <path> <schema-file>...` runs end-to-end and produces deterministic output. (origin: SC1)
- R2. Human output follows rustc-style format with file path, line:col (when available), error code, message, schema JSON Pointer, and profile reference. (origin: SC2)
- R3. JSON output is a structured object containing schema version, tool metadata, summary counts, and a per-diagnostic array. (origin: SC3)
- R4. The OpenAI profile has zero `unknown` states for all keywords emitted by Pydantic v2 and `zod-to-json-schema`. (origin: SC4)
- R5. All 50 schemas in the regression corpus produce expected diagnostics when run through the diff tool. (origin: SC5)
- R6. Single 200-property nested schema, single profile: < 1 ms on reference hardware (Apple M3 or equivalent 2-core CI runner). (origin: SC6)
- R7. Project of 500 schemas, 1 profile, cold start: < 500 ms on reference hardware. (origin: SC7)
- R8. Incremental run within a single batch invocation (in-memory cache hit): < 5 ms on reference hardware. (origin: SC8)
- R9. Every rule (Class A auto-generated + Class B data-driven) has at least one positive and one negative test. (origin: QT1)
- R10. Snapshot testing for human and JSON output formats. (origin: QT2)
- R11. Property tests for normalizer round-trips. (origin: QT3)

**Origin actors:** A1 (engineer authoring JSON Schema), A2 (CI pipeline maintainer)
**Origin flows:** F1 (single-schema check), F2 (batch project check), F3 (CI integration)
**Origin acceptance examples:** AE1 (allOf forbidden → error), AE2 (uniqueItems warned → warning with hint)

---

## Scope Boundaries

- No SARIF, GitHub Actions annotation, or JUnit XML output (Phase 2)
- No Pydantic or Zod ingestion helpers (Phase 3–4)
- No multi-profile composition or server mode (Phase 2)
- No hand-written Class B semantic rules (infrastructure only; rules deferred to Phase 2)
- No auto-fix or schema rewriting (out of scope for v1 per SOW §3.2)
- No disk-based incremental cache (in-memory only for Phase 1; persistence deferred to Phase 2 `--watch` server)
- No built-in profile resolution by profile ID without file path (Phase 2; Phase 1 requires `--profile <path>`)
- No source spans (line:col) for raw `.json` files (available only via Pydantic/Zod ingestion in Phase 3+)

### Deferred to Follow-Up Work

- Disk-based incremental cache and cache invalidation contract: Phase 2 `--watch` server mode
- Built-in profile ID resolution (e.g., `--profile openai.so.2026-04-30` without path): Phase 2
- Hand-written Class B semantic rules (cycle depth bounds, empty-object detection, etc.): Phase 2
- Source span attribution for raw JSON (spanned JSON parser or byte-offset tracking): Phase 3+
- Diagnostic deduplication when multiple rules fire on the same node: Phase 2

---

## Context & Research

### Relevant Code and Patterns

- Greenfield repository; no existing Rust code or `Cargo.toml`. All patterns must be established in this phase.
- The SOW (`docs/sow.md`) prescribes arena allocation via `Vec<Node>` indexed by `NodeId(u32)`, parent links, stable JSON Pointers, source spans, and `$ref` edges. (SOW §6.2)
- The normalizer pipeline is specified as: dialect detection → `$ref` graph resolution (no inline expansion) → Tarjan SCC → type-array desugaring → parent/depth/JSON Pointer DFS → content-hash caching. (SOW §6.3)
- `linkme` distributed slices are chosen over `inventory` for cross-platform reliability. (origin: §5)

### Institutional Learnings

- No `docs/solutions/` exists yet. Phase 1 decisions will become the first institutional learnings.
- Performance constraints are treated as CI-blocking: profile rule maps must compile to `HashMap<&'static str, Severity>` once per process; regexes compiled once and cached; JSON Pointer strings built lazily. (SOW §12.1)
- Monolithic crate eliminates cross-crate refactoring tax during the most volatile phase. Only `schemalint-profiles` is separate from day one because it ships on a different cadence. (origin: §5)

### External References

- OpenAI Structured Outputs docs (`developers.openai.com/api/docs/guides/structured-outputs`), scraped 2026-04-30 — authoritative contract for the profile. (origin: §7)
- `linkme` crate docs — distributed slice registration mechanism.
- `toml` crate docs — profile TOML parsing.
- `serde_json` docs — JSON Schema parsing (note: `serde_json` does not provide byte offsets; source spans for raw JSON are deferred).

---

## Key Technical Decisions

- **Monolithic `schemalint` crate + separate `schemalint-profiles` crate:** Eliminates cross-crate refactoring tax during the most volatile phase. Only `schemalint-profiles` is separate because it ships on a different release cadence. (origin: §5)
- **`linkme` over `inventory` for rule auto-registration:** More reliable across platforms. No proc-macro complexity. Adding a Class B rule requires only authoring `impl Rule`. (origin: §5)
- **Arena-allocated IR with `NodeId(u32)`:** Mandated by performance targets. `Vec<Node>` indexing avoids `Rc/Arc` overhead. Parent links and `$ref` edges stored as `NodeId` indices. (origin: §4, SOW §6.2)
- **All standard JSON Schema keywords in typed `Annotations` struct:** Ensures Class A rules can inspect every profile-mapped keyword. The `unknown` map holds only non-standard keys. (resolved during planning)
- **Unresolved internal `$ref` → fatal error:** Normalization aborts with a clear message. Keeps the IR valid-by-construction; rules never handle invalid `NodeId`s. (resolved during planning)
- **Depth on cyclic graphs: shortest acyclic path; DFS stops at back-edges:** Prevents infinite traversal. Structural limit `max_object_depth` uses the first-visit depth. (resolved during planning)
- **In-memory cache only for Phase 1:** The <5 ms incremental target applies to batch process reuse within a single CLI invocation, not cold CLI disk persistence. Disk cache deferred to Phase 2 server mode. (resolved during planning)
- **Source spans omitted for raw `.json` in Phase 1:** `serde_json` does not provide byte offsets. Line:col attribution available only via Pydantic/Zod ingestion helpers (Phase 3+). Human output shows file path only for raw JSON. (resolved during planning)
- **Exit code contract:** 0 if no `error` severity diagnostics; 1 if any `error` or fatal parse error; warnings alone exit 0. Standard linter behavior for CI integration. (resolved during planning)
- **Overlapping diagnostics from multiple rules: both emitted, no deduplication in Phase 1:** Keeps the rule engine simple. Deduplication deferred to Phase 2 when more rules exist. (resolved during planning)

---

## Open Questions

### Resolved During Planning

- **Does the CLI accept a directory or only a single file?** The CLI accepts multiple schema files and directories. File discovery: recursive, `.json` extension filter, no symlink follow. Required to satisfy the 500-schema cold-start target. (origin CLI syntax was incomplete)
- **What is the failure mode for an unresolved internal `$ref`?** Fatal error with clear message. Normalization aborts. (resolved: valid-by-construction IR)
- **How is `max_object_depth` calculated on cyclic graphs?** Shortest acyclic path; DFS stops at back-edges. (resolved)
- **Where does the incremental cache live?** In-memory only for Phase 1. Disk cache deferred to Phase 2. (resolved)
- **How do Class A rules inspect keywords not in typed `Annotations`?** All standard JSON Schema keywords are in `Annotations`. `unknown` holds only non-standard keys. (resolved)
- **What is the exit code contract?** 0 = no errors; 1 = any error or fatal parse error; warnings = 0. (resolved)
- **How is JSON output selected?** `--format human|json` flag, default human when TTY detected. (resolved)
- **Should built-in profile IDs resolve without a path in Phase 1?** No — deferred to Phase 2. Phase 1 requires `--profile <path>`. (resolved)
- **How are source spans produced for raw `.json`?** Omitted in Phase 1. Available only via Pydantic/Zod ingestion (Phase 3+). (resolved)
- **Are overlapping diagnostics deduplicated?** No — both emitted in Phase 1. Deduplication deferred. (resolved)

### Deferred to Implementation

- Exact `Annotations` struct field naming and granularity (e.g., whether to group string constraints, number constraints) — depends on ergonomic access patterns during rule implementation.
- Exact content-hash algorithm for cache keys (e.g., `blake3`, `xxhash`, `sha256`) — benchmarked during implementation.
- Exact TOML profile schema for the `[structural]` section key names — finalized during profile loader implementation.
- Whether to use `clap` derive or builder API for CLI parsing — decision deferred to implementation based on complexity of argument combinations.
- Specific error code prefix scheme beyond `OAI-K-*` and `OAI-S-*` — finalized when registry assigns codes.

---

## Output Structure

```
.
├── Cargo.toml                          # Workspace root
├── Cargo.lock
├── rust-toolchain.toml                 # stable
├── .gitignore
├── README.md
├── AGENTS.md
├── crates/
│   ├── schemalint/
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   │   ├── main.rs                 # CLI entry point
│   │   │   ├── lib.rs                  # Public API surface
│   │   │   ├── ir/
│   │   │   │   ├── mod.rs              # Node, NodeId, NodeKind, Annotations
│   │   │   │   └── arena.rs            # Arena allocator, NodeId(u32) indexing
│   │   │   ├── normalize/
│   │   │   │   ├── mod.rs              # Normalizer pipeline orchestrator
│   │   │   │   ├── dialect.rs          # Dialect detection
│   │   │   │   ├── refs.rs             # $ref graph resolution, Tarjan SCC
│   │   │   │   ├── desugar.rs          # Type-array desugaring
│   │   │   │   └── traverse.rs         # Parent/depth/JSON Pointer DFS
│   │   │   ├── profile/
│   │   │   │   ├── mod.rs              # Profile loader, Severity, Profile struct
│   │   │   │   └── parser.rs           # TOML -> Profile
│   │   │   ├── rules/
│   │   │   │   ├── mod.rs              # Rule trait, RuleId, Diagnostic
│   │   │   │   ├── registry.rs         # linkme distributed slice registry
│   │   │   │   ├── class_a.rs          # Auto-generated keyword rules from profile
│   │   │   │   └── class_b.rs          # Structural rule infrastructure
│   │   │   ├── cli/
│   │   │   │   ├── mod.rs              # CLI module
│   │   │   │   ├── args.rs             # clap argument definitions
│   │   │   │   ├── discover.rs         # File/directory discovery
│   │   │   │   ├── emit_human.rs       # rustc-style human output formatter
│   │   │   │   └── emit_json.rs        # JSON output formatter
│   │   │   └── cache.rs                # In-memory content-hash cache
│   │   └── tests/
│   │       ├── ir_tests.rs             # IR and parser tests
│   │       ├── profile_tests.rs        # Profile loader tests
│   │       ├── normalizer_tests.rs     # Normalizer pipeline tests
│   │       ├── rules_tests.rs          # Rule registry and Class A tests
│   │       ├── structural_tests.rs     # Class B structural rule tests
│   │       ├── cli_tests.rs            # CLI argument and output tests
│   │       ├── integration_tests.rs    # End-to-end CLI tests
│   │       ├── snapshot_tests.rs       # Human/JSON output snapshots
│   │       ├── property_tests.rs       # Normalizer round-trip properties
│   │       ├── corpus_tests.rs         # Regression corpus validation
│   │       └── corpus/
│   │           ├── README.md           # Corpus documentation
│   │           ├── schema_01.json      # 50 curated schemas
│   │           ├── schema_01.expected  # Expected diagnostics per schema
│   │           └── ...
│   └── schemalint-profiles/
│       ├── Cargo.toml
│       ├── src/
│       │   └── lib.rs                  # Profile data constants, built-in profiles
│       └── profiles/
│           └── openai.so.2026-04-30.toml
└── benches/
    └── schemalint_benchmarks.rs        # Criterion benchmarks for performance targets
```

---

## High-Level Technical Design

> *This illustrates the intended approach and is directional guidance for review, not implementation specification. The implementing agent should treat it as context, not code to reproduce.*

### Component Diagram

```
+-------------------------------------------------------------+
|                        CLI (main.rs)                         |
|  args.rs -> discover.rs -> [cache.rs] -> normalize -> rules  |
|                                            |                 |
|                                            v                 |
|                                       emit_human / emit_json |
+-------------------------------------------------------------+
                              |
                              v
+-------------------------------------------------------------+
|                     schemalint (lib.rs)                      |
|  +-------------+  +-------------+  +---------------------+  |
|  |   IR /      |  |  Normalizer |  |   Rule Engine       |  |
|  |   Arena     |  |  Pipeline   |  |   (linkme slices)   |  |
|  |             |  |             |  |                     |  |
|  | NodeId(u32) |<-| dialect     |  | Class A: auto-gen   |  |
|  | Vec<Node>   |  | refs+Tarjan |  |   from profile      |  |
|  | parent/depth|  | desugar     |  | Class B: structural |  |
|  | JSON Pointer|  | traverse    |  |   infra only        |  |
|  +-------------+  +-------------+  +---------------------+  |
|        ^                                    |                |
|        |                                    v                |
|  +-------------+                   +---------------------+  |
|  | Profile     |                   |   Diagnostics       |  |
|  | Loader      |                   |   + Emitters        |  |
|  | (TOML)      |                   |   human / json      |  |
|  +-------------+                   +---------------------+  |
+-------------------------------------------------------------+
                              |
                              v
+-------------------------------------------------------------+
|             schemalint-profiles (data crate)                 |
|        openai.so.2026-04-30.toml (built-in)                  |
+-------------------------------------------------------------+
```

### Normalizer Pipeline Flow

```
Raw JSON Schema bytes
        |
        v
[1] Content-Hash Cache ----> Hash raw bytes -> cache hit? return cached graph
        |
        v  (cache miss)
[2] Parse JSON ------------> serde_json::Value
        |
        v
[3] Dialect Detection -----> $schema keyword? heuristic fallback?
        |
        v
[4] Parse into Arena IR ---> Vec<Node>, NodeId(u32), retain all keywords
        |
        v
[5] $ref Graph Resolution -> Build edges, no inline expansion
        |
        v
[6] Tarjan SCC -----------> Detect recursive cycles, mark cyclic nodes
        |
        v
[7] Type-Array Desugaring -> ["string", "null"] -> anyOf union
        |
        v
[8] DFS: Parent/Depth/Pointer -> Stops at back-edges for cycles
        |
        v
[9] Store in Cache --------> Save normalized graph by content hash
        |
        v
Normalized Schema Graph (ready for rule engine)
```

---

## Implementation Units

- U1. **Bootstrap Cargo Workspace and Crate Skeleton**

**Goal:** Establish the repository structure, workspace configuration, and build toolchain.

**Requirements:** R1 (enables CLI), R6–R8 (enables performance benchmarking infrastructure)

**Dependencies:** None

**Files:**
- Create: `Cargo.toml` (workspace)
- Create: `rust-toolchain.toml`
- Create: `crates/schemalint/Cargo.toml`
- Create: `crates/schemalint/src/main.rs`
- Create: `crates/schemalint/src/lib.rs`
- Create: `crates/schemalint-profiles/Cargo.toml`
- Create: `crates/schemalint-profiles/src/lib.rs`
- Create: `benches/schemalint_benchmarks.rs`
- Create: `README.md`
- Create: `AGENTS.md`
- Create: `crates/schemalint/src/cache.rs`
- Modify: `.gitignore`

**Approach:**
- Workspace root with `resolver = "2"`, two member crates.
- `schemalint` crate: `bin` + `lib`, dependencies include `serde_json`, `toml`, `indexmap`, `linkme`, `clap`.
- `schemalint-profiles` crate: `lib` only, minimal dependencies (`toml` for validation if needed).
- Set `rust-toolchain.toml` to `stable`.
- Add `.gitignore` entries for `target/`, `Cargo.lock` (keep for bin), `.schemalint/`.
- Create empty module files to establish directory structure.

**Patterns to follow:**
- Standard Cargo workspace layout. No existing patterns to follow (greenfield).

**Test scenarios:**
- **Happy path:** `cargo build` succeeds with zero warnings.
- **Edge case:** `cargo test --workspace` runs and passes with zero tests (empty test suite).
- **Integration:** `cargo bench` compiles (no benchmarks yet, but harness exists).

**Verification:**
- `cargo build --workspace` succeeds.
- `cargo test --workspace` passes.
- `cargo clippy --workspace` passes with no warnings.

---

- U2. **Arena-Allocated IR and JSON Schema Parser**

**Goal:** Define the internal representation and implement JSON Schema → IR parsing.

**Requirements:** R1 (enables schema ingestion), R6–R8 (arena allocation is the performance foundation), R11 (property tests need round-trippable IR)

**Dependencies:** U1

**Files:**
- Create: `crates/schemalint/src/ir/mod.rs`
- Create: `crates/schemalint/src/ir/arena.rs`
- Modify: `crates/schemalint/src/lib.rs`
- Test: `crates/schemalint/tests/ir_tests.rs`

**Approach:**
- `NodeId(u32)` newtype wrapping `u32`, with `Index` impls into `Arena`.
- `Arena` struct wrapping `Vec<Node>`, providing `alloc(node) -> NodeId`.
- `Node` struct containing: `kind: NodeKind`, `annotations: Annotations`, `unknown: IndexMap<String, Value>`, `parent: Option<NodeId>`, `depth: u32`, `json_pointer: String`, `source_span: Option<Span>`, `ref_target: Option<NodeId>`, `is_cyclic: bool`.
- `NodeKind` enum: `Object`, `Array`, `String`, `Integer`, `Number`, `Boolean`, `Null`, `Any`, `Ref`, `AnyOf`, `OneOf`, `AllOf`, `Not`.
- `Annotations` struct: one field for every standard JSON Schema keyword (`type`, `properties`, `required`, `additional_properties`, `items`, `prefix_items`, `min_items`, `max_items`, `unique_items`, `contains`, `minimum`, `maximum`, `exclusive_minimum`, `exclusive_maximum`, `multiple_of`, `min_length`, `max_length`, `pattern`, `format`, `enum_values`, `const_value`, `pattern_properties`, `unevaluated_properties`, `property_names`, `min_properties`, `max_properties`, `description`, `title`, `default`, `discriminator`, `$ref`, `$defs`, `definitions`, `any_of`, `all_of`, `one_of`, `not`, `if_schema`, `then_schema`, `else_schema`, `dependent_required`, `dependent_schemas`).
- `unknown: IndexMap<String, Value>` holds non-standard keywords.
- Parser: recursively walk `serde_json::Value`, allocate `Node`s into arena, populate `Annotations` from known keywords, push unknowns to `unknown` map.
- Handle boolean schemas: `true` → `NodeKind::Any`, `false` → `NodeKind::Not` with empty inner.

**Execution note:** Implement parser test-first. Start with happy-path JSON objects, then add edge cases.

**Patterns to follow:**
- Arena pattern: `bumpalo`-style but manual `Vec` for control over `NodeId` stability.
- `IndexMap` for keyword order preservation (required for deterministic output).

**Test scenarios:**
- **Happy path:** Parse a simple object schema with `type`, `properties`, `required` → all keywords land in `Annotations`.
- **Happy path:** Parse a nested schema with `$ref` to `$defs` → `$ref` keyword captured in `Annotations`, target resolution deferred to normalizer (U4).
- **Edge case:** Parse a schema with unknown keywords → land in `unknown` map, not dropped.
- **Edge case:** Parse boolean schema `true` → `NodeKind::Any`.
- **Edge case:** Parse boolean schema `false` → `NodeKind::Not`.
- **Edge case:** Parse empty object `{}` → `NodeKind::Any` with empty annotations.
- **Error path:** Parse invalid JSON → fatal error propagated to caller.
- **Integration:** Parse schema with duplicate JSON keys → last value wins (consistent with `serde_json` behavior), documented as known limitation.

**Verification:**
- All parser tests pass.
- `cargo test --test ir_tests` passes.
- Memory layout verified: `Node` size is reasonable for arena (no accidental `String` bloat in hot path).

---

- U3. **Profile Loader and Severity Model**

**Goal:** Load TOML capability profiles and compile them into an efficient in-memory rule map.

**Requirements:** R1 (profile needed for diff), R4 (OpenAI profile completeness), R6–R8 (profile compilation to `HashMap` once per process)

**Dependencies:** U1

**Files:**
- Create: `crates/schemalint/src/profile/mod.rs`
- Create: `crates/schemalint/src/profile/parser.rs`
- Modify: `crates/schemalint/src/lib.rs`
- Create: `crates/schemalint-profiles/profiles/openai.so.2026-04-30.toml`
- Test: `crates/schemalint/tests/profile_tests.rs`

**Approach:**
- `Severity` enum: `Allow`, `Warn`, `Strip`, `Forbid`, `Unknown`.
- Output severity mapping: `Forbid` → `Error`, `Strip` → `Error` (treated as forbidden in Phase 1), `Warn` → `Warning`, `Allow`/`Unknown` → no diagnostic.
- `Profile` struct: `name: String`, `version: String`, `keyword_map: HashMap<&'static str, Severity>`, `restrictions: HashMap<&'static str, Restriction>`, `structural: StructuralLimits`.
- `StructuralLimits` struct: fields matching TOML keys (`require_object_root`, `require_additional_properties_false`, `require_all_properties_in_required`, `max_object_depth`, `max_total_properties`, `max_total_enum_values`, `max_string_length_total`).
- TOML format: `keyword = "severity"` at top level, `[structural]` section for limits, `[[restrictions]]` tables for value-restricted keywords.
- Profile compilation: parse TOML once, build `HashMap` with `&'static str` keys (leak string for lookup performance), validate that all keywords are known.
- OpenAI profile: encode the complete keyword table from origin §7, with `[structural]` section per origin §7. `format` restricted to `date-time, time, date, duration, email, hostname, ipv4, ipv6, uuid`. `additionalProperties` restricted to `false`.

**Patterns to follow:**
- Compile profile to `HashMap<&'static str, Severity>` once per process (SOW §12.1).
- Profile TOML syntax matches origin §7 exactly.

**Test scenarios:**
- **Happy path:** Load a valid profile TOML → `Profile.keyword_map` contains all entries.
- **Happy path:** Look up severity for known keyword → returns correct `Severity`.
- **Edge case:** Load profile with extra whitespace, comments → parses correctly.
- **Error path:** Invalid TOML syntax → fatal error with clear message.
- **Error path:** Unknown severity string in TOML → fatal error listing valid values.
- **Error path:** Missing required `[structural]` section → fatal error or default values (decision: fatal error for Phase 1 to enforce explicitness).
- **Integration:** OpenAI profile loads with zero `unknown` states for Pydantic/Zod-emitted keywords → verified by programmatic scan of `keyword_map`.

**Verification:**
- OpenAI profile loads successfully.
- All profile tests pass.
- `cargo test --test profile_tests` passes.

---

- U4. **Schema Normalizer Pipeline**

**Goal:** Transform parsed IR into a normalized graph ready for rule evaluation, with dialect detection, `$ref` resolution, cycle detection, desugaring, and depth/JSON Pointer computation.

**Requirements:** R1 (normalizer is core of diff tool), R6–R8 (performance-critical), R11 (property tests for round-trips)

**Dependencies:** U2 (needs IR)

**Files:**
- Create: `crates/schemalint/src/normalize/mod.rs`
- Create: `crates/schemalint/src/normalize/dialect.rs`
- Create: `crates/schemalint/src/normalize/refs.rs`
- Create: `crates/schemalint/src/normalize/desugar.rs`
- Create: `crates/schemalint/src/normalize/traverse.rs`
- Modify: `crates/schemalint/src/lib.rs`
- Test: `crates/schemalint/tests/normalizer_tests.rs`

**Approach:**
- Orchestrator (`normalize/mod.rs`): runs the pipeline in order, returns `NormalizedSchema { arena, root_id }`.
- **Step 0 — Content-Hash Cache Lookup** (`cache.rs`): Before any work, hash raw JSON bytes using a fast hash (benchmarked during implementation). Check in-memory `HashMap` for `hash -> NormalizedSchema`. On cache hit, return cached result immediately. On miss, proceed.
- **Step 1 — Dialect Detection** (`dialect.rs`): Inspect `$schema` keyword. If present, record dialect version. If absent, use heuristic (presence of Draft 2020-12 keywords like `prefix_items`). Does not reject unknown dialects — just records for potential future use.
- **Step 2 — `$ref` Graph Resolution** (`refs.rs`): Walk all nodes, collect `$ref` edges. Internal refs (starting with `#`) resolved to `$defs` or `definitions` targets. External refs (starting with `http://`, `https://`, or absolute path) detected but not resolved in Phase 1 — marked for future handling. Build adjacency list of `NodeId` → `NodeId`.
- **Step 3 — Tarjan SCC** (`refs.rs`): Run Tarjan's algorithm on the `$ref` graph. Detect strongly connected components. Mark nodes in SCCs of size > 1 or with self-loops as `is_cyclic = true`.
- **Step 4 — Type-Array Desugaring** (`desugar.rs`): Convert `type: ["string", "null"]` to `anyOf: [{"type": "string"}, {"type": "null"}]`. Update `NodeKind` from `Any` (or specific type) to `AnyOf` with two children. This simplifies rule evaluation by making unions explicit.
- **Step 5 — DFS: Parent, Depth, JSON Pointer** (`traverse.rs`): Single DFS from root. For each node: set `parent`, compute `depth = parent.depth + 1`, compute `json_pointer` from parent pointer + property name or array index. **On back-edges (cyclic $ref)**: stop traversal, do not increase depth. This gives shortest acyclic path depth.
- **Step 6 — Content-Hash Cache Store** (`cache.rs`): Store the newly normalized `NormalizedSchema` in the in-memory `HashMap` keyed by content hash.

**Execution note:** Add characterization coverage before modifying the normalizer once it is working. The normalizer is the most complex component; regressions are costly.

**Patterns to follow:**
- No inline `$ref` expansion (SOW §6.3): `$ref` nodes remain as `NodeKind::Ref` with `ref_target` set. Rules that need to inspect the target follow the edge.
- Depth stops at back-edges for cycles (resolved during planning).

**Test scenarios:**
- **Happy path:** Normalize simple object schema → all parent links correct, depth = 1 for properties, JSON Pointer = `/properties/foo`.
- **Happy path:** Normalize schema with internal `$ref` → `ref_target` points to correct `$defs` node.
- **Happy path:** Normalize schema with `type: ["string", "null"]` → desugared to `AnyOf` with two children.
- **Edge case:** Normalize empty object `{}` → root `NodeKind::Any`, depth = 0.
- **Edge case:** Normalize boolean schema `true` → root `NodeKind::Any`.
- **Edge case:** Normalize deeply nested schema (depth > 10) → `depth` field correctly computed.
- **Edge case:** Cyclic `$ref` (A -> B -> A) → both nodes marked `is_cyclic = true`, depth computed as shortest acyclic path.
- **Edge case:** Self-referential `$ref` (A -> A) → marked `is_cyclic = true`.
- **Error path:** Unresolved internal `$ref` (points to missing `$defs` entry) → fatal error, normalization aborts.
- **Integration:** Content-hash cache hit on identical schema → normalization skipped, cached result returned.
- **Property test:** Round-trip invariant: any valid JSON Schema parses to IR, and the IR retains all keywords (nothing lost).

**Verification:**
- All normalizer tests pass.
- `cargo test --test normalizer_tests` passes.
- Property tests run and pass (`proptest` or `quickcheck`).

---

- U5. **Auto-Registered Rule Registry with Class A Rule Generation**

**Goal:** Build the rule trait, diagnostic types, `linkme`-based auto-registration, and auto-generate Class A keyword rules from the loaded profile.

**Requirements:** R1 (rules produce diagnostics), R2–R3 (diagnostics need codes and structure), R9 (every rule has tests)

**Dependencies:** U2 (needs IR to inspect), U3 (needs Profile to know which rules to generate)

**Files:**
- Create: `crates/schemalint/src/rules/mod.rs`
- Create: `crates/schemalint/src/rules/registry.rs`
- Create: `crates/schemalint/src/rules/class_a.rs`
- Modify: `crates/schemalint/src/lib.rs`
- Test: `crates/schemalint/tests/rules_tests.rs`

**Approach:**
- `Rule` trait: `fn check(&self, node: NodeId, arena: &Arena, profile: &Profile) -> Vec<Diagnostic>`.
- `Diagnostic` struct: `code: String`, `severity: Severity`, `message: String`, `pointer: String`, `source: Option<SourceSpan>`, `profile: String`, `hint: Option<String>`.
- `RuleId` struct: stable identifier for each rule.
- Registry: use `linkme` distributed slices. Each rule implementation registers itself via `#[linkme::distributed_slice(RULES)] static RULE: &dyn Rule = &MyRule;`.
- Class A rule generation: After loading profile, iterate over `keyword_map`. For each keyword with severity != `Allow`, generate a rule that checks if any node's `Annotations` contains that keyword. The rule emits a `Diagnostic` with severity from the profile.
- Restricted keyword rules: For keywords in `profile.restrictions`, generate a rule that checks if the keyword's value is in the allowed set. If not, emit an `Error` diagnostic (e.g., `format: "credit-card"` → `OAI-K-format-restricted` error).
- Error code scheme: `OAI-K-<keyword>` for keyword rules (e.g., `OAI-K-allOf`), `OAI-K-<keyword>-restricted` for value-restricted keywords, `OAI-S-<limit>` for structural rules.

**Patterns to follow:**
- `linkme` distributed slices for auto-registration (origin: §5).
- `HashMap<&'static str, Severity>` compiled once per process (SOW §12.1).

**Test scenarios:**
- **Happy path:** Profile marks `allOf` as `Forbid`. Schema contains `allOf` → Class A rule fires, diagnostic with `severity = Error`, `code = OAI-K-allOf`.
- **Happy path:** Profile marks `uniqueItems` as `Warn`. Schema contains `uniqueItems` → Class A rule fires, diagnostic with `severity = Warning`, `code = OAI-K-uniqueItems`.
- **Happy path:** Profile marks `type` as `Allow`. Schema contains `type` → no diagnostic.
- **Happy path:** Profile restricts `format` to `date-time, email`. Schema uses `format: "date-time"` → no diagnostic.
- **Edge case:** Profile restricts `format` to `date-time, email`. Schema uses `format: "credit-card"` → `OAI-K-format-restricted` error.
- **Edge case:** Schema contains keyword in `unknown` map (non-standard key) → no Class A rule fires (unknown keys not in profile).
- **Edge case:** Multiple schemas in batch → rules run per-schema, diagnostics aggregated.
- **Integration:** Class A rules + profile loader → load OpenAI profile, run on schema with mixed keywords, output matches expected diagnostic set.

**Verification:**
- All rule tests pass.
- `cargo test --test rules_tests` passes.
- Auto-registration verified: adding a new `impl Rule` in a test module automatically appears in registry.

---

- U6. **Class B Structural Rule Infrastructure**

**Goal:** Implement the data-driven structural rule engine that reads limits from the profile `[structural]` section and enforces them during schema traversal.

**Requirements:** R1 (structural rules are part of diff output), R4 (OpenAI profile structural limits), R6–R8 (structural checks must be fast)

**Dependencies:** U4 (needs normalized IR with depth computed), U5 (needs registry to register Class B rules)

**Files:**
- Create: `crates/schemalint/src/rules/class_b.rs`
- Modify: `crates/schemalint/src/rules/registry.rs`
- Test: `crates/schemalint/tests/structural_tests.rs`

**Approach:**
- After loading profile, read `structural` limits. Auto-generate structural rules from these values:
  - `require_object_root = true` → rule checks `root.kind == NodeKind::Object`.
  - `require_additional_properties_false = true` → rule checks all `Object` nodes have `additional_properties == Some(false)`.
  - `require_all_properties_in_required = true` → rule checks all keys in `properties` are present in `required`.
  - `max_object_depth = 10` → rule checks `node.depth <= 10` for all nodes.
  - `max_total_properties = 5000` → rule counts total `properties` entries across entire schema, checks `<= 5000`.
  - `max_total_enum_values = 1000` → rule counts total `enum_values` entries across entire schema, checks `<= 1000`.
  - `max_string_length_total = 120000` → rule sums length of all property names and string enum values across the schema, checks `<= 120000`.
  - `external_refs` → rule checks all `$ref` values; if any start with `http://`, `https://`, or are absolute paths, emit error.
- Structural rules are registered in the same `linkme` registry as Class A rules, but generated from profile data rather than keyword map.
- Error code prefix: `OAI-S-*` (e.g., `OAI-S-object-root`, `OAI-S-max-depth`).

**Patterns to follow:**
- Data-driven rules: changing a limit requires only editing TOML, no code changes.
- Same `Rule` trait as Class A for uniform registry handling.

**Test scenarios:**
- **Happy path:** Schema with root object, `additionalProperties: false`, all properties required, depth <= 10 → no structural diagnostics.
- **Edge case:** Root is `type: string` (not object) → `OAI-S-object-root` error.
- **Edge case:** Object with `additionalProperties: true` → `OAI-S-additional-properties-false` error.
- **Edge case:** Object with `properties: {a: ..., b: ...}` but `required: ["a"]` only → `OAI-S-all-properties-required` error.
- **Edge case:** Nested schema with depth = 11 → `OAI-S-max-depth` error.
- **Edge case:** Schema with 5001 total properties across all objects → `OAI-S-max-total-properties` error.
- **Edge case:** Schema with 1001 enum values → `OAI-S-max-enum-values` error.
- **Edge case:** Schema with external `$ref: "https://example.com/schema.json"` → `OAI-S-external-refs` error.
- **Integration:** Structural rules + Class A rules run together on same schema → both diagnostic sets emitted, no deduplication.

**Verification:**
- All structural tests pass.
- `cargo test --test structural_tests` passes.
- Structural rule generation verified: editing TOML limit changes rule behavior without code changes.

---

- U7. **CLI: Argument Parsing, File Discovery, and Output Formatting**

**Goal:** Build the `schemalint check` command that ties all components together: discovers files, loads profile, normalizes schemas, runs rules, and emits human or JSON output.

**Requirements:** R1 (CLI runs end-to-end), R2 (human output), R3 (JSON output), R6–R8 (performance targets), R10 (snapshot testing for output)

**Dependencies:** U5 (needs rules), U6 (needs structural rules)

**Files:**
- Create: `crates/schemalint/src/cli/mod.rs`
- Create: `crates/schemalint/src/cli/args.rs`
- Create: `crates/schemalint/src/cli/discover.rs`
- Create: `crates/schemalint/src/cli/emit_human.rs`
- Create: `crates/schemalint/src/cli/emit_json.rs`
- Modify: `crates/schemalint/src/main.rs`
- Modify: `crates/schemalint/src/cache.rs`
- Test: `crates/schemalint/tests/cli_tests.rs`
- Test: `crates/schemalint/tests/snapshot_tests.rs`

**Approach:**
- `args.rs`: `clap` derive macro. `schemalint check --profile <path> [--format human|json] <schema-path>...`. Accepts multiple files and directories.
- `discover.rs`: For each path: if file and ends with `.json`, add to list; if directory, recursively scan for `.json` files (no symlink follow). Sort for deterministic order.
- `cache.rs`: In-memory `HashMap<[u8; 32], NormalizedSchema>` keyed by content hash. Cleared between CLI invocations (Phase 1).
- Parallel processing: Use `rayon` for parallel schema processing across files. Profile is read-only and shared across threads. Normalization and rule evaluation per schema are independent.
- `emit_human.rs`: rustc-style formatter. Format per diagnostic:
  ```
  error[OAI-K-allOf]: keyword 'allOf' is not supported by OpenAI Structured Outputs
     --> schema.json
       |
       = profile: openai.so.2026-04-30
       = schema path: /properties/items
       = see: https://schemalint.dev/rules/OAI-K-allOf
  ```
  For raw JSON, omit line:col, code snippet, and caret (source spans unavailable). Show file path only.
  Footer: `N issues found (E errors, W warnings) across F schemas in Dms`. (Strip diagnostics counted as errors in summary).
- `emit_json.rs`: Structured JSON matching origin §6 exactly. `schema_version: "1.0"`, `tool: { name, version }`, `profiles`, `summary`, `diagnostics`.
- Exit code: 0 if no `Error` severity diagnostics; 1 if any `Error` or fatal parse error; `Warning` alone exits 0. `Strip` severity is treated as `Error` for exit code purposes in Phase 1.
- `--format`: default to `human` if stdout is TTY, `json` otherwise (or explicit `--format`).

**Execution note:** Start with a failing integration test for the request/response contract: given a schema file and profile, invoke CLI, capture stdout/stderr, verify exit code.

**Patterns to follow:**
- Standard Rust CLI patterns: `clap` for args, `std::process::exit` for codes, `println!`/`eprintln!` for output.
- Snapshot testing with `insta` crate for output format stability.

**Test scenarios:**
- **Happy path:** Single schema, single profile, human format → deterministic output, exit code 0 if clean, 1 if errors.
- **Happy path:** Single schema, JSON format → valid JSON matching schema, deterministic.
- **Happy path:** Directory with 3 schemas → all discovered, all linted, aggregated summary.
- **Happy path:** Multiple explicit files → linted in order, aggregated summary.
- **Edge case:** Empty directory → `0 issues found` summary, exit code 0.
- **Edge case:** Directory with no `.json` files → `0 issues found` summary, exit code 0.
- **Edge case:** Missing schema file → fatal error, exit code 1.
- **Edge case:** Missing profile file → fatal error, exit code 1.
- **Edge case:** Invalid JSON in schema file → fatal error, exit code 1.
- **Edge case:** Invalid TOML in profile file → fatal error, exit code 1.
- **Edge case:** Schema with no diagnostics → `0 issues found`, exit code 0.
- **Edge case:** Schema with warnings only → `N warnings found`, exit code 0.
- **Edge case:** Very long JSON Pointer (> terminal width) → no truncation in Phase 1, raw pointer emitted.
- **Edge case:** Unicode in schema property names → correctly escaped in JSON output, correctly displayed in human output.
- **Integration:** Run CLI on 500-schema project → completes in < 500 ms cold start.
- **Integration:** Run CLI twice on same 500-schema project (in-memory cache) → second run < 5 ms.
- **Snapshot:** Human output for `schema_with_allof.json` matches stored snapshot.
- **Snapshot:** JSON output for `schema_with_allof.json` matches stored snapshot.

**Verification:**
- All CLI tests pass.
- All snapshot tests pass.
- `cargo test --test cli_tests` passes.
- `cargo test --test snapshot_tests` passes.
- Manual benchmark: 500-schema project cold start < 500 ms.
- Manual benchmark: incremental run < 5 ms.

---

- U8. **Regression Corpus Curation**

**Goal:** Assemble 50 real-world JSON Schemas with deterministic expected diagnostic sets, sourced from public bug reports, OpenAI Community forum, Pydantic AI issues, and SDK forums.

**Requirements:** R5 (all 50 produce expected diagnostics)

**Dependencies:** U7 (needs working CLI to validate corpus)

**Files:**
- Create: `crates/schemalint/tests/corpus/README.md`
- Create: `crates/schemalint/tests/corpus/schema_*.json` (50 files)
- Create: `crates/schemalint/tests/corpus/schema_*.expected` (50 files)
- Test: `crates/schemalint/tests/corpus_tests.rs`

**Approach:**
- Each corpus entry: one `.json` schema file + one `.expected` file containing the expected diagnostic set (same format as JSON output `diagnostics` array).
- Schema sources: public OpenAI Community posts about structured output failures, Pydantic AI GitHub issues with schema problems, SDK forum threads, Stack Overflow questions about JSON Schema + OpenAI.
- Schema variety: simple objects, nested objects, arrays, unions (`anyOf`), enums, refs, cyclic refs, edge cases from origin §7 keyword table.
- Expected diagnostics: hand-curated by running the CLI and verifying each diagnostic is correct against the OpenAI docs.
- `corpus_tests.rs`: For each schema in `tests/corpus/`, run CLI with OpenAI profile, compare JSON output diagnostics to `.expected` file. Test fails on mismatch.

**Patterns to follow:**
- Corpus schemas are static files, not generated.
- `.expected` files are the source of truth; updating them requires explicit human review.

**Test scenarios:**
- **Happy path:** All 50 corpus schemas produce output matching their `.expected` files.
- **Edge case:** New schema added to corpus without `.expected` → test fails with clear message.
- **Integration:** Adding a new rule (in future phases) does not silently change corpus output — `.expected` files must be explicitly updated.

**Verification:**
- `cargo test --test corpus_tests` passes with all 50 schemas.
- Corpus README documents sourcing and curation methodology.

---

- U9. **Integration Tests, Snapshot Tests, and Property Tests**

**Goal:** Validate end-to-end behavior, output format stability, and normalizer correctness through multiple test strategies.

**Requirements:** R9 (positive/negative tests per rule), R10 (snapshot tests), R11 (property tests)

**Dependencies:** U7, U8

**Files:**
- Create: `crates/schemalint/tests/integration_tests.rs`
- Create: `crates/schemalint/tests/property_tests.rs`

**Approach:**
- **Integration tests:** Spawn CLI as child process, feed schema files and profiles, capture stdout/stderr/exit code. Assert on output structure and content.
- **Snapshot tests:** Use `insta` crate. For a set of representative schemas (subset of corpus + synthetic edge cases), capture human and JSON output. Compare to stored snapshots. Fail on unexpected changes.
- **Property tests:** Use `proptest` or `quickcheck`. Generate random valid JSON Schemas (constrained to OpenAI-accepted subset), parse into IR, verify:
  - All keywords from input are present in IR (round-trip completeness).
  - Normalization is idempotent: normalizing an already-normalized schema produces identical IR.
  - Content-hash cache hit: parsing same bytes twice returns identical `NormalizedSchema`.

**Patterns to follow:**
- `insta` for snapshot testing in Rust ecosystem.
- `proptest` for property-based testing.

**Test scenarios:**
- **Integration — Happy path:** CLI on valid schema → exit 0, empty diagnostics.
- **Integration — Error path:** CLI on schema with forbidden keywords → exit 1, diagnostics present.
- **Integration — Batch:** CLI on directory → aggregated summary with correct counts.
- **Snapshot — Human format:** Output matches stored snapshot for `allof_schema.json`.
- **Snapshot — JSON format:** Output matches stored snapshot for `allof_schema.json`.
- **Property — Round-trip:** Random schema retains all keywords after parse.
- **Property — Idempotency:** normalize(normalize(schema)) == normalize(schema).
- **Property — Cache hit:** Same bytes → cache hit, identical result.

**Verification:**
- `cargo test --test integration_tests` passes.
- `cargo test --test snapshot_tests` passes.
- `cargo test --test property_tests` passes.

---

- U10. **Performance Benchmark Harness**

**Goal:** Establish Criterion benchmarks to measure and enforce the performance targets.

**Requirements:** R6 (< 1 ms single schema), R7 (< 500 ms cold start, 500 schemas), R8 (< 5 ms incremental)

**Dependencies:** U7 (needs complete pipeline to benchmark)

**Files:**
- Create: `benches/schemalint_benchmarks.rs`
- Create: `benches/fixtures/single_large_schema.json`
- Create: `benches/fixtures/project_500_schemas/` (500 generated schemas)

**Approach:**
- **Single schema benchmark:** Parse and lint a 200-property nested schema, single profile. Target: < 1 ms.
- **Cold start benchmark:** Parse and lint 500 schemas, 1 profile, no cache. Target: < 500 ms.
- **Incremental benchmark:** Parse and lint 500 schemas, 1 profile, with in-memory cache (simulate single-file edit by changing one schema). Target: < 5 ms.
- Use `criterion` for statistical rigor. Generate benchmark fixtures programmatically (synthetic schemas with controlled complexity).
- Fail CI if benchmarks regress beyond targets.

**Patterns to follow:**
- Criterion benchmark groups: `single_schema`, `cold_start`, `incremental`.
- Synthetic fixtures: generate schemas with N properties, M nesting depth, K refs.

**Test scenarios:**
- **Happy path:** Single schema benchmark reports mean < 1 ms.
- **Happy path:** Cold start benchmark reports mean < 500 ms.
- **Happy path:** Incremental benchmark reports mean < 5 ms.
- **Edge case:** Benchmark with 5000-schema monorepo → reports mean < 5 s (cross-phase constraint from phases.md).

**Verification:**
- `cargo bench` runs successfully.
- Benchmark results meet targets on reference hardware (document hardware specs in README).
- CI workflow runs benchmarks and fails on regression.

---

## System-Wide Impact

- **Interaction graph:** The normalizer pipeline (`normalize/mod.rs`) is the central bottleneck. Changes to normalizer ordering or IR shape affect rules, cache keys, and output. The rule registry (`rules/registry.rs`) is a global singleton via `linkme`; rules are discovered at link time, not runtime.
- **Error propagation:** Fatal errors (invalid JSON, missing profile, unresolved `$ref`) propagate via `Result<T, Error>` up to `main.rs`, which prints to stderr and exits with code 1. Diagnostics (rule violations) are collected in a `Vec<Diagnostic>` and emitted after all rules run — they do not short-circuit execution.
- **State lifecycle risks:** Arena `Vec<Node>` grows monotonically per schema. No partial-write risk because normalization is atomic: either the entire schema normalizes successfully, or it fails before any rules run. In-memory cache holds `NormalizedSchema` structs with owned arena data — no reference lifetime issues.
- **API surface parity:** This plan modifies only the `schemalint` crate internal structure. The `schemalint-profiles` crate is data-only and has no behavioral API. No external consumers exist yet.
- **Integration coverage:** Cross-layer scenarios that unit tests alone will not prove:
  - CLI → discover → normalize → rules → emit_human (full human output pipeline)
  - CLI → discover → normalize → rules → emit_json (full JSON output pipeline)
  - Cache hit path: CLI → discover → cache hit → skip normalize → rules (performance path)
  - Batch aggregation: 500 schemas → single summary with correct counts
- **Unchanged invariants:**
  - `serde_json` parsing behavior (duplicate keys = last wins) is preserved, not modified.
  - `linkme` distributed slice registration mechanism is used as-is; no custom registration logic.
  - TOML parsing semantics from `toml` crate are preserved.

---

## Risks & Dependencies

| Risk | Mitigation |
|------|------------|
| **Performance targets not met** | Arena allocation and lazy JSON Pointer construction are the baseline. If targets are missed, profile with `cargo flamegraph` and optimize hot paths (string allocation, hash lookups). Benchmark early (U10) and often. |
| **`linkme` fails on target platform** | `linkme` is chosen for cross-platform reliability, but exotic targets (WASM, musl static) may have issues. CI matrix tests Linux/macOS/Windows. If `linkme` fails, fallback to `inventory` or manual registry. |
| **OpenAI docs drift** | Profile is dated (`2026-04-30`). Docs may change. Mitigation: profile TOML is versioned; updating it is a data change, not code change. Future phases include live-API burn windows (SOW §5.5). |
| **Corpus curation takes longer than expected** | Sourcing 50 real-world schemas with known issues is research-heavy. Mitigation: start corpus curation immediately in parallel with engine development. Use synthetic schemas to unblock rule testing while real schemas are sourced. |
| **Unresolved `$ref` handling too strict** | Fatal error on unresolved internal `$ref` may reject schemas that are technically valid JSON Schema (external refs are common). Mitigation: this is Phase 1 behavior. Phase 2 may add external ref resolution or downgrade to warning. Document the behavior. |
| **Snapshot test fragility** | Output format changes (e.g., adding a field) break all snapshots. Mitigation: use `insta`'s pending snapshot review workflow. Snapshot tests are a safety net, not the primary correctness mechanism — corpus tests verify semantics. |

---

## Documentation / Operational Notes

- `README.md`: Project overview, installation (`cargo install`), basic usage, performance targets.
- `AGENTS.md`: Agent-specific instructions for future AI-assisted development (build steps, test commands, conventions).
- `crates/schemalint/tests/corpus/README.md`: Corpus sourcing methodology, schema provenance, expected diagnostic curation process.
- Rule reference URLs (`https://schemalint.dev/rules/OAI-K-*`) are placeholders. Actual documentation site is Phase 5 deliverable. For Phase 1, URLs may 404 — acceptable.

---

## Sources & References

- **Origin document:** [docs/brainstorms/phase1-requirements.md](docs/brainstorms/phase1-requirements.md)
- **SOW:** [docs/sow.md](docs/sow.md) (architecture, IR design, performance targets, distribution plan)
- **Phases:** [docs/phases.md](docs/phases.md) (engineering phase breakdown, cross-phase constraints)
- **Ideation:** [docs/ideation/schemalint-improvements-2026-04-30.md](docs/ideation/schemalint-improvements-2026-04-30.md) (strategic ideas, ranked survivors)
- External docs: OpenAI Structured Outputs (`developers.openai.com/api/docs/guides/structured-outputs`), scraped 2026-04-30
