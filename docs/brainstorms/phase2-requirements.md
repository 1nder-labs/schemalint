---
date: 2026-04-30
topic: phase-2-rules-multi-profile
---

# Phase 2 Requirements — Rules, Multi-Profile, and Output Formats

## Summary

Extend the existing OpenAI-only diff tool into a multi-provider rule engine with an Anthropic profile, hand-written semantic checks, additional CI output formats, built-in profile resolution, and a long-running JSON-RPC server mode with persistent disk cache.

---

## Problem Frame

Phase 1 delivered a single-profile linter for OpenAI Structured Outputs. Engineers building schemas for Anthropic's Claude have the same runtime-failure problem — SDKs silently strip unsupported keywords, and the API rejects schemas with unsupported constraints — but the two providers accept different subsets of JSON Schema. There is no static tool that checks a schema against both providers simultaneously. Phase 2 closes this gap by adding a second provider profile, multi-profile composition, deeper semantic rules, CI-ready output formats, and a server mode for incremental linting.

---

## Assumptions

*This requirements doc was authored without synchronous user confirmation. The items below are agent inferences that fill gaps in the input — un-validated bets that should be reviewed before planning proceeds.*

- **Anthropic `default` is supported.** The live Anthropic docs explicitly list `default` as supported for all supported types. The Python SDK's `transform_schema` does not handle `default` (it appends it to `description`), but the API docs are the authoritative contract; the SDK may be conservative.
- **Anthropic `pattern` is supported.** The live docs explicitly list `pattern` as supported with limited regex. The SDK strips it, but the docs are the contract.
- **OpenAI `default` is accepted.** The OpenAI docs do not mention `default`, but the OpenAI Python SDK's `to_strict_json_schema` only strips `default: null` and leaves all other defaults in place. This implies non-null defaults are safe to send.
- **Anthropic request-level budgets are out of scope.** The Anthropic docs list request-level limits (20 strict tools, 24 optional parameters, 16 union-type parameters). These apply to the full API request, not to a single schema file. The static linter operates on single schemas; request-level validation belongs in ingestion helpers (Phases 3–4) or a separate request validator.
- **Server mode does not watch files.** The server is a JSON-RPC endpoint that waits for `check` requests. Clients (IDEs, build systems) are responsible for sending requests when files change. This matches the "only the emitter changes" constraint from `docs/phases.md`.

---

## Requirements

**Multi-profile composition**
- R1. The CLI accepts multiple `--profile <id-or-path>` arguments.
- R2. Each active profile runs an independent rule set against every schema.
- R3. Every diagnostic is tagged with the profile that produced it.
- R4. Exit code is `1` if any profile produces any error; `0` if all profiles produce only warnings or no issues.
- R5. If a built-in profile ID is passed (no path separator), it resolves to the bundled TOML in the `schemalint-profiles` crate. If a path is passed, it loads from the filesystem.

**Anthropic Structured Outputs profile**
- R6. The Anthropic profile is named `anthropic.so.2026-04-30` and is bundled in `schemalint-profiles`.
- R7. The profile keyword map is grounded in live Anthropic docs (scraped 2026-04-30) and SDK source inspection (`anthropic` v0.97.0):
  - `allow`: `type`, `properties`, `required`, `items`, `enum`, `const`, `anyOf`, `allOf`, `$ref`, `$defs`, `definitions`, `description`, `title`, `default`, `pattern`, `format`
  - `restricted`: `additionalProperties` → `[false]`; `format` → `date-time`, `time`, `date`, `duration`, `email`, `hostname`, `uri`, `ipv4`, `ipv6`, `uuid`; `minItems` → `[0, 1]`
  - `forbid`: `minimum`, `maximum`, `exclusiveMinimum`, `exclusiveMaximum`, `multipleOf`, `minLength`, `maxLength`, `maxItems`, `uniqueItems`, `contains`, `prefixItems`, `not`, `if`, `then`, `else`, `dependentRequired`, `dependentSchemas`, `discriminator`, recursive `$ref`, external `$ref`
  - `unknown`: `oneOf`, `patternProperties`, `unevaluatedProperties`, `propertyNames`, `minProperties`, `maxProperties`
- R8. The `[structural]` section has `require_additional_properties_false = true`, `external_refs = true`, and all numeric limits set to `0` (disabled) because Anthropic docs do not specify per-schema numeric limits.

**OpenAI profile corrections**
- R9. Correct `max_object_depth` from `20` to `10` to match the live OpenAI docs.
- R10. Change `oneOf` from `warn` to `unknown` because the OpenAI docs are silent on this keyword.

