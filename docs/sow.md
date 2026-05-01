# `schemalint` — Statement of Work

A static analyzer for JSON Schemas intended for LLM structured-output endpoints. Runs alongside `ruff`, `oxlint`, `biome`, `eslint`, and other project linters. Reports schema features that target providers reject, silently strip, or fail to enforce.

---

## 1. Project Summary

`schemalint` is a standalone command-line tool that statically analyzes JSON Schemas — written directly, generated from Pydantic, or generated from Zod — against per-model capability profiles for LLM providers. It emits diagnostics in standard linter output formats (human, JSON, SARIF, GitHub Actions, JUnit) and is intended to run as part of a project's normal lint pipeline.

The core is implemented in Rust. The first-class interface is the CLI. Python and TypeScript bindings are provided for programmatic use. Capability profiles are TOML files versioned independently from the engine.

---

## 2. Problem Statement

LLM providers accept a strict subset of JSON Schema for their structured-output and tool-use endpoints. The accepted subset:

1. Differs between providers (OpenAI, Anthropic, Google, Groq, Azure OpenAI, Bedrock, Mistral).
2. Differs between API versions and model snapshots within a single provider.
3. Is incompletely documented and changes without announcement.
4. Is enforced inconsistently. Some unsupported keywords cause API errors; some are accepted and silently ignored at decoding time; some are silently stripped by the official SDK before the request is sent.

