---
title: Phase 2 — Rules, Multi-Profile, Output Formats, and Server Mode
type: feat
status: active
date: 2026-04-30
origin: docs/brainstorms/phase2-requirements.md
---

# Phase 2 — Rules, Multi-Profile, Output Formats, and Server Mode

## Summary

Extend the existing OpenAI-only diff tool into a multi-provider rule engine. Deliver an Anthropic profile, four new hand-written semantic rules, multi-profile CLI composition, three new CI output formats, a JSON-RPC server mode with persistent disk cache, and an Anthropic-specific regression corpus.

---

## Problem Frame

Phase 1 shipped a single-profile linter for OpenAI Structured Outputs. Engineers building schemas for Anthropic Claude hit the same runtime failures — silently stripped keywords, cryptic 400 errors — but Anthropic accepts a different JSON Schema subset. No static tool checks against both providers simultaneously. Phase 2 closes this gap.

---

## Requirements

- R1–R5. Multi-profile composition (multiple `--profile` args, independent rule sets per profile, profile-tagged diagnostics, exit-code semantics, built-in ID resolution).
- R6–R8. Anthropic Structured Outputs profile with doc-grounded keyword map and structural rules.
- R9–R10. OpenAI profile corrections (`max_object_depth` 20→10, `oneOf` warn→unknown).
- R11–R14. Four new Class B semantic rules (empty object, `additionalProperties` object value, `anyOf` over objects, `allOf` with `$ref`).
- R15–R17. SARIF v2.1.0, GitHub Actions annotation, and JUnit XML output formats.
- R18–R21. JSON-RPC server mode over stdin/stdout with `check`/`shutdown` methods and persistent disk cache.
- R22–R23. 25-schema Anthropic regression corpus with deterministic expected diagnostics.
- R24–R26. Performance targets (single schema 3 profiles <1ms, 500 schemas cold <500ms, incremental <5ms).

---

## Scope Boundaries

- **In scope:** Everything listed in Requirements above.
- **Deferred for later:** Pydantic/Zod ingestion (Phase 3–4), auto-fix (out of scope for v1 per SOW §3.2), packaging/distribution (Phase 5), live-API conformance harness (Phase 5).
- **Outside this product's identity:** Generic JSON Schema linter; schema auto-fix or code generation.

---

## Context & Research

### Relevant Code and Patterns

- **Arena IR:** `crates/schemalint/src/ir/arena.rs` — `NodeId(u32)`, `Arena(Vec<Node>)`, `Annotations` with 44 keyword fields, `NodeKind` enum.
- **Normalizer pipeline:** `normalize/mod.rs` → `dialect.rs` → `traverse.rs` (DFS expansion) → `refs.rs` (resolution + Tarjan SCC) → `desugar.rs` (type-array → `AnyOf`).
- **Rule trait:** `rules/registry.rs` — `check(&self, node: NodeId, arena: &Arena, profile: &Profile) -> Vec<Diagnostic>`.
- **RuleSet:** Combines static `linkme::distributed_slice![RULES]` + dynamic rules from `generate_class_b_rules(profile)`.
- **Class A:** Auto-generated in `rules/class_a.rs` from profile `keyword_map` and `restrictions`.
- **Class B:** Hand-written in `rules/class_b.rs`, generated from profile `[structural]` section.
- **CLI:** `cli/mod.rs` runs `run_check()` — loads profile → discovers files → parallel `rayon` map → normalizes + caches → checks → aggregates → emits.
- **Cache:** `cache.rs` — in-memory `HashMap<u64, NormalizedSchema>` keyed by `FxHasher` content hash.
- **Profile parser:** `profile/parser.rs` — manual TOML walk, `Box::leak` for `&'static str` keys, `toml_to_json` conversion.
- **Output emitters:** `cli/emit_human.rs` (rustc-style), `cli/emit_json.rs` (structured JSON).
- **Tests:** 9 integration test files covering CLI, IR, normalizer, profile, rules, structural, snapshots, corpus, property tests.
- **schemalint-profiles crate:** Zero deps; `include_str!` re-exports bundled TOML as `&str` constants.

### External References

- OpenAI Structured Outputs docs (platform.openai.com/api/docs/guides/structured-outputs) — scraped 2026-04-30.
- Anthropic Structured Outputs docs (docs.anthropic.com/en/docs/build-with-claude/structured-outputs) — scraped 2026-04-30.
- Anthropic Python SDK v0.97.0 `transform_schema` source — strips unsupported constraints, forces `additionalProperties: false`.
- OpenAI Python SDK v2.33.0 `to_strict_json_schema` source — preserves most constraints, forces `additionalProperties: false`, flattens single-branch `allOf`.
- SARIF v2.1.0 spec (OASIS standard) — `sarif-schema-2.1.0.json`.
- GitHub Actions workflow commands docs — `::error file=...,line=...,col=...::message`.
- JSON-RPC 2.0 spec — simple request/response over stdin/stdout.

---

## Key Technical Decisions

