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

Source: `rust/tcl-compiler/src/cfg_builder/mod.rs` (`build_cfg` at line 1700),
`rust/tcl-compiler/src/cfg_builder/cfg_lower.rs` (the per-construct
`lower_*` methods), `rust/tcl-compiler/src/cfg.rs` (the data structures —
`Terminator` at line 69, `Block` at line 144, `Function` at line 208,
`CfgModule` at line 496)

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

- [Examples 5–10 in walkthroughs](../../../docs/design/example-script-walkthroughs.md#example-5-if-x--set-y-10-)
- [GLOSSARY.md — Basic block, CFG](../../GLOSSARY.md#basic-block)
- [kcs-cfg-ssa-fact-model.md](../../../docs/design/compiler/cfg-ssa-fact-model.md)
- [kcs-compiler-pipeline-overview.md](../../../docs/design/compiler/compiler-pipeline-overview.md)
