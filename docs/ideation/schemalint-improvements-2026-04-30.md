# schemalint Improvement Ideas

**Date:** 2026-04-30  
**Focus:** Open-ended strategic and architectural improvements for the schemalint project (greenfield, pre-implementation)  
**Source:** SOW (`sow.md`) + codebase scan  
**Agents dispatched:** 8 (codebase scan, learnings, web research [unavailable], 6 ideation frames)

---

## Grounding Summary

- **Project shape** — `schemalint` is a greenfield Rust CLI for static analysis of JSON Schemas against LLM provider capability profiles. Only `sow.md` exists; no code, crates, or CI. The SOW prescribes arena-allocated IR, profile-driven rules (Class A/B), a five-state severity model, child-process ingestion for Pydantic/Zod, a 13-week timeline across 6 milestones, 28 rules, 14 deliverables, and 5 output formats.
- **Pain points** — Zero implementation despite a 737-line spec; highly ambitious 13-week scope; no regression corpus yet; $100–200/mo operational cost before users; workspace fragmentation (5 crates) may slow early velocity; child-process ingestion is an unvalidated latency risk.
- **Leverage points** — Start with a minimal crate structure; build the regression corpus before writing rules; lean on the `unknown` keyword state to ship faster; validate child-process ingestion latency early.
- **External context** — Web research unavailable in this environment.
- **Past learnings** — None (`docs/solutions/` does not exist).

---

## Ranked Survivors (Top 7)

### 1. Corpus-Driven Monolith Bootstrap
**Frame:** Cross-cutting (Pain + Inversion + Leverage)  
**Summary:** Restructure M1 around three sequential deliverables: (1) collapse the 5-crate workspace into a single `schemalint` crate, (2) spend week 1 curating a 50-schema regression corpus from public bug reports and provider forums, and (3) build a minimal schema-to-profile diff CLI as the first executable artifact in week 2. Defer the full rule engine, output formats beyond human/JSON, and all bindings until the foundation is proven.  
**Warrant:** `direct:` Grounding summary flags "zero implementation despite 737-line spec," "workspace fragmentation (5 crates) may slow early velocity," and "no regression corpus yet." SOW §16 prescribes five crates before any code; §17 M1 demands 17 rules and 50 corpus schemas with expected diagnostics.  
**Why it matters:** This converts M1 from speculative parallel development into a concrete, verifiable foundation. The diff tool gives users value in week 2. The regression corpus turns rule development into a pass/fail exercise against empirical reality. The monolith eliminates cross-crate refactoring tax during the most volatile phase.  
**Meeting test:** Can we restructure M1 around a diff tool and corpus rather than 17 rules?

### 2. Keep `unknown` keyword states in v1 profiles
**Frame:** Pain + Assumption-Breaking + Leverage  
**Summary:** Retain the `unknown` keyword state in v1 OpenAI and Anthropic profiles, default `--strict-unknown` to `warn`, and remove SOW §18 acceptance criterion #10 ("No `unknown` keyword states") as a v1 blocker. Map every unverified keyword explicitly to `unknown` with a `source_url = "unverified"` placeholder.  
**Warrant:** `direct:` SOW §7.1 defines `unknown` as a first-class state ("Unverified by the conformance suite") with default severity `warning`. Grounding summary explicitly lists "lean on `unknown` keyword state to ship faster" as a leverage point. Criterion #10 contradicts this by mandating elimination.  
**Why it matters:** Forcing full keyword classification for two moving targets before launch adds a research overhead that could consume weeks. The `unknown` state exists precisely to signal honest ambiguity. Shipping with `unknown` keywords is safe under `warn` severity and prevents false confidence.  
**Meeting test:** Is a profile with explicit `unknown` keywords acceptable for a v1.0 release?

### 3. Child-process ingestion spike in Week 0
**Frame:** Pain + Assumption-Breaking + Leverage  
**Summary:** Before M1 coding begins, build a throwaway prototype that spawns Python and Node child processes to measure end-to-end latency on a 500-schema monorepo. Define a hard budget (e.g., 100 ms per process or 500 ms total cold start). If the overhead exceeds the budget, pivot immediately to an alternative (build-time helpers, embedded interpreters, or raw JSON default).  
**Warrant:** `direct:` Grounding summary tags "child-process ingestion is unvalidated latency risk" and lists "validate child-process ingestion early" as a leverage point. SOW §12 target: "Project of 500 schemas, 3 profiles, cold start < 500 ms." SOW §10.2-10.3 prescribes child-process ingestion without latency validation.  
**Why it matters:** If child-process overhead pushes cold start over 500 ms, the architecture fails its own performance contract before M3 begins. Discovering this in week 10 leaves no runway to re-architect. A one-day spike in week 0 preserves 12 weeks to pivot.  
**Meeting test:** Should we run a one-day latency spike before M1 starts, and pivot to an alternative if it misses the target?

### 4. SDK-Derived Profiles via Static Analysis
**Frame:** Inversion + Cross-Domain Analogy  
**Summary:** Instead of relying solely on documentation research, write static analyzers for the official OpenAI (`openai-python`) and Anthropic SDK repositories that extract keyword allow-lists, stripping behavior, and error messages directly from source code. Emit TOML profile drafts automatically on each SDK release.  
**Warrant:** `external:` Pydantic AI Issue #4438 (cited in SOW §20) documents the empirical difficulty of maintaining keyword compatibility by hand. Adjacent pattern: `mypy` and `pyright` derive type stubs from CPython source; AWS policy linting tools scrape Boto3 internals.  
**Why it matters:** Provider documentation is incomplete and lags behind SDK behavior. Scraping the SDK source that actually performs the stripping/rejection captures ground truth, reduces profile maintenance labor, and surfaces undocumented changes automatically. This compounds: every SDK release becomes a profile update signal.  
**Meeting test:** Should we invest in SDK source scrapers to auto-generate profiles?

