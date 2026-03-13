# KCS: CFG construction (Stage 4)

## Symptom

A contributor needs to understand how structured IR control flow (`IRIf`,
`IRWhile`, `IRFor`) is decomposed into basic blocks, or is debugging
incorrect block connectivity or missing edges.

## Context

`build_cfg()` in `cfg.py` transforms an `IRModule` into a `CFGModule` by
decomposing structured IR into basic blocks with explicit terminators
(`CFGGoto`, `CFGBranch`, `CFGReturn`).  Each block is a straight-line
sequence of IR statements with no branches except at the end.

Source: [`core/compiler/cfg.py`](../../../core/compiler/cfg.py) (`build_cfg` at line 1058, `CFGBlock` at line 374)

## Content

### Decomposition patterns

**`if` / `elseif` / `else`** → fan-out with merge:

```
  entry_block:
    [...statements before if...]
    terminator: CFGBranch(condition, true→if_then, false→if_next)

  if_then:     [...body...]  terminator: CFGGoto(if_end)
  if_next:     [...or chain to next elseif...]
  if_end:      terminator: CFGGoto(exit)
```

Each `elseif` clause chains to a new dispatch block with its own
`CFGBranch`.  The `else` body, if present, is the final false target.
All branches merge at `if_end`.

**`while`** → header with back-edge:

```
  entry: [...init...]  terminator: CFGGoto(while_header)

  while_header:  terminator: CFGBranch(cond, true→body, false→while_end)
  while_body:    [...body...]  terminator: CFGGoto(while_header)  ← back-edge
  while_end:     terminator: CFGGoto(exit)
```

The back-edge from `while_body` to `while_header` creates a loop.

**`for`** → init + header + body + step:

```
  entry: [...init clause...]  terminator: CFGGoto(for_header)

  for_header:  terminator: CFGBranch(cond, true→body, false→for_end)
  for_body:    [...body...]  terminator: CFGGoto(for_step)
  for_step:    [...step clause...]  terminator: CFGGoto(for_header)  ← back-edge
  for_end:     terminator: CFGGoto(exit)
```

**`foreach`** — at top level, `foreach` is NOT inlined into a loop CFG.
It is emitted as an opaque `IRCall` with `defs` for the iteration variable.
Inside a `proc`, it produces specialised `foreach_start`/`foreach_step`
opcodes at codegen time.

### Block naming convention

Blocks are named systematically:
- `entry_N` — function entry
- `exit_N` — function exit
- `if_then_N`, `if_next_N`, `if_end_N` — conditional branches
- `while_header_N`, `while_body_N`, `while_end_N` — while loops
- `for_header_N`, `for_body_N`, `for_step_N`, `for_end_N` — for loops

### Worked example — `if {$x} { set y 10 }`

```
  entry_1:
    statements: [IRAssignConst(name="x", value="1")]
    terminator: CFGBranch(ExprVar("$x"), true→if_then_3, false→if_next_4)

  if_then_3:
    statements: [IRAssignConst(name="y", value="10")]
    terminator: CFGGoto(if_end_2)

  if_next_4:
    statements: []
    terminator: CFGGoto(if_end_2)

  if_end_2:
    terminator: CFGGoto(exit_5)
```

### Worked example — `while {$i < 5} { incr i }`

```
  entry_1: [i = "0"]  → CFGGoto(while_header_3)

  while_header_3:
    terminator: CFGBranch($i < 5, true→while_body_4, false→while_end_5)

  while_body_4: [incr i]  → CFGGoto(while_header_3)  ← back-edge

  while_end_5:  → CFGGoto(exit_6)
```

The back-edge creates a cycle that the SSA builder handles with phi nodes.

## Decision rule

- Every basic block must have exactly one terminator (or `None` for the
  implicit exit).
- Back-edges always go to header blocks, never into the middle of a block.
- If a new control-flow construct is added (e.g. a new loop type), add its
  decomposition pattern to `build_cfg()` following the same conventions.
- `CFGFunction.loop_nodes` tracks metadata about `for` loops for the
  codegen's bottom-tested loop reordering.

## Related docs

- [Examples 5–10 in walkthroughs](../../example-script-walkthroughs.md#example-5-if-x--set-y-10-)
- [GLOSSARY.md — Basic block, CFG](../../GLOSSARY.md#basic-block)
- [kcs-cfg-ssa-fact-model.md](kcs-cfg-ssa-fact-model.md)
- [kcs-compiler-pipeline-overview.md](kcs-compiler-pipeline-overview.md)
