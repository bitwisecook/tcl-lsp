# KCS: Diagnostics calculation — two-phase architecture and scheduling

## Symptom

A contributor needs to understand why some diagnostics appear instantly and
others after a delay, how the diagnostics worker manages debouncing and
supersession, or needs to add a new diagnostic to the correct tier.

## Context

The LSP server publishes diagnostics in two tiers: a cheap, flicker-safe
**fast tier** for immediate feedback, and the full **deep tier** that runs the
compiler / optimiser passes and the cross-file resolution.  A per-URI
debounced worker owns the lifecycle, with currency guards rather than explicit
task cancellation.

Source: `rust/tcl-lsp-server/src/lib.rs` (`spawn_diagnostics_worker`,
`run_diagnostics_core`, `publish_fast_tier`, `run_deep_diagnostics`,
`is_fast_tier`), `rust/tcl-compiler/src/compiler_checks.rs`
(`run_all_checks`), `rust/tcl-core-types/src/diag_code.rs`
(`DiagCode::refined_by_workspace`)

## Content

### Tier 1 — Fast tier (workspace-independent, lifted off the event loop)

`publish_fast_tier` publishes the subset of the per-file analyser walk that
does **not** depend on workspace state, plus the pure source-style lints.
The partition is a single predicate:

```rust
const fn is_fast_tier(code: DiagCode) -> bool {
    !code.refined_by_workspace()
}
```

`DiagCode::refined_by_workspace()` is the single source of truth and is
exactly `{W120, W123}` — the two codes the deep tier can *retract* once it has
resolved the workspace.  Everything else the analyser produces (syntax,
structure, arity, variable-lifecycle, style, iRules event checks) is fast-tier.
Codes the deep pass only ever *adds* — the compiler / optimiser findings, the
synthesised cross-file arity — are simply absent from the fast tier, which is
additive rather than a retraction.

The lift itself runs on `spawn_blocking`, so it never occupies the event loop,
and delivery goes through `DeliveryCtx::deliver_fast_tier_if_current` — push
only, never priming the pull cache.

### Tier 2 — Deep tier

`run_deep_diagnostics` runs three independent whole-file analyses
concurrently (`tokio::join!`) and then combines them:

- the per-file analyser walk (shared with the fast tier as a `Shared` future,
  so it is computed once);
- the compiler / optimiser checks — `compiler_checks::run_all_checks`, which
  covers SCCP constant branches, GVN full / partial / loop redundancies, the
  intrep-shimmer family (S100/S101, thunking S102, shared-value copy S103,
  byte-array S110), the taint checks, and the iRules module-level checks;
- the cross-file resolution.

The W120 / W123 workspace refinement, the diagnostic lifts, and one
authoritative currency-guarded publish then follow.

Only the downstream refine and lift consume all three, so overlapping them
collapses the deep pass towards its longest single pass.  A deterministic
worker panic in a *secondary* pass degrades that pass to its empty
fallback and still publishes, so the reduced fast-tier set is never left as
the terminal state.

### Scheduling, debouncing, and supersession

```
Document edit (version N)
    │
    └─► Backend::schedule_diagnostics — mark the URI's DiagSlot dirty,
        refresh latest_inputs, spawn the worker if not already running
          │
          ├─► DIAGNOSTICS_DEBOUNCE (50 ms) — a keystroke burst collapses
          │   into one analysis
          │
          └─► run_diagnostics_core
                │
                ├─► the fast tier lands only if the deep pass overruns its
                │   budget (a small or warm file never pays the extra publish)
                │
                └─► run_deep_diagnostics → currency-guarded publish
```

Key properties:
- **Debounce, not cancel**: the worker reads the document's *current* state at
  drain time, so a late lower-version edit cannot clobber a captured job —
  there is no captured job.
- **Currency guards**: a publish only lands if it is still current for the
  document's version; a superseded run is discarded.  `run_diagnostics_core`
  returns whether the version *settled*, and `false` (a salsa cancellation)
  tells the caller to retry the document's latest state.
- **Live config**: `DiagSlot::latest_inputs` is refreshed on every schedule, so
  a toggle change arriving mid-flight is honoured on this run rather than the
  next edit.
- **Monotone quality**: the deep tier is always a strict superset of the fast
  tier for the same version.

### Suppression with `# noqa`

```tcl
set x 42    ;# noqa: O109  — suppress dead store warning
eval $cmd   ;# noqa: *     — suppress ALL warnings on this line
```

The suppression map is `AnalysisResult::suppressed_lines: HashMap<i32,
HashSet<String>>` (`rust/tcl-compiler/src/analyser/types.rs`), built during
semantic analysis and consumed by both tiers before emitting.

### Grouped optimisations

Related optimisation edits share a `group` ID.  The diagnostics publisher
emits one primary diagnostic with others as `DiagnosticRelatedInformation`:

```
Primary: O100 "Propagate constant into expression" (+1 dead store eliminated)
  └─ Related: O109 "Dead store: x is set but never read"
```

The LSP client applies all grouped edits atomically via a single code action.

## Decision rule

- A new analyser-walk diagnostic is fast-tier **by default** — the partition
  is `DiagCode::refined_by_workspace()`, so nothing needs adding unless the
  deep tier can *retract* the code once the workspace resolves.
- A diagnostic that needs `CompilationUnit` data (CFG, SSA, SCCP, taint)
  belongs in the compiler-checks pass (`run_all_checks`), which is deep-tier
  by construction — it is never part of the analyser walk the fast tier
  publishes.
- If a code *can* be withdrawn by workspace knowledge, add it to
  `refined_by_workspace` so the fast tier holds it back; the invariant is
  pinned by `refined_by_workspace_is_exactly_w120_and_w123` in
  `rust/tcl-core-types/src/diag_code.rs`.

## Related docs

- [Diagnostics section in walkthroughs](../../../docs/design/example-script-walkthroughs.md#how-diagnostics-are-calculated)
- [kcs-async-diagnostics-tiering.md](../../../docs/design/compiler/async-diagnostics-tiering.md)
- [kcs-diagnostics-integration.md](../../../docs/design/compiler/diagnostics-integration.md)
- [kcs-troubleshooting-duplicate-diagnostics.md](../../../docs/kcs/kcs-issue-duplicate-diagnostics.md)