### 5. Progressive Conformance Automation
**Frame:** Cross-cutting (Pain + Inversion + Analogy)  
**Summary:** Replace the fixed $100–200/month monthly live-API conformance suite with a three-tier strategy: (1) a TOML-driven synthetic mock server for daily CI that simulates provider validation from profiles, (2) weekly 15-minute "burn windows" against live APIs using minimal schemas per keyword, and (3) auto-escalation of `unknown` keywords based on observed behavior (surviving N runs without rejection → `allow`; causing errors → `forbid`).  
**Warrant:** `direct:` Grounding summary flags "$100-200/mo operational cost before users." SOW §13.2 schedules monthly live-API runs; §19 states "Profiles without conformance verification rot within months." SOW §7.1 and §15.3 describe the `unknown` state and automated drift reporting.  
**Why it matters:** This slashes the operational burn while improving feedback frequency from monthly to daily. The mock gives developers immediate conformance feedback on every PR. The burn windows catch drift faster and cheaper. Auto-escalation turns conformance spend into a self-maintaining profile hardening system.  
**Meeting test:** Can we accept a hybrid conformance strategy that trades full integration tests for speed and cost?

### 6. v1 Distribution via Cargo and GitHub Releases Only
**Frame:** Inversion + Assumption-Breaking  
**Summary:** Defer PyPI wheels, npm packages, Homebrew formula, GitHub Action, and Docker image to a post-v1 "packaging sprint." Release v1 exclusively as a `cargo install`-able crate and a GitHub Release binary. Bindings and additional channels ship once the core is proven.  
**Warrant:** `direct:` SOW §5.1 states "CLI is the product. Bindings are convenience surfaces. 95% of usage is `schemalint check` invoked from a script or CI step." Yet SOW §14 and §17 M5 schedule seven distribution channels for weeks 11–12. Grounding summary flags "14 deliverables" alongside "highly ambitious 13-week scope."  
**Why it matters:** Packaging for five ecosystems with native bindings (PyO3, napi-rs), cross-compilation, and marketplace publishing consumes two full milestones. Since 95% of usage is CLI invocation, a static binary satisfies early adopters; bindings can ship once the core is proven and there is demand.  
**Meeting test:** Does delaying PyPI and npm packages until v1.1 kill adoption among Python and TypeScript teams?

### 7. Self-Registering `Rule` Trait with Derive Macro
**Frame:** Leverage and Compounding  
**Summary:** Define a `Rule` trait and a small proc-macro (`#[derive(RegisterRule)]`) that automatically enrolls each rule implementation in a global registry at compile time. Adding a new rule should never require editing a central dispatch file.  
**Warrant:** `reasoned:` The SOW prescribes 28 rules. A central match statement or dispatch table scales linearly in merge conflicts and review friction as contributor count grows. Auto-registration is O(1) effort per rule. `insta` and `ruff` use similar patterns for extensibility.  
**Why it matters:** At 28 rules, a manual registry becomes a bottleneck for every PR. A self-registering trait makes the rule set open-ended: external contributors can add rules without touching core engine code, and the team can ship rules in parallel without coordination overhead. The same registry can later drive documentation and profile generation automatically.  
**Meeting test:** Is the upfront cost of a proc-macro acceptable to eliminate the central registry as a merge bottleneck?

---

## Notable Rejections

| Idea | Reason for rejection |
|---|---|
| Crowdsourced conformance (I4) | Trust issues with anonymous data before any user base exists; too speculative. |
| Build-time ingestion (I8) | Premature architectural commitment; measurement (survivor #3) must come first. |
| Pre-compiled profile rules (A8) | Premature optimization; TOML parsing is not the performance bottleneck. |
| WASM ingestion (F3) | Speculative alternative; spike measurement must precede investment. |
| Severity agnosticism (F5) | Undermines the carefully designed five-state model without empirical evidence. |
| User-authored DSL (F8) | Overkill for 28 rules; adds interpreter complexity without proven need. |
| Free-tier architecture (F7) | Over-rotates on acceptable operational cost; verification is core value. |
| Rule-first, IR-second (F2) | Risks wrong abstractions that contradict performance targets. |
| Stratigraphic inheritance (C2), SCRAM auto-escalation (C7), Air-traffic handoff (C6) | Valuable but v1.1/v2 scope; too complex for pre-implementation. |
| Snapshot testing via `insta` (L6), JSON Schema Test Suite (L8) | Good practices, not ideation-level strategic moves. |

---

## Dimension Spread

| Dimension | Covered by |
|---|---|
| Architecture / foundation | #1 (monolith + diff tool), #3 (ingestion spike) |
| Scope / velocity | #2 (`unknown` states), #6 (defer distribution) |
| Operations / cost | #5 (progressive conformance) |
| Strategic / long-term leverage | #4 (SDK-derived profiles), #7 (auto-registration) |
| Quality / correctness | #1 (regression corpus), #4 (ground-truth profiles) |

---

## Checkpoints

- Raw candidates: `/tmp/compound-engineering/ce-ideate/69f34361/raw-candidates.md`
- Ranked survivors: `/tmp/compound-engineering/ce-ideate/69f34361/ranked-survivors.md`
