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

### Changed

### Fixed

### Removed

## [1.0.0] - 2026-05-05

### Added
- v1.0 release with multi-channel distribution (cargo-dist, maturin, npm)
- Coverage and benchmark CI gates
- Automated release workflow with cross-platform smoke tests
- Docusaurus documentation site

## [0.1.0] - Unreleased
