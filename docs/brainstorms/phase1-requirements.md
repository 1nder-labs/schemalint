# Phase 1 Requirements — schemalint Foundation

**Date:** 2026-04-30
**Scope:** Phase 1 only (Foundation)
**Source of truth:** `docs/phases.md` + live OpenAI docs (`developers.openai.com/api/docs/guides/structured-outputs`)

---

## 1. Problem Statement

Engineers write JSON Schemas for LLM structured-output endpoints. The endpoints accept a strict subset of JSON Schema. There is no static tool that checks a schema against the provider's accepted subset before the API call. Failures appear at runtime as cryptic 400 errors, or worse, schemas are silently transformed by SDKs, dropping constraints the engineer expected to be enforced.

## 2. Goal

Ship a minimal, correct, and fast CLI that reads a JSON Schema and an OpenAI capability profile, then emits a typed inventory of keyword-profile mismatches. This is the "diff tool" — the first executable artifact of the project.

## 3. Success Criteria

1. `schemalint check --profile <path> <schema-file>` runs end-to-end and produces deterministic output.
2. Human output follows rustc-style format with file:line:col, error code, message, schema path, and profile reference.
3. JSON output is a structured diagnostic array with schema version, tool metadata, summary counts, and per-diagnostic details.
4. The OpenAI profile has **zero `unknown` states** for all keywords emitted by Pydantic v2 and `zod-to-json-schema`.
5. All 50 schemas in the regression corpus produce expected diagnostics when run through the diff tool.
6. Single 200-property nested schema, single profile: < 1 ms.
7. Project of 500 schemas, 1 profile, cold start: < 500 ms.
8. Incremental run after single-file edit: < 5 ms.

## 4. Scope Boundaries

### In Scope
- Monolithic `schemalint` crate (IR, normalizer, profile loader, rule registry, CLI)
- `schemalint-profiles` crate (data-only, independent release cadence)
- Arena-allocated IR with `NodeId(u32)` indexing
- Schema normalizer: dialect detection, `$ref` graph resolution, Tarjan SCC, type-array desugaring, parent/depth/JSON Pointer computation, content-hash caching
- Profile loader: TOML parser for five-state severity model (`allow`, `warn`, `strip`, `forbid`, `unknown`)
- Auto-registered rule registry using `linkme` distributed slices
  - Class A rules: auto-generated from loaded profile keywords
  - Class B infrastructure: registry ready, zero hand-written rules
- Diff CLI with human and JSON output
- OpenAI Structured Outputs profile (complete, doc-grounded, Apr 2026)
- Regression corpus: 50 schemas with expected diagnostics

## 5. Key Decisions

| Decision | Rationale |
|---|---|
| **Monolithic crate** | Eliminates cross-crate refactoring tax during the most volatile phase. Only `schemalint-profiles` is separate because it ships on a different cadence. |
| **`linkme` over `inventory`** | More reliable across platforms. No proc-macro complexity. Adding a Class B rule requires only authoring `impl Rule`. |
| **Docs as ground truth** | The live OpenAI docs (Apr 2026) contradict the SOW on several keywords. The docs are the contract; the SOW was written against older behavior. |
| **`allow` for number/string/array constraints** | OpenAI docs explicitly list `minimum`, `maximum`, `exclusiveMinimum`, `exclusiveMaximum`, `multipleOf`, `minLength`, `maxLength`, `pattern`, `minItems`, `maxItems` as supported for regular structured outputs. |
| **`warn` for `uniqueItems` and `prefixItems`** | Docs are silent. Pydantic v2 (`Set`, `Tuple`) and Zod (`z.set()`, `z.tuple()`) emit these. Historical OpenAI behavior suggests they are accepted but not enforced by the grammar engine. `warn` is the honest classification. |
| **`forbid` for `discriminator`** | Docs are silent. `discriminator` is not standard JSON Schema. OpenAI's grammar engine does not support propertyName-based dispatch. |
| **Data-driven structural rules** | Numeric limits (max depth, max properties, max enum size, max string length) live in the profile TOML under `[structural]`. The engine auto-generates Class B rules from these values. Changing a limit requires only editing TOML. |
| **`forbid` for composition keywords** | `allOf`, `not`, `if/then/else`, `dependentRequired`, `dependentSchemas` are explicitly listed as "not yet supported" in the OpenAI docs. |
| **Root must be object** | OpenAI docs explicitly state: "Root objects must not be anyOf and must be an object." |
| **All fields required** | OpenAI docs: "All fields must be required." Optional parameters are emulated via `["string", "null"]` unions. |
| **`additionalProperties: false` mandatory** | OpenAI docs: "Structured Outputs only supports generating specified keys / values, so we require developers to set additionalProperties: false." |

## 6. User-Facing Behavior

### CLI Interface

```bash
schemalint check --profile profiles/openai.so.2026-04-30.toml schema.json
```

### Human Output

