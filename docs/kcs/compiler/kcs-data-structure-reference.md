# KCS: Data structure reference — pipeline types at each stage

## Symptom

A contributor needs to understand what types are produced at each compiler
stage, their field meanings, or how data flows from one representation to
the next.

## Context

Every Tcl source string passes through 7 stages, each producing typed
dataclasses.  All types live under `core/` and are frozen dataclasses
unless noted.  Understanding the shapes at each boundary is essential for
adding new analyses or debugging data-flow issues.

Source: [`core/parsing/tokens.py`](../../../core/parsing/tokens.py),
[`core/parsing/command_segmenter.py`](../../../core/parsing/command_segmenter.py),
[`core/compiler/ir.py`](../../../core/compiler/ir.py),
[`core/compiler/cfg.py`](../../../core/compiler/cfg.py),
[`core/compiler/ssa.py`](../../../core/compiler/ssa.py),
[`core/compiler/core_analyses.py`](../../../core/compiler/core_analyses.py),
[`core/compiler/codegen/_types.py`](../../../core/compiler/codegen/_types.py),
[`core/compiler/compilation_unit.py`](../../../core/compiler/compilation_unit.py)

## Content

### Stage 1 — Lexer types (`tokens.py`)

| Type | Purpose |
|------|---------|
| `TokenType` | Enum: `ESC`, `STR`, `CMD`, `VAR`, `SEP`, `EOL`, `EOF`, `COMMENT`, `EXPAND` |
| `SourcePosition` | `(line, character, offset)` — 0-based, UTF-16 per LSP |
| `Token` | `(type, text, start, end, in_quote)` — one lexical unit |

- `ESC` = plain word fragment, `STR` = braced string `{…}`, `CMD` = command
  substitution `[…]`, `VAR` = variable `$name`.

### Stage 2 — Segmenter types (`command_segmenter.py`)

| Type | Purpose |
|------|---------|
| `SegmentedCommand` | One command: `range`, `argv`, `texts[]`, `single_token_word[]`, `all_tokens[]` |

- `texts[0]` = command name, `texts[1:]` = arguments.
- `single_token_word[i]` = `True` when word `i` is one atomic token (no
  interpolation) — important for constant tracking.

### Stage 3 — IR types (`ir.py`)

| Type | When used |
|------|-----------|
| `IRAssignConst` | `set x 42` — constant assignment |
| `IRAssignExpr` | `set x [expr {…}]` — expression assignment |
| `IRAssignValue` | `set x $y` — variable/interpolated assignment |
| `IRIncr` | `incr i` / `incr i 5` |
| `IRCall` | Generic command (`puts`, `regexp`, etc.) with `defs`/`reads` |
| `IRReturn` | `return` statement |
| `IRBarrier` | `eval`/`uplevel`/`upvar` — defeats static analysis |
| `IRIf` | `if/elseif/else` with `IRIfClause` list |
| `IRFor` | `for {init} {cond} {step} {body}` |
| `IRWhile` | `while {cond} {body}` |
| `IRForeach` | `foreach var list body` |
| `IRCatch` | `catch` with optional variable targets |
| `IRTry` | `try/on/trap/finally` with `IRTryHandler` |
| `IRSwitch` | `switch` with `IRSwitchArm` patterns |
| `IRScript` | Container: `tuple[IRStatement, ...]` |
| `IRModule` | Top-level + `procedures dict` + `redefined_procedures` |

Every IR node carries a `Range` for precise diagnostic mapping.

### Expression AST (`expr_ast.py`)

| Type | Example |
|------|---------|
| `ExprLiteral` | `42`, `3.14`, `true` |
| `ExprVar` | `$x`, `${arr(idx)}` |
| `ExprBinary` | `$a + $b`, `$x < 10` |
| `ExprUnary` | `-$x`, `!$flag` |
| `ExprCall` | `sin($x)`, `int($y)` |
| `ExprCommand` | `[clock seconds]` |
| `ExprRaw` | Fallback for unparseable expressions |

### Stage 4 — CFG types (`cfg.py`)

| Type | Purpose |
|------|---------|
| `CFGGoto` | Unconditional jump to `target` block |
| `CFGBranch` | Conditional: `condition` → `true_target` / `false_target` |
| `CFGReturn` | Procedure exit with optional value |
| `CFGBlock` | `name`, `statements tuple`, `terminator` |
| `CFGFunction` | `entry` block name, `blocks dict`, `loop_nodes` |
| `CFGModule` | `top_level` + `procedures dict` |

### Stage 5 — SSA types (`ssa.py`)

| Type | Purpose |
|------|---------|
| `SSAValueKey` | `(variable_name, version)` tuple — unique SSA identity |
| `SSAPhi` | Phi node: `name`, `version`, `incoming dict[block→version]` |
| `SSAStatement` | Original `IRStatement` + `uses dict` + `defs dict` |
| `SSABlock` | `phis tuple`, `statements tuple`, `entry_versions`, `exit_versions` |
| `SSAFunction` | `blocks dict`, `idom dict`, `dominance_frontier dict`, `dominator_tree dict` |

### Stage 6 — Analysis types (`core_analyses.py`, `types.py`)

| Type | Purpose |
|------|---------|
| `LatticeValue` | SCCP result: `UNKNOWN` / `CONST(value)` / `OVERDEFINED` |
| `TypeLattice` | Type inference: `UNKNOWN` / `KNOWN(type)` / `SHIMMERED(from,to)` / `OVERDEFINED` |
| `FunctionAnalysis` | `live_in/live_out`, `dead_stores`, `unreachable_blocks`, `constant_branches`, `values`, `types`, `read_before_set`, `unused_variables` |

### Stage 7 — Codegen types (`codegen/`)

| Type | Purpose |
|------|---------|
| `Op` | Enum of ~100 Tcl 9.0.2 bytecode opcodes |
| `Instruction` | `(op, operands, comment, offset)` |
| `LiteralTable` | Intern pool: string → object-array index |
| `LocalVarTable` | LVT: variable name → slot index |
| `FunctionAsm` | `name`, `literals`, `lvt`, `instructions`, `labels` |
| `ModuleAsm` | `top_level` + `procedures dict` |

### Orchestration (`compilation_unit.py`)

| Type | Purpose |
|------|---------|
| `FunctionUnit` | `cfg` + `ssa` + `analysis` + `execution_intent` per function |
| `CompilationUnit` | `source`, `ir_module`, `cfg_module`, `top_level FunctionUnit`, `procedures dict`, `interproc`, `connection_scope` |

`compile_source()` at `compilation_unit.py:89` orchestrates all stages and
returns a `CompilationUnit`.

## Decision rule

- When adding a new field to a pipeline type, check whether downstream
  consumers need updating (each stage feeds the next).
- Types are frozen dataclasses for immutability — use `dataclasses.replace()`
  to create modified copies.
- `IRModule.procedures` and `CompilationUnit.procedures` use fully qualified
  names as keys (e.g. `"::mylib::helper"`).

## Related docs

- [Data structure reference in walkthroughs](../../example-script-walkthroughs.md#data-structure-reference)
- [GLOSSARY.md](../../GLOSSARY.md)
- [kcs-compiler-pipeline-overview.md](kcs-compiler-pipeline-overview.md)
- [kcs-compilation-unit-contracts.md](kcs-compilation-unit-contracts.md)
