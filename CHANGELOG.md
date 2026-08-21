# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.2.1](https://github.com/1nder-labs/schemalint/compare/v1.2.0...v1.2.1) (2026-08-21)


### Bug Fixes

* **release:** install dist per runner instead of reusing the Linux binary ([#16](https://github.com/1nder-labs/schemalint/issues/16)) ([3cecac7](https://github.com/1nder-labs/schemalint/commit/3cecac7da1e5ef11e309b46012b89fe18d949a27))

## [1.2.0](https://github.com/1nder-labs/schemalint/compare/v1.1.1...v1.2.0) (2026-08-21)


### Features

* cite provider evidence for every rule ([#17](https://github.com/1nder-labs/schemalint/issues/17)) ([3392668](https://github.com/1nder-labs/schemalint/commit/33926688f44d786b02ca7a366ab22f129bf62f0e))

## [1.1.1](https://github.com/1nder-labs/schemalint/compare/v1.1.0...v1.1.1) (2026-08-20)


### Bug Fixes

* correct lint accuracy and provider-profile fidelity ([#13](https://github.com/1nder-labs/schemalint/issues/13)) ([09ef119](https://github.com/1nder-labs/schemalint/commit/09ef119a3b83be52bb324d18e41365afc4f56f9a))
* discover schemas that reach a provider through a wrapper ([#14](https://github.com/1nder-labs/schemalint/issues/14)) ([114b96f](https://github.com/1nder-labs/schemalint/commit/114b96f40507b82264b7c4aed2c2f773bf5a1ac8))

## [1.1.0](https://github.com/1nder-labs/schemalint/compare/v1.0.1...v1.1.0) (2026-07-03)


### Features

* **cli:** auto-detect provider + profile aliases so --profile is optional ([#8](https://github.com/1nder-labs/schemalint/issues/8)) ([a232388](https://github.com/1nder-labs/schemalint/commit/a232388f849d047ebea7c82b531aefde60910d1f))

## [Unreleased]

### Fixed
- A `definitions` entry shadowed by a same-named `$defs` entry is now linted. It was never allocated, so a forbidden keyword inside it reported nothing while the provider still received that subtree
- A `$ref` to any in-document pointer now resolves, not only `#/$defs/<name>` and `#/definitions/<name>`. A Zod 3 project that reuses one sub-schema emits `{"$ref": "#/properties/a"}` and previously failed to lint entirely
- The draft-07 tuple form `items: [A, B]` now normalizes. Zod 3 emits it for `z.tuple()`, and it previously aborted the whole target
- A `$ref` cycle is now detected wherever it closes, including through several levels of nesting or with no `$defs` entry involved, and is reported once per named definition
- JSON pointers are escaped per RFC 6901 in the Rust normalizer and both sidecars, so a property, pattern, or definition name containing `/` or `~` addresses the right node
- A nested diagnostic falls back to its nearest mapped ancestor for file and line instead of reporting no location
- An empty Zod discovery names its cause: no file on disk, files outside the TypeScript program, or files with no schema

### Added
- A profile-level `unknown_keyword_policy` (allow / warn / forbid, default warn) reporting keywords the engine does not recognize, and traversal into subschemas nested inside `unevaluatedItems`, `additionalItems`, `contentSchema`, and draft-07 `dependencies`
- Anthropic rules for recursive schemas and non-object roots, both documented as unsupported

### Changed
- Anthropic's `max_optional_properties` and `max_union_properties` are removed. Anthropic publishes no such limits, so they rejected schemas the API accepts
- OpenAI's `minLength`, `maxLength`, `patternProperties`, and `discriminator` now warn. None appears in OpenAI's supported list, and none is documented as rejected

### Upgrade notes
- These fixes make the linter see schema content it previously could not, so a schema you did not edit can newly report errors: a shadowed `definitions` entry, a keyword nested inside an unrecognized applicator, a recursive schema, or a non-object root under the Anthropic profile. That is the defect being corrected, not a regression
- The unknown-keyword rule and the four OpenAI reclassifications emit warnings only, which never affect the exit code
- Both dated profiles were corrected in place, so a pinned `openai.so.2026-04-30` or `anthropic.so.2026-04-30` resolves to a changed ruleset. The date names the provider's capability snapshot, not a schemalint release: none of these changes reflect a provider changing behavior, they correct what we believed the provider accepted on that date. A new dated profile is reserved for an actual provider-side change

### Added
- Multi-provider static analysis for JSON Schema compatibility with LLM structured-output APIs
- Built-in capability profiles for OpenAI and Anthropic Structured Outputs
- Profile-aware rule engine with Class A (keyword), Class B (structural), and semantic rules
- CLI with check, check-python, check-node, and server subcommands
- Multiple output formats: human, json, sarif, gha, junit
- Python and Node.js ingestion helpers via JSON-RPC
- Regression corpus with 80+ synthetic schemas
- JSON output schema 1.1 with additive target, coverage, failure, and warning data
- Current OpenAI, Anthropic, and AI SDK 6 structured-output helper discovery
- Packed-runtime verification for Node 18/20/22 and wheel verification for Python 3.9+ with Pydantic 1.10/2.x

### Changed
- Exit 0 now requires complete discovery, conversion, attribution, and lint coverage in addition to zero error diagnostics
- The PyPI wheel installs the public `schemalint` command and bundles the Pydantic sidecar
- The npm runtime bundles its TypeScript loader and handles Zod v3, v4 from 4.0.1, current v4, and `zod/mini`
- Provider budgets and envelope checks now match retained provider evidence, including OpenAI enum-string budgets
- Release packaging uses one immutable, digest-anchored artifact bundle for smoke tests and publication

### Fixed
- Node and Python discovery failures can no longer be reported as successful lint runs
- Provider identity and required output/tool metadata are retained per SDK usage site
- Invalid profile keyword names or restriction shapes are rejected during profile loading

### Removed
- Persistent on-disk schema caching; caches are bounded and process-local

## [1.0.0] - 2026-05-05

### Added
- v1.0 release with multi-channel distribution (cargo-dist, maturin, npm)
- Coverage and benchmark CI gates
- Automated release workflow with cross-platform smoke tests
- Docusaurus documentation site

## [0.1.0] - Unreleased
