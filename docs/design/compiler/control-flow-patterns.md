# Control flow patterns — if, while, for, foreach, proc

How each Tcl control-flow construct is compiled end to end, from IR through
CFG and SSA to bytecode. Read this when changing a construct's lowering, or
when checking that the emitted layout matches tclsh for a given pattern.

Each control-flow construct follows a specific decomposition pattern through
the pipeline.  The bytecode layout is designed to match tclsh 9.0 exactly,
including condition-at-bottom loops, fall-through branch ordering, and the
`nop` / empty-string conventions.

Source: Examples 5–11 in [walkthroughs](../../../docs/design/example-script-walkthroughs.md)

### `if` / `elseif` / `else`

**IR**: `Statement::If` with a `Vec<IfClause>` + optional `else_body`.
**CFG**: `cfg_lower::lower_if` — fan-out via `Terminator::Branch` per
condition, merge at `if_end`.
**SSA**: Phi nodes at merge when multiple branches define the same variable.
**Bytecode**: `jumpFalse` to skip past taken branch, `jump` to common exit.

```
  (condition)  jumpFalse → else
  (then-body)  jump → end
  (else-body)
  (end) done
```

`ordering::linearise` visits a branch's **false** target first so that,
after the post-order reversal, the then-body sits immediately after the
condition block — which is why the emitter can use a single `jumpFalse`
and fall through into the then-body.

Tcl `if` returns the empty string if no branch is taken — codegen emits
`push ""` before `done` for the false path.

### `while` loop

**IR**: `Statement::While { condition, body, .. }`.
**CFG**: `cfg_lower::lower_while` — header block with `Terminator::Branch`,
body with back-edge to header.
**SSA**: Loop phi at header merging initial value + loop-carried update.
**Bytecode**: Condition-at-bottom layout, produced by
`ordering::reorder_bottom_tested` moving the body and step blocks ahead of
their header:

```
  jump → condition
  (body)         ← loop body
  (condition)    ← condition test
  jumpTrue → body
  push ""
  done
```

The initial `jump` skips to the condition. `jumpTrue` with negative offset
jumps back to the body.  This avoids an extra unconditional jump per
iteration.

A `while` whose condition is a bare command substitution (`while [cond]
{…}`) is not lowered into a loop at all: `lower_while_or_frozen` pushes a
frozen `Statement::Barrier` carrying the source words, so the runtime
`while` builtin re-evaluates condition and body itself.

### `for` loop

**IR**: `Statement::For { init, condition, next, body, .. }`.
**CFG**: `cfg_lower::lower_for` — the init clause is lowered into the
incoming block, then header → body → step → back to header.
**Bytecode**: Same condition-at-bottom as `while`, with step clause between
body and condition:

```
  (init)
  jump → condition
  (body)
  (step)
  (condition)
  jumpTrue → body
  push ""
  done
```

`Function::loop_nodes` records each `for` loop's exit block → `LoopNode`,
which is what the bottom-tested reordering and SCCP's static-loop summary
both read.

### `foreach` (opaque vs inlined)

**Inlined** (`cfg_lower::lower_foreach`): a `foreach_header` block holding
a synthetic variable-definition `Statement::Call`, a `foreach_body`, and a
`foreach_end`, with the opaque `<foreach_has_next>` branch condition.  This
is the shape procedure bodies always get.

**Opaque**: a single `Statement::Call` carrying the iteration variables as
`defs`, compiled as a generic `invokeStk`.  `lower_foreach_dispatch` takes
this path when loop inlining is off for the body — the top level under
`build_cfg`'s `defer_top_level` — or when any iteration variable is
`::`-qualified.

`dict for` / `dict map` / `array for` inline in analysis builds and lower
to a `Statement::Barrier` re-emitting the ensemble invoke in codegen
builds, so the bytecode stays byte-identical to C Tcl.

**Bytecode**: an inlined loop emits the native
`foreach_start`/`foreach_step`/`foreach_end` opcodes and keeps a **top-test**
layout — `reorder_bottom_tested` explicitly skips `foreach_header_*` blocks.

### `proc` definition

**IR**: `Procedure` extracted from the top-level script into
`Module::procedures`.
**Top-level bytecode**: `invokeStk 4` with `"proc"`, name, params, body.
**Procedure body bytecode**: Uses `LocalVarTable` (LVT) — `loadScalar1 %v0`
instead of `loadStk`.