**Hand-written Class B semantic rules**
- R11. `EmptyObjectRule` — warn when an object schema has `additionalProperties: false` and either missing `properties` or `properties: {}`. Code: `OAI-S-empty-object` / `ANT-S-empty-object`.
- R12. `AdditionalPropertiesObjectRule` — error when `additionalProperties` is an object value (e.g., `{}`) instead of `false`. Code: `OAI-S-additional-properties-object` / `ANT-S-additional-properties-object`.
- R13. `AnyOfObjectsHint` — warn when `anyOf` contains only object-typed branches. Code: `OAI-S-anyof-objects` / `ANT-S-anyof-objects`.
- R14. `AllOfWithRefRule` — error when `allOf` contains a branch with `$ref` (Anthropic-specific). Code: `ANT-S-allof-with-ref`.

**Output formats**
- R15. Add `--format sarif` emitting SARIF v2.1.0.
- R16. Add `--format gha` emitting GitHub Actions workflow commands (`::error::`, `::warning::`).
- R17. Add `--format junit` emitting JUnit XML.

**Server mode**
- R18. Add `schemalint server` subcommand that starts a JSON-RPC 2.0 server over stdin/stdout.
- R19. Support `check` method accepting `{ "schema": <json>, "profiles": ["id", ...], "format": "human"|"json" }` and returning diagnostics.
- R20. Support `shutdown` method.
- R21. Use a persistent disk cache keyed by content hash (e.g., `~/.cache/schemalint/`). Cache invalidation is by content hash only — the server does not watch files.

**Regression corpus**
- R22. Add 25 Anthropic-specific schemas to `tests/corpus/` covering: `minimum` rejection, `maxItems` rejection, `allOf` + `$ref` rejection, recursive `$ref` rejection, complex enum types, external `$ref`, `minItems` > 1 rejection, and `pattern` with backreferences.
- R23. All 25 new schemas produce deterministic expected diagnostics.

**Performance**
- R24. Single 200-property schema, 3 profiles: < 1 ms.
- R25. Project of 500 schemas, 3 profiles, cold start: < 500 ms.
- R26. Incremental run after single-file edit: < 5 ms.

---

## Success Criteria

- The Anthropic profile passes a manual audit against live docs and SDK source.
- All 29 rules from the v1 catalogue (per `docs/phases.md`) are implemented and registered.
- The existing Phase 1 corpus of 50 schemas still passes deterministically.
- The new Anthropic corpus of 25 schemas produces expected diagnostics.
- Multi-profile runs (`--profile openai.so.2026-04-30 --profile anthropic.so.2026-04-30`) produce correct unions of diagnostics.
- SARIF, GHA, and JUnit output formats are snapshot-tested.
- Server mode starts, accepts a `check` request, and returns correct diagnostics.

---

## Scope Boundaries

### Deferred for later
- Pydantic and Zod ingestion helpers (Phase 3 and 4).
- Auto-fix or schema rewriting (out of scope for v1 per SOW §3.2).
- Packaging and distribution (Phase 5).
- Live-API conformance harness and burn windows (Phase 5).
- IDE integrations or LSP protocol.
- Request-level Anthropic budgets (20 tools, 24 optional params, 16 union params) — these require request-context awareness.

### Outside this product's identity
- A schema auto-fix tool or code generator.
- A provider-agnostic generic JSON Schema linter — the product is specifically for LLM structured-output providers.

---

## Key Decisions

| Decision | Rationale |
|---|---|
| **Anthropic `allOf` as `allow` + semantic rule for `$ref`** | The docs say `allOf` is supported but not with `$ref`. This is a cross-keyword condition, which is the definition of a Class B semantic rule. The TOML stays honest. |
| **Disable Anthropic numeric structural limits** | Anthropic docs specify request-level budgets, not per-schema depth/property limits. Enforcing arbitrary limits would create false positives. |
| **Independent rule sets per profile** | Simpler than merging profiles. Each diagnostic naturally carries its profile provenance. No complex merge logic. |
| **Server mode without file watching** | Matches "only the emitter changes" from `docs/phases.md`. File watching adds OS-specific complexity and belongs in the client. |
| **OpenAI `max_object_depth` 20 → 10** | Direct doc correction. The existing value contradicts live docs. |
| **OpenAI `oneOf` warn → unknown** | Docs are silent. Conservative classification is correct and easy to upgrade later. |
| **Trust API docs over SDK for `default` and `pattern`** | The API docs are the contract. The SDK is a client convenience and may be conservative. |

---

## Dependencies / Assumptions

- Rust toolchain (latest stable)
- `linkme` crate for distributed slice registration
- `toml` crate for profile parsing
- `serde_json` for JSON Schema parsing
- `indexmap` for keyword order preservation
- `rayon` for parallel schema processing
- Anthropic Python SDK v0.97.0 behavior inspected for ground truth
- OpenAI Python SDK v2.33.0 behavior inspected for ground truth
- Pydantic v2 behavior inspected for emission surface

---

## Outstanding Questions

### Resolve Before Planning
*(none)*

### Deferred to Planning
- **[Needs research]** Exact SARIF v2.1.0 schema shape for diagnostic locations (line:col vs schema path).
- **[Needs research]** JUnit XML format for static analysis tools (no standard exists; follow `pytest-junit` or `eslint-junit` conventions).
