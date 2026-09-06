# Data structure reference — pipeline types at each stage

The types produced at each compiler stage, what their fields mean, and how one
representation becomes the next. Read this when adding an analysis or when
tracking a value across a stage boundary.

Every Tcl source string passes through 7 stages, each producing typed
Rust structs and enums.  Lexer types live in `rust/tcl-lexer/`, bytecode
types in `rust/tcl-bytecode/`, and everything from segmentation onwards in
`rust/tcl-compiler/src/`.  Understanding the shapes at each boundary is
essential for adding new analyses or debugging data-flow issues.

Source: `rust/tcl-lexer/src/tokens.rs`,
`rust/tcl-compiler/src/segmenter.rs`,
`rust/tcl-compiler/src/ir.rs`,
`rust/tcl-compiler/src/cfg.rs`,
`rust/tcl-compiler/src/ssa.rs`,
`rust/tcl-compiler/src/analyses.rs` / `rust/tcl-compiler/src/sccp.rs`,
`rust/tcl-compiler/src/codegen/mod.rs`, `rust/tcl-bytecode/src/lib.rs`,
`rust/tcl-compiler/src/compilation_unit.rs`

### Stage 1 — Lexer types (`tokens.rs`)

| Type | Purpose |
|------|---------|
| `TokenType` | Enum: `ESC`, `STR`, `CMD`, `VAR`, `SEP`, `EOL`, `EOF`, `COMMENT`, `EXPAND` |
| `SourcePosition` | `line`, `character`, `offset` — 0-based, UTF-16 per LSP |
| `Token` | `kind`, `text`, `start`, `end`, `in_quote` — one lexical unit |

- `ESC` = plain word fragment, `STR` = braced string `{…}`, `CMD` = command
  substitution `[…]`, `VAR` = variable `$name`.

### Stage 2 — Segmenter types (`segmenter.rs`)

| Type | Purpose |
|------|---------|
| `SegmentedCommand` | One command: `span`, `argv`, `texts`, `single_token_word`, `all_tokens` |

- `texts[0]` = command name, `texts[1..]` = arguments.
- `single_token_word[i]` = `true` when word `i` is one atomic token (no
  interpolation) — important for constant tracking.

### Stage 3 — IR types (`ir.rs`)

The IR statement forms are variants of one `Statement` enum, not separate
types:

| Variant | When used |
|---------|-----------|
| `Statement::AssignConst` | `set x 42` — constant assignment |
| `Statement::AssignExpr` | `set x [expr {…}]` — expression assignment |
| `Statement::AssignValue` | `set x $y` — variable/interpolated assignment |
| `Statement::ExprEval` | `expr {…}` evaluated for side-effects (result discarded) |
| `Statement::Incr` | `incr i` / `incr i 5` |
| `Statement::Call` | Generic command (`puts`, `regexp`, etc.) with `defs`/`reads` |
| `Statement::Return` | `return` statement |
| `Statement::Barrier` | `eval`/`uplevel`/`upvar` — defeats static analysis |
| `Statement::If` | `if/elseif/else` with a `Vec<IfClause>` |
| `Statement::For` | `for {init} {cond} {step} {body}` |
| `Statement::While` | `while {cond} {body}` |
| `Statement::Foreach` | `foreach var list body` |
| `Statement::Catch` | `catch` with optional variable targets |
| `Statement::Try` | `try/on/trap/finally` with `TryHandler` |
| `Statement::Switch` | `switch` with `SwitchArm` patterns |

The containers around them:

| Type | Purpose |
|------|---------|
| `Script` | Container: `statements: Vec<Statement>` |
| `Procedure` | A lowered `proc` body |
| `MethodDef` | A TclOO method body lifted from `oo::class create` / `oo::define`: `class_name`, `method_name`, `params`, `body: Script`, `kind`, `instance_vars` — analysis-only (codegen never reads it) |
| `Module` | `source`, `top_level: Script`, `procedures: HashMap<String, Procedure>`, `methods: HashMap<String, MethodDef>`, `redefined_procedures`, plus the namespace / trace / TclOO evidence maps |

Every IR statement carries a `Span` for precise diagnostic mapping.

### Expression AST (`rust/tcl-syntax/src/expr/ast.rs`)

| Type | Example |
|------|---------|
| `ExprLiteral` | `42`, `3.14`, `true` |
| `ExprVar` | `$x`, `${arr(idx)}` |
| `ExprBinary` | `$a + $b`, `$x < 10` |
| `ExprUnary` | `-$x`, `!$flag` |
| `ExprCall` | `sin($x)`, `int($y)` |
| `ExprCommand` | `[clock seconds]` |
| `ExprRaw` | Fallback for unparseable expressions |

### Stage 4 — CFG types (`cfg.rs`)

