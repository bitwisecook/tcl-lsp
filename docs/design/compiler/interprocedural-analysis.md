# Interprocedural analysis — ProcSummary construction

How the compiler reasons about cross-procedure behaviour — purity,
constant-folding eligibility, and effect propagation — and how the resulting
summaries decide whether ICIP (O103) folds a call.

`build_interprocedural_analysis` builds a `ProcSummary` for each procedure by
first collecting per-procedure scratch facts (`LocalFacts`), then running
fixpoints over the call graph to propagate purity and effects.  Summaries are
consumed by ICIP (O103), the elimination passes, unused-proc detection (O124),
and taint analysis.

Source: `rust/tcl-compiler/src/interprocedural.rs`

### Summary construction

```rust
pub fn build_interprocedural_analysis(
    ir_module: &crate::ir::Module,
    registry: &tcl_registry::CommandRegistry,
    dialect: Option<&str>,
    object_types: ObjectTypeMap<'_>,
    identities: &crate::realm::CommandBindingRealm,
) -> InterproceduralAnalysis
```

**Step 1 — Local facts (`scan_all_procs` → `LocalFacts`):**

`LocalFacts` is private scratch state, one per procedure.  Walking each body
records:

- Direct calls resolved to another proc in the module (`direct_calls`)
- Barrier presence (`Statement::Barrier`, or a direct call to
  `eval` / `uplevel` / `interp eval` / `namespace eval`)
- Local purity, global writes, unknown calls
- Local effect regions (reads/writes)
- One `ReturnKind` per `return` statement

**Step 2 — Closures and fixpoints:**

- `compute_all_transitive_calls` closes `direct_calls` into the full
  reachable set.
- `fixpoint_pure` takes the least fixpoint of "locally pure ∧ every direct
  callee pure".
- `fixpoint_effects` unions each procedure's local effect regions with its
  transitive callees'.

**Step 3 — Materialisation (`materialise_summaries`):**

`writes_global` and `has_unknown_calls` are OR-ed across the whole transitive
closure, not copied from local facts, so a proc that only writes a global via
a callee still reports `true`.  `has_barrier` is **not** widened this way — it
stays the procedure's own local fact, which is why O124's dynamic-dispatch
guard checks `has_barrier` on every reachable proc individually rather than
just on the event handlers.  `summarise_returns` collapses the `ReturnKind`
list into `(returns_constant, constant_return, return_passthrough_param,
return_depends_on_params)`: a constant return needs *every* return to be the
same literal, a passthrough needs every return to be `$param` for the same
parameter, and anything else contributes to `return_depends_on_params`.

**Step 4 — Method summaries** (`build_method_summaries`, below).

### Constant-folding eligibility

```rust
let can_fold = is_pure && (returns_constant || passthrough.is_some());
```

That is the whole rule.  Barrier, unknown-call, and global-write freedom are
subsumed by `pure`; there is no "single expression body" condition.  A
procedure whose return merely *depends on* its parameters —
`return [expr {$x * 2}]` — is `UsesParam`, so `can_fold_static_calls` is
`false`.

Such a procedure can still be folded at a *constant* call site: O103's
command-substitution path (`try_o103_proc_fold` in
`rust/tcl-compiler/src/optimiser/propagation.rs`) falls back to `summary.pure`
plus `evaluate_proc_with_constants`, re-running the callee body under the
literal arguments.  `can_fold_static_calls` gates only the
argument-independent fold, which replaces the call with
`summary.constant_return`.

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

`::helper`: no calls, no barrier, `pure: true`; its single return is
`UsesParam(["x"])`, so `returns_constant: false`,
`return_depends_on_params: ["x"]`, and `can_fold_static_calls: false`.

`::main` calls `::helper` (pure) and `puts` (a `FileIo` write, whose coarse
region is `EffectRegion::NONE`) → `pure: false`.

When the optimiser meets `[helper 21]` it takes the `summary.pure` fallback,
evaluates the body with `x = 21` → `42`, and O103 fires.  A `[helper $n]` with
no constant for `n` folds neither way.

