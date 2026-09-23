# CFG construction (Stage 4)

How structured IR control flow — `Statement::If`, `Statement::While`,
`Statement::For`, `Statement::Foreach`, `Statement::Switch`, and
`Statement::Try` — is decomposed into basic blocks with explicit
terminators. Read this when adding a lowering that introduces control
flow, or when debugging block connectivity and missing edges.

`build_cfg()` in `cfg_builder/mod.rs` transforms an `ir::Module` into a
`CfgModule` by decomposing structured IR into basic blocks with explicit
terminators (`Terminator::Goto`, `Terminator::Branch`,
`Terminator::Return`).  Each block is a straight-line sequence of IR
statements with no branches except at the end.

Source: `rust/tcl-compiler/src/cfg_builder/mod.rs` (`build_cfg`),
`rust/tcl-compiler/src/cfg_builder/cfg_lower.rs` (the per-construct
`lower_*` methods), `rust/tcl-compiler/src/cfg.rs` (`Terminator`, `Block`,
`Function`, `CfgModule`)

### Block identity

Blocks are keyed by an interned `BlockId(u32)`, not by name string.
`Function::intern_block` assigns the next dense id the first time a name
is seen, so `BlockId`'s `Ord` is block-*creation* order — a deterministic
source-top-to-bottom ordering.  `Function::block_name` resolves an id back
to its display name, `Function::block_id` goes the other way, and
`Function::block_by_name` / `block_by_name_mut` borrow a block by name.
`Function::blocks` is a `HashMap<BlockId, Block>`.

### Two builds of the same IR

| Entry point | `faithful_exceptions` | Used for |
|---|---|---|
| `build_cfg` | on | analysis (SSA, SCCP, diagnostics, optimiser) |
| `build_cfg_codegen` | off | bytecode / WASM emission |

The analysis-only transforms are all gated on `faithful_exceptions`:
`try` body→handler exception edges, the `tailcall` and all-arms-exit
opaque-switch terminator promotions, the opaque-switch loop-jump edges,
and the guaranteed-iteration loop rotation described below.  Codegen
never sees them, so the emitted bytecode shape stays identical to the
unannotated source.  `build_cfg_function` and
`build_cfg_function_with_upvars` build a single script body (`TclOO`
method bodies take the latter).

`build_cfg`'s `defer_top_level` flag turns off loop inlining for the
top-level script only; procedure bodies always inline.

### Decomposition patterns

**`if` / `elseif` / `else`** (`lower_if`) → fan-out with merge:

```
  entry_block:
    [...statements before if...]
    terminator: Branch { condition, true_target: if_then, false_target: if_next }

  if_then:     [...body...]  terminator: Goto(if_end)
  if_next:     [...or chain to next elseif...]
  if_end:      (continues with whatever follows the `if`)
```

The `if_end` block is allocated **first**, before any clause block, so it
carries the lowest counter of the group.  Each `elseif` clause reuses the
previous clause's `if_next` block as its dispatch and allocates a fresh
`if_then` / `if_next` pair.  The `else` body, if present, is lowered into
the final `if_next`; without one, that block gets a `Goto(if_end)`.  A
condition containing a command substitution first gets a synthetic
`<cond>` `Statement::Call` pushed into the dispatch block, carrying the
result variables the substitution writes as `defs`.

**`while`** (`lower_while`) → header with back-edge:

```
  entry: [...init...]  terminator: Goto(while_header)

  while_header:  terminator: Branch { cond, true_target: while_body, false_target: while_end }
  while_body:    [...body...]  terminator: Goto(while_header)  ← back-edge
  while_end:     (continues with whatever follows the loop)
```

`break` targets `while_end`; `continue` re-tests at `while_header`.

**`for`** (`lower_for`) → init + header + body + step:

```
  entry: [...init clause...]  terminator: Goto(for_header)

  for_header:  terminator: Branch { cond, true_target: for_body, false_target: for_end }
  for_body:    [...body...]  terminator: Goto(for_step)
  for_step:    [...step clause...]  terminator: Goto(for_header)  ← back-edge
  for_end:     (continues with whatever follows the loop)
```

The init clause is lowered into the *incoming* block, not a block of its
own.  An empty init or step clause gets a placeholder `<empty_clause>`
`Statement::Call` so the block is not empty.  `break` targets `for_end`;
`continue` runs the step at `for_step`.

**Loop rotation (analysis builds only).**  When `for_runs_at_least_once`
proves the condition holds on entry, `lower_for` rotates the loop: the
header's terminator is replaced with a synthetic always-true `Branch`
(span `None`, so the optimiser's constant-branch source rewriter leaves
the source condition alone), and the real condition moves to the step
block's terminator as the back-edge test.  SCCP then prunes the
zero-iteration header→`for_end` edge, so a body-assigned variable read
after the loop is not a false read-before-set.

**`foreach` / `lmap`** (`lower_foreach_dispatch` → `lower_foreach`) →
header + body + end:

