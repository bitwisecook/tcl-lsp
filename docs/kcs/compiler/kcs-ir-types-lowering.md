# KCS: IR types and lowering basics (Stage 3)

## Symptom

A contributor needs to understand the IR node hierarchy, how commands are
lowered from `SegmentedCommand` into typed IR nodes, or why a particular
command produces a specific IR node type.

## Context

IR lowering transforms `SegmentedCommand` objects into an `IRModule`
containing typed `IRStatement` nodes.  The lowerer selects the most specific
IR node possible — `IRAssignConst` for constants, `IRAssignExpr` for
expressions, `IRIf`/`IRFor`/`IRWhile` for control flow — falling back to
`IRCall` for generic commands.  Every IR node carries a `Range` for
diagnostic mapping.

Source: [`core/compiler/ir.py`](../../../core/compiler/ir.py),
[`core/compiler/lowering.py`](../../../core/compiler/lowering.py)

## Content

### IR node selection rules

| Tcl pattern | IR node | Why |
|-------------|---------|-----|
| `set x 42` (constant value) | `IRAssignConst` | Value known at compile time |
| `set x [expr {…}]` | `IRAssignExpr` | Expression can be statically analysed |
| `set x $y` / `set x "hi $n"` | `IRAssignValue` | Variable/interpolated — runtime resolution |
| `incr i` / `incr i 5` | `IRIncr` | Specialised increment |
| `expr {2 + 3}` | `IRExprEval` | Standalone expression evaluation |
| `if {…} {…} else {…}` | `IRIf` + `IRIfClause` | Structured control flow |
| `while {…} {…}` | `IRWhile` | Loop with structured condition |
| `for {…} {…} {…} {…}` | `IRFor` | Loop with init/cond/step/body |
| `foreach var list body` | `IRForeach` | Iteration |
| `catch {…} result` | `IRCatch` | Exception handling |
| `try {…} on … {…}` | `IRTry` + `IRTryHandler` | Structured exception |
| `switch $x {…}` | `IRSwitch` + `IRSwitchArm` | Multi-way branch |
| `return $val` | `IRReturn` | Procedure exit |
| `eval $script` | `IRBarrier` | Defeats static analysis |
| `puts $msg`, `regexp …` | `IRCall` | Generic command invocation |

### `IRAssignConst` vs `IRAssignValue`

The key distinction: does the lowerer know the value at compile time?

- `set x 42` → `argv[2].type == ESC` and the text is a simple literal →
  `IRAssignConst(name="x", value="42")`
- `set x $y` → `argv[2].type == VAR` → `IRAssignValue(name="x", value="${y}")`
- `set x {hello}` → `argv[2].type == STR` → `IRAssignConst(name="x", value="hello")`

### `IRCall.defs` — variable definitions from commands

Commands like `regexp` define variables via match capture groups.  The lowerer
uses `ArgRole.VAR_NAME` to identify which arguments are variable names:

```python
# regexp {(\d+)} $input match submatch
IRCall(command="regexp", args=(...), defs=("match", "submatch"))
```

The `defs` tuple tells the SSA builder that `regexp` creates new definitions
for `match` and `submatch`.

### `IRBarrier` — analysis boundary

Commands in `_DYNAMIC_BARRIER_COMMANDS` (`eval`, `uplevel`, `upvar`) always
produce `IRBarrier`.  All downstream passes stop reasoning about variable
state at barrier points — the command can read/write any variable.

### `IRModule` structure

```python
IRModule(
    top_level=IRScript(statements=(...)),          # code outside procs
    procedures={"::add": IRProcedure(...)},        # qualified name → proc
    redefined_procedures=set(),                    # procs defined twice
)
```

Procedures are extracted from `top_level` into the `procedures` dict during
lowering.  Top-level code emits `proc` registration as `invokeStk` calls
at codegen time.

### Expression bodies

Braced `expr` bodies are parsed into `ExprNode` AST trees at lowering time.
The AST lives inside `IRAssignExpr.expr`, `IRExprEval.expr`, `IRIf` clause
conditions, and loop conditions.  Unbraced expressions fall back to
`ExprRaw` (diagnostic W100).

## Decision rule

- Use `IRAssignConst` only when the value is a compile-time constant
  (single-token `ESC` or `STR` with no interpolation).
- If a new command needs special IR, register a lowering hook rather than
  adding to the match/case in `_lower_command()`.
- Commands with `ArgRole.BODY` arguments that are not explicitly handled
  produce `IRBarrier` — conservative but correct.
- Every IR node must carry a `Range` — diagnostics need source positions.

## Related docs

- [Examples 1–4 in walkthroughs](../../example-script-walkthroughs.md#example-1-set-x-42)
- [Data structure reference — IR types](../../example-script-walkthroughs.md#stage-3--ir-types-corecompilerirpy)
- [kcs-lowering-dispatch.md](kcs-lowering-dispatch.md)
- [kcs-lowering-contracts.md](kcs-lowering-contracts.md)