| Decision | Rationale |
|---|---|
| **Profile `code_prefix` field in TOML** | Rule codes currently hardcode `OAI-` prefix. Adding `code_prefix = "ANT"` to the Anthropic profile makes the rule registry provider-agnostic without string parsing. |
| **Static registration for universal semantic rules** | `EmptyObjectRule`, `AdditionalPropertiesObjectRule`, `AnyOfObjectsHint` apply to all providers. Registering them via `linkme` distributed slice keeps them separate from data-driven structural rules and avoids per-profile generation boilerplate. The `check` method receives `&Profile`, so each rule can use `profile.code_prefix` for its diagnostic code. |
| **Profile-generated `AllOfWithRefRule`** | This rule is Anthropic-specific. Including it in `generate_class_b_rules` keeps it with other profile-driven rules and avoids a runtime branch inside a hot `check` loop. It is only generated when the profile has a specific flag or when `code_prefix == "ANT"`. |
| **Independent rule sets per profile** | Simpler than merging profiles. The CLI builds one `RuleSet` per loaded profile, runs all against each schema, and tags diagnostics with `profile.name`. Most-restrictive severity wins implicitly because both rule sets run. No merge logic needed. |
| **Manual SARIF/GHA/JUnit emission** | The existing codebase already emits JSON manually. SARIF is JSON with a defined schema; GHA is simple string formatting; JUnit XML is straightforward. Adding crates (`serde_sarif`, `junit-report`) adds dependency weight for formats that are simple to emit directly. |
| **Custom JSON-RPC parser (no external crate)** | Only two methods (`check`, `shutdown`) over stdin/stdout. A custom line-delimited JSON parser is ~100 lines and avoids pulling in `jsonrpc-core` or `tower-lsp` with their async runtimes. |
| **Disk cache via `bincode` + individual files** | `NormalizedSchema` derives `Serialize`/`Deserialize`. Each cache entry is a separate file `~/.cache/schemalint/<hash>.bin` for simple eviction (delete oldest files). `bincode` is compact and fast. |
| **Server mode does not watch files** | Matches Phase 2 constraint from `docs/phases.md`: "only the emitter changes." File watching is client responsibility. |

---

## Open Questions

### Resolved During Planning

- **How to handle provider-specific rule codes?** → Add `code_prefix` to `Profile` struct and TOML. Rules use `profile.code_prefix` at runtime.
- **Where do new semantic rules live?** → Universal rules (`EmptyObjectRule`, `AdditionalPropertiesObjectRule`, `AnyOfObjectsHint`) go in static `RULES` slice. Anthropic-specific `AllOfWithRefRule` goes in `generate_class_b_rules`.
- **Cache serialization approach?** → Derive `Serialize`/`Deserialize` for `NormalizedSchema`, `Arena`, `Node`, etc. Use `bincode` for compact disk storage.
- **JSON-RPC server architecture?** → Blocking read-loop on `stdin` (line-delimited), spawn blocking thread pool for `check` requests, write responses to `stdout`. Simple and matches the existing synchronous engine.
- **Cache versioning / schema migration** → Add a 4-byte version header to every cache file. Version = 1 for Phase 2. On read, version mismatch → treat as miss and overwrite.
- **`code_prefix` backward compatibility** → Make `code_prefix` optional in TOML. Default derived from profile name: uppercase first segment before first dot. `openai.so.2026-04-30` → `OAI`, `custom-profile.toml` → `CUSTOM-PROFILE`.
- **Anthropic `require_object_root` and `require_all_properties_in_required`** → Set both to `false` in the Anthropic profile TOML. Anthropic docs do not mention these constraints.
- **Profile resolution on Windows** → Use `Path::new(input).extension().is_some()` as primary heuristic (built-in IDs lack `.toml` extension). Fallback to `Path::is_file()`.
- **Server mode request timeout / size limits** → Cap request payload at 10 MB. Cap `check` execution at 30 seconds. Return JSON-RPC error on exceeded limits.
- **Corpus coverage gaps** → Expand Anthropic corpus from 25 to 30 schemas. Add: `anyOf` mixed types (1), `allOf` without `$ref` (1), `enum` mixed types (1), internal `$ref` (1), `default` on various types (1). Remove 2 redundant schemas from original 25 to stay at 30 total.

### Deferred to Implementation

- **Exact SARIF property bag shapes** for schema-path attribution. Will be determined by SARIF schema compliance during implementation.
- **JUnit XML root `<testsuites>` vs `<testsuite>` wrapper** — depends on whether we model each schema as a test case or each diagnostic as a test case. Decide during implementation based on what CI systems (GitLab, Jenkins, CircleCI) expect.
- **Cache eviction strategy** — simple size-based or count-based. Requirements don't specify; implement the simplest approach (max 1000 entries, LRU by file mtime) during implementation.

---

## Output Structure

No new directory hierarchy is created. All new files fit into existing module structure:

```
crates/schemalint/
  src/
    cli/
      server.rs          (new)
      emit_sarif.rs      (new)
      emit_gha.rs        (new)
      emit_junit.rs      (new)
    rules/
      semantic.rs        (new — static semantic rules)
  tests/
    corpus/
      ant_schema_*.json  (25 new)
      ant_schema_*.expected  (25 new)
crates/schemalint-profiles/
  profiles/
    anthropic.so.2026-04-30.toml  (new)
```

---

## Implementation Units

- U1. **[Profile data: OpenAI corrections and Anthropic profile]**

**Goal:** Correct the existing OpenAI profile per live docs, create the Anthropic profile, and add `code_prefix` support to the profile parser.

**Requirements:** R6–R10

**Dependencies:** None

**Files:**
- Modify: `crates/schemalint-profiles/profiles/openai.so.2026-04-30.toml`
- Create: `crates/schemalint-profiles/profiles/anthropic.so.2026-04-30.toml`
- Modify: `crates/schemalint-profiles/src/lib.rs`
- Modify: `crates/schemalint/src/profile/parser.rs`
- Modify: `crates/schemalint/src/profile/mod.rs`
- Test: `crates/schemalint/tests/profile_tests.rs`

**Approach:**
1. In OpenAI TOML: change `max_object_depth` from `20` to `10`; change `oneOf` from `warn` to `unknown`.
2. Create Anthropic TOML with all keyword mappings per requirements doc R7 and structural section per R8. Set `require_object_root = false` and `require_all_properties_in_required = false` (Anthropic docs do not mention these constraints).
3. Add optional `code_prefix` field to profile TOML. Default: uppercase first segment of profile name before first dot. Explicit values: `code_prefix = "OAI"` for OpenAI, `code_prefix = "ANT"` for Anthropic.
4. Update `Profile` struct to include `pub code_prefix: String`.
5. Update `schemalint-profiles/src/lib.rs` to export `ANTHROPIC_SO_2026_04_30` constant.
6. Update `profile_tests.rs` to load and validate both built-in profiles.

**Patterns to follow:**
- Existing `schemalint-profiles/src/lib.rs` pattern: one `pub const` per bundled profile.
- Existing `profile/parser.rs` pattern: manual TOML field extraction with `ProfileError` variants.

**Test scenarios:**
- Happy path: Load `openai.so.2026-04-30.toml` → `code_prefix == "OAI"`, `max_object_depth == 10`, `oneOf == Unknown`.
- Happy path: Load `anthropic.so.2026-04-30.toml` → `code_prefix == "ANT"`, `minimum == Forbid`, `allOf == Allow`, `minItems` restriction is `[0, 1]`, `require_object_root == false`.
- Edge case: Profile without explicit `code_prefix` → derives default from name (e.g., `custom-profile.toml` → `CUSTOM-PROFILE`).
- Edge case: Profile with invalid `code_prefix` (non-alphanumeric) → accept but sanitize; no strong validation needed.

**Verification:**
- `cargo test --test profile_tests` passes.
- Both bundled profiles load successfully with zero parse errors.
- `schemalint check --profile openai.so.2026-04-30 <schema>` still works (backward compat).

---

- U2. **[Multi-profile CLI args and built-in profile resolution]**

**Goal:** Enable multiple `--profile` arguments and resolve built-in profile IDs without filesystem paths.

**Requirements:** R1, R5

**Dependencies:** U1

**Files:**
- Modify: `crates/schemalint/src/cli/args.rs`
- Modify: `crates/schemalint/src/cli/mod.rs`
- Test: `crates/schemalint/tests/cli_tests.rs`