```
error[OAI-K-allOf]: keyword 'allOf' is not supported by OpenAI Structured Outputs
   --> schema.json:15:8
    |
 15 |       "allOf": [...]
    |       ^^^^^^^
    |
    = profile: openai.so.2026-04-30
    = schema path: /properties/items
    = see: https://schemalint.dev/rules/OAI-K-allOf

warning[OAI-K-uniqueItems]: keyword 'uniqueItems' is accepted but not enforced by OpenAI Structured Outputs
   --> schema.json:42:5
    |
 42 |       "uniqueItems": true
    |       ^^^^^^^^^^^^^
    |
    = profile: openai.so.2026-04-30
    = schema path: /properties/tags
    = hint: validate uniqueness at the application layer
    = see: https://schemalint.dev/rules/OAI-K-uniqueItems

3 issues found (1 error, 2 warnings) across 1 schema in 2ms
```

### JSON Output

```json
{
  "schema_version": "1.0",
  "tool": { "name": "schemalint", "version": "0.1.0" },
  "profiles": ["openai.so.2026-04-30"],
  "summary": { "errors": 1, "warnings": 2, "files": 1, "duration_ms": 2 },
  "diagnostics": [
    {
      "code": "OAI-K-allOf",
      "severity": "error",
      "message": "keyword 'allOf' is not supported by OpenAI Structured Outputs",
      "pointer": "/properties/items",
      "source": { "file": "schema.json", "line": 15, "col": 8 },
      "profile": "openai.so.2026-04-30",
      "hint": null
    }
  ]
}
```

## 7. OpenAI Profile — Complete Keyword Table

Based on `developers.openai.com/api/docs/guides/structured-outputs`, scraped 2026-04-30.

### Types
- `type: object`, `type: array`, `type: string`, `type: integer`, `type: number`, `type: boolean`, `type: null` → `allow`
- `enum` → `allow`
- `const` → `allow`
- `anyOf` → `allow`
- `allOf` → `forbid`
- `not` → `forbid`

### String
- `pattern` → `allow`
- `format` → `restricted` (`date-time`, `time`, `date`, `duration`, `email`, `hostname`, `ipv4`, `ipv6`, `uuid`)
- `minLength` → `allow`
- `maxLength` → `allow`

### Number
- `minimum` → `allow`
- `maximum` → `allow`
- `exclusiveMinimum` → `allow`
- `exclusiveMaximum` → `allow`
- `multipleOf` → `allow`

### Array
- `minItems` → `allow`
- `maxItems` → `allow`
- `uniqueItems` → `warn` (docs silent; historical acceptance)
- `prefixItems` → `warn` (docs silent; tuple semantics)
- `contains` → `unknown` (not emitted by Pydantic/Zod)

### Object
- `properties` → `allow`
- `required` → `allow`
- `additionalProperties` → `restricted` (only `false`)
- `patternProperties` → `unknown` (not emitted by Pydantic/Zod)
- `unevaluatedProperties` → `unknown` (not emitted by Pydantic/Zod)
- `propertyNames` → `unknown` (not emitted by Pydantic/Zod)
- `minProperties` → `unknown` (not emitted by Pydantic/Zod)
- `maxProperties` → `unknown` (not emitted by Pydantic/Zod)

### References
- `$ref` (internal) → `allow`
- `$ref` (external) → `forbid`
- `$defs`, `definitions` → `allow`
- recursive `$ref` → `allow`

### Composition
- `if`, `then`, `else` → `forbid`
- `dependentRequired` → `forbid`
- `dependentSchemas` → `forbid`
- `oneOf` → `unknown` (not emitted by Pydantic/Zod)

### Structural
- `description` → `allow`
- `title` → `allow`
- `default` → `allow`
- `discriminator` → `forbid` (not standard JSON Schema; docs silent)

### Structural Limits
```toml
[structural]
require_object_root                 = true
require_additional_properties_false = true
require_all_properties_in_required  = true
max_object_depth                    = 10
max_total_properties                = 5000
max_total_enum_values               = 1000
max_string_length_total             = 120000
```

## 8. Regression Corpus

- 50 schemas sourced from public bug reports, OpenAI Community forum, Pydantic AI issues, and SDK forums.
- Each schema annotated with expected diagnostic set.
- Located in `tests/corpus/`.
- Acceptance: diff tool produces deterministic, matching output for all 50.

## 9. Quality Targets

- Every rule (Class A auto-generated + Class B data-driven) has at least one positive and one negative test.
- Snapshot testing for human and JSON output formats.
- Property tests for normalizer round-trips.

## 10. Dependencies / Assumptions

- Rust toolchain (latest stable)
- `linkme` crate for distributed slice registration
- `toml` crate for profile parsing
- `serde_json` for JSON Schema parsing
- `indexmap` for preserving keyword order
- Arena allocation via `Vec<Node>` indexed by `NodeId(u32)`

## 11. Handoff

Ready for `/ce-plan`.
