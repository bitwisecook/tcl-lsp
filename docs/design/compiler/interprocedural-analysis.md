# Interprocedural analysis — ProcSummary construction

How the compiler reasons about cross-procedure behaviour — purity,
constant-folding eligibility, and effect propagation — and how the resulting
summaries decide whether ICIP (O103) folds a call.

`build_interprocedural_analysis` builds a `ProcSummary` for each procedure by
first collecting per-procedure scratch facts (`LocalFacts`), then running
fixpoints over the call graph to propagate purity and effects, and — from a
compilation unit, which holds each procedure's flow graph and SSA — reading
each pure procedure's return from its own lattice, run with no call-site
seed.  Summaries are consumed by ICIP (O103), the elimination passes,
unused-proc detection (O124), and taint analysis.

Source: `rust/tcl-compiler/src/interprocedural.rs`

### Summary construction

```rust
pub fn build_interprocedural_analysis(
    ir_module: &crate::ir::Module,
    registry: &tcl_registry::CommandRegistry,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
    object_types: ObjectTypeMap<'_>,
    identities: &crate::realm::CommandBindingRealm,
    declared: Option<&tcl_registry::model::DeclaredSurface>,
) -> InterproceduralAnalysis

pub(crate) fn build_interprocedural_analysis_for_unit(
    cu: &crate::compilation_unit::CompilationUnit,
    registry: &tcl_registry::CommandRegistry,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
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
- One `ReturnKind` per `return` statement, read from its word: a literal
  through the exact value ingress (`recorded_word_value`, then
  `ExactValue::from_literal`: a braced word is its content, a bare or quoted
  one its escapes decoded, nothing trimmed), `$param` a passthrough, and
  `return [expr {…}]` a literal only where the operand is a canonical decimal
  integer or a fixed string — `0x10`, `010` and `true` are values the
  expression route decides

**Step 2 — Closures and fixpoints:**

- `compute_all_transitive_calls` closes `direct_calls` into the full
  reachable set.
- `fixpoint_pure` takes the least fixpoint of "locally pure ∧ every direct
  callee pure".
- `fixpoint_effects` unions each procedure's local effect regions with its
  transitive callees'.

**Step 2b — The seedless returns (`seedless_returns`):**

Each pure procedure's own lattice is run again with its parameters unknown —
no call-site seed ([interprocedural-call-site-seeding.md](interprocedural-call-site-seeding.md)),
so a value exact only under the literal every caller passes never enters a
summary — under the whole-module trust a rewrite folds under, and read at
every way the procedure returns (`exit_value`): each executable `return`, a
literal word through the exact ingress, a `$name` as the version there holds
it, an `expr` on the shared expression route; and a fall-through to the end
of the body through its last command. Every exit must give the one value,
and no statement the flow graph keeps whole may run a `return` of its own
(#2393). The stage follows the purity fixpoint, which decides whose returns
may fold, and reads no summary: the lattice driver takes a call to a
procedure of the module for a command it cannot see, so one stage is the
fixed point, and a return that passes through a recursive call is computed.
The bottom-up composition and its cycle bound arrive with slice 13: VT13.1
composes a callee's summary into its caller's, and VT13.2's driver is the
first lattice to read one. Only the compilation unit's entry
(`build_interprocedural_analysis_for_unit`, which `with_interprocedural` and
the optimiser call) has the lattices to run, and a procedure the complexity
guard stopped is not run; `build_interprocedural_analysis`, from IR alone,
answers from the return shapes, as does every procedure no run was made for.

**Step 3 — Materialisation (`materialise_summaries`):**

`writes_global` and `has_unknown_calls` are OR-ed across the whole transitive
closure, not copied from local facts, so a proc that only writes a global via
a callee still reports `true`.  `has_barrier` is **not** widened this way — it
stays the procedure's own local fact, which is why O124's dynamic-dispatch
guard checks `has_barrier` on every reachable proc individually rather than
just on the event handlers.  `summarise_returns` collapses the `ReturnKind`
list and the seedless run's answer into `(returns_constant, constant_return,
return_passthrough_param, return_depends_on_params)`. Where the seedless run
was made, the constant is its answer: the one value every exit gives, or
none — and then the shapes say only a passthrough and the parameters the
value depends on, never a constant (a procedure whose loop ends only by
raising reaches no exit, so `return 5` after it is no constant). Without a run, a constant return needs
*every* return to be the same literal. A passthrough needs every return to be
`$param` for the same parameter, and anything else contributes to
`return_depends_on_params`. `constant_return` is the typed projection of the
exact value, an integer, a double or a boolean only where
`ConstantReturn::text` spells that value back byte for byte: `1.0` is a
double, and `1.00`, `1e3`, `007`, `TRUE` and ` 5` are strings, so O103 spells
a folded call exactly as the procedure returns it.

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
literal arguments and reading the value every executable exit gives. A
`return` that runs inside a statement the flow graph keeps whole — an
opaque `switch`'s arm — leaves the procedure where no exit block stands for
it, so a block holding such a statement stops the reading
(`interprocedural::statement_may_return`, #2393); a `catch` body's `return`
does not count, since the `catch` absorbs it.  `can_fold_static_calls` gates only the
argument-independent fold, which replaces the call with
`summary.constant_return` — and only a call the re-run could make: every
word after the head literal (a `$name` the caller proves constant, or a
`[…]` that folds, counts), and as many as the parameters accept. A word that
substitutes runs before the call and may raise or write, and a count the
parameters do not accept raises, which the constant does not say: `[p [incr
n]]`, `[p $undefined]` and `[p]` are not folded for `proc p {a} {return foo}`
(#2389). The bare-statement hint takes the same test, over the words the
lowering records as literal.

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
`UsesParam(["x"])`, and its seedless run cannot read `$x * 2` with `x`
unknown, so `returns_constant: false`, `return_depends_on_params: ["x"]`, and
`can_fold_static_calls: false`.

`::main` calls `::helper` (pure) and `puts` (a `FileIo` write, whose coarse
region is `EffectRegion::NONE`) → `pure: false`.

When the optimiser meets `[helper 21]` it takes the `summary.pure` fallback,
evaluates the body with `x = 21` → `42`, and O103 fires.  A `[helper $n]` with
no constant for `n` folds neither way.

A return need not be a literal to be the same for every caller:

```tcl
proc prefix {} {
    set x [string range foobar 0 2]
    return $x
}
puts [prefix]
```

`::prefix`'s seedless run holds `x` at `foo` where it returns, so its summary
is `returns_constant: true`, `constant_return: Str("foo")` and
`can_fold_static_calls: true`; O103's summary path folds `[prefix]` to `foo`,
and the explorer's interprocedural view shows `return shape: const('foo')`.

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

`resolve_internal_call(command, caller_qname, known)` derives the caller's
namespace from its qualified name and applies the shared two-level rule
(`tcl_syntax::naming::resolve_command_with`, see
[namespace-resolution.md](namespace-resolution.md)): an absolute name is
looked up directly; a relative name tries `::caller_namespace::command`, then
`::command`, never an intermediate ancestor. It returns the first name present
in `known`, or `None` if the callee is external.

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

`command_prefix_head` is shared with the
[call-site scan](interprocedural-call-site-seeding.md), the other consumer
that has to answer "which command does this callback prefix name", so a new
prefix-building shape lands in both at once (issue #978).

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

- [Example 23 in walkthroughs](../../../docs/design/compiler/example-walkthroughs.md#example-23-interprocedural-analysis--summary-construction)
- [GLOSSARY.md — ICIP](../../GLOSSARY.md#icip)
- [compiler-pipeline-overview.md](compiler-pipeline-overview.md)