```
  entry: [...]  terminator: Goto(foreach_header)

  foreach_header:  [foreach <vars> <lists>]   ← synthetic var-def statement
                   terminator: Branch { <foreach_has_next>, foreach_body, foreach_end }
  foreach_body:    [...body...]  terminator: Goto(foreach_header)  ← back-edge
  foreach_end:
```

The header carries one synthetic `Statement::Call` whose `defs` are the
flattened iteration variables and whose `foreach_groups` records each
iterator group's size, so codegen can reconstruct the original
var-list ↔ list-arg pairing.  The branch condition is the opaque
`ExprNode::Raw { text: "<foreach_has_next>" }`.

A `foreach` is **not** inlined — it stays a single opaque
`Statement::Call` with the iteration variables as `defs` — when loop
inlining is off for this body (the top level under `defer_top_level`) or
when any iteration variable is `::`-qualified.  `dict for` / `dict map`
and `array for` inline in analysis builds and lower to a
`Statement::Barrier` re-emitting the ensemble invoke (`::tcl::dict::for`,
`array`) in codegen builds, keeping the emitted bytecode byte-identical
to C Tcl.

When `foreach_runs_at_least_once` proves the lists are non-empty
literals, analysis builds rotate the loop the same way `for` does: the
header becomes a statically-true entry guard, the var-def moves to the
top of `foreach_body`, and a fresh `foreach_latch` block carries the
`<foreach_has_next>` back-edge test.

**`switch`** (`lower_switch`) — an *exact*, non-`-nocase` switch with no
fall-through arm is flattened into a chain of arm-dispatch `Branch`es on a
foldable `STR_EQ(subject, pattern)` through `switch_next` blocks, with one
`switch_arm_body` block per arm, `switch_default` for the default arm, and
`switch_end` as the merge.  A glob/regexp switch, a `-nocase` one, or an
exact one with any fall-through arm stays **opaque**: a single
`Statement::Switch` in the block whose arm bodies are never lowered.  SSA
recovers the names such arms read via `ssa::switch_reads`.
In analysis builds `lower_opaque_switch` still models how the opaque
switch can leave its block — promoting to `Return` when every arm exits
the procedure, or wiring non-deterministic edges to the enclosing loop's
break / continue targets through `switch_jump` blocks.

**`try` / `catch`** (`lower_try_dispatch` → `lower_try`) — the body,
handlers, and `finally` clause are lowered into `try_body`,
`try_handler`, `try_ok`, `try_finally`, `try_after_finally`, and `try_end`
blocks.  A plain `catch` is emitted as an opaque `Statement::Call` with
`defs` covering the body's writes plus the result and options variables.

### Exception edges

