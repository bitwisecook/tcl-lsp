# Data structure reference — pipeline types at each stage

The types produced at each compiler stage, what their fields mean, and how one
representation becomes the next. Read this when adding an analysis or when
tracking a value across a stage boundary.

Every Tcl source string passes through seven stages, each producing typed
Rust structs and enums.  Lexer types live in `rust/tcl-lexer/`, bytecode
types in `rust/tcl-bytecode/`, and everything from segmentation onwards in
`rust/tcl-compiler/src/`.

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
| `TokenType` | Enum: `Esc`, `Str`, `Cmd`, `Var`, `Sep`, `Eol`, `Eof`, `Comment`, `Expand`, `ExprSugar` |
| `SourcePosition` | `line`, `character: ByteCol`, `offset` — 0-based line and *byte* column plus byte offset; the UTF-16 counterpart is `Utf16Position` |
| `Token` | `kind`, `span`, `content_offset`, `in_quote` — one lexical unit |

- `Esc` = plain word fragment, `Str` = braced string `{…}`, `Cmd` = command
  substitution `[…]`, `Var` = variable `$name`, `ExprSugar` = Jim's `$(…)`.

### Stage 2 — Segmenter types (`segmenter.rs`)

| Type | Purpose |
|------|---------|
| `SegmentedCommand` | One command: `span`, `argv`, `texts`, `word_fragments`, `single_token_word`, `all_tokens`, `is_partial`, `partial_delimiter`, `expand_word` |

- `texts[0]` = command name, `texts[1..]` = arguments.
- `single_token_word[i]` = `true` when word `i` is one atomic token (no
  interpolation) — important for constant tracking.

### Stage 3 — IR types (`ir.rs`)

The IR statement forms are variants of one `Statement` enum:

| Variant | When used |
|---------|-----------|
| `Statement::AssignConst` | `set x 42` — constant assignment |
| `Statement::AssignExpr` | `set x [expr {…}]` — expression assignment |
| `Statement::AssignValue` | `set x $y` — variable/interpolated assignment |
| `Statement::Incr` | `incr i` / `incr i 5` |
| `Statement::ExprEval` | `expr {…}` evaluated for side-effects (result discarded) |
| `Statement::Call` | Generic command (`puts`, `regexp`, etc.) with `defs`/`reads` |
| `Statement::Return` | `return` statement |
| `Statement::Barrier` | `eval`/`uplevel`/`upvar` — defeats static analysis |
| `Statement::Block` | An inlined body evaluated in the enclosing scope (an inlined passthrough call, a constant-propagated `eval`) |
| `Statement::UpFrame` | `uplevel ?level? {body}` with a literal body, lowered inline |
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
| `Module` | `source`, `top_level: Script`, `procedures: HashMap<String, Procedure>`, `methods: HashMap<String, MethodDef>`, `body_units`, `redefined_procedures`, plus the namespace / trace / TclOO evidence maps |

Every IR statement carries a `Span` for precise diagnostic mapping.

### Expression AST (`rust/tcl-syntax/src/expr/ast.rs`, re-exported as `tcl_compiler::expr_ast`)

| `ExprNode` variant | Example |
|------|---------|
| `Literal` | `42`, `3.14`, `true` |
| `String` | `"…"` / `{…}` — source text including delimiters |
| `CompiledWord` | A word already reduced to its value (a `switch` subject); never produced by the parser |
| `Var` | `$x`, `${arr(idx)}` |
| `Binary` | `$a + $b`, `$x < 10` |
| `Unary` | `-$x`, `!$flag` |
| `Ternary` | `$c ? 1 : 0` |
| `Call` | `sin($x)`, `int($y)` |
| `Command` | `[clock seconds]` |
| `Raw` | Fallback for unparseable expressions |

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
| `SsaStatement` | The original `Statement` plus `uses`, `defs`, `may_defs`, `quoted_uses`, `name_only_uses` |
| `SsaBlock` | `name`, `phis: Vec<Phi>`, `statements: Vec<SsaStatement>`, `entry_versions`, `exit_versions` |
| `SsaFunction` | `entry: BlockId`, `blocks`, `idom`, `dominance_frontier`, `dominator_tree` |

