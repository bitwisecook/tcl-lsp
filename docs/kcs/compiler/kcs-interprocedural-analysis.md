# KCS: Interprocedural analysis — ProcSummary construction

## Symptom

A contributor needs to understand how the compiler reasons about cross-procedure
behaviour (purity, constant folding eligibility, effect propagation), or needs
to debug why ICIP (O103) does or does not fold a procedure call.

## Context

`InterproceduralAnalysis` builds `ProcSummary` objects for each procedure by
first collecting local facts (`ProcLocalSummary`), then running a transitive
closure over the call graph to propagate effects.  Summaries are consumed by
ICIP, ADCE, and taint analysis.

Source: [`core/compiler/interprocedural.py`](../../../core/compiler/interprocedural.py)

## Content

### Three-phase summary construction

**Phase 1 — Local facts (`ProcLocalSummary`):**

For each procedure, walk the IR and record:
- Parameters and arity
- Internal calls (`calls` tuple)
- Barrier presence (`eval`/`uplevel`)
- Global writes, unknown calls
- Effect regions (reads/writes)
- Return-value dependency on parameters

**Phase 2 — Transitive closure:**

Iterate over the call graph until fixpoint:
1. Leaf procedures (no callees) have final summaries immediately.
2. For callers, propagate callee effects upward:
   - If callee has barrier → caller inherits barrier.
   - If callee writes global → caller inherits global write.
   - Effect reads/writes are unioned.

**Phase 3 — Constant folding eligibility:**

A procedure qualifies for `can_fold_static_calls` when:
- No barrier
- No unknown calls
- No global writes
- Return depends only on parameters
- Body is a single expression

### Worked example

```tcl
proc helper {x} {
    return [expr {$x * 2}]
}

proc main {a b} {
    set r [helper $a]
    puts $r
}
```

`::helper` local summary: no calls, no barrier, return depends on `x`,
single-expression body → `can_fold_static_calls=True`.

`::main` calls `::helper` (pure) and `puts` (LOG_IO write) →
`pure=False`, `effect_writes=LOG_IO`.

When the optimiser encounters `[helper 21]`, it evaluates the body with
`x₁ = 21` → `21 * 2 = 42` → O103 fires.

### Call resolution

`resolve_internal_call(callee_name, caller_qname, known_procs)`:
1. Extract namespace parts from the caller's qualified name.
2. Try `::caller_namespace::callee_name` first.
3. Walk up the namespace hierarchy to `::callee_name` (global).
4. Return the first match, or `None` if the callee is external.

## Decision rule

- If a procedure call is not being folded by O103, check `can_fold_static_calls`
  on its summary — the most common blockers are `has_barrier` or
  `has_unknown_calls`.
- To expose a new procedure-level fact, add it to `ProcLocalSummary`, ensure
  transitive closure propagates it, and expose it on `ProcSummary`.
- Summaries are recomputed per `CompilationUnit` — they are not cached across
  compilation runs.

## Related docs

- [Example 24 in walkthroughs](../../example-script-walkthroughs.md#example-24-interprocedural-analysis--summary-construction)
- [GLOSSARY.md — ICIP](../../GLOSSARY.md#icip)
- [kcs-compiler-pipeline-overview.md](kcs-compiler-pipeline-overview.md)
