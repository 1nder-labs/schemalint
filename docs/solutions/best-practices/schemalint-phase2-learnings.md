---
title: "Phase 2 Code Review — Testing, Safety, and Performance Patterns"
date: 2026-05-01
module: schemalint
problem_type: best_practice
component: tooling
severity: high
related_components:
  - testing_framework
  - documentation
applies_when:
  - "Writing corpus/regression tests that compare structured JSON output"
  - "Emitting output into delimiter-based formats (GHA workflow commands, CSV, XML, Markdown)"
  - "Handling I/O in non-critical fallback or cache paths"
  - "Locking shared state in parallel iterator chains (rayon, tokio, thread::scope)"
  - "Choosing between binary and human-readable serialization for persistent caches"
  - "Adding new rule types or extending the rule registry in linkme-based architectures"
symptoms:
  - "80+ corpus test cases silently passing due to pointer field name mismatch"
  - "CI workflow commands silently corrupted by unescaped newlines in diagnostic content"
  - "Cache I/O errors silently discarded by let-bindings with no handling"
  - "Rayon worker parallelism degraded to sequential by Mutex held across check_all"
tags:
  - rust
  - cli
  - static-analysis
  - json-schema
  - schemalint
  - code-review
  - testing
  - serialization
  - error-handling
---

# Phase 2 Code Review — Testing, Safety, and Performance Patterns

## Context

During Phase 2 implementation of schemalint (a JSON Schema static-analysis CLI), a code review
across 11 specialist reviewers surfaced five anti-patterns involving test assertions, output
escaping, error suppression, lock scoping, and architectural choices. Each was fixed in-place;
this document captures the patterns so they are not repeated in future schemalint work or
similar Rust CLI projects.

No prior session history was found for this repository (75 sessions across Claude Code, Codex,
and Cursor yielded zero schemalint results), confirming this is the first documented learning
in `docs/solutions/`.

## Guidance

### 1. Test assertions must use emitter field names, not internal struct fields

When comparing structured output (JSON) in corpus tests, reference the field names the *emitter*
produces — not the Rust struct fields in the source.

Anti-pattern (`corpus_tests.rs` — before):
```rust
v["schemaPath"].as_str().unwrap_or("").to_string()
//          ^^^^^^^^^^^ Rust struct field, NOT the emitted JSON key
```

Correct (`corpus_tests.rs` — after):
```rust
v["pointer"].as_str().unwrap_or("").to_string()
//   ^^^^^^^ matches the actual JSON output from emit_json.rs
```

The comparison `a.get("schemaPath") != e.get("schemaPath")` returned `None` on both sides
because the JSON emitter produces `"pointer"`, not `"schemaPath"`. The comparison was
vacuously equal — none of the 80+ corpus test cases were actually validating pointer content.

### 2. Escape content values in delimiter-based output formats

Any output format that relies on a delimiter or inline marker syntax (GitHub Actions workflow
commands, CSV, XML, Markdown tables) must escape content values that could contain the
delimiter.

Anti-pattern (`emit_gha.rs` — before):
```rust
out.push_str(&format!("::{cmd} file={file},title={code}::{message}\n"));
//                          ^ newline in message would break the :: delimiter
```

Correct (`emit_gha.rs` — after):
```rust
fn encode_gha_value(s: &str) -> String {
    s.replace('%', "%25")
     .replace('\r', "%0D")
     .replace('\n', "%0A")
     .replace(':', "%3A")
}
let file = encode_gha_value(&path.display().to_string());
let code = encode_gha_value(&d.code);
let message = encode_gha_value(&format!("{} [profile: {}]", d.message, d.profile));
out.push_str(&format!("::{cmd} file={file},title={code}::{message}\n"));
```

### 3. Log I/O errors in non-critical paths — never discard silently

`let _ =` on I/O results discards all error information. When the operation is non-critical
(background cache writes, directory creation), log the failure so it is diagnosable.

Anti-pattern (`cache.rs` — before):
```rust
let _ = fs::create_dir_all(dir);
let _ = fs::write(&path, &buf);
let _ = fs::remove_file(entry.path());
```

Correct (`cache.rs` — after):
```rust
if let Err(e) = fs::create_dir_all(dir) {
    eprintln!("warning: failed to create cache directory '{}': {}", dir.display(), e);
}
if let Err(e) = fs::write(&path, &buf) {
    eprintln!("warning: failed to write cache file '{}': {}", path.display(), e);
}
```