**Approach:**
1. In `args.rs`: change `profile: PathBuf` to `profiles: Vec<PathBuf>` with `#[arg(short, long)]` (clap automatically collects repeated flags into a Vec).
2. Add `Server` subcommand variant to `Commands` enum with `ServerArgs` (initially empty; populated in U7).
3. In `cli/mod.rs`: add `resolve_profile(path_or_id: &str) -> Result<Vec<u8>, String>` function.
   - If input contains a path separator (`/` or `\`), treat as filesystem path → `fs::read`.
   - If no path separator, match against built-in IDs:
     - `openai.so.2026-04-30` → `schemalint_profiles::OPENAI_SO_2026_04_30.as_bytes()`
     - `anthropic.so.2026-04-30` → `schemalint_profiles::ANTHROPIC_SO_2026_04_30.as_bytes()`
   - Unknown built-in ID → error.
4. Update `run_check` to accept `Vec<Profile>` instead of single `Profile`.

**Patterns to follow:**
- Existing `fs::read(&args.profile)` pattern in `cli/mod.rs`.
- Existing clap derive API for argument parsing.

**Test scenarios:**
- Happy path: `schemalint check --profile openai.so.2026-04-30 schema.json` resolves built-in.
- Happy path: `schemalint check --profile /path/to/custom.toml schema.json` loads from filesystem.
- Happy path: `schemalint check --profile openai.so.2026-04-30 --profile anthropic.so.2026-04-30 schema.json` loads both.
- Error path: `schemalint check --profile unknown-profile schema.json` → error message.
- Error path: `schemalint check --profile /nonexistent.toml schema.json` → error message.

**Verification:**
- `cargo test --test cli_tests` passes with new multi-profile tests.
- `cargo test --test integration_tests` passes.

---

- U3. **[Multi-profile engine orchestration]**

**Goal:** Run independent rule sets per active profile and aggregate profile-tagged diagnostics.

**Requirements:** R2–R4

**Dependencies:** U1, U2

**Files:**
- Modify: `crates/schemalint/src/cli/mod.rs`
- Modify: `crates/schemalint/src/rules/registry.rs`
- Test: `crates/schemalint/tests/integration_tests.rs`
- Test: `crates/schemalint/tests/structural_tests.rs`

**Approach:**
1. In `cli/mod.rs` `run_check`:
   - Load all profiles into `Vec<Profile>`.
   - Build one `RuleSet` per profile: `let rulesets: Vec<(String, RuleSet)> = profiles.iter().map(|p| (p.name.clone(), RuleSet::from_profile(p))).collect();`
   - For each schema file, normalization is done once (cache is shared across profiles).
   - For each profile/ruleset, run `ruleset.check_all(&normalized.arena, profile)`.
   - Collect all diagnostics into `Vec<(PathBuf, Vec<Diagnostic>)>` where diagnostics are already tagged with their profile name.
2. In `rules/registry.rs`:
   - Ensure `RuleSet::from_profile` uses the profile's `code_prefix` when generating Class A/B rule codes.
   - Class A rules currently hardcode `"OAI-K-{keyword}"` — change to `"{prefix}-K-{keyword}"`.
   - Class B rules currently hardcode `"OAI-S-..."` — change to `"{prefix}-S-..."`.
3. Aggregation: sort by (path, profile) for deterministic output.
4. Exit code: `1` if any diagnostic has `severity == Error` across any profile; `0` otherwise (warnings are OK).

**Patterns to follow:**
- Existing `rayon` parallel map for file processing.
- Existing `cache.lock().unwrap()` pattern for cross-thread cache sharing.

**Test scenarios:**
- Happy path: Single profile → same behavior as Phase 1 (backward compat).
- Happy path: Two profiles, schema clean for both → exit 0, 0 issues.
- Happy path: Two profiles, schema has OpenAI error but Anthropic clean → exit 1, diagnostics tagged with OpenAI profile only.
- Happy path: Two profiles, schema has errors for both → exit 1, diagnostics tagged with both profiles.
- Happy path: Two profiles, schema has warnings for both → exit 0, warnings displayed.
- Edge case: Three profiles on 500 schemas → performance < 500ms (measured via existing benchmark).

**Verification:**
- `cargo test --test integration_tests` passes with multi-profile tests.
- `cargo test --test structural_tests` passes (backward compat).
- `cargo bench --no-run` compiles.

---

- U4. **[New Class B semantic rules]**

**Goal:** Implement four hand-written semantic rules with provider-aware codes.

**Requirements:** R11–R14

**Dependencies:** U1

**Files:**
- Create: `crates/schemalint/src/rules/semantic.rs`
- Modify: `crates/schemalint/src/rules/mod.rs`
- Modify: `crates/schemalint/src/rules/class_b.rs`
- Modify: `crates/schemalint/src/rules/registry.rs`
- Test: `crates/schemalint/tests/rules_tests.rs`
- Test: `crates/schemalint/tests/structural_tests.rs`

**Approach:**
1. Create `rules/semantic.rs` for universal static semantic rules.
2. Implement `EmptyObjectRule`:
   - Checks object schemas where `additionalProperties` is `false` and `properties` is missing or empty.
   - Severity: `Warning`.
   - Code: `{prefix}-S-empty-object`.
3. Implement `AdditionalPropertiesObjectRule`:
   - Checks object schemas where `additionalProperties` is an object value (e.g., `{}`) instead of `false`.
   - Severity: `Error`.
   - Code: `{prefix}-S-additional-properties-object`.
4. Implement `AnyOfObjectsHint`:
   - Checks `anyOf` schemas where all branches are object-typed.
   - Severity: `Warning`.
   - Code: `{prefix}-S-anyof-objects`.
   - Hint: "Consider merging object branches into a single object schema for better provider compatibility."
5. Register the three universal rules in the static `RULES` distributed slice in `registry.rs`.
6. Implement `AllOfWithRefRule` in `class_b.rs` (profile-generated):
   - Only generated when `profile.code_prefix == "ANT"`.
   - Checks `allOf` schemas where any branch contains `$ref`.
   - Severity: `Error`.
   - Code: `ANT-S-allof-with-ref`.
   - Message: "Anthropic Structured Outputs does not support allOf combined with $ref".

**Patterns to follow:**
- Existing `class_b.rs` pattern: `struct RuleName { profile_name: String }`, `impl Rule for ...`.
- Existing `schema_is_object` helper in `class_b.rs`.
- Static registration pattern from `registry.rs`: `#[distributed_slice(RULES)] static RULE_NAME: &'static dyn Rule = &RuleName;` (wait, check the exact pattern).

Actually, looking at the existing `registry.rs`:
```rust
#[linkme::distributed_slice]
pub static RULES: [&'static dyn Rule];
```

Rules are registered as:
```rust
#[distributed_slice(RULES)]
static MY_RULE: &'static dyn Rule = &MyRule;
```

But the new semantic rules need access to `profile.code_prefix`. Since they're static, they receive `&Profile` in `check()`. They can construct the code dynamically.

Wait, but if the code is dynamic based on profile, the static rule just does:
```rust
fn check(&self, node: NodeId, arena: &Arena, profile: &Profile) -> Vec<Diagnostic> {
    // ...
    Diagnostic {
        code: format!("{}-S-empty-object", profile.code_prefix),
        // ...
    }
}
```

This works because `code` is a `String`, not `&'static str`.

For `AllOfWithRefRule`, since it's generated per-profile, it can store the prefix directly:
```rust
struct AllOfWithRefRule {
    profile_name: String,
    code_prefix: String,
}
```

**Test scenarios:**
- `EmptyObjectRule`:
  - Happy path (fires): `{ "type": "object", "additionalProperties": false }` → warning.
  - Happy path (fires): `{ "type": "object", "properties": {}, "additionalProperties": false }` → warning.
  - Negative: `{ "type": "object", "properties": { "x": {} }, "additionalProperties": false }` → no warning.
  - Negative: `{ "type": "object" }` → no warning (additionalProperties not false).
- `AdditionalPropertiesObjectRule`:
  - Happy path (fires): `{ "type": "object", "additionalProperties": {} }` → error.
  - Negative: `{ "type": "object", "additionalProperties": false }` → no error.
  - Negative: `{ "type": "object", "additionalProperties": true }` → no error (different keyword state handles this).
- `AnyOfObjectsHint`:
  - Happy path (fires): `{ "anyOf": [{ "type": "object" }, { "type": "object" }] }` → warning.
  - Negative: `{ "anyOf": [{ "type": "string" }, { "type": "object" }] }` → no warning.
  - Negative: `{ "anyOf": [] }` → no warning.
- `AllOfWithRefRule`:
  - Happy path (fires): `{ "allOf": [{ "$ref": "#/$defs/X" }] }` with Anthropic profile → error.
  - Negative: Same schema with OpenAI profile → no error (rule not generated).
  - Negative: `{ "allOf": [{ "type": "string" }] }` with Anthropic profile → no error (no $ref).

**Verification:**
- `cargo test --test rules_tests` passes with new rule tests.
- `cargo test --test structural_tests` passes.

---

- U5. **[New output formats: SARIF, GHA, JUnit]**

**Goal:** Implement three new CI-friendly output formatters.

**Requirements:** R15–R17

**Dependencies:** U3

**Files:**
- Create: `crates/schemalint/src/cli/emit_sarif.rs`
- Create: `crates/schemalint/src/cli/emit_gha.rs`
- Create: `crates/schemalint/src/cli/emit_junit.rs`
- Modify: `crates/schemalint/src/cli/args.rs`
- Modify: `crates/schemalint/src/cli/mod.rs`
- Test: `crates/schemalint/tests/snapshot_tests.rs`

**Approach:**
1. In `args.rs`: extend `OutputFormat` enum with `Sarif`, `Gha`, `JUnit` variants.
2. In `cli/mod.rs`: extend the `format` match to dispatch to new emitters.
3. `emit_sarif.rs`:
   - Build SARIF JSON manually using `serde_json::Value` or structs.
   - Top-level: `$schema`, `version`, `runs[0].tool.driver.name = "schemalint"`.
   - Per diagnostic: `results[]` with `ruleId` (code), `message.text`, `locations[0].physicalLocation.artifactLocation.uri` (file path), `locations[0].physicalLocation.region.startLine` (if source span available; else omit).
   - Severity mapping: `Error` → `"error"`, `Warning` → `"warning"`.
4. `emit_gha.rs`:
   - One line per diagnostic: `::error file={path},line={line},col={col},title={code}::{message} [profile: {profile}]`.
   - If source span is `None`, omit `line` and `col`.
   - For warnings: `::warning ...` instead of `::error`.
5. `emit_junit.rs`:
   - Generate JUnit XML with one `<testsuite>` per schema file.
   - Each diagnostic is a `<testcase>` with `<failure>` or `<skipped>` (for warnings, use `<failure>` with `type="warning"`).
   - Alternative: one `<testsuite>` per profile, `<testcase>` per schema. Decide during implementation based on CI expectations.
   - Include `name`, `tests`, `failures`, `time` attributes.

**Patterns to follow:**
- Existing `emit_json.rs` pattern: build a struct/Value and `serde_json::to_string_pretty`.
- Existing `emit_human.rs` pattern: format strings, accumulate into `String`.

**Test scenarios:**
- SARIF:
  - Happy path: Single error → valid SARIF JSON with one result.
  - Happy path: Mixed errors/warnings → valid SARIF JSON with correct severity levels.
  - Edge case: Empty diagnostics → SARIF with empty `results` array.
- GHA:
  - Happy path: Single error → `::error file=schema.json,title=OAI-K-allOf::keyword 'allOf' is not supported [profile: openai.so.2026-04-30]`.
  - Happy path: Warning → `::warning ...`.
- JUnit:
  - Happy path: Single schema with errors → XML with `<testsuite>` containing failing `<testcase>`s.
  - Happy path: Clean schema → XML with passing `<testcase>`.

**Verification:**
- `cargo test --test snapshot_tests` passes with new snapshots for SARIF, GHA, JUnit.
- Manual validation: SARIF output validates against SARIF schema (can use online validator or JSON Schema).

---

- U6. **[Persistent disk cache]**

**Goal:** Extend the in-memory cache to persist across process invocations on disk.

**Requirements:** R21

**Dependencies:** None (but needed by U7)

**Files:**
- Modify: `crates/schemalint/src/cache.rs`
- Modify: `crates/schemalint/Cargo.toml`
- Modify: `crates/schemalint/src/ir/arena.rs`
- Modify: `crates/schemalint/src/normalize/mod.rs`
- Test: `crates/schemalint/tests/property_tests.rs`

**Approach:**
1. Add dependencies to `Cargo.toml`:
   - `bincode = "1.3"`
   - `dirs = "5.0"`
   - Update `indexmap` to `indexmap = { version = "2.0", features = ["serde"] }`
2. Derive `Serialize` + `Deserialize` for:
   - `NodeId` (transparent over `u32`)
   - `NodeKind`
   - `Annotations`
   - `Node`
   - `Arena`
   - `NormalizedSchema`
3. In `cache.rs`:
    - Add `DiskCache` struct wrapping the existing in-memory cache.
    - Cache directory: `dirs::cache_dir().join("schemalint")` (e.g., `~/.cache/schemalint/` on Linux).
    - On `get(hash)`: check in-memory first, then try to read `cache_dir/{hash:016x}.bin`.
    - Validate 4-byte version header (version = 1 for Phase 2). Mismatch → treat as miss.
    - `bincode::deserialize` the remainder. Any error → treat as miss.
    - On `insert(hash, schema)`: insert in-memory, then write version header + `bincode::serialize` to `cache_dir/{hash:016x}.bin`.
    - Add simple eviction: if directory contains > 1000 files, delete oldest by mtime.
4. Keep the existing `Cache` struct API unchanged for backward compatibility.

**Patterns to follow:**
- Existing `Cache` API: `new()`, `get(hash) -> Option<&NormalizedSchema>`, `insert(hash, schema)`.
- `rustc_hash::FxHasher` for hashing (already used).

**Test scenarios:**
- Happy path: Insert schema → file written to cache dir.
- Happy path: Get cached schema → returns without re-normalization.
- Happy path: Cache hit after process restart → disk read succeeds, schema is valid.
- Edge case: Corrupt cache file → treat as miss, re-normalize and overwrite.
- Edge case: Cache dir not writable → fall back to in-memory only (no panic).
- Property test: Round-trip serialization/deserialization produces identical `NormalizedSchema`.

**Verification:**
- `cargo test --test property_tests` passes with new round-trip property test.
- `cargo test --workspace` passes.
- Cache directory is created and populated during integration tests.

---

- U7. **[JSON-RPC server mode]**

**Goal:** Implement a long-running JSON-RPC 2.0 server over stdin/stdout.

**Requirements:** R18–R21

**Dependencies:** U2, U3, U6

**Files:**
- Create: `crates/schemalint/src/cli/server.rs`
- Modify: `crates/schemalint/src/cli/mod.rs`
- Modify: `crates/schemalint/src/cli/args.rs`
- Test: `crates/schemalint/tests/integration_tests.rs` (or new `server_tests.rs`)

**Approach:**
1. In `server.rs`:
   - `pub fn run_server()` — blocking read-loop on `std::io::stdin()`.
    - Read one line at a time (line-delimited JSON-RPC).
    - Enforce max request payload size: 10 MB. Exceed → JSON-RPC error response.
    - Parse each line as `serde_json::Value`.
    - Validate JSON-RPC 2.0 envelope (`jsonrpc: "2.0"`, `method`, `id`).
    - Dispatch:
      - `method == "check"` → parse params (`schema: Value`, `profiles: Vec<String>`, `format: String`), load profiles, normalize schema, run all rule sets, emit diagnostics in requested format, return JSON-RPC result. Enforce max check execution time: 30 seconds. Exceed → JSON-RPC error response.
      - `method == "shutdown"` → break the read-loop.
    - Use the persistent `DiskCache` from U6 for normalization caching.
    - Spawn `check` requests in a `rayon` thread pool (or `std::thread::spawn`) to keep the read-loop responsive.
2. In `cli/mod.rs`:
   - Add `Commands::Server` match arm calling `server::run_server()`.
3. In `args.rs`:
   - Add `Server` subcommand (no args for now).

**Patterns to follow:**
- Existing `run_check` logic for profile loading, normalization, rule checking.
- Existing `emit_json::emit_json_to_string` and `emit_human::emit_human_to_string` for output formatting.

**Test scenarios:**
- Happy path: Send `{"jsonrpc":"2.0","method":"check","params":{"schema":{"type":"object"},"profiles":["openai.so.2026-04-30"],"format":"json"},"id":1}` → receive result with diagnostics array.
- Happy path: Send `{"jsonrpc":"2.0","method":"shutdown","id":2}` → server exits.
- Error path: Invalid JSON-RPC (missing `jsonrpc` field) → error response.
- Error path: Unknown method → error response.
- Error path: Unknown profile ID → error response in result.
- Edge case: Large schema (200 properties) → responds within <1ms.

**Verification:**
- `cargo test --test integration_tests` passes with server spawn + request/response tests.
- Manual test: `echo '{"jsonrpc":"2.0","method":"check","params":{...},"id":1}' | cargo run -- server` produces valid JSON-RPC response.

---

- U8. **[Anthropic regression corpus]**

**Goal:** Add 25 Anthropic-specific test schemas with deterministic expected diagnostics.

**Requirements:** R22–R23

**Dependencies:** U1

**Files:**
- Create: `crates/schemalint/tests/corpus/ant_schema_*.json` (25 files)
- Create: `crates/schemalint/tests/corpus/ant_schema_*.expected` (25 files)
- Modify: `crates/schemalint/tests/corpus_tests.rs`

**Approach:**
1. Create 30 schema files covering:
    - `minimum` / `maximum` / `multipleOf` rejection (3 schemas)
    - `minLength` / `maxLength` rejection (2 schemas)
    - `maxItems` / `uniqueItems` / `contains` / `prefixItems` rejection (4 schemas)
    - `allOf` + `$ref` rejection (2 schemas)
    - `allOf` without `$ref` (allowed) (1 schema)
    - Recursive `$ref` rejection (1 schema)
    - External `$ref` rejection (1 schema)
    - Complex enum types (objects in enum) (1 schema)
    - `enum` with mixed types (allowed: strings/numbers/bools/null only) (1 schema)
    - `minItems` > 1 rejection (1 schema)
    - `pattern` with backreferences / lookahead (1 schema)
    - `not` / `if-then-else` rejection (3 schemas)
    - `dependentRequired` / `dependentSchemas` rejection (2 schemas)
    - `discriminator` rejection (1 schema)
    - `anyOf` with mixed types (allowed) (1 schema)
    - Internal `$ref` (allowed) (1 schema)
    - `default` on string/number/boolean (allowed) (1 schema)
    - Clean Anthropic-compatible schemas (3 schemas — should produce no errors)
2. Generate `.expected` files by running the CLI with Anthropic profile and capturing JSON output.
3. Update `corpus_tests.rs` to run Anthropic corpus in addition to OpenAI corpus.

**Patterns to follow:**
- Existing corpus test pattern: `tests/corpus_tests.rs` shells out to debug binary, compares JSON output.
- Existing schema naming: `schema_001.json`, `schema_002.json`, etc.

**Test scenarios:**
- Each of the 30 schemas produces deterministic output matching its `.expected` file.
- Clean schemas produce zero errors.
- Forbidden-keyword schemas produce exactly one error per forbidden keyword.
- Allowed-feature schemas (e.g., `allOf` without `$ref`, `anyOf` mixed types) produce zero errors.

**Verification:**
- `cargo test --test corpus_tests` passes.
- `INSTA_UPDATE=always cargo test --test corpus_tests` can regenerate expecteds if needed.

---

- U9. **[Integration and snapshot tests]**

**Goal:** Add comprehensive tests for multi-profile runs, new output formats, and server mode.

**Requirements:** (supports all success criteria)

**Dependencies:** U3, U5, U7

**Files:**
- Modify: `crates/schemalint/tests/integration_tests.rs`
- Modify: `crates/schemalint/tests/snapshot_tests.rs`
- Create: `crates/schemalint/tests/server_tests.rs` (if not added to integration_tests)
- Modify: `crates/schemalint/tests/rules_tests.rs`
- Modify: `crates/schemalint/tests/structural_tests.rs`

**Approach:**
1. `integration_tests.rs`:
   - Add test: `check_multi_profile_union` — runs with both OpenAI and Anthropic profiles, asserts union diagnostics.
   - Add test: `check_multi_profile_exit_code` — errors from any profile cause exit 1.
   - Add test: `check_builtin_profile_resolution` — uses bare ID without path.
2. `snapshot_tests.rs`:
   - Add snapshots for SARIF, GHA, JUnit output on a representative schema.
   - Use `insta` with regex filters for temp paths.
3. `server_tests.rs`:
   - Spawn `schemalint server` as a child process.
   - Send JSON-RPC requests via stdin, read responses from stdout.
   - Assert response shape and diagnostic content.
   - Send `shutdown` and assert process exits cleanly.
4. `rules_tests.rs`:
   - Add unit tests for each new semantic rule with positive and negative cases.
5. `structural_tests.rs`:
   - Add tests for multi-profile structural rules (e.g., OpenAI requires object root, Anthropic does not).

**Patterns to follow:**
- Existing `assert_cmd` + `predicates` for CLI integration tests.
- Existing `insta` snapshot testing with regex path normalization.
- Existing `tempfile` for temp directories.

**Test scenarios:**
- Integration: Multi-profile run produces correct union.
- Integration: Built-in profile resolution works for both OpenAI and Anthropic IDs.
- Snapshot: SARIF output matches expected snapshot.
- Snapshot: GHA output matches expected snapshot.
- Snapshot: JUnit output matches expected snapshot.
- Server: Spawn server, send check request, receive valid JSON-RPC response.
- Server: Send shutdown request, server exits.
- Rules: Each new rule has positive and negative unit tests.

**Verification:**
- `cargo test --workspace` passes.
- `cargo test --test snapshot_tests` passes.
- `cargo test --test integration_tests` passes.
- `cargo test --test server_tests` passes.

---

## System-Wide Impact

- **Interaction graph:** The CLI `run_check` function changes from single-profile to multi-profile orchestration. The `RuleSet` is now built N times per invocation. The `Cache` is shared across all profiles for a given schema (normalization happens once).
- **Error propagation:** Profile load errors (invalid TOML, missing built-in ID) are fatal and emitted to stderr before any schema processing. JSON-RPC server returns errors in JSON-RPC error responses.
- **State lifecycle risks:** Disk cache files are written after normalization. Corrupt cache files are treated as misses and overwritten. Cache directory creation is lazy.
- **API surface parity:** New CLI subcommand `server` and new `--format` variants. Existing `check` subcommand behavior is backward-compatible for single-profile invocations.
- **Unchanged invariants:**
  - Arena allocation and `NodeId(u32)` indexing remain unchanged.
  - Normalizer pipeline remains unchanged.
  - Class A auto-generation from profile keywords remains unchanged.
  - Existing error codes (`OAI-K-*`, `OAI-S-*`) remain valid for OpenAI profile.
  - Exit code semantics (0 = no errors, 1 = errors, 2 = I/O error) remain unchanged for single-profile runs.

---

## Risks & Dependencies

| Risk | Mitigation |
|---|---|
| `Serialize`/`Deserialize` derive on `NormalizedSchema` fails due to complex types | The types are `Vec<Node>`, `IndexMap`, `String`, `u32`, `bool`, `Option`, and `serde_json::Value` — all serializable. If derive fails, implement manual serialization or store raw normalized JSON in cache. |
| JSON-RPC server deadlocks on stdin/stdout | Use line-buffered I/O and spawn check requests in a thread pool. Keep read-loop single-threaded and non-blocking. |
| Multi-profile performance degrades linearly | Normalization is cached and shared across profiles. Rule checking is the dominant cost; running 3 rule sets in sequence is ~3x the time of 1, but the <1ms target is for a single schema. Benchmark and optimize if needed. |
| SARIF/JUnit format incompatibilities with CI systems | Emit standard-compliant formats. Test against GitHub Actions (for SARIF upload), GitLab (for JUnit), and Jenkins (for JUnit). Adjust during implementation if specific systems expect variant shapes. |
| New semantic rules produce false positives | Each rule has conservative logic and comprehensive negative tests. `AnyOfObjectsHint` is a warning, not an error. |

---

## Documentation / Operational Notes

- Update `AGENTS.md` if new build steps or test commands are added.
- No user-facing documentation changes required in Phase 2 (Phase 5 covers the documentation site).

---

## Sources & References

- **Origin document:** [docs/brainstorms/phase2-requirements.md](docs/brainstorms/phase2-requirements.md)
- **Phase document:** [docs/phases.md](docs/phases.md)
- Related code:
  - `crates/schemalint/src/cli/mod.rs`
  - `crates/schemalint/src/rules/registry.rs`
  - `crates/schemalint/src/rules/class_b.rs`
  - `crates/schemalint/src/profile/parser.rs`
  - `crates/schemalint/src/cache.rs`
  - `crates/schemalint/src/ir/arena.rs`
- External docs:
  - OpenAI Structured Outputs: https://platform.openai.com/api/docs/guides/structured-outputs
  - Anthropic Structured Outputs: https://docs.anthropic.com/en/docs/build-with-claude/structured-outputs
  - SARIF v2.1.0: https://docs.oasis-open.org/sarif/sarif/v2.1.0/sarif-v2.1.0.html
  - GitHub Actions workflow commands: https://docs.github.com/en/actions/writing-workflows/choosing-what-your-workflow-does/workflow-commands-for-github-actions
  - JSON-RPC 2.0: https://www.jsonrpc.org/specification