There is no existing tool that performs static, multi-provider, language-agnostic linting of JSON Schemas against these constraints. Existing artifacts are either runtime transformers tied to one language ecosystem (Pydantic AI's `OpenAIJsonSchemaTransformer`, OpenAI's `pydantic_function_tool`, Anthropic's per-language strippers, the `instructor` library) or generic JSON Schema validators that operate on data instances rather than schemas (`jsonschema`, `ajv`, `openapi-schema-validator`).

The result for users today: failures appear at runtime as cryptic 400 errors, or worse, do not appear at all because the SDK silently dropped the constraint. Both modes cost engineering time and produce silent data integrity bugs.

---

## 3. Scope

### 3.1 In Scope (v1)

- Static analysis of JSON Schema files
- Ingestion of Pydantic models (v1 and v2) via Python helper
- Ingestion of Zod schemas via TypeScript helper
- Capability profiles for OpenAI Structured Outputs and Anthropic Structured Outputs
- Diagnostics with JSON Pointer locations and source spans where available
- Output formats: human, JSON, SARIF v2.1.0, GitHub Actions annotations, JUnit XML
- Configuration via `pyproject.toml`, `package.json`, or `schemalint.toml`
- Caching by file content hash for incremental runs
- Parallel file processing
- Distribution as standalone binary, PyPI wheel, npm package, Cargo crate
- Homebrew formula, GitHub Action, and Docker image are lower-priority v1 targets (best-effort)

### 3.2 Out of Scope (v1)

- Auto-fix / schema rewriting
- LSP server and editor extensions
- WASM target / browser embedding
- Profiles for Google, Mistral, Cohere, Bedrock, Azure OpenAI
- Static AST-walking ingestion for TypeScript (runtime Zod conversion only in v1)
- Pydantic field-precise AST source mapping (heuristic line resolution only in v1)
- Profile intersection mode ("find a schema valid on all selected providers")
- Runtime data validation (use `jsonschema` / `ajv` for that — out of scope by design)
- JSON Schema spec validity checking (use `jsonschema` / `ajv` for that — out of scope by design)

### 3.3 Explicitly Not Replaced

`schemalint` is not a substitute for runtime validation. Consumers should still validate model responses against their original schemas using `pydantic`, `zod`, `jsonschema`, or `ajv`. `schemalint` only checks that the schema *sent* to the provider is one the provider will accept and enforce as the user expects.

---

## 4. Deliverables

| ID | Deliverable | Format |
|---|---|---|
| D1 | `schemalint` CLI binary | Linux/macOS/Windows × x86_64/arm64 |
| D2 | `schemalint` Rust crate (engine: IR, normalizer, rules, CLI) | crates.io |
| D3 | `schemalint-profiles` Rust crate (data only; independent release cadence) | crates.io |
| D4 | `schemalint` Python package (CLI bundled) | PyPI wheel |
| D5 | `schemalint-py` Python library bindings | PyPI wheel (PyO3) |
| D6 | `@schemalint/cli` npm package (CLI bundled) | npm |
| D7 | `@schemalint/core` TypeScript library bindings | npm (napi-rs) |
| D8 | `@schemalint/zod` Zod ingestor | npm |
| D9 | OpenAI Structured Outputs profile | TOML, dated |
| D10 | Anthropic Structured Outputs profile | TOML, dated |
| D11 | Documentation site (auto-generated rule reference) | mdBook, hosted |
| D12 | Conformance test suite (synthetic mock + live-API burn windows) | Repository, runnable |

---

## 5. Architecture

```
┌────────────────────────────────────────────────────────────────┐
│                         Front-ends                             │
│   CLI │ Python bindings (PyO3) │ TS bindings (napi-rs)         │
└──────────────┬───────────────────────┬─────────────────────────┘
               │                       │
        ingest │                       │ ingest
               ▼                       ▼
   ┌───────────────────────┐ ┌──────────────────────┐
   │ Pydantic ingestor     │ │ Zod ingestor         │
   │ (Python child proc:   │ │ (Node child proc:    │
   │  JSON-RPC over stdin) │ │  JSON-RPC over stdin)│
   └───────────┬───────────┘ └──────────┬───────────┘
               │                        │
               │     raw JSON Schema    │
               └────────────┬───────────┘
                            ▼
   ┌─────────────────────────────────────────────────────┐
   │  schemalint crate (monolithic in v1)                │
   │  ┌───────────────────────────────────────────────┐  │
   │  │ Schema normalization layer                    │  │
   │  │ Dialect detection · $ref graph · type-array   │  │
   │  │ desugaring · property order · cycle detection │  │
   │  └──────────────────────┬────────────────────────┘  │
   │                         ▼                            │
   │  ┌───────────────────────────────────────────────┐  │
   │  │ Internal Representation                       │  │
   │  │ Arena-allocated AST · stable JSON Pointers    │  │
   │  └──────────────────────┬────────────────────────┘  │
   │                         ▼                            │
   │  ┌───────────────────────────────────────────────┐  │
   │  │ Rule engine                                   │  │
   │  │ Profile-derived auto-rules · hand-written     │  │
   │  │ semantic rules · auto-registered registry     │  │
   │  └──────────────────────┬────────────────────────┘  │
   │                         ▼                            │
   │  ┌───────────────────────────────────────────────┐  │
   │  │ Diagnostic emitter                            │  │
   │  │ human · JSON · SARIF · GitHub Actions · JUnit │  │
   │  └───────────────────────────────────────────────┘  │
   └─────────────────────────────────────────────────────┘
                            ▲
               ┌────────────┴───────────┐
               │   schemalint-profiles    │
               │   (independent crate)    │
               │   TOML profiles · dated  │
               └──────────────────────────┘
```

### 5.1 Design Decisions

- **Rust core.** Single binary distribution; no runtime dependency. Algebraic data types map directly to JSON Schema's variant structure. PyO3 and napi-rs let one compiled core serve Python, Node, and CLI users.
- **Monolithic crate in v1, with one exception.** The engine (IR, normalizer, rules, CLI) lives in a single `schemalint` crate. Only `schemalint-profiles` is separate from day one because profiles ship on a different release cadence than the engine — a structural fact of the domain. Split further only when compile times or API stability force it.
- **Arena allocation for IR.** All schema nodes live in a `Vec<Node>` indexed by `NodeId(u32)`. Rules pass `NodeId` rather than references. Cache locality and parallelism follow from this.
- **CLI is the product.** Bindings are convenience surfaces. 95% of usage is `schemalint check` invoked from a script or CI step. PyPI and npm distribution are therefore v1 blockers: a Python team installs via `pip`, not `cargo`.
- **Profiles are data, not code.** Adding a new model version is a TOML PR, not a release cycle.
- **No embedded interpreters; long-lived child processes.** The Rust core does not link Python or Node. Pydantic and Zod ingestion happens in child processes that communicate via JSON-RPC over stdin/stdout. The CLI maintains a long-lived process pool to avoid the 30–100 ms spawn cost per invocation. A latency spike must validate this design against the 500 ms cold-start budget (§12) before M1 begins.

---

## 6. Internal Representation

The IR is the only structure rules see. It is constructed once per schema and is immutable thereafter.

### 6.1 Required Properties

1. **Lossless** — preserves keyword order, unknown keywords, `$comment`, `title`, `description`.
2. **Position-stable** — every node carries a canonical JSON Pointer (RFC 6901) computed once during normalization.
3. **Source-mappable** — every node optionally carries `Span { file, line, col, len }` from the upstream ingestor.
4. **Reference-aware** — `$ref` targets are resolved into edges in a graph; cycles detected once.

### 6.2 Type Sketch

```rust
pub struct Schema {
    pub root:    NodeId,
    pub nodes:   Vec<Node>,
    pub defs:    IndexMap<String, NodeId>,
    pub dialect: Dialect,
    pub source:  Option<SourceMap>,
}

pub struct Node {
    pub id:          NodeId,
    pub parent:      Option<NodeId>,
    pub pointer:     JsonPointer,
    pub span:        Option<Span>,
    pub kind:        NodeKind,
    pub annotations: Annotations,
    pub unknown:     IndexMap<String, Value>,  // unrecognized keywords, preserved verbatim
}

pub enum NodeKind {
    Object(ObjectSchema),
    Array(ArraySchema),
    String(StringSchema),
    Number(NumberSchema),  // covers "integer" via NumberKind
    Boolean,
    Null,
    Enum(EnumSchema),
    Const(Value),
    Ref(NodeId),
    Union(UnionSchema),    // anyOf, oneOf, type-arrays
    AllOf(Vec<NodeId>),
    Not(NodeId),
    Any,                    // empty schema {}
}
```

### 6.3 Normalization Steps (in order)

1. Parse and detect dialect (`$schema` URI; fall back to keyword heuristics).
2. Resolve `$ref` into graph edges; do not expand inline.
3. Detect reference cycles via Tarjan's SCC.
4. Desugar type arrays (`["string", "null"]` → `Union`).
5. Compute parent links, depth, JSON Pointers in a single DFS.
6. Compute stable hash for cache key.

After normalization, every rule sees the same canonical structure regardless of input dialect or source language.

---

## 7. Capability Profiles

A capability profile is a TOML file describing what a given provider+endpoint+model accepts in strict mode. Profiles are versioned by date.

### 7.1 Severity Model

Each keyword in a profile has one of five states:

| State | Meaning | Default Diagnostic Severity |
|---|---|---|
| `allow` | Accepted and enforced by constrained decoding | none |
| `warn` | Accepted by API; not enforced at decoding time | warning |
| `strip` | Silently removed by official SDK before transmission | warning |
| `forbid` | Rejected by API with an error | error |
| `unknown` | Unverified by the conformance suite | warning (`error` under `--strict-unknown`) |

The five-state model exists because a binary supported/unsupported model fails to express the cases users actually hit. `uniqueItems` on OpenAI is accepted by the API but ignored by the grammar (`warn`). `minimum` on Anthropic is silently dropped by the SDK (`strip`). `format: "credit-card"` on OpenAI is rejected (`forbid` via the `restricted` allow-list mechanism).

### 7.2 Profile File Format

```toml
[profile]
id          = "openai.structured-outputs.2026-04"
provider    = "openai"
endpoint    = "structured-outputs"
models      = ["gpt-4o-2024-08-06", "gpt-4o-mini-2024-07-18", "gpt-4.1", "gpt-4.5", "gpt-5", "gpt-5.2", "gpt-5.4"]
strict_only = true
revision    = "2026-04-30"
source_url  = "https://developers.openai.com/api/docs/guides/structured-outputs"

[structural]
require_object_root                 = true
require_additional_properties_false = true
require_all_properties_in_required  = true
max_object_depth                    = 10
max_total_properties                = 5000
max_total_enum_values               = 1000
max_string_length_total             = 120000

[keywords.string]
pattern   = "allow"
minLength = "allow"
maxLength = "allow"
format    = { kind = "restricted", allowed = ["date-time", "date", "time", "duration", "email", "uuid", "ipv4", "ipv6", "hostname"] }

[keywords.number]
minimum          = "allow"
maximum          = "allow"
exclusiveMinimum = "allow"
exclusiveMaximum = "allow"
multipleOf       = "allow"

[keywords.array]
minItems    = "allow"
maxItems    = "allow"
uniqueItems = "warn"
contains    = "unknown"
prefixItems = "warn"

[keywords.object]
patternProperties     = "unknown"
additionalProperties  = { kind = "restricted", allowed = ["false"] }
unevaluatedProperties = "unknown"
propertyNames         = "unknown"
minProperties         = "unknown"
maxProperties         = "unknown"

[keywords.composition]
anyOf             = "allow"
oneOf             = "unknown"
allOf             = "forbid"
not               = "forbid"
if_then_else      = "forbid"
dependentRequired = "forbid"
dependentSchemas  = "forbid"

[keywords.refs]
ref_to_root_only = false
recursive_refs   = "allow"
external_refs    = "forbid"

[rules]
"OAI-S001-no-empty-object"              = "error"
"OAI-S002-recursive-depth-bound"        = "warn"
"OAI-R001-required-must-list-all-props" = "error"
```

### 7.3 Distribution

Profiles ship in the `schemalint-profiles` crate, released independently from the engine. The `--profile-dir` flag accepts a local path for vendored or in-development profiles. Project configuration pins exact profile revisions.

---

## 8. Rule System

### 8.1 Rule Classes

**Class A — Profile-derived rules.** For every keyword marked `forbid`, `strip`, `warn`, or `unknown` in a profile, the engine generates a rule of the form `<PROVIDER>-K-<KEYWORD>`. These do not require code; the profile drives generation.

**Class B — Hand-written semantic rules.** For checks that are not single-keyword presence tests. Examples: cycle depth bounds, total enum cardinality, "all properties in required," empty-object detection, `additionalProperties: {}` detection, discriminator hints for `anyOf` over objects.

### 8.2 Rule Trait and Registration

```rust
pub trait Rule: Sync {
    fn id(&self) -> &'static str;
    fn default_severity(&self) -> Severity;
    fn check(&self, schema: &Schema, ctx: &mut LintCtx);
}

pub struct LintCtx<'a> {
    pub profile: &'a Profile,
    pub config:  &'a Config,
    diagnostics: Vec<Diagnostic>,
}

pub struct Diagnostic {
    pub code:     String,        // e.g. "OAI-K-minimum"
    pub severity: Severity,
    pub message:  String,
    pub pointer:  JsonPointer,
    pub span:     Option<Span>,
    pub hint:     Option<Hint>,
}
```

**Auto-registration.** Class B (hand-written) rules are auto-registered at compile time via `inventory` or `linkme` distributed slices. Adding a rule requires only authoring the `impl Rule` — no edits to a central dispatch file. The same registry drives auto-generated rule documentation (§9). Reach for a proc-macro only when the distributed-slice ergonomics actually hurt.

### 8.3 Composition

Multiple profiles can be active in a single run via repeated `--profile` flags. The engine emits the union of forbid/strip/warn rules. Each diagnostic identifies which profile produced it.

**Multi-profile is the primary use case.** Most teams adopting `schemalint` target both OpenAI and Anthropic. The canonical invocation is `schemalint check --profile openai.so.2026-04 --profile anthropic.so.2025-11 src/schemas/`. The value proposition is not "lint your schemas" but "write one schema that works on both providers." Rule design, documentation, and examples must reflect this framing.

### 8.4 Configuration

Project configuration lives in one of: `[tool.schemalint]` in `pyproject.toml`, `"schemalint"` field in `package.json`, or a standalone `schemalint.toml`.

```toml
[lint]
profiles = ["openai.so.2026-04-30", "anthropic.so.2026-04-30"]
fail_on  = "error"

[lint.severity]
"OAI-K-uniqueItems"      = "off"
"ANTH-K-format-restrict" = "warn"

[ingest]
include = ["src/schemas/*.json", "src/**/models.py"]
exclude = ["**/test/**", "**/__pycache__/**"]

[ingest.python]
discover = "auto"

[ingest.typescript]
discover = "auto"
```

Inline ignores follow ruff conventions:

```python
class Order(BaseModel):
    quantity: int = Field(ge=1)  # schemalint: ignore[OAI-K-allOf]
```

---

## 9. Rule Catalogue (v1)

### OpenAI Structured Outputs Rules

| ID | Provider | Default | Description |
|---|---|---|---|
| `OAI-K-allOf` | OpenAI | error | `allOf` is not supported |
| `OAI-K-not` | OpenAI | error | `not` is not supported |
| `OAI-K-if-then-else` | OpenAI | error | `if`/`then`/`else` is not supported |
| `OAI-K-dependentRequired` | OpenAI | error | `dependentRequired` is not supported |
| `OAI-K-dependentSchemas` | OpenAI | error | `dependentSchemas` is not supported |
| `OAI-K-discriminator` | OpenAI | error | `discriminator` is not standard JSON Schema and not supported |
| `OAI-K-format-restricted` | OpenAI | error | `format` value outside the allowed enum |
| `OAI-K-uniqueItems` | OpenAI | warn | Accepted but not enforced by grammar engine; runtime check required |
| `OAI-K-prefixItems` | OpenAI | warn | Accepted but tuple position semantics not enforced |
| `OAI-S-additionalProperties-required` | OpenAI | error | All objects must declare `additionalProperties: false` |
| `OAI-S-required-lists-all` | OpenAI | error | All `properties` keys must appear in `required` |
| `OAI-S-empty-object` | OpenAI | error | `{"type":"object", "properties":{}}` rejected |
| `OAI-S-empty-additional-props-schema` | OpenAI | error | `additionalProperties: {}` rejected |
| `OAI-S-depth-budget` | OpenAI | error | Object nesting exceeds 10 levels |
| `OAI-S-property-budget` | OpenAI | error | Total property count exceeds 5000 |
| `OAI-S-enum-budget` | OpenAI | error | Total enum value count exceeds 1000 |
| `OAI-S-string-length-budget` | OpenAI | error | Total string length of names exceeds 120000 chars |
| `OAI-S-root-object-required` | OpenAI | error | Root schema must be an object, not `anyOf` |

### Anthropic Structured Outputs Rules

| ID | Provider | Default | Description |
|---|---|---|---|
| `ANTH-K-strip-numeric` | Anthropic | warn | `minimum`/`maximum` etc. silently stripped by SDK |
| `ANTH-K-strip-string-len` | Anthropic | warn | `minLength`/`maxLength` silently stripped |
| `ANTH-K-format-filter` | Anthropic | warn | Format filtered to supported list by SDK |
| `ANTH-S-optional-param-budget` | Anthropic | error | Optional parameters across strict schemas exceed 24 |
| `ANTH-S-union-budget` | Anthropic | error | Parameters using `anyOf` / type arrays exceed 16 |
| `ANTH-S-strict-tool-budget` | Anthropic | error | More than 20 tools with `strict: true` |
| `ANTH-S-citations-incompat` | Anthropic | error | `output_config.format` combined with citations returns 400 |

### Generic Rules (All Providers)

| ID | Provider | Default | Description |
|---|---|---|---|
| `GEN-S-recursive-depth` | All | warn | Recursive `$ref` exceeds profile depth budget |
| `GEN-S-external-ref` | All | error | `$ref` to external URL/file unsupported |
| `GEN-S-discriminator-hint` | All | hint | `anyOf` over objects without a clear discriminator |
| `GEN-S-description-budget` | All | warn | Total description bytes exceed profile budget |

Total: 29 rules at v1 launch.

### 9.1 Rule Documentation

Every rule in the catalogue auto-generates a documentation page at `https://schemalint.dev/rules/<id>`. The page is built from the rule's `id`, `message`, `hint`, and an embedded example schema that triggers the rule. Rule metadata lives in the Rust source; the docs site renders it at build time. This is not a post-v1 nice-to-have — the diagnostic output already references these URLs.

---

## 10. Language Ingestion

### 10.1 Raw JSON Schema

Files matching `**/*.json` (and `.yaml`/`.yml` if explicitly enabled) are read as JSON Schema directly. No language runtime required. This is the language-agnostic backstop for Go, Java, Ruby, .NET, etc.

**Other schema generators.** v1 ingestion supports Pydantic and Zod directly. For `dataclasses-json`, `attrs+cattrs`, `marshmallow`, `valibot`, `arktype`, and OpenAPI specs, the answer is the JSON-file fallback: export to JSON Schema first, then lint. This is a deliberate scope boundary, not a gap. It keeps the core hermetic and fast.

### 10.2 Python — Pydantic

Two entry points:

**Direct file ingestion.** `.json` files in include globs.

**Pydantic model discovery.** The CLI maintains a pool of long-lived Python child processes that communicate via JSON-RPC over stdin/stdout. The helper:

1. Imports the target package.
2. Walks for `pydantic.BaseModel` subclasses.
3. Calls `Model.model_json_schema()` (v2) or `Model.schema()` (v1).
4. Resolves source location via `inspect.getsourcefile()` and `inspect.getsourcelines()`.
5. Heuristically resolves per-field source line via string match against `Model.model_fields[name]`.
6. Returns JSONL records `{schema, source_map}` via JSON-RPC.

The Rust core sends discovery requests and receives responses without spawning a new interpreter per invocation.

**Latency validation (Week 0 spike).** Before M1 begins, benchmark the end-to-end latency of this design on a representative Pydantic codebase (≥ 50 models). Cold import of Pydantic v2 routinely hits 800 ms–2 s; the long-lived process must amortize this across all schemas. If the 500 ms cold-start budget (§12) is breached, pivot to a pre-generation workflow or an embedded interpreter.

**Source-span limitation:** for fields using `Annotated[T, Field(...)]` inside generics, Pydantic does not preserve the original AST location of constraint arguments. Diagnostics resolve to the field declaration line, not the constraint argument. Higher precision requires `ast` or `libcst` walking; deferred to v2.

### 10.3 TypeScript — Zod

Two entry points:

**Direct file ingestion.** `.json` files in include globs.

**Zod schema discovery.** The CLI maintains a pool of long-lived Node child processes that communicate via JSON-RPC over stdin/stdout. The helper:

1. Loads the TypeScript project.
2. Locates `z.object(...)` call expressions.
3. Evaluates each schema in a sandboxed context.
4. Converts via `zod-to-json-schema` (the same library OpenAI's SDK uses).
5. Captures the original `CallExpression` location.
6. Returns JSONL records `{schema, source_map}` via JSON-RPC.

For programmatic use, `@schemalint/zod` exposes:

```ts
import { lint } from "@schemalint/zod";
import { OrderResponse } from "./schemas";
const result = await lint([OrderResponse], { profile: "openai.so.2026-04-30" });
```

---

## 11. Output Formats

### 11.1 Human

```
error[OAI-K-allOf]: keyword 'allOf' is not supported by OpenAI Structured Outputs
   --> src/schemas/order.json:15:8
    |
 15 |       "allOf": [...]
    |       ^^^^^^^
    |
    = profile: openai.so.2026-04-30
    = schema path: /properties/items
    = see: https://schemalint.dev/rules/OAI-K-allOf

3 issues found (1 error, 2 warnings) across 47 schemas in 312ms
```

### 11.2 JSON

```json
{
  "schema_version": "1.0",
  "tool": { "name": "schemalint", "version": "1.0.0" },
  "profiles": ["openai.so.2026-04-30"],
  "summary": { "errors": 1, "warnings": 2, "files": 47, "duration_ms": 312 },
  "diagnostics": [
    {
      "code": "OAI-K-allOf",
      "severity": "error",
      "message": "keyword 'allOf' is not supported by OpenAI Structured Outputs",
      "pointer": "/properties/items",
      "source": { "file": "src/schemas/order.json", "line": 15, "col": 8 },
      "profile": "openai.so.2026-04-30",
      "hint": null
    }
  ]
}
```

### 11.3 SARIF v2.1.0

Each diagnostic emits a `sarif.Result` with:
- `locations[].physicalLocation` from the source span
- `locations[].logicalLocation` from the JSON Pointer
- Rule definitions in `runs[].tool.driver.rules`

Required for GitHub code scanning, Azure DevOps Advanced Security, and most enterprise CI dashboards.

### 11.4 GitHub Actions

```
::error file=src/schemas/order.json,line=15,col=8,title=OAI-K-allOf::keyword 'allOf' is not supported by OpenAI Structured Outputs
```

### 11.5 JUnit XML

For CI systems that surface JUnit-format results in PR checks (GitLab, Jenkins, CircleCI, others).

### 11.6 Server Mode (`--watch` / JSON-RPC)

The CLI supports a `--watch` mode that emits diagnostics as a streaming JSON-RPC server over stdin/stdout. This is the protocol shape that a future LSP would use. Designing it in v1 costs nothing and prevents a refactor later. The server reuses the same engine, normalizer, and rule registry as the batch CLI; only the emitter changes.

## 12. Performance Requirements

| Workload | Target | Strategy |
|---|---|---|
| Single 200-property nested schema, single profile | < 1 ms | Arena allocation; single-pass rule iteration; profile compiled to `HashMap` once |
| Project of 500 schemas, 3 profiles, cold start | < 500 ms | `rayon` parallel ingest; read-only profile sharing across threads |
| Incremental run after single-file edit | < 5 ms | Cache by file content hash; only changed files re-normalized |
| Monorepo of 5000 schemas, CI cold start | < 5 s | Same parallelism; schemas are independent units of work |

Performance regressions are CI-blocking. Each PR runs a benchmark suite against a fixed corpus; >5% regression on any workload fails the build.

### 12.1 Performance-Critical Implementation Rules

- Compile regexes once, cache by pattern string.
- Build JSON Pointer strings lazily; only allocate for nodes that emit diagnostics.
- Cap source-span string matching to the field's declaration line range.
- Profile rule maps are `HashMap<&'static str, Severity>`, loaded once per process.

---

## 13. Quality and Testing

### 13.1 Test Layers

1. **Unit tests** for the IR, normalizer, profile loader, and each rule.
2. **Snapshot tests** for diagnostic output across all formats.
3. **Property tests** (via `proptest`) for the normalizer: random valid JSON Schema in, IR + JSON Pointer round-trip out.
4. **Conformance tests** against real provider APIs (see §13.2).
5. **Regression corpus** of schemas pulled from public bug reports (OpenAI Community forum, `openai-python` issues, Pydantic AI issue #4438, Anthropic forum). Each corpus entry has expected diagnostics.

### 13.2 Conformance Test Suite

For each profile, a corpus of test schemas with expected provider behavior:
- Schemas expected to pass.
- Schemas expected to fail with a specific provider error code.
- Schemas expected to be silently transformed by the SDK.

**Three-tier strategy.**

1. **Synthetic mock (daily CI).** A local mock server driven directly from the profile TOML simulates provider validation responses. Every PR runs conformance-like tests for free. This catches *configuration regressions in the linter itself*.

2. **Live-API burn windows (weekly).** Run 15-minute sessions against real provider APIs using minimal schemas that exercise one keyword at a time. This catches *provider drift* at a frequency that matters — monthly is too slow to catch changes introduced in the same sprint.

3. **Full conformance (monthly, retained).** The complete corpus runs against live APIs as the ground-truth arbiter. This remains the authoritative signal; the mock and burn windows are early-warning layers.

Drift between any layer and real provider behavior is reported as a profile bug. Estimated $50–200/month in provider credits per profile maintained for the monthly + weekly runs combined. Plan for it from day one.

**No auto-escalation in v1.** The idea of automatically promoting `unknown` keywords to `allow` after N non-rejections is deferred. "Did not error" is not "was enforced." Auto-escalation requires generating violation cases and observing whether the constraint is actually respected, which is a legitimately hard sub-problem. Revisit in v1.1 with a rigorous design.

### 13.3 Coverage Requirements

- Line coverage on core crate: ≥ 90%
- Branch coverage on rule implementations: ≥ 95%
- Every rule has at least one positive test (rule fires) and one negative test (rule does not fire on adjacent valid schemas).

---

## 14. Distribution

| Channel | Artifact | Tooling | Priority |
|---|---|---|---|
| GitHub Releases | Standalone binaries (Linux, macOS, Windows × x86_64, arm64) | `cargo-dist` | v1 blocker |
| PyPI | `schemalint` (CLI bundled), `schemalint-py` (library) | `maturin` | v1 blocker |
| npm | `@schemalint/cli`, `@schemalint/core`, `@schemalint/zod` | `napi-rs`, `pkg-pr-new` | v1 blocker |
| crates.io | `schemalint`, `schemalint-profiles` | `cargo publish` | v1 blocker |
| Homebrew | `schemalint/schemalint/schemalint` | tap repository | v1.1 |
| GitHub Action | `schemalint/action@v1` | composite action wrapping the CLI | v1.1 |
| Docker | `ghcr.io/schemalint/schemalint:<version>` | minimal Alpine image | v1.1 |

PyPI and npm are v1 blockers because Python and TypeScript teams install tools via `pip` and `npm`, not `cargo`. `maturin` and `napi-rs` make packaging from a Rust core nearly free once the core compiles; the work is in first-time setup of the build matrix, not ongoing maintenance. Homebrew, Docker, and GitHub Action are best-effort in v1 and hardened in v1.1.

Binary size target: < 8 MB compressed. All artifacts published from a single GitHub Actions release workflow triggered by tags.

---

## 15. Versioning and Drift Management

### 15.1 Semantic Versioning

The engine, profiles crate, and bindings each version independently under SemVer.

- Engine major: breaking change to CLI flags, configuration format, or output schema.
- Engine minor: new rule classes, new output formats, new ingestors.
- Engine patch: bug fixes, performance improvements.
- Profile patch: any change to a profile's keyword table.
- Profile minor: new profiles added.
- Profile major: format-breaking changes to the profile schema itself.

### 15.2 Profile Pinning

Project configuration pins exact profile revisions:

```toml
[lint]
profiles = ["openai.so.2026-04-30", "anthropic.so.2026-04-30"]
```

Profile updates are explicit user actions. CI does not silently pull newer profiles.

### 15.3 Drift Reporting

The conformance suite (§13.2) runs monthly. When a profile entry diverges from observed provider behavior:

1. An issue is auto-filed in `schemalint-profiles`.
2. A draft PR updates the affected profile with a new dated revision.
3. Maintainers review, run the full corpus, merge.
4. The old profile revision remains available; users opt in to the new revision.

---

## 16. Repository Structure

```
schemalint/
├── Cargo.toml                    (workspace)
├── crates/
│   ├── schemalint/               # IR, normalizer, rule engine, CLI
│   │   ├── src/
│   │   │   ├── main.rs           # CLI binary
│   │   │   ├── lib.rs            # Public API
│   │   │   ├── ir/               # Arena-allocated IR
│   │   │   ├── normalize/        # Schema normalization
│   │   │   ├── rules/            # Built-in rules (auto-registered)
│   │   │   ├── output/           # Diagnostic emitters
│   │   │   └── server.rs         # --watch / JSON-RPC server mode (LSP-ready)
│   │   └── Cargo.toml
│   └── schemalint-profiles/      # Profile data; independent release cadence
│       ├── src/
│       │   └── lib.rs            # Profile types and loader
│       ├── profiles/
│       │   ├── openai/
│       │   │   └── structured-outputs-2026-04-30.toml
│       │   └── anthropic/
│       │       └── structured-outputs-2026-04-30.toml
│       └── Cargo.toml
├── bindings/
│   ├── python/                   # PyO3 + maturin
│   └── node/                     # napi-rs
├── ingestors/
│   ├── schemalint-pydantic/      # Python helper (pip-installable)
│   └── schemalint-zod/           # Node helper (npm-installable)
├── tests/
│   ├── corpus/                   # Regression schemas with expected diagnostics
│   └── conformance/              # Synthetic mock + live-API harness
├── docs/                         # mdBook source (auto-generated rule reference)
└── .github/workflows/
    ├── ci.yml
    ├── release.yml
    ├── conformance.yml           # Synthetic mock (daily) + burn windows (weekly)
    └── conformance-full.yml      # Live-API full corpus (monthly)
```

---

## 17. Milestones

### M0 — Validation Spike (Pre-M1)

- **Child-process latency spike.** Benchmark Pydantic and Zod ingestion via long-lived JSON-RPC processes against the 500 ms cold-start budget. If breached, pivot before M1 begins.
- **Regression corpus curation.** Scrape 50+ real-world schemas from the sources in §20. Annotate each with expected diagnostics. This corpus becomes the acceptance test for all subsequent milestones.

### M1 — Core Engine + OpenAI Profile (Weeks 1–4)

- Single `schemalint` crate (monolithic) + `schemalint-profiles` crate.
- IR, normalizer, profile loader, auto-registered rule registry.
- Schema-to-profile diff tool (minimal CLI that emits raw keyword-profile mismatches).
- 18 OpenAI rules (rule catalogue OpenAI section).
- CLI with human and JSON output only.
- Raw JSON Schema ingestion only.
- Acceptance: all 50 corpus schemas produce expected diagnostics; diff tool runs end-to-end.

### M2 — Anthropic Profile + Multi-Profile + Output Formats (Weeks 5–6)

- 11 Anthropic rules (rule catalogue Anthropic section).
- Multi-profile composition (canonical use case: `--profile openai --profile anthropic`).
- SARIF, GitHub Actions, JUnit output formats.
- Acceptance: existing corpus passes; Anthropic-specific corpus of 25 schemas added.

### M3 — Pydantic Ingestion (Weeks 7–8)

- `schemalint-pydantic` helper with JSON-RPC over stdin/stdout.
- Source-span heuristic resolution.
- Python configuration via `pyproject.toml`.
- Acceptance: lint a representative real Pydantic codebase end-to-end with correct diagnostics on at least 10 known issues.

### M4 — Zod Ingestion (Weeks 9–10)

- `schemalint-zod` helper with JSON-RPC over stdin/stdout.
- TypeScript configuration via `package.json`.
- Acceptance: lint a representative real Zod codebase end-to-end with correct diagnostics on at least 10 known issues.

### M5 — Distribution (Weeks 11–12)

- Core packaging: PyPI, npm, crates.io, GitHub Releases.
- Documentation site published (with auto-generated rule reference).
- Conformance test harness operational: synthetic mock (daily CI) + live-API burn windows (weekly).
- Acceptance: clean install on Linux/macOS/Windows from PyPI, npm, and GitHub Releases; documented invocation from each entry point produces expected output.

### M6 — v1.0 Release (Week 13)

- Tag, release notes, blog post, announcement.
- Acceptance: release artifacts pass smoke tests on all platforms; documentation complete.

Total: 13 weeks for v1.0.

---

## 18. Acceptance Criteria

`schemalint` v1.0 is complete when all of the following hold:

1. CLI installable from PyPI, npm, crates.io, and GitHub Releases on Linux, macOS, and Windows. Homebrew, Docker, and GitHub Action are best-effort v1 targets.
2. Both OpenAI and Anthropic profiles published with dated revisions and source citations.
3. All 29 rules in the v1 catalogue implemented, tested, and documented (with auto-generated rule reference pages).
4. Regression corpus of ≥ 75 schemas; all produce expected diagnostics.
5. Conformance suite runs against live OpenAI and Anthropic APIs (monthly full corpus + weekly burn windows) and produces a passing report.
6. Performance targets in §12 met on the reference benchmark corpus.
7. Documentation site live with rule reference, configuration reference, and getting-started guides for Python and TypeScript projects.
8. Coverage thresholds in §13.3 met.
9. No `unknown` keyword states in the v1 OpenAI or Anthropic profiles for keywords emitted by Pydantic v2 and `zod-to-json-schema`. Less common keywords outside these generators' output may remain `unknown` with an explicit scope note in the profile header.

---

## 19. Operational Cost (Ongoing)

| Item | Estimated Monthly Cost |
|---|---|
| OpenAI API credits for conformance suite | $50–100 |
| Anthropic API credits for conformance suite | $50–100 |
| GitHub Actions minutes (public repo) | $0 |
| Documentation hosting | $0 (GitHub Pages) |
| Domain (`schemalint.dev`) | ~$2 |
| **Total** | **~$100–200/month** |

This cost is non-negotiable for the tool to remain trustworthy. Profiles without conformance verification rot within months.

---

## 20. References

The rule decisions in this document are sourced from the following public artifacts. Each rule in the catalogue cites its source(s) in inline documentation.

- OpenAI Developer Community thread on `minItems` / `maxItems` (Sep 2024 – Feb 2026): documents the historical rejection and 2025–2026 addition of array length keywords to regular structured outputs (fine-tuned models still reject these).
- OpenAI Structured Outputs documentation: `developers.openai.com/api/docs/guides/structured-outputs`.
- Anthropic Structured Outputs documentation: `platform.claude.com/docs/en/build-with-claude/structured-outputs`. Documents SDK silent-stripping behavior for `minimum`, `maximum`, `minLength`, `maxLength`, and format filtering.
- Pydantic AI Issue #4438 (Feb 2026): `OpenAIJsonSchemaTransformer` missing strict-incompatible keys. Establishes the keyword set and the difficulty of maintaining it.
- OpenAI Community thread #1126645 (Feb 2025): "Improve the error messages for JSON Schema Structured Output." Establishes the user-pain motivation.
- Microsoft Q&A entry (Oct 2025): Azure OpenAI tightened structured-output enforcement without announcement, breaking previously-working schemas. Establishes drift risk.
- Aviad Rozenhek's `sanitize_for_openai_schema` gist (Aug 2025): documents the empirical set of OpenAI rejections discovered in production.
- OpenAI Cookbook structured outputs guide.
- Anthropic strict tool use documentation.

All cited sources are URLs in the published profile files; profile maintainers must update citations when sources change.