### TclOO method summaries (`MethodSummary`)

When `ir_module.methods` is populated (TclOO method bodies lifted by
lowering — see [data-structure-reference](data-structure-reference.md)),
`build_method_summaries` also builds a `MethodSummary` (a struct wrapping
a `ProcSummary` in its `base` field, plus `class_name`, `method_kind`,
`reads_instance_vars` / `writes_instance_vars`, `calls_my`, and `calls_next`)
for each method, keyed by `{class_qname}::{method_name}` on
`InterproceduralAnalysis::methods`.

Three of those fields are declared but not yet populated:
`reads_instance_vars` is always empty, `calls_my` is always empty, and
`calls_next` is always `false` — read-set and MRO-dispatch tracking are not
implemented, and the purity gate consumes only `base.pure`.
`writes_instance_vars` *is* populated.  `base.can_fold_static_calls` is
hard-wired `false`: methods are never folded at static call sites.  A method
retained in `Module::redefined_methods` is scanned into the *same* accumulators
as its primary body, so the summary describes the union of every body a
dispatch may run.

Method purity is **conservative by design** — a method is `pure` iff:
- its own body has no observable side effect (no barrier, no unknown call,
  no global write, no local effect-writes), **and**
- it writes no in-scope instance variable (class-level `variable` decls +
  the method's own `variable` decls — a write there mutates object state
  that survives the call), **and**
- every *proc* it calls is pure.

A `my <method>` / `next` self-dispatch surfaces as an unknown call, which
already forces the method impure — so a method is never marked pure on the
strength of an unproven peer method (sound: false negatives only). The
summaries are consumed by the O126 `set unused [my <pure-method>]` deletion
gate (`rust/tcl-compiler/src/optimiser/elimination.rs`); SF-2 / FP-OPT-12.

### Call resolution

`resolve_internal_call(command, caller_qname, known)`:
1. Extract namespace parts from the caller's qualified name.
2. Try `::caller_namespace::command` first.
3. Walk up the namespace hierarchy to `::command` (global).
4. Return the first name present in `known`, or `None` if the callee is
   external.

Not every callee is named by a command *word*. Two registry-declared
indirections also produce edges, so a procedure reachable only through them
is not mistaken for dead code:

- an **`ArgRole::CommandPrefix` callback** (`lsort -command cb`, `trace add
  variable v write cb`) — the callee is the prefix's head, read by
  `command_prefix_head`, which also destructures a prefix *built* by a
  `Traits::BUILDS_COMMAND_PREFIX` command (`[list cb $x]`) rather than
  misreading its head as `[list`;
- a **`Traits::INVOKES_USER_PROC` head** (the iRules `call PROC ?args?`
  form) — the callee is the first argument, not the invoker.

`command_prefix_head` is shared with
[`call_site_scan`](interprocedural-call-site-seeding.md), the other consumer
that has to answer "which command does this callback prefix name". Fixing the
two independently is exactly what let the `[list cb]` shape work in one and
not the other (issue #978); one primitive means a new prefix-building shape
lands in both at once.

## Decision rule

- If a procedure call is not being folded by O103, check `pure` first — it is
  the precondition for both fold paths, and the most common blockers are
  `has_barrier` or `has_unknown_calls` feeding into it.  Then check
  `constant_return` (argument-independent fold) or whether the call site's
  arguments are all literals (`evaluate_proc_with_constants` fold).
- To expose a new procedure-level fact, add it to `LocalFacts`, propagate it
  in `compute_all_transitive_calls` / `fixpoint_pure` / `fixpoint_effects` (or
  the transitive OR in `materialise_summaries`), and expose it on
  `ProcSummary`.
- Summaries are recomputed per `CompilationUnit` — they are not cached across
  compilation runs.

## Related docs

- [Example 23 in walkthroughs](../../../docs/design/example-script-walkthroughs.md#example-23-interprocedural-analysis--summary-construction)
- [GLOSSARY.md — ICIP](../../GLOSSARY.md#icip)
- [kcs-compiler-pipeline-overview.md](../../../docs/design/compiler/compiler-pipeline-overview.md)
