---
title: Speed up Node schema discovery
date: 2026-08-20
artifact_contract: ce-unified-plan/v1
artifact_readiness: implementation-ready
execution: code
product_contract_source: ce-plan-bootstrap
---

# Speed up Node schema discovery

## Goal Capsule

Reduce the real `check-node` latency observed on the Brain TypeScript project without weakening schema discovery, provider metadata, source maps, failure isolation, or output determinism.

The reported baseline is about 5.6 seconds for one `src/**/*.ts` glob and 10 discovered Zod schemas. The implementation begins with phase timings and changes only the measured dominant path. Success is a repeatable reduction in median end-to-end runtime, with the same discovered targets, coverage counts, and diagnostics.

## Problem Frame

The Rust normalization and rule engine are already benchmarked at sub-millisecond scale. `check-node` instead launches the Node sidecar, parses the TypeScript project, constructs a typed program, resolves provider-facing targets, and evaluates target modules. Runtime evaluation is serial, and synthetic targets each create, import, and remove their own temporary module. There is currently no benchmark covering this end-to-end path.

## Scope

### In scope

- Measure the Brain workload in release mode and attribute time across Node discovery phases.
- Optimize the phase shown to dominate, preferring reuse and batching over new infrastructure.
- Preserve target order and all existing discovery/error semantics.
- Add a deterministic regression test for the optimization invariant.
- Record before/after median timing evidence in the pull request.

### Out of scope

- Changing provider inference or ambiguity behavior.
- Adding a tight wall-clock gate to ordinary CI.
- Replacing the TypeScript compiler, weakening type-aware wrapper/carrier resolution, or adding a dependency.
- Persistent cross-command daemons or disk caches.

## Product Contract

### Requirements

- **R1.** One warmup and at least five measured release-mode runs against Brain must establish the median full CLI time and the median sidecar discovery time.
- **R2.** Temporary phase probes must separate process/bootstrap and dependency loading, configuration/file selection, TypeScript program and target discovery, runtime schema evaluation, and response work; probes must not remain in production output.
- **R3.** The implementation must address the measured dominant phase and improve median Brain runtime materially; target at least 30%, and explain measured constraints if the safe ceiling is lower.
- **R4.** Discovery must return the same target identities, provider/envelope metadata, source maps, failure ordering, and coverage accounting as before.
- **R5.** Ordinary tests must verify behavior or work-count invariants, not machine-sensitive elapsed-time thresholds.
- **R6.** The solution must use existing runtime/compiler facilities and add no dependency or persistent cache.

### Acceptance Examples

- **AE1 (R1-R3).** Five post-warmup runs of the release CLI on Brain show at least a 30% lower median than the five-run baseline, or the PR records the measured constraint that makes the safe ceiling lower, with phase data identifying why.
- **AE2 (R4).** Existing npm discovery tests and Rust `check-node` end-to-end tests pass unchanged apart from deliberate regression coverage.
- **AE3 (R4-R5).** Contextual carrier signatures are collected once per carrier, while existing wrapper/carrier discovery cases return the same targets.
- **AE4 (R4).** Exported targets, mixed exported/synthetic targets, user code that writes stdout, and zero-target discovery retain current behavior.

## Key Technical Decisions

- **KTD1 — Measure before selecting the optimization.** The user explicitly requested a small experiment first. The production change is selected from measured phase dominance rather than assumption.
- **KTD2 — Prefer batching/reuse inside the existing request.** This keeps lifecycle and correctness boundaries intact. Persistent daemons and caches are rejected because they add invalidation and cleanup behavior before evidence requires them.
- **KTD3 — Preserve deterministic ordering.** Any batched or concurrent internal work must return results and failures in original target order. Unbounded parallel evaluation is rejected because stdout redirection is process-global and project module side effects may be order-sensitive.
- **KTD4 — No timing assertion in the normal suite.** CI hosts are noisy. A deterministic work-count/behavior test protects the optimization; real timings are PR evidence and may later inform a dedicated benchmark gate.

## High-Level Technical Design

`schemalint check-node` remains a Rust CLI using the existing JSON-RPC sidecar. The experiment captures full-command and direct-sidecar medians, then adds temporary local phase probes around the current `discoverZodSchemas` stages. The probes are removed before review.

Typed target resolution repeatedly collected the same contextual signatures for every carrier/call-site pair. Collect each carrier's signatures once before walking call sites, then reuse that immutable result. This retains the complete `Program` and `TypeChecker` semantics and changes only duplicated checker work.

Experiments that disabled ambient TypeScript declarations or evaluated user modules concurrently were faster but rejected: the former can hide wrappers supplied by configured types, and the latter changes import-time side-effect ordering. Synthetic batching was not retained because Brain has one target per source file, so it does not address this workload.

## Implementation Units

### U1 — Establish the performance baseline and phase attribution

**Files:** no retained production changes; optional temporary probes in `npm/schemalint/src/discover.ts`.

**Work:** Build the release binary once. From Brain, run one warmup and five measured `check-node -S 'src/**/*.ts' -p openai.so.2026-04-30 -f json` invocations. Measure the sidecar directly and temporarily time startup/bootstrap plus the four internal discovery phases. Record target composition, especially synthetic versus exported targets. Remove probes before U2.

**Test scenarios:** repeated runs use identical arguments; discovery counts stay constant; medians exclude the warmup; full CLI and sidecar measurements agree on whether Node discovery dominates.

### U2 — Implement the smallest measured optimization

**Files:** `npm/schemalint/src/target_resolution.ts` and generated npm `dist` output.

**Work:** Precompute contextual signature declarations once per carrier before the source-file walk and reuse them for every call-site comparison. Preserve the carrier object and existing matching helper; add no cache, worker pool, or new abstraction.

**Test scenarios:** existing carrier, wrapper, alias, and nested-call discovery tests return unchanged targets. Brain's canonical JSON output, excluding duration, hashes identically before and after.

### U3 — Lock correctness and remeasure

**Files:** no new test file; existing discovery and Node end-to-end suites cover the unchanged semantics.

**Work:** Run npm and Rust Node suites, workspace gates, compare canonical Brain output, then repeat the Brain benchmark protocol and capture before/after medians and percentage change. A checker-call-count seam was rejected as test-only production complexity; output equivalence and the direct loop placement protect this local memoization adequately.

**Test scenarios:** AE2-AE4 plus five-run post-change measurement under the same environment and command.

## Verification Contract

- `npm test` in `npm/schemalint`
- `cargo test --test node_tests`
- `cargo test --workspace --no-fail-fast`
- `cargo clippy --workspace -- -D warnings`
- `cargo fmt --all -- --check`
- `git diff --check`
- Repeat U1's Brain benchmark and compare medians, counts, and diagnostic identity.

## Risks and Mitigations

- **Module side effects or global stdout races:** keep evaluation serial at the module/group level and redirect stdout around the complete operation.
- **Synthetic modules needing different dependency context:** group only compatible targets from the same source file and retain nearest-`node_modules` resolution.
- **A misleading warm cache:** use identical warmup and measured protocols before and after, and report raw samples with medians.
- **Optimizing the wrong phase:** U1 is a hard gate; if program construction dominates, do not ship synthetic batching as the headline fix.

## Definition of Done

- Baseline and post-change samples are recorded with medians and identical discovery counts.
- The measured dominant phase is reduced with a minimal, dependency-free change.
- Existing discovery semantics and output ordering are preserved by deterministic tests.
- All Verification Contract gates pass.
- The branch is pushed and an open PR contains the timing evidence.
