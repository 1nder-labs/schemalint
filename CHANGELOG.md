# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.1.0](https://github.com/1nder-labs/schemalint/compare/v1.0.1...v1.1.0) (2026-07-03)


### Features

* **cli:** auto-detect provider + profile aliases so --profile is optional ([#8](https://github.com/1nder-labs/schemalint/issues/8)) ([a232388](https://github.com/1nder-labs/schemalint/commit/a232388f849d047ebea7c82b531aefde60910d1f))

## [Unreleased]

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