### Stage 6 — Analysis types (`analyses.rs`, `types.rs`)

| Type | Purpose |
|------|---------|
| `LatticeValue` | SCCP result: `Unknown` / `Const(ConstValue)` / `ConstSet(Vec<ConstValue>)` / `Overdefined` |
| `TypeLattice` | Type inference (`rust/tcl-compiler/src/types.rs`); its `TypeKind` reads `Unknown` / `Known` / `Shimmered` / `Overdefined` over a bounded set of `TypeShape`s |
| `SccpResult` | What `sccp()` (`rust/tcl-compiler/src/sccp.rs`) returns and `FunctionUnit.sccp` carries: `values`, `executable_blocks`, `executable_edges`, `constant_branches` |

Per-function results live on `FunctionUnit` (orchestration table below):
`sccp: SccpResult` carries the SCCP lattice and constant branches, `types`
carries type inference, and `liveness_dead_stores()`
(`rust/tcl-compiler/src/dead_stores.rs`) returns the `DeadStore` list.

### Stage 7 — Codegen types (`codegen/`, `rust/tcl-bytecode/src/lib.rs`)

| Type | Purpose |
|------|---------|
| `Op` | Enum of Tcl bytecode opcodes |
| `Instruction` | `op`, `operands: Vec<Operand>`, `comment`, `offset`, plus source-mapping and emitter hint fields |
| `LiteralTable` | Intern pool: string → object-array index |
| `LocalVarTable` | LVT: variable name → slot index |
| `FunctionAsm` | `name`, `literals`, `lvt`, `instructions`, `labels`, `loop_targets`, `proc_body_src`, `error_regions`, and the command/procedure binding requirements |
| `ModuleAsm` | `top_level` (as a script) + `top_level_body` (the same source as a proc body) + `procedures: HashMap<String, FunctionAsm>`, with the profile and source identity |

### Orchestration (`compilation_unit.rs`)

| Type | Purpose |
|------|---------|
| `FunctionUnit` | `cfg` + `ssa` + `def_use` + `sccp` + `types` + `taints` + `rendered_props` + `memory_ssa` + `semantic_facts` per function (also built per TclOO method) |
| `CompilationUnit` | `source`, `ir_module`, `cfg_module`, `top_level: FunctionUnit`, `procedures`, `methods` (per-method `FunctionUnit`s), `body_units`, `interproc`, `connection_scope`, `caller_scope` |

`CompilationUnit::build_for` (and `build_for_dialect`, `build_with_options`,
`build_for_memoized`) orchestrates all stages and returns a `CompilationUnit`.

## Decision rule

- When adding a new field to a pipeline type, check whether downstream
  consumers need updating (each stage feeds the next).
- `Module::procedures` and `CompilationUnit::procedures` use fully qualified
  names as keys (e.g. `"::mylib::helper"`).
- `Module::methods` / `CompilationUnit::methods` are keyed by
  `"{class_qname}::{method_name}"` (constructors/destructors use the synthetic
  names `<constructor>` / `<destructor>`). They are populated by a cache-
  independent post-pass (`Lowerer::extract_oo_methods_pass` in
  `rust/tcl-compiler/src/lowering/mod.rs`) and consumed by
  interprocedural method-purity and the O126 `my <method>` gate — **not** by
  codegen.

## Related docs

- [Data structure reference in walkthroughs](example-walkthroughs.md#data-structure-reference)
- [GLOSSARY.md](../../GLOSSARY.md)
- [compiler-pipeline-overview.md](compiler-pipeline-overview.md)
- [compilation-unit-contracts.md](compilation-unit-contracts.md)
