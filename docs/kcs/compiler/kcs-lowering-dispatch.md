# KCS: Lowering dispatch — `arg_roles` and command classification

## Symptom

A contributor needs to understand how a Tcl command is classified and lowered
into the correct IR node, or needs to add lowering support for a new command.

## Context

`_lower_command()` in `lowering.py` dispatches each command through a hierarchy:
registered lowering hooks → match/case on command name → fallthrough via
`arg_roles`.  The dispatch produces specific IR nodes (`IRAssignConst`,
`IRAssignExpr`, `IRIf`, etc.) rather than generic `IRCall` wherever possible.

Source: [`core/compiler/lowering.py`](../../../core/compiler/lowering.py),
[`core/compiler/lowering_hooks/`](../../../core/compiler/lowering_hooks/)

## Content

### Dispatch hierarchy

```
_lower_command(cmd)
    │
    ├─ Check lowering hook on CommandSpec → spec.lowering(lowerer, cmd)
    │   (e.g. set → lower_set(), incr → lower_incr())
    │
    ├─ match cmd_name:
    │   ├─ "proc"     → extract params, lower body, register IRProcedure
    │   ├─ "when"     → lower iRules event handler body
    │   ├─ "if"       → _lower_if() → IRIf with IRIfClause list
    │   ├─ "for"      → _lower_for() → IRFor (init, cond, step, body)
    │   ├─ "while"    → _lower_while() → IRWhile (cond, body)
    │   ├─ "foreach"  → _lower_foreach() → IRForeach
    │   ├─ "catch"    → _lower_catch() → IRCatch
    │   ├─ "try"      → _lower_try() → IRTry with IRTryHandler
    │   ├─ "switch"   → _lower_switch() → IRSwitch with IRSwitchArm
    │   ├─ eval/uplevel/upvar → IRBarrier (defeats static analysis)
    │   │
    │   └─ default (fallthrough):
    │       ├─ arg_indices_for_role(BODY) → IRBarrier (has body args)
    │       ├─ arg_indices_for_role(VAR_NAME) → IRCall with defs
    │       └─ else → IRCall (generic)
```

### Lowering hooks — `lower_set()` example

`set` has a registered lowering hook (`lowering_hooks/_var.py:36`).  It
pattern-matches on the second argument's token type:

| Token type of `args[1]` | IR node produced | Example |
|-------------------------|-----------------|---------|
| `STR` (braced string) | `IRAssignConst` | `set x {hello}` |
| `ESC` (decimal integer) | `IRAssignConst` | `set x 42` |
| `CMD` wrapping `expr` | `IRAssignExpr` | `set x [expr {$a + 1}]` |
| `VAR` or interpolated | `IRAssignValue` | `set x $y`, `set x "hi $name"` |
| 0 args (getter) | `IRCall` | `set x` (read variable) |

### Fallthrough with `arg_roles`

For commands not handled by hooks or match/case (e.g. `regexp`), the
registry's `ArgRole` annotations guide lowering:

```tcl
regexp {(\d+)} $input match submatch
```

```python
var_indices = arg_indices_for_role("regexp", args, ArgRole.VAR_NAME)
# → {2, 3}  (match, submatch)

IRCall(
    command="regexp",
    args=(r"(\d+)", "${input}", "match", "submatch"),
    defs=("match", "submatch"),   # SSA tracks these as definitions
)
```

### Barrier commands

Commands in `_DYNAMIC_BARRIER_COMMANDS` (`eval`, `uplevel`, `upvar`) always
produce `IRBarrier` — telling all downstream passes to stop reasoning about
variable state at that point.

## Decision rule

- To add lowering for a new command: if it needs special IR, register a
  lowering hook on its `CommandSpec`.  If it just needs `defs` tracking,
  annotate its registry entry with `ArgRole.VAR_NAME` at the right indices.
- Commands with `BODY`-role args that are not explicitly handled produce
  `IRBarrier` — which is conservative but correct.
- Never add a match/case branch for a command that can be handled by a hook —
  hooks are more modular and testable.

## Related docs

- [Example 22 in walkthroughs](../../example-script-walkthroughs.md#example-22-lowering-dispatch--arg_roles-and-command-classification)
- [GLOSSARY.md — IR](../../GLOSSARY.md#ir)
- [kcs-lowering-contracts.md](kcs-lowering-contracts.md)
- [kcs-compiler-pipeline-overview.md](kcs-compiler-pipeline-overview.md)