### 4. Scope mutex guards to the smallest critical section

In parallel code (rayon `par_iter`), holding a mutex guard across expensive work serializes
that work and defeats parallelism.

Anti-pattern (`cli/mod.rs` — before):
```rust
let cache_guard = cache.lock().unwrap();
let diags = ruleset.check_all(&cached.arena, profile);
// Lock still held during the expensive check_all traversal
```

Correct (`cli/mod.rs` — after):
```rust
let cached_schema = {
    let cache_guard = cache.lock().unwrap();
    cache_guard.get(hash).cloned()
}; // lock dropped here — only the clone lives past the scope
let diags = ruleset.check_all(&cached_schema.arena, profile);
```

### 5. Prefer human-readable serialization for persistent caches

The Phase 2 plan specified `bincode` for disk cache serialization. The implementation used
`serde_json` instead. JSON is debuggable with standard tools (`cat`, `jq`), survives schema
evolution more gracefully, and the performance difference is negligible for schemalint's
workload. A binary format would add a dependency without measurable benefit.

### 6. Three rule registration patterns

The codebase now uses three complementary patterns for registering lint rules:

| Pattern | Mechanism | Use case |
|---|---|---|
| Static `linkme` slice | `#[distributed_slice(RULES)]` | Universal rules compiled into the binary (semantic.rs) |
| Dynamic profile-generated | `RuleSet::from_profile()` | Class A / Class B rules from user profile TOML |
| Profile-gated dynamic | `RuleSet::from_profile()` with checks | Rules enabled only when a profile flag or code_prefix matches |

The diagnostic code prefix now comes from `profile.code_prefix` (e.g., `OAI`, `ANT`) rather
than being hardcoded, making the registry provider-agnostic.

### 7. Orchestration duplication between `run_check` and `handle_check`

Both functions duplicate the `load -> parse -> normalize -> check -> report` pipeline.
Extracted as a known debt item for a future refactoring pass; the duplication is ~30 lines
each and acceptable at current scale.

## Why This Matters

- **Test field mismatch** causes both false positives and false negatives in regression tests.
  When the emitter evolves, tests using internal names silently break with no warning.
- **Unescaped GHA values** can corrupt workflow command syntax. A diagnostic message
  containing `\n` or `::` produces malformed commands and potentially masks errors.
- **Silent I/O suppression** makes cache-write failures invisible. Debugging a stale-cache
  issue becomes guesswork when write errors were never logged.
- **Mutex scope leaks** turn parallel code into effectively sequential code. With N threads,
  the wall-clock time becomes `N × serial_time` instead of approximately `serial_time`.
- **Human-readable cache** reduces debugging friction. When a user reports incorrect results,
  inspecting the cache with `jq` is immediate; decoding bincode requires tooling.
- **Rule registration patterns** give contributors a clear mental model for where to add
  a rule and how it gets activated.

## When to Apply

- Adding or modifying a test that compares emitted output: always verify field names against
  the emitter source, not the IR struct.
- Writing a new output format (GHA, SARIF, JUnit, Markdown): review whether content values
  need escaping. If the format has a delimiter or inline syntax, the answer is almost always yes.
- Adding any `fs::write`, `fs::create_dir_all`, or network call in a fallback or cache path:
  use `if let Err(e) = ...` with `eprintln!`, never `let _ =`.
- Touching any `Mutex` or `RwLock` that protects data used in a parallel iterator: verify the
  guard is dropped before the expensive body of the loop.
- Adding a new serialization format or cache backend: default to `serde_json` unless a
  benchmark proves the need for a binary format.
- Adding a new lint rule: check which of the three registration patterns fits.

## Examples

See the before/after snippets in the Guidance section above. Each anti-pattern has a concrete
code example showing the incorrect and corrected form.

## Related

- `crates/schemalint/src/cli/emit_gha.rs` — workflow command escaping
- `crates/schemalint/src/cache.rs` — cache I/O error handling and mutex scoping
- `crates/schemalint/tests/corpus_tests.rs` — `diagnostics_match()` function
- `crates/schemalint/src/rules/mod.rs` — rule registration patterns
- `crates/schemalint/src/cli/mod.rs` — `run_check` / `handle_check` orchestration
- `docs/plans/2026-04-30-002-feat-phase-2-rules-multi-profile-plan.md` — Phase 2 plan
