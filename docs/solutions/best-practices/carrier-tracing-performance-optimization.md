---
title: "Speeding Up check-node Discovery: Syntactic Pre-Filtering and V8 Compile Cache"
date: 2026-09-25
module: schemalint
problem_type: performance_optimization
component: node_sidecar
severity: high
related_components:
  - carrier_tracing
  - schema_evaluation
  - measurement_harness
applies_when:
  - "TypeScript checker queries (getResolvedSignature/getSymbolAtLocation/getContextualType) run inside per-item loops"
  - "The sidecar bootstrap (tsx/typescript imports) is a visible fraction of CLI latency"
  - "An optimization looks like a regression but the machine was under external load"
  - "Choosing between measurement-median claims and warm-cluster claims"
symptoms:
  - "Carrier/wrapper tracing consumes ~1.5s of a ~3s check-node run"
  - "CPU profile shows instantiateType*/recursiveTypeRelatedTo dominating, not AST walking"
  - "A code change is correct and algorithmically better but produces zero measurable e2e gain"
tags:
  - typescript
  - performance
  - optimization
  - checker
  - carrier-tracing
  - node
  - measurement
---

# Context

`check-node` on a real 839-file TypeScript project took ~3.05s end-to-end.
Phase attribution (via a probe script that replicates `discover.ts`'s phases
against the target project, plus `--cpu-prof` on the sidecar entrypoint) showed
`collectCarrierTargets` alone was ~1.5s — almost entirely inside the
TypeScript checker (`instantiateType`, `recursiveTypeRelatedTo`), triggered by
`checker.getResolvedSignature` being called for every CallExpression × every
carrier × every hop.

# What worked

**Syntactic pre-filter before checker calls** (−37% median e2e,
`carrier_calls.ts`). For each carrier, derive a conservative superset of the
callee names it can be invoked under; return `undefined` (never filter) when
the name can't be determined syntactically — contextual function types,
`export default`, `export =`, `module.exports`, anonymous/factory-returned
functions. Filter calls by arity first (cheapest), then by name membership
against an alias-expanded set, and only then pay for
`getResolvedSignature`/`getSymbolAtLocation`. On the workload this cut
checker calls 28 → 6 with byte-identical output. The rule that made it safe:
**the filter only ever skips checker work; `callsCarrier` remains the final
source of truth for every call it admits.**

**V8 compile cache at the entrypoint** (−~110ms warm). One guarded line —
`module.enableCompileCache?.()` — at the top of `bin/schemalint-zod.js`, plus
converting the static `import 'tsx'` into `await import('tsx')` because
static ESM dependencies execute before the module body and would bypass the
cache. Node ≥22.1 caches bytecode in the platform default location with
automatic invalidation; Node 18/20 no-op via `?.`. This is not a
schemalint-managed cache — no invalidation machinery to own.

# What didn't work (measured, reverted)

- **Shared eval tempdir + memoized node_modules lookup** (+13ms):
  per-target `mkdtemp`/`symlink`/`existsSync`-walk cost only ~5ms each on a
  4-target workload; the cost is the tsx `import()` of each synthetic module,
  not the filesystem staging. Would matter at high target counts.
- **Merged carrier passes** (call index riding the existing walk, verdict
  WeakMaps, lazy alias collection) (−6ms interleaved): correct and removes
  ~2N redundant AST walks, but residual carrier cost is the handful of
  *true-positive* checker calls — not traversal. Worth reviving only for
  carrier-heavy/multi-hop workloads.

# Measurement discipline that mattered

- Canonical-output gate: hash the JSON with `summary.duration_ms` stripped;
  any experiment that drops a schema or reorders output fails the gate,
  which is what makes "faster" safe to claim.
- Median of warmup + 5 runs. The first worktree run is cold (compile-cache
  write); the warm cluster is what users see.
- External machine load can inject >1s of noise into e2e samples. When a
  batch looks anomalous (monotonic rise, unrelated `build_ms` ballooning),
  interleave control-branch runs A/B/A/B under the same conditions before
  believing a regression.
- Remaining cost is `ts.createProgram` (~700–800ms warm), `import
  typescript` (~200ms), `getTypeChecker` (~185ms) — all inside TypeScript
  itself and fenced off by the "no weakened type-aware resolution / no
  persistent caches" constraints. That's the documented safe ceiling:
  ~40% median improvement end-to-end.