The single-successor terminator cannot express a throw, so analysis
builds record `try` body→handler edges separately in
`Function::exception_edges` as `(from_block, handler_block)` pairs.
`Function::block_successors` folds them into a block's successor list, so
every consumer built on it — predecessors, reachability, reverse
post-order, dominators — sees them.  SSA consumes them as extra phi
predecessors (so a handler sees the body's versions) and SCCP as extra
reachability edges (so handler bodies are not falsely unreachable).  The
vector is empty in codegen builds.

A `try` with a `finally` clause records one more kind: →`try_end`, from
every block of the body **or of a handler** that leaves it by ending in a
`Return` (a `return`, `error` or `throw`).  Without them nothing reaches `try_end`
on those paths — a body or handler that always leaves supplies no normal
edge — so a `finally` reached only that way read as dead and O107 emptied
it, though Tcl runs `finally` on every completion path (#2142).  The exits
are read off the construct's own blocks, not its resting tail:
`if {$c} {return ok} else {error boom}` cannot fall through yet still ends
in a resting `if_end` block.  Two kinds of `Return` block are not exits:
a `return` a nested `try` / `catch` already
intercepts with its own edge — control reaches this `finally` only after
the inner clause has run; and a process exit (`Traits::TERMINATES_PROCESS`,
e.g. `exit`), which ends the interpreter without unwinding, so no `finally`
runs — but only when nothing can stop it from running: it is the block's
sole statement, every word is literal, the command-binding owner resolves
the call site to one registry-backed target (whose alias prefix joins the
written words — `interp alias {} bye {} exit abc` makes `bye` raise, as
does the same alias named `::foo::exit` called as `exit` inside `::foo`),
and `CommandRegistry::exact_invocation_completion` classifies that
composed invocation as `ProcessExit` (which statuses are valid is that
owner's release-aware answer, not restated here).  Anything else may `return` or raise an error
first — an earlier statement, a substituted word (`exit [error boom]`), a
status the registry rejects (`exit abc`, or `exit 09` in 8.x),
an `if` condition or `switch` subject — and those do run the clause, so an
enclosing construct is never looked through.  `tailcall` *is* an exit: the
clause runs before the call.

A `break` / `continue` is not given an extra edge: its jump itself is
retargeted at `try_end`, and the clause's own last block records an edge on
to the saved loop target, because the clause runs *before* the loop sees the
jump.  Not `try_after_finally`: the statements after the `try` are appended
there, and a jump does not run them.
An edge alongside the jump left a path into the loop that skipped the
clause, and `while {$first || $x} { try {set first 0; continue} finally
{set x 0} }` reported `x` read before it is set.  An enclosing
`try … finally` reroutes the resumed edge through its own clause the same
way, so a jump out of nested clauses passes each of them in order.  A jump
is rerouted even from a block a nested construct intercepts: that only
replaces an edge that skipped this clause with one through it.

Whether the clause then falls through into `try_after_finally` — and so
into the statements after the `try` — depends on whether anything *can*
complete normally.  `try_completes_normally` asks the graph, not the
resting tails: it walks the construct's own blocks from the pre-`try`
block, through the handlers' exception edges, and looks for a normal edge
into `try_end`.  When there is none, every way into the clause is an exit,
so the clause's last block ends in a `Return` as an `error` does and is
recorded as a throw point for the constructs around it; the code after the
`try` is unreachable, as in Tcl.  A clause that itself leaves —
`finally {break}` — keeps its own terminator either way: its transfer
overrides the pending completion, so it neither falls through nor resumes
a saved jump, and in `while 1 { try {return} finally {break} }` the code
after the loop runs.  Otherwise the clause falls through, and
exit paths share that edge with normal completion — they add paths, never
remove one.

A handler of a body with a resting tail takes its exception edges from the
pre-`try` block, the tail, **and** every recorded throw point: an `error`
inside a nested `if`, or a `finally` that only resumes unwinding, raises
with its own block's definitions live.  Without them
`try { if {$c} {set x 1; error b} else {set x 2; error c} } on error {} {}`
called both stores dead.

The edges are not added without a `finally`:
there the tail really is unreachable on those paths, because the exception
resumes unwinding past it.

### Block naming convention

`CfgBuilder::new_block(prefix)` names each block `{prefix}_{counter}`,
where `counter` is a single monotonically increasing counter shared by
every prefix within one function, incremented before use (so the entry
block is `entry_1`).  The prefixes are:

- `entry`, `exit` — function entry and the synthetic fall-through exit
- `unreachable` — dead code after an unconditional terminator, routed
  into an orphan block with no incoming edge so SCCP marks it unreachable
  and O107 can flag it
- `if_then`, `if_next`, `if_end`
- `inline_block_body`, `inline_block_end` — an inlined `Statement::Block`
- `while_header`, `while_body`, `while_end`
- `for_header`, `for_body`, `for_step`, `for_end`
- `foreach_header`, `foreach_body`, `foreach_latch`, `foreach_end`
- `switch_next`, `switch_arm_body`, `switch_default`, `switch_end`,
  `switch_cont`, `switch_jump`, `switch_jump_dead`
- `try_body`, `try_handler`, `try_ok`, `try_finally`,
  `try_after_finally`, `try_end`

### Worked example — `set x 1; if {$x} { set y 10 }`

```
  entry_1:
    statements: [Statement::AssignConst { name: "x", value: "1", .. }]
    terminator: Branch { condition: $x, true_target: if_then_3, false_target: if_next_4 }

  if_then_3:
    statements: [Statement::AssignConst { name: "y", value: "10", .. }]
    terminator: Goto(if_end_2)

  if_next_4:
    statements: []
    terminator: Goto(if_end_2)

  if_end_2:
    terminator: Goto(exit_5)
```

`if_end_2` takes counter 2 because `lower_if` allocates the merge block
before the clause blocks.

### Worked example — `set i 0; while {$i < 5} { incr i }`

```
  entry_1: [i = "0"]  → Goto(while_header_2)

  while_header_2:
    terminator: Branch { condition: $i < 5, true_target: while_body_3, false_target: while_end_4 }

  while_body_3: [incr i]  → Goto(while_header_2)  ← back-edge

  while_end_4:  → Goto(exit_5)
```

The back-edge creates a cycle that the SSA builder handles with phi nodes.

## Decision rule

- Every basic block must have exactly one terminator, or `None` for a
  block control never leaves (an unreachable orphan, or the synthetic
  exit).
- Back-edges always go to header blocks, never into the middle of a block.
- If a new control-flow construct is added (e.g. a new loop type), add its
  decomposition pattern as a `lower_*` method in `cfg_lower.rs`,
  dispatched from `lower_script_statement`, and route its recursion
  through `lower_script` so the `MAX_LOWER_DEPTH` guard bounds it.
- Any analysis-only shape change must be gated on `faithful_exceptions`,
  or the emitted bytecode stops matching tclsh.
- `Function::loop_nodes` maps a `for` loop's **exit** block id to a
  `LoopNode { entry_block, span, for_stmt }`.  It retains the original
  `Statement::For` so SCCP can statically summarise a bounded loop, and
  the codegen's bottom-tested loop reordering reads it.

## Related docs

- [Examples 5–10 in walkthroughs](example-walkthroughs.md#example-5-if-x--set-y-10-)
- [GLOSSARY.md — Basic block, CFG](../../GLOSSARY.md#basic-block)
- [cfg-ssa-fact-model.md](cfg-ssa-fact-model.md)
- [compiler-pipeline-overview.md](compiler-pipeline-overview.md)