LVT slots are allocated in parameter order, then in first-use order for
local variables.  LVT-indexed access is faster than name-based `loadStk`.

`apply` lambdas and `namespace eval` bodies are lowered into `Procedure`s
too, but into `Module::body_units` rather than `Module::procedures`, so the
analysis pipeline reaches inside them while codegen never materialises them
as callable procs.

### `switch`

**IR**: `Statement::Switch` with `Vec<SwitchArm>` and a `SwitchMode`
(`Exact` / `Glob` / `Regexp`).
**CFG**: `cfg_lower::lower_switch` — an `Exact`, non-`-nocase` switch with
no fall-through arm becomes a cascade of `Terminator::Branch` blocks, one
`switch_next` dispatch per arm testing a foldable `STR_EQ(subject,
pattern)`, merging at `switch_end`.

Every other switch stays **opaque**: a single `Statement::Switch` in the
block, with its arm bodies never lowered, and codegen emits a generic
`switch` invoke.  A `Glob`/`Regexp` topology cannot be expressed as
structured control flow and tclsh 9.0 does not compile it either; a
`-nocase` exact switch would need a case-insensitive test the `STR_EQ`
dispatch cannot express.  SSA recovers the names such arms read through
`ssa::switch_reads`.

A pattern body of `-` is a fallthrough: the arm shares the next non-`-` arm's
body. The lowerer marks it `SwitchArm.fallthrough = true` with a `None` body.

### `catch` / `try`

**IR**: `Statement::Catch` / `Statement::Try` with `Vec<TryHandler>`.
**CFG**: `catch` is emitted opaquely by `emit_opaque_catch` — a
`Statement::Call` whose `defs` cover the body's writes plus the result and
options variables.  `try` is lowered by `cfg_lower::lower_try` into
`try_body`, `try_handler`, `try_ok`, `try_finally`, `try_after_finally`,
and `try_end` blocks.

Because a single-successor terminator cannot express a throw, analysis
builds record body→handler edges in `Function::exception_edges` instead;
`Function::block_successors` folds them into the successor list so SSA sees
them as extra phi predecessors and SCCP as extra reachability edges.  The
on-error edge is sourced from each explicit `error`/`throw` block inside the
body, so a handler sees the versions live at the throw point.

An `on`/`trap` handler body of `-` is a fallthrough — the same mechanism
`switch` uses — sharing the next non-`-` handler's body. The lowerer marks it
`TryHandler.fallthrough = true` with an empty body, so the `-` is not mistaken
for a zero-argument command call (issue #703). `switch` and `try` are the only
two Tcl commands with this `-` fallthrough form.

### Key bytecode conventions matching tclsh

| Convention | Purpose |
|-----------|---------|
| `nop` between a bare-variable (or `[catch …]`) condition and its conditional jump | Placeholder for tclsh's `tryCvtToNumeric` |
| `push ""` before `done` | Tcl commands return empty string when no value |
| Condition-at-bottom loops (`while` / `for`) | Avoids extra jump per iteration |
| Top-test layout for `foreach` | Matches tclsh's `foreach_start`/`step` shape |
| `pop` between statements | Discard intermediate command results |
| `storeStk` (top-level) vs `storeScalar1` (proc) | LVT optimisation |

## Decision rule

- `while` / `for` bytecode should always use condition-at-bottom layout to
  match tclsh; `foreach` stays top-tested.
- `if` bytecode should fall through to the then-body (not jump to it) with
  `jumpFalse` skipping to the else path.
- A `foreach` is inlined unless loop inlining is off for the body or an
  iteration variable is `::`-qualified.
- Any analysis-only CFG shape (exception edges, terminator promotions, loop
  rotation) must stay behind `faithful_exceptions` so codegen is unaffected.
- When adding a new control-flow construct, follow the pattern: structured
  IR → CFG decomposition → SSA with phis → codegen with tclsh-matching layout.

## Related docs

- [Examples 5–11 in walkthroughs](../../../docs/design/example-script-walkthroughs.md#example-5-if-x--set-y-10-)
- [kcs-cfg-construction.md](../../../docs/design/compiler/cfg-construction.md)
- [kcs-ssa-construction.md](../../../docs/design/compiler/ssa-construction.md)
- [kcs-codegen-internals.md](../../../docs/design/compiler/codegen-internals.md)
- [kcs-bytecode-boundary.md](../../../docs/design/compiler/bytecode-boundary.md)