| Type | Purpose |
|------|---------|
| `Terminator::Goto` | Unconditional jump to the target block |
| `Terminator::Branch` | Conditional: condition → true / false target |
| `Terminator::Return` | Procedure exit with optional value |
| `Block` | `name`, `statements: Vec<Statement>`, `terminator: Option<Terminator>` |
| `Function` | `name`, `entry: BlockId`, `blocks: HashMap<BlockId, Block>`, `loop_nodes`, `exception_edges` |
| `CfgModule` | `top_level: Function` + `procedures: HashMap<String, Function>` |

### Stage 5 — SSA types (`ssa.rs`)

| Type | Purpose |
|------|---------|
| `ValueKey` | `(Symbol, Version)` — unique SSA identity |
| `Phi` | Phi node: `name: Symbol`, `version: Version`, `incoming: HashMap<BlockId, Version>` |
| `SsaStatement` | The original `Statement` plus `uses`, `defs`, `may_defs`, `quoted_uses` |
| `SsaBlock` | `name`, `phis: Vec<Phi>`, `statements: Vec<SsaStatement>`, `entry_versions`, `exit_versions` |
| `SsaFunction` | `entry: BlockId`, `blocks`, `idom`, `dominance_frontier`, `dominator_tree` |

### Stage 6 — Analysis types (`analyses.rs`, `types.rs`)

| Type | Purpose |
|------|---------|
| `LatticeValue` | SCCP result: `Unknown` / `Const(ConstValue)` / `ConstSet(Vec<ConstValue>)` / `Overdefined` |
| `TypeLattice` | Type inference (`rust/tcl-compiler/src/types.rs`); its `TypeKind` reads `Unknown` / `Known` / `Shimmered` / `Overdefined` over a bounded set of `TypeShape`s |
| `SccpResult` | What `sccp()` (`rust/tcl-compiler/src/sccp.rs`) returns and `FunctionUnit.sccp` carries: `values`, `executable_blocks`, `executable_edges`, `constant_branches` |
Per-function results live on `FunctionUnit`
(`CompilationUnit::build_for()`, orchestration table below): `sccp:
SccpResult` carries the SCCP lattice and constant branches, `types` carries
type inference, and `liveness_dead_stores()`
(`rust/tcl-compiler/src/dead_stores.rs`) returns the `DeadStore` list.

### Stage 7 — Codegen types (`codegen/`, `rust/tcl-bytecode/src/lib.rs`)

| Type | Purpose |
|------|---------|
| `Op` | Enum of Tcl bytecode opcodes |
| `Instruction` | `op`, `operands: Vec<Operand>`, `comment`, `offset`, plus source-mapping and emitter hint fields |
| `LiteralTable` | Intern pool: string → object-array index |
| `LocalVarTable` | LVT: variable name → slot index |
| `FunctionAsm` | `name`, `literals`, `lvt`, `instructions`, `labels`, `loop_targets`, `proc_body_src`, `error_regions` |
| `ModuleAsm` | `top_level` (as a script) + `top_level_body` (the same source as a proc body) + `procedures: HashMap<String, FunctionAsm>` |

### Orchestration (`compilation_unit.rs`)

| Type | Purpose |
|------|---------|
| `FunctionUnit` | `cfg` + `ssa` + `def_use` + `sccp` + `types` + `taints` + `rendered_props` + `memory_ssa` + `semantic_facts` per function (also built per TclOO method) |
| `CompilationUnit` | `source`, `ir_module`, `cfg_module`, `top_level: FunctionUnit`, `procedures`, `methods` (per-method `FunctionUnit`s), `body_units`, `interproc`, `connection_scope`, `caller_scope` |

`compile_source` (`rust/tcl-compiler/src/compilation_unit.rs`) orchestrates all stages and
returns a `CompilationUnit`.

## Decision rule

- When adding a new field to a pipeline type, check whether downstream
  consumers need updating (each stage feeds the next).
- `Module::procedures` and `CompilationUnit::procedures` use fully qualified
  names as keys (e.g. `"::mylib::helper"`).
- `Module::methods` / `CompilationUnit::methods` are keyed by
  `"{class_qname}::{method_name}"` (constructors/destructors use the synthetic
  names `<constructor>` / `<destructor>`). They are populated by a cache-
  independent post-pass (`Lowering::extract_oo_methods_pass` in
  `rust/tcl-compiler/src/lowering/mod.rs`) and consumed by
  interprocedural method-purity and the O126 `my <method>` gate — **not** by
  codegen.

## Related docs

- [Data structure reference in walkthroughs](../../../docs/design/example-script-walkthroughs.md#data-structure-reference)
- [GLOSSARY.md](../../GLOSSARY.md)
- [kcs-compiler-pipeline-overview.md](../../../docs/design/compiler/compiler-pipeline-overview.md)
- [kcs-compilation-unit-contracts.md](../../../docs/design/compiler/compilation-unit-contracts.md)
