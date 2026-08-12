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

**IR**: `IRIf` with `IRIfClause` list + optional `else_body`.
**CFG**: Fan-out via `CFGBranch` per condition, merge at `if_end`.
**SSA**: Phi nodes at merge when multiple branches define the same variable.
**Bytecode**: `jumpFalse` to skip past taken branch, `jump` to common exit.

```
  (condition)  jumpFalse → else
  (then-body)  jump → end
  (else-body)
  (end) done
```

Tcl `if` returns the empty string if no branch is taken — codegen emits
`push ""` before `done` for the false path.

### `while` loop

**IR**: `IRWhile(condition, body)`.
**CFG**: Header block with `CFGBranch`, body with back-edge to header.
**SSA**: Loop phi at header merging initial value + loop-carried update.
**Bytecode**: Condition-at-bottom layout:

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

### `for` loop

**IR**: `IRFor(init, condition, step, body)`.
**CFG**: init → header → body → step → back to header.
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

### `foreach` (top-level vs proc)

**Top-level**: Compiled as generic `invokeStk` call — no loop inlining.
**Inside proc**: Uses specialised `foreach_start`/`foreach_step`/`foreach_end`
opcodes (matches tclsh's behaviour).

### `proc` definition

**IR**: `IRProcedure` extracted from `top_level` into `IRModule.procedures`.
**Top-level bytecode**: `invokeStk 4` with `"proc"`, name, params, body.
**Procedure body bytecode**: Uses `LocalVarTable` (LVT) — `loadScalar1 %v0`
instead of `loadStk`.

LVT slots are allocated in parameter order, then in first-use order for
local variables.  LVT-indexed access is faster than name-based `loadStk`.

### `switch`

**IR**: `IRSwitch` with `IRSwitchArm` patterns.
**CFG**: Cascade of `CFGBranch` blocks, one per arm, merging at `switch_end`.

A pattern body of `-` is a fallthrough: the arm shares the next non-`-` arm's
body. The lowerer marks it `SwitchArm.fallthrough = true` with a `None` body.

### `catch` / `try`

**IR**: `IRCatch` / `IRTry` with `IRTryHandler`.
**CFG**: Body block + handler blocks, merging after.

An `on`/`trap` handler body of `-` is a fallthrough — the same mechanism
`switch` uses — sharing the next non-`-` handler's body. The lowerer marks it
`TryHandler.fallthrough = true` with an empty body, so the `-` is not mistaken
for a zero-argument command call (issue #703). `switch` and `try` are the only
two Tcl commands with this `-` fallthrough form.

### Key bytecode conventions matching tclsh

| Convention | Purpose |
|-----------|---------|
| `nop` at offset 9 in `if` | tclsh alignment artifact |
| `push ""` before `done` | Tcl commands return empty string when no value |
| Condition-at-bottom loops | Avoids extra jump per iteration |
| `pop` between statements | Discard intermediate command results |
| `storeStk` (top-level) vs `storeScalar1` (proc) | LVT optimisation |

## Decision rule

- Loop bytecode should always use condition-at-bottom layout to match tclsh.
- `if` bytecode should fall through to the then-body (not jump to it) with
  `jumpFalse` skipping to the else path.
- Top-level `foreach` is an `invokeStk` call; inside `proc` it is inlined.
- When adding a new control-flow construct, follow the pattern: structured
  IR → CFG decomposition → SSA with phis → codegen with tclsh-matching layout.

## Related docs

- [Examples 5–11 in walkthroughs](../../../docs/design/example-script-walkthroughs.md#example-5-if-x--set-y-10-)
- [kcs-cfg-construction.md](../../../docs/design/compiler/cfg-construction.md)
- [kcs-ssa-construction.md](../../../docs/design/compiler/ssa-construction.md)
- [kcs-codegen-internals.md](../../../docs/design/compiler/codegen-internals.md)
- [kcs-bytecode-boundary.md](../../../docs/design/compiler/bytecode-boundary.md)
