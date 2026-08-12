# Example Script Walkthroughs

This document traces the full compilation pipeline for progressively complex
Tcl scripts, from a single `set` command through control flow with
optimisation opportunities.  Each example shows the concrete data structures
produced at every stage, with field-level detail.

> **Prerequisite reading:** [compiler-architecture.md](compiler-architecture.md)
> for the pipeline overview and stage diagrams.

---

## Glossary

| Term | Meaning |
|------|---------|
| **AST** | Abstract Syntax Tree — a tree representation of parsed source code structure. In this compiler, expression bodies (`expr {…}`) are parsed into `ExprNode` AST trees (`rust/tcl-syntax/src/expr/ast.rs`). |
| **Basic block** | A straight-line sequence of IR statements with no branches except at the end.  Represented by `Block` (`rust/tcl-compiler/src/cfg.rs`), addressed by a `BlockId`. |
| **CFG** | Control Flow Graph — a directed graph of basic blocks connected by jumps and branches.  Built by `build_cfg()` / `build_cfg_function()` (`rust/tcl-compiler/src/cfg_builder/mod.rs`) into the types in `rust/tcl-compiler/src/cfg.rs`. |
| **Dominator / idom** | Block A *dominates* block B if every path from the entry to B passes through A.  The *immediate dominator* (`idom`) is the closest dominator.  Stored in `SsaFunction::idom` (`rust/tcl-compiler/src/ssa.rs`). |
| **Dominance frontier** | The set of blocks where a variable's dominance "ends" — these are where phi nodes must be inserted.  Stored in `SsaFunction::dominance_frontier` (`rust/tcl-compiler/src/ssa.rs`). |
| **GVN** | Global Value Numbering — an optimisation that detects redundant computations by assigning a canonical identity to each expression.  See `rust/tcl-compiler/src/gvn.rs`. |
| **IR** | Intermediate Representation — a structured, typed representation of Tcl commands between parsing and code generation.  Defined in `rust/tcl-compiler/src/ir.rs`; every statement kind is a variant of the one `Statement` enum (`rust/tcl-compiler/src/ir.rs`). |
| **Lattice** | A mathematical structure used in dataflow analysis where values flow from *bottom* (unknown) toward *top* (overdefined).  The SCCP value lattice is `LatticeValue` (`rust/tcl-compiler/src/analyses.rs`); the type lattice is `TypeLattice` (`rust/tcl-compiler/src/types.rs`). |
| **Liveness** | A dataflow analysis that determines which SSA values are "live" (may still be read) at each program point.  The `FunctionAnalysis::live_in / live_out` fields that would hold the results are declared but unpopulated — see the note under [Stage 6](#stage-6--analysis-types-rusttcl-compilersrcanalysesrs) and issue #1406. |
| **LVT** | Local Variable Table — maps variable names to integer slot indices for fast access inside procedures.  See `LocalVarTable` (`rust/tcl-bytecode/src/lib.rs`). |
| **Phi node (φ)** | An SSA construct placed at control flow merge points.  `φ(x₁, x₃)` means "use `x₁` if control arrived from predecessor 1, or `x₃` if from predecessor 2."  Represented by `Phi` (`rust/tcl-compiler/src/ssa.rs`). |
| **SCCP** | Sparse Conditional Constant Propagation — a combined constant propagation and unreachable-code analysis that runs over the SSA graph.  Implemented in `sccp()` (`rust/tcl-compiler/src/sccp.rs`), which returns an `SccpResult`; the lattice types live in `rust/tcl-compiler/src/analyses.rs`. |
| **Shimmer** | Tcl's internal type coercion: when a value's string representation is reinterpreted as a different type (e.g. `"42"` read as an integer).  Tracked by `TypeKind::Shimmered` on the `TypeLattice` (`rust/tcl-compiler/src/types.rs`). |
| **SSA** | Static Single Assignment — a form where every variable is defined exactly once.  Multiple definitions of the same source variable get unique *version numbers* (e.g. `x₁`, `x₂`).  Built by `build_ssa()` (`rust/tcl-compiler/src/ssa.rs`). |
| **SSA value key** | A `(Symbol, Version)` pair that uniquely identifies one definition of a variable, where `Symbol` is the interned variable name.  Type alias `ValueKey` (`rust/tcl-compiler/src/ssa.rs`). |
| **Taint analysis** | Tracks whether values originate from untrusted sources (user input).  Uses `TaintLattice` (`rust/tcl-compiler/src/taint.rs`). |
| **Taint colour** | A `bitflags` set describing safety properties of tainted data (e.g. `CRLF_FREE`, `URL_ENCODED`, `HTML_ESCAPED`).  Colours compose with `\|` and join by intersection (`&`) — only properties shared by all incoming paths survive.  Defined as `TaintColour` (`rust/tcl-registry/src/taint.rs`). |
| **Taint source** | A command whose return value introduces tainted data (e.g. `HTTP::host`, `HTTP::uri`).  Declared as `taint_source: Option<TaintColour>` on the command's registry spec, resolved by `is_taint_source` / `taint_source_colour` (`rust/tcl-registry/src/taint.rs`). |
| **Taint sink** | A dangerous argument position where tainted data can cause harm (XSS, header injection, SSRF).  Classified by `classify_sink()` (`rust/tcl-compiler/src/taint.rs`). |
| **CSE** | Common Subexpression Elimination — detects when the same pure computation is evaluated more than once and suggests extracting it to a variable.  Part of the GVN pass, reported as `O105`.  See `rust/tcl-compiler/src/gvn.rs`. |
| **ICIP** | Interprocedural Constant/Inline Propagation — evaluates procedure calls with known constant arguments at compile time and replaces the call with the result.  Reported as `O103`.  See `optimise_static_proc_calls()` (`rust/tcl-compiler/src/optimiser/propagation.rs`). |
| **LCP** | Loop Constant Propagation / Code Sinking — moves invariant assignments out of the hot path into the specific branch that uses them.  Reported as `O125`.  See `rust/tcl-compiler/src/optimiser/code_sinking.rs`. |
| **DCE** | Dead Code Elimination — removes code whose result is never used.  `O107` (basic DCE), `O108` (aggressive DCE tracking statement liveness), `O109` (dead store elimination).  See `rust/tcl-compiler/src/optimiser/elimination.rs`. |
| **InstCombine** | Instruction Combine — canonicalises and simplifies expressions by applying algebraic identities (e.g. `$x * 1` → `$x`, DeMorgan's law).  Reported as `O110`.  See `rust/tcl-compiler/src/optimiser/helpers/expr_simplify.rs`. |
| **CommandSpec** | The central metadata type for a Tcl command — describes its argument layout, purity, side effects, taint properties, event validity, and dialect membership.  See `rust/tcl-registry/src/spec.rs`. |
| **SubCommand** | An ensemble operation selected by the first argument (e.g. `string length`, `HTTP::header value`).  Each has its own arity, purity, return type, and taint transform.  See `rust/tcl-registry/src/spec.rs`. |
| **FormSpec** | The documentation descriptor for an invocation form of a command — getter (reads state) or setter (writes state).  See `rust/tcl-registry/src/hover.rs`; the behavioural twin that carries per-form arity and routing is `CommandForm` (`rust/tcl-registry/src/forms.rs`). |

---

## Pipeline stage summary

Every Tcl source string passes through these stages. The orchestrating
entry point is `CompilationUnit::build_for()` in
`rust/tcl-compiler/src/compilation_unit.rs`:

```
Source text
  │
  ▼
┌───────────────────────────────────────────────────────────────────────┐
│ 1. Lexer         Lexer::tokenise_all()    → Vec<Token>               │  rust/tcl-lexer/src/lexer.rs
│ 2. Segmenter     segment_commands()       → Vec<SegmentedCommand>    │  rust/tcl-compiler/src/segmenter.rs
│      (derived byte-identically from the red-green CST)               │  rust/tcl-compiler/src/parsing/syntax/
│ 3. IR Lowering   lower_to_ir()            → ir::Module               │  rust/tcl-compiler/src/lowering/mod.rs
│ 4. CFG           build_cfg_function()     → cfg::Function            │  rust/tcl-compiler/src/cfg_builder/mod.rs
│ 5. SSA           build_ssa()              → SsaFunction              │  rust/tcl-compiler/src/ssa.rs
│ 6. Core analyses sccp / type_infer / …    → FunctionUnit / SccpResult│  rust/tcl-compiler/src/sccp.rs
│ 7. Codegen       codegen_module()         → ModuleAsm                │  rust/tcl-compiler/src/codegen/emitter/mod.rs
└───────────────────────────────────────────────────────────────────────┘
```

---

## Data structure reference

Before diving into examples, here are the key types that appear at each
stage. They live in `rust/tcl-lexer`, `rust/tcl-compiler`, `rust/tcl-syntax`,
`rust/tcl-registry`, and `rust/tcl-bytecode`.

> The type sketches below are shape summaries, not verbatim source. They
> name the fields a reader needs to follow the worked examples; consult the
> cited module for the exact declaration.

### Stage 1 — Lexer types (`rust/tcl-lexer/src/tokens.rs`)

```rust
// rust/tcl-lexer/src/tokens.rs
pub enum TokenType {
    Esc,     // plain string / word fragment (possibly with escape sequences)
    Str,     // braced string {…}
    Cmd,     // command substitution [… ]
    Var,     // variable substitution $name
    Sep,     // whitespace separator
    Eol,     // end-of-line / semicolon
    Eof,     // end-of-input
    Comment, // comment (# to end of line)
    Expand,  // {*} expansion prefix
}

// rust/tcl-lexer/src/span.rs — a byte range [start, end); 8 bytes, Copy
pub struct Span { /* start: u32, end: u32 — read via .start() / .end() */ }

// rust/tcl-lexer/src/tokens.rs
pub struct SourcePosition {
    pub line: u32,           // 0-based line number
    pub character: ByteCol,  // 0-based column, in bytes from the line start
    pub offset: u32,         // byte offset into the source string
}

// rust/tcl-lexer/src/tokens.rs — the LSP-facing counterpart, in UTF-16 units
pub struct Utf16Position {
    pub line: u32,
    pub character: Utf16Col,
    pub offset: u32,
}

// rust/tcl-lexer/src/tokens.rs — 16 bytes, Copy, no lifetime
pub struct Token {
    pub kind: TokenType,
    pub span: Span,           // byte range; text and positions come from SourceMap
    pub content_offset: u8,   // leading delimiter bytes to strip ($, ${, [, {, ")
    pub in_quote: bool,
}
```

- `Token.kind` distinguishes variables (`$x` → `Var`), braced strings
  (`{hello}` → `Str`), command substitutions (`[foo]` → `Cmd`), and plain
  word fragments (`set` → `Esc`).
- A `Token` carries only a `Span`, never inline text or positions.  Text and
  `(line, character)` are resolved on demand through a `SourceMap`
  (`SourceMap::text`, `SourceMap::range_positions`), so a token stays cheap
  to copy and store on IR and CFG nodes.
- `SourcePosition` counts its column in **bytes**; `Utf16Position` counts it
  in UTF-16 code units, which is the unit LSP's `Position.character` is
  defined in.  `ByteCol` and `Utf16Col` are distinct newtypes so a byte
  column can never be handed to an LSP consumer without an explicit
  conversion through `LineIndex::position_at_utf16`.

### Stage 2 — Segmenter types (`rust/tcl-compiler/src/segmenter.rs`)

> `segment_commands()` builds the canonical lossless **red-green concrete syntax
> tree** (`rust/tcl-compiler/src/parsing/syntax/`, see
> [syntax-tree.md](compiler/syntax-tree.md)) and derives the `SegmentedCommand`
> list from it.  The tree is the *backing* for the parallel-array view below,
> not a different shape: every example's Stage 2 data structure is exactly what
> a token-level walk of the same command yields.

```rust
// rust/tcl-compiler/src/segmenter.rs
pub struct SegmentedCommand {
    pub span: Span,                             // byte span of the whole command
    pub argv: Vec<Token>,                       // representative token per word
    pub texts: Vec<String>,                     // reconstructed text per word
    pub word_fragments: Vec<Vec<WordFragment>>, // lossless per-word fragments
    pub single_token_word: Vec<bool>,           // true when a word is one token
    pub all_tokens: Vec<Token>,                 // every token, separators included
    pub is_partial: bool,                       // unclosed delimiter detected
    pub partial_delimiter: Option<UnclosedDelimiter>,
    pub expand_word: Option<Vec<bool>>,         // {*} expansion per word
    pub preceding_comment: Option<String>,
}
```

- `texts[0]` is the command name, `texts[1..]` are the arguments;
  `SegmentedCommand::name()` and `::args()` return those two views directly.
- `single_token_word[i]` is `true` when word `i` is a single atomic token
  (no interpolation) — important for constant tracking downstream.
- `argv[i]` is the *representative* (first) token of word `i`; multi-token
  words (e.g. `$prefix.txt`) are concatenated into `texts[i]`.
  `word_fragments[i]` is the lossless companion to that pair, preserving the
  ordered substitution fragments of the word for consumers that need them.

### Stage 3 — IR types (`rust/tcl-compiler/src/ir.rs`)

The IR statement forms are **variants of one `Statement` enum**, not separate
types.  Every variant carries a `Span`:

```rust
// rust/tcl-compiler/src/ir.rs
pub enum Statement {
    // set a 1 — constant assignment
    AssignConst { span: Span, name: String, name_braced: bool,
                  value: String, value_span: Option<Span> },

    // set x [expr {$a + 1}] — expression assignment
    AssignExpr { span: Span, name: String, name_braced: bool,
                 expr: ExprNode, expr_base: Option<u32> },

    // set x $y, set x "hello $name" — interpolated assignment
    AssignValue { span: Span, name: String, name_braced: bool, value: String,
                  value_needs_backsubst: bool, tokens: Option<CommandTokens> },

    // incr i, incr i 5 — amount None means +1
    Incr { span: Span, name: String, name_braced: bool,
           amount: Option<String>, safe_on_uninit: bool },

    // expr {…} evaluated for its side effects, result discarded
    ExprEval { span: Span, expr: ExprNode, expr_base: Option<u32> },

    // generic command invocation (puts, append, …)
    Call { span: Span, command: String, canonical_command: Option<String>,
           args: Vec<String>,
           defs: Vec<String>,      // variables this command defines
           reads: Vec<String>,     // variables it reads by name
           reads_own_defs: bool,   // true for read-modify-write (append, lappend)
           safe_on_uninit: bool, tokens: Option<CommandTokens>,
           foreach_groups: Option<Vec<usize>> },

    Return { span: Span, value: Option<String>,
             expr: Option<ExprNode>, braced: bool },

    // eval / uplevel / upvar — defeats static analysis
    Barrier { span: Span, reason: String, command: String,
              canonical_command: Option<String>, args: Vec<String>,
              tokens: Option<CommandTokens> },

    // a pre-lowered body spliced into the enclosing scope
    Block { span: Span, body: Script, namespace: String,
            tokens: Option<CommandTokens>,
            error_context: Option<InlineBodyErrorContext> },

    // uplevel ?level? {body} with a statically-known body
    UpFrame { span: Span, frame_shift: i32, absolute: bool,
              body: Script, tokens: Option<CommandTokens> },

    If { span: Span, clauses: Vec<IfClause>,
         else_body: Option<Script>, else_span: Option<Span> },

    For { span: Span, init: Script, init_span: Span,
          condition: ExprNode, condition_span: Span, condition_base: Option<u32>,
          next: Script, next_span: Span, body: Script, body_span: Span,
          raw_args: Vec<String>, raw_tokens: Option<CommandTokens> },

    While { span: Span, condition: ExprNode, condition_span: Span,
            condition_base: Option<u32>, body: Script, body_span: Span,
            raw_args: Vec<String>, raw_tokens: Option<CommandTokens> },

    // foreach / lmap / dict for / dict map / array for
    Foreach { span: Span, iterators: Vec<ForeachIterator>,
              body: Script, body_span: Span, is_lmap: bool,
              raw_args: Vec<String>, is_dict_iteration: bool,
              is_array_iteration: bool, raw_tokens: Option<CommandTokens> },

    Catch { span: Span, body: Script, body_span: Span,
            result_var: Option<String>, options_var: Option<String>,
            raw_args: Vec<String>, tokens: Option<CommandTokens> },

    Try { span: Span, body: Script, body_span: Span,
          handlers: Vec<TryHandler>, finally_body: Option<Script>,
          finally_span: Option<Span>, raw_args: Vec<String> },

    Switch { span: Span, subject: String, subject_span: Span,
             arms: Vec<SwitchArm>, default_body: Option<Script>,
             default_span: Option<Span>, mode: SwitchMode,
             nocase: bool, raw_args: Vec<String>, patterns_braced: bool },
}

// One if/elseif clause.
pub struct IfClause {
    pub condition: ExprNode,
    pub condition_span: Span,
    pub condition_base: Option<u32>,  // absolute offset of the condition text
    pub body: Script,
    pub body_span: Span,
}

// One (var_list, list_arg) iterator group of a foreach / lmap.
pub struct ForeachIterator {
    pub vars: Vec<String>,
    pub list_arg: String,
    pub list_braced: bool,
}
```

The containers around them:

```rust
// rust/tcl-compiler/src/ir.rs
pub struct Script {
    pub statements: Vec<Statement>,
}

pub struct Procedure {
    pub name: String,
    pub qualified_name: String,       // e.g. ::ns::proc
    pub params: Vec<String>,
    pub span: Span,
    pub body: Script,
    pub params_raw: String,
    pub body_source: Option<String>,
    pub namespace_scoped: bool,
    pub base_priority: u32,
}

pub struct Module {
    pub source: String,                          // the text spans index into
    pub top_level: Script,                       // statements outside any proc
    pub procedures: HashMap<String, Procedure>,  // qualified name → proc
    pub methods: HashMap<String, MethodDef>,     // "class::method" → TclOO method
    pub body_units: HashMap<String, Procedure>,  // apply / namespace eval bodies
    pub lambda_body_units: BTreeSet<String>,
    pub redefined_procedures: HashSet<String>,
    pub redefined_methods: HashMap<String, Vec<MethodDef>>,
    // …plus the namespace / trace / TclOO evidence maps
}
```

- Every IR statement carries a `Span`; `Statement::span()` returns it for any
  variant, whichever it is.
- `Statement::Barrier` marks commands (`eval`, `uplevel`, `upvar`) whose side
  effects defeat static analysis — no constant propagation or dead-store
  reasoning can cross them.
- Expression conditions are parsed into `ExprNode` AST trees at lowering time.
- `Statement::Call::canonical_command` holds the registry-resolved name when
  an alias resolves; `Statement::canonical_command_or_source()` falls back to
  the source-surface `command` when it is `None`.

### Expression AST (`rust/tcl-syntax/src/expr/ast.rs`)

The expression forms are likewise variants of one `ExprNode` enum.  Offsets
are `ExprOffset` (a `u32`) relative to the expression text, not the module:

```rust
// rust/tcl-syntax/src/expr/ast.rs
pub enum ExprNode {
    // 42, 3.14, true
    Literal { text: String, start: ExprOffset, end: ExprOffset },
    // "…" or {…} string literal
    String { text: String, start: ExprOffset, end: ExprOffset },
    // $x, ${var}, $arr(idx)
    Var { text: String, name: String, start: ExprOffset, end: ExprOffset },
    // [clock seconds] — an opaque boundary
    Command { text: String, start: ExprOffset, end: ExprOffset },
    // $a + $b, $x < 10
    Binary { op: BinOp, left: Box<ExprNode>, right: Box<ExprNode> },
    // -$x, !$flag
    Unary { op: UnaryOp, operand: Box<ExprNode> },
    // cond ? a : b
    Ternary { condition: Box<ExprNode>,
              true_branch: Box<ExprNode>, false_branch: Box<ExprNode> },
    // sin($x), int($y), max($a, $b)
    Call { function: String, args: Vec<ExprNode>,
           start: ExprOffset, end: ExprOffset },
    // fallback for unparseable expressions — every consumer treats it as "give up"
    Raw { text: String },
}

// BinOp:   Add, Sub, Mul, Div, Mod, Pow, LShift, RShift, BitAnd, BitOr, BitXor,
//          And, Or, Eq, Ne, Lt, Le, Gt, Ge, StrEq, StrNe, StrLt, StrLe, StrGt
// UnaryOp: Neg, Pos, BitNot, Not, WordNot
```

### Stage 4 — CFG types (`rust/tcl-compiler/src/cfg.rs`)

Blocks are addressed by an interned `BlockId`, not by name; resolve one back
to its display name with `Function::block_name`.  The three ways control
leaves a block are variants of one `Terminator` enum:

```rust
// rust/tcl-compiler/src/cfg.rs
pub struct BlockId(pub u32);

pub enum Terminator {
    // unconditional jump
    Goto { target: BlockId, span: Option<Span> },
    // conditional jump
    Branch { condition: ExprNode, true_target: BlockId, false_target: BlockId,
             span: Option<Span>, condition_base: Option<u32> },
    // procedure exit
    Return { value: Option<String>, span: Option<Span>,
             expr: Option<ExprNode>, braced: bool },
}

pub struct Block {
    pub name: String,                     // e.g. "entry_1", "if_then_2"
    pub statements: Vec<Statement>,       // straight-line IR statements
    pub terminator: Option<Terminator>,   // None for unreachable/incomplete blocks
}

pub struct Function {
    pub name: String,                          // e.g. "::top", "::ns::proc"
    pub entry: BlockId,
    pub blocks: HashMap<BlockId, Block>,
    pub loop_nodes: HashMap<BlockId, LoopNode>,      // exit block → loop info
    pub exception_edges: Vec<(BlockId, BlockId)>,    // try body → handler
    pub inline_body_error_sites: Vec<InlineBodyErrorSite>,
    pub caller_frame_barrier: DynamicNameBarrier,
    pub alias_observed_vars: BTreeSet<String>,
    // …plus the private block-name interner
}

pub struct CfgModule {
    pub top_level: Function,
    pub procedures: HashMap<String, Function>,   // keyed by qualified name
}
```

### Stage 5 — SSA types (`rust/tcl-compiler/src/ssa.rs`)

Variable names are interned per function into a `Symbol`, so the hot
per-statement maps key on a `Copy` `u32` rather than a `String`.  Resolve a
symbol's display name with `SsaFunction::var_name`.

```rust
// rust/tcl-compiler/src/ssa.rs
pub type Version = u32;                 // each definition gets a unique version
pub struct Symbol(pub u32);             // interned variable name
pub type ValueKey = (Symbol, Version);  // unique SSA value identity

// A phi node (see Glossary).
pub struct Phi {
    pub name: Symbol,
    pub version: Version,                        // version produced by this phi
    pub incoming: HashMap<BlockId, Version>,     // predecessor block → version
}

pub struct SsaStatement {
    pub statement: Statement,                // the original IR statement
    pub uses: HashMap<Symbol, Version>,      // variables read → their versions
    pub defs: HashMap<Symbol, Version>,      // variables written → new versions
    pub may_defs: HashSet<Symbol>,           // synthetic array-element writes
    pub quoted_uses: HashSet<Symbol>,        // brace-quoted, unsubstituted uses
}

pub struct SsaBlock {
    pub name: String,
    pub phis: Vec<Phi>,                            // phi nodes at merge points
    pub statements: Vec<SsaStatement>,
    pub entry_versions: HashMap<Symbol, Version>,
    pub exit_versions: HashMap<Symbol, Version>,
}

pub struct SsaFunction {
    pub name: String,
    pub entry: BlockId,
    pub blocks: HashMap<BlockId, SsaBlock>,
    pub idom: HashMap<BlockId, Option<BlockId>>,          // (see Glossary)
    pub dominance_frontier: HashMap<BlockId, Vec<BlockId>>,  // (see Glossary)
    pub dominator_tree: HashMap<BlockId, Vec<BlockId>>,
    // …plus the private block-name and variable-name interners
}
```

- `may_defs` is the subset of `defs` the statement does not write itself — the
  base refresh alongside an element write (`set arr(k) v` also defs `arr`), and
  the element fan of a dynamic-key write.  Type inference *joins* across one;
  write-sensitive passes must not count it as a real write.
- `quoted_uses` is the subset of `uses` carried by a brace-quoted word the
  statement never substitutes.  Liveness must keep the use (the text may be
  evaluated later); read-before-set must ignore it.

### Stage 6 — Analysis types (`rust/tcl-compiler/src/analyses.rs`)

```rust
// rust/tcl-compiler/src/analyses.rs (see Glossary → Lattice)
pub enum LatticeKind {
    Unknown,      // not yet analysed (bottom)
    Const,        // provably constant
    ConstSet,     // a small set of possible constants
    Overdefined,  // too many values to track (top)
}

// The lattice value itself carries its payload in the variant.
pub enum LatticeValue {
    Unknown,
    Const(ConstValue),
    ConstSet(Vec<ConstValue>),   // up to MAX_CONSTSET_SIZE, then widened
    Overdefined,
}

pub enum ConstValue {
    Int(i64),
    Float(f64),
    Bool(bool),
    String(String),
}

// Aggregate per-function analysis shape.  Declared, but not part of the
// live pipeline — see the note below.
pub struct FunctionAnalysis {
    pub live_in: HashMap<String, HashSet<ValueKey>>,   // (see Glossary → Liveness)
    pub live_out: HashMap<String, HashSet<ValueKey>>,
    pub dead_stores: Vec<DeadStore>,
    pub unreachable_blocks: HashSet<String>,
    pub constant_branches: Vec<ConstantBranch>,
    pub values: HashMap<ValueKey, LatticeValue>,   // SCCP (see Glossary → SCCP)
    pub types: HashMap<ValueKey, TypeLattice>,     // type-inference results
    pub read_before_set: Vec<ReadBeforeSet>,
    pub unused_variables: Vec<UnusedVariable>,
    pub unused_params: Vec<String>,
}
```

> **`FunctionAnalysis` is not on the live path.**  The struct is declared in
> `rust/tcl-compiler/src/analyses.rs`, but nothing in the compiler builds,
> returns, or reads one — its only construction is `::default()` inside that
> module's own tests.  It is kept here as the shape a per-function analysis
> aggregate would take; issue #1406 tracks the gap.

What the pipeline actually produces is the per-function `FunctionUnit` on the
`CompilationUnit` (below), built by `CompilationUnit::build_for()`
(`rust/tcl-compiler/src/compilation_unit.rs`).  Its `sccp: SccpResult` — the
return value of `sccp()` (`rust/tcl-compiler/src/sccp.rs`) — carries `values`,
`executable_blocks`, `executable_edges`, and `constant_branches`; its `types`,
`taints`, `def_use`, and `return_type` fields carry the remaining core-analysis
results.  Liveness itself is computed where a consumer needs it rather than
being stashed on an aggregate: `live_out_by_name()`
(`rust/tcl-compiler/src/slot_allocation.rs`) for slot interference, and
`liveness_dead_stores()` (`rust/tcl-compiler/src/dead_stores.rs`) for the
`DeadStore` list.  Nothing populates the `live_in` / `live_out` fields above.

#### Type lattice (`rust/tcl-compiler/src/types.rs`)

```rust
// rust/tcl-registry/src/types.rs — the coarse registry vocabulary
pub enum TclType {
    String,
    Int,
    Double,
    Boolean,
    List,
    Dict,
    ByteArray,
    Numeric,   // abstract join of Int and Double
    Object,    // TclOO object instance
    Channel,   // I/O channel handle
}

// rust/tcl-compiler/src/types.rs — the compiler's finer shape vocabulary
pub enum TypeShape {
    String, Int, Bignum, Double, Boolean, Numeric, ByteArray,
    List(Elements),            // with optional element facts
    Dict(Elements),            // with optional facts about its values
    Object(Option<Box<str>>),  // with its class when known
    Channel,
}

pub enum TypeKind {
    Unknown,      // bottom — no information
    Known,        // exactly one possible shape
    Shimmered,    // two or more shapes (see Glossary → Shimmer)
    Overdefined,  // top — too many types to track
}

// A lattice element: Unknown, a canonicalised bounded union of TypeShapes
// (never empty), or Overdefined.  The representation is private; read it
// through TypeLattice::kind() and the shape accessors.
pub struct TypeLattice { /* repr: Unknown | Union(BoundedSet<TypeShape>) | Overdefined */ }

// Lattice order:  Unknown < Known(t) < Shimmered(a, b) < Overdefined
```

### Stage 7 — Codegen types (`rust/tcl-compiler/src/codegen/`, `rust/tcl-bytecode/src/lib.rs`)

```rust
// rust/tcl-bytecode/src/lib.rs — the Tcl 9.0.2 bytecode opcodes
pub enum Op {
    PUSH1, PUSH4, POP, DUP,
    LOAD_SCALAR1, LOAD_SCALAR4, STORE_SCALAR1, STORE_SCALAR4,
    INVOKE_STK1, INVOKE_STK4, EVAL_STK, EXPR_STK,
    JUMP1, JUMP4, JUMP_TRUE1, JUMP_FALSE1,
    ADD, SUB, // …
}

// An operand is an immediate or a symbolic label resolved during layout.
pub enum Operand {
    Imm(i32),
    Label(String),
}

pub struct Instruction {
    pub op: Op,
    pub operands: Vec<Operand>,
    pub comment: String,           // for disassembly
    pub offset: i32,               // filled by the layout pass; -1 before it
    pub source_line: u32,          // 1-based, for errorInfo
    pub source_cmd_text: String,   // original command text, for errorInfo
    pub source_span: Option<Span>, // byte span this was lowered from
    // …plus the per-opcode emitter hints (jump_table, foreach_vars, dict_vars,
    //   no_fold, foreach_collect, push_verbatim)
}

// Intern pool: string → object-array index.  Fields are private.
pub struct LiteralTable { /* entries: Vec<String>, index: HashMap<String, usize> */ }
// LVT: variable name → slot index (see Glossary).
pub struct LocalVarTable { /* … */ }

pub struct FunctionAsm {
    pub name: String,
    pub literals: LiteralTable,
    pub lvt: LocalVarTable,
    pub instructions: Vec<Instruction>,
    pub labels: HashMap<String, usize>,   // label → byte offset
    pub loop_targets: HashMap<usize, (Option<i32>, Option<i32>)>,
    pub body_base_line: u32,
    pub error_regions: Vec<ErrorRegion>,
}

pub struct ModuleAsm {
    pub top_level: FunctionAsm,
    pub procedures: HashMap<String, FunctionAsm>,   // keyed by qualified name
}
```

### Orchestration (`rust/tcl-compiler/src/compilation_unit.rs`)

```rust
// rust/tcl-compiler/src/compilation_unit.rs
pub struct FunctionUnit {
    pub name: String,                  // qualified name, e.g. ::top, ::foo::bar
    pub cfg: cfg::Function,
    pub ssa: SsaFunction,
    pub def_use: Arc<DefUseResult>,
    pub sccp: SccpResult,              // lattice values, executable blocks, branches
    pub types: Arc<HashMap<ValueKey, TypeLattice>>,
    pub return_type: TypeLattice,
    pub taints: Arc<HashMap<ValueKey, TaintLattice>>,
    pub rendered_props: Arc<HashMap<ValueKey, RenderedValueProps>>,
    pub memory_ssa: Option<MemorySsaFunction>,
    pub dynamic_names: DynamicNameBarrier,
    pub complexity_guarded: bool,
    pub base_offset: i64,
    pub method_facts: Option<Arc<MethodBodyFacts>>,
    pub semantic_facts: SemanticAnalysisBundle,
}

// Produced by CompilationUnit::build_for().
pub struct CompilationUnit {
    pub source: String,
    pub ir_module: ir::Module,
    pub cfg_module: CfgModule,
    pub top_level: FunctionUnit,
    pub procedures: HashMap<String, FunctionUnit>,
    pub methods: HashMap<String, FunctionUnit>,     // per TclOO method
    pub body_units: HashMap<String, FunctionUnit>,  // apply / namespace eval bodies
    pub interproc: Option<InterproceduralAnalysis>, // …/interprocedural.rs
    pub connection_scope: Option<ConnectionScope>,  // …/connection_scope.rs
    pub caller_scope: UnitCallerScope,
}
```

The `Arc`-shared fields (`def_use`, `types`, `taints`, `rendered_props`) are
span-free, so a memoised unit taken from the incremental cache and rebased to
a new offset keeps the very same lattice without a deep copy.

---

## Command infrastructure

The compiler's view of every Tcl command — its argument layout, purity,
side effects, taint properties, event validity, and dialect membership —
comes from the **command registry**.  This section explains each layer
of that infrastructure and how the pieces connect.

### Overview

```
┌──────────────────────────────────────────────────────────────────────┐
│                          CommandRegistry                             │
│                                                                      │
│   ┌───────────┐  ┌───────────┐  ┌──────────┐  ┌─────────────┐       │
│   │ Tcl defs  │  │ iRules    │  │ iApps    │  │ Tk / tcllib │  …    │
│   │ commands/ │  │ commands/ │  │ commands/│  │ commands/   │       │
│   │   tcl/    │  │  irules/  │  │  iapps/  │  │  tk/ stdlib/│       │
│   └─────┬─────┘  └─────┬─────┘  └────┬─────┘  └──────┬──────┘       │
│         │              │             │               │              │
│         │   each file: pub fn spec() -> CommandSpec                  │
│         │   each mod.rs: <dialect>_command_specs() -> Vec<CommandSpec>│
│         ▼              ▼             ▼               ▼              │
│                       CommandSpec                                    │
│     ┌────────────────────────────────────────────────┐              │
│     │ name, dialects, arity, traits, forms,          │              │
│     │ subcommands, arg_roles, event_requires,        │              │
│     │ side_effects, taint_source, taint_*_sink*, …   │              │
│     └────────────────────────────────────────────────┘              │
│         │          │            │            │                       │
│         ▼          ▼            ▼            ▼                       │
│     FormSpec   SubCommand   TaintColour   SideEffect                 │
│                                                                      │
└──────────────────────────────────────────────────────────────────────┘
```

Every command lives in its own module at
`rust/tcl-registry/src/commands/<dialect>/<name>.rs`, which exposes a single
`pub fn spec() -> CommandSpec` (`rust/tcl-registry/src/spec.rs`) returning a
struct literal.  Each dialect's `mod.rs` collects those into a
`Vec<CommandSpec>` from one `<dialect>_command_specs()` function that lists
every `spec()` call in a `vec![]`, and `CommandRegistry`
(`rust/tcl-registry/src/registry.rs`) merges the vectors into a unified lookup
table.  There is no registration decorator and no per-command type — a command
exists in a dialect because its `spec()` appears in that dialect's vector.

### Defining a command

Most fields come from a base constant via struct-update syntax
(`..CommandSpec::DEFAULT`, or a more specific base such as
`..CommandSpec::CLOSED_REFERENTIALLY_TRANSPARENT`), so a spec literal names
only what differs from the base.  Taint metadata is data on the same literal
(`taint_source`, `taint_output_sink`, …), not a second callback.

**Concrete example** — `string` (`rust/tcl-registry/src/commands/tcl/string_.rs`):

```rust
use crate::prelude::*;

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "string option arg ?arg ...?",
    dialects: None,
}];

const SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "length",
        arity: Arity::exact(1),
        detail: "Return the number of characters.",
        return_type: Some(TclType::Int),
        pure: true,
        ..SubCommand::DEFAULT
    },
    // …
];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "string",
        dialects: Some(DialectSet::ALL_TCL.union(DialectSet::IRULES)),
        traits: Traits::FRAMELESS_RUNTIME
            | Traits::NOT_PROC_FACTORY
            | Traits::BYTE_COMPILED
            | Traits::CSE_CANDIDATE,
        arity: Arity::at_least(1),
        subcommands: SUBCOMMANDS,
        forms: FORMS,
        inline_codegen_hook: Some(InlineCodegenHookId::String),
        ..CommandSpec::DEFAULT
    }
}
```

The behavioural flags live in the single `traits: Traits` field — a `u128`
bitset over the `Trait` enum (`rust/tcl-registry/src/traits.rs`) — set by
`|`-ing named constants; a bit that is not named is unset.

### FormSpec — invocation forms

A single command can have multiple invocation forms — e.g. a getter
(no args, reads state) and a setter (one arg, writes state).  Two distinct
types carry them.

`FormSpec` (`rust/tcl-registry/src/hover.rs`), reached via
`CommandSpec.forms`, is the thin documentation descriptor consumed by hover
and completion:

```rust
// rust/tcl-registry/src/hover.rs
pub enum FormKind {
    Default,
    Getter,   // read-only
    Setter,   // modifying
}

pub struct FormSpec {
    pub kind: FormKind,
    pub synopsis: &'static str,          // human-readable signature
    pub dialects: Option<DialectSet>,    // None = inherit the command's own
}
```

`CommandForm` (`rust/tcl-registry/src/forms.rs`), reached via
`CommandSpec.command_forms` — and `SubCommandForm` via
`SubCommand.subcommand_forms` — is the behavioural descriptor that drives
compiler routing: `name`, per-form `arity`, `arg_roles`, `options`,
`option_constraints`, `dialects`, per-form overrides of the
`result_stability` / `world_effects` / `state_transitions` /
`dispatch_dependencies` / `representation_effect` descriptors, and per-form
`lowering_hook` / `codegen_hook` routing.

**Form resolution** — `pick_form` (`rust/tcl-registry/src/registry.rs`)
matches a call's argument count and dialect against the form list; the chosen
form is reported as the `form` field of the resolved invocation.  Example for
`HTTP::host` (`rust/tcl-registry/src/commands/irules/http__host.rs`), whose
purity and header read are declared on the command spec itself:

```rust
traits: Traits::PURE
    .union(Traits::CSE_CANDIDATE)
    .union(Traits::DIAGRAM_ACTION),
forms: &[
    FormSpec { kind: FormKind::Getter, synopsis: "HTTP::host", dialects: None },
    FormSpec { kind: FormKind::Setter, synopsis: "HTTP::host <name>", dialects: None },
],
side_effects: &[SideEffect {
    target: SideEffectTarget::HttpHeader,
    reads: true,
    writes: false,
    connection_side: ConnectionSide::Both,
    dialects: None,
}],
```

When a command has both getter and setter forms, the resolved form determines
whether the invocation is treated as a read or a write.

### Arity — argument count constraints

```rust
// rust/tcl-registry/src/arity.rs
pub struct Arity {
    pub min: u16,                  // minimum args (after the command name)
    pub max: u16,                  // maximum args (u16::MAX = unlimited)
    pub step: u16,                 // 0 = no parity constraint; S restricts to
                                   //   min, min+S, min+2S, …
    pub also_exact: Option<u16>,   // one extra exact count valid despite `step`
}

// Constructors: Arity::exact(n), Arity::at_least(min),
//               Arity::new(min, max), Arity::any()
```

The `arity: Arity` field on each `CommandSpec` holds the overall arity, and
each `SubCommand` has its own.  The arity checker emits diagnostic `W101`
(wrong number of arguments) when an invocation falls outside the bounds.  A
command whose valid shapes are not a single `min..=max` range (`if`'s
`elseif`/`else` chain) declares a `clause_shape_check` hook instead and sets
`Traits::STRUCTURALLY_CHECKED_ARITY` so the generic check steps aside.

### SubCommand — ensemble commands

Commands like `string`, `dict`, `info`, and `HTTP::header` use
subcommands (the first argument selects the operation).  Each
subcommand is a `SubCommand` (`rust/tcl-registry/src/spec.rs`):

```rust
// rust/tcl-registry/src/spec.rs
pub struct SubCommand {
    pub name: &'static str,            // "length", "match", "replace", …
    pub arity: Arity,                  // arg count after the subcommand word
    pub traits: Traits,                // composed with the parent's traits
    pub detail: &'static str,          // completion-list description
    pub synopsis: &'static str,
    pub hover: Option<HoverSnippet>,
    pub pure: bool,                    // no side effects
    pub mutator: bool,                 // modifies state
    pub return_type: Option<TclType>,
    pub var_write_typing: VarWriteTyping,   // overrides the parent's
    pub arg_roles: &'static [(u8, ArgRole)],
    pub arg_role_resolver: Option<ArgRoleResolver>,
    pub arg_types: &'static [(u8, ArgTypeHint)],
    pub arg_values: &'static [(u8, &'static [ArgValue])],   // completions
    pub options: &'static [OptionSpec],       // per-subcommand options
    pub dialects: Option<DialectSet>,         // None = inherit from parent
    pub lifecycle: Lifecycle,                 // introduced / deprecated / retired
    pub subcommand_forms: &'static [SubCommandForm],
    pub sub_subcommands: &'static [SubSubCommand],   // a third dispatch level
    pub const_fold: Option<ConstFoldFn>,
    pub lowering_hook: Option<LoweringHookId>,       // IR lowering hook
    pub codegen_hook: Option<CodegenHookId>,         // bytecode specialisation
    pub taint_transform: Option<TaintColour>,        // colour added to output
    pub side_effects: &'static [SideEffect],
    // …
}
```

Hooks are typed IDs, not function pointers into a consumer crate: the registry
names the hook (`rust/tcl-registry/src/hooks.rs`) and the consumer owns the
dispatch table.

**Example** — `string length` declares `arity: Arity::exact(1)`,
`pure: true`, and `return_type: Some(TclType::Int)`.  The arity checker
validates each subcommand invocation independently.

Subcommands can be dialect-filtered by their own `dialects` set, which
overrides the parent command's when present and inherits it when `None`.
Ensemble dispatch resolves a subcommand prefix-aware (`string le` ⇒
`length`), matching C Tcl's `Tcl_GetIndexFromObj`, unless the spec sets
`prefix_matching: PrefixMatching::Strict`.

### OptionSpec and option terminators

Commands that accept `-flag` switches declare them via `OptionSpec`
(`rust/tcl-registry/src/hover.rs`):

```rust
// rust/tcl-registry/src/hover.rs
pub struct OptionSpec {
    pub name: &'static str,             // e.g. "-nocase", "-length"
    pub value: OptionValue,             // a boolean flag, or a described value
    pub detail: &'static str,           // completion description
    pub dialects: Option<DialectSet>,   // None = inherit from the parent spec
    pub aliases: &'static [&'static str],   // documented alternate spellings
    pub lifecycle: Lifecycle,
    pub min_abbrev: Option<u8>,         // None = any unique prefix resolves
}
```

**Option terminators** (`--`) prevent a dynamic argument from being
mistaken for a flag.  `W304` has no dedicated flag field: it is driven by an
`OptionSpec` named `"--"` in `options` (on the command or on a subcommand),
plus the command's `reserved_trailing_words`, resolved by
`CommandRegistry::resolve_option_terminator`
(`rust/tcl-registry/src/registry.rs`) into a `ResolvedTerminator`:

```rust
// rust/tcl-registry/src/registry.rs
pub struct ResolvedTerminator {
    pub scan_start: usize,          // arg index where option scanning begins
    pub subcommand: Option<&'static str>,   // set for a subcommand-scoped match
    pub options: &'static [OptionSpec],     // every option on the matched spec
    pub reserved_trailing_words: usize,     // trailing words never scanned
}
```

`reserved_trailing_words` mirrors C Tcl: `switch`'s implementation scans for
`-flag` words only up to `objc - 2`, so `switch $x $caseListVar` needs no
`--`.

When a command like `string match` receives a dynamic pattern (`$pat`)
without `--`, the checker emits `W304` because `$pat` could start with
`-` and be misinterpreted as the `-nocase` flag:

```tcl
# W304: use -- before dynamic pattern
string match $pat $str        ;# risky: $pat could be "-nocase"
string match -- $pat $str     ;# safe:  -- terminates option scanning
```

### Validation

Validation is layered:

1. **Arity** — `CommandSpec.arity` sets the overall arg count.  Each
   `SubCommand` has its own `arity`.  Violations produce `W101`.

2. **Option terminator** — an `OptionSpec` named `"--"` triggers `W304`
   when `--` is missing before dynamic arguments.

3. **Validation hooks** — `analyser_hook` (a typed `AnalyserHookId`
   selecting a handler in the analyser's central dispatch) and
   `literal_argument_validator` run per-command checks beyond arity.

4. **Event validity** — `event_requires` and `excluded_events` are
   checked against the active event context (see [Events](#events-irules-only)
   below).

### Argument processing — roles, values, and types

Beyond arity and options, the registry describes the *semantic role* of
each argument position, what values are valid there, what type the
command expects, and what hover/completion information to present.

#### ArgRole — what each argument means

`ArgRole` (`rust/tcl-registry/src/arg_role.rs`) classifies how the compiler
should treat each argument position:

```rust
// rust/tcl-registry/src/arg_role.rs
pub enum ArgRole {
    Body,             // Tcl script body — recursively lowered into IR
    Expr,             // Expression — parsed into an ExprNode AST
    VarWrite,         // Variable name written by the command (set, incr, lassign)
    VarRead,          // Variable name read without modification (info exists)
    LoopVarList,      // A *list* of loop variable names (dict for {k v} …)
    ParamList,        // Procedure parameter list (proc)
    Name,             // Symbolic name (proc name, namespace name)
    Pattern,          // Pattern or regex argument
    Option,           // A switch/flag argument
    Value,            // Generic value (default for unlisted positions)
    Subcommand,       // The subcommand word ("length" in "string length")
    OptionTerminator, // The "--" terminator
    FormatString,     // A `format` %-string
    ScanFormat,       // A `scan` %-string
    Channel,          // Channel identifier (stdout, channelId)
    Index,            // List/string index expression
    Keyword,          // A structural keyword (if's then/elseif/else)
    CommandPrefix,    // A callback reference invoked with args appended
    CommandName,      // A bare command name held as data (info body PROC)
    CommandNameProbe,
    LambdaLiteral,    // An {argList body ?namespace?} anonymous-lambda literal
    NamespaceName,
    Boolean,
    NumericOrBoolean,
    Result,
    Unknown,
}
```

Roles are declared via `CommandSpec.arg_roles` or `SubCommand.arg_roles` — a
`&'static [(u8, ArgRole)]` of `(index, role)` pairs, the index 0-based after
the command name (or after the subcommand word).

For variable-layout commands like `if`, `try`, and `switch` (where argument
structure depends on the actual arguments), an `ArgRoleResolver`
(`rust/tcl-registry/src/spec.rs`) maps argument values to roles dynamically:

```rust
pub type ArgRoleResolver = fn(args: &[&str]) -> Vec<(u8, ArgRole)>;
```

A third declaration form, `repeated_args: &'static [RepeatedArgLayout]`,
covers the regular unbounded tails (`global a b c`, `foreach v1 $l1 v2 $l2
body`) that neither a fixed index table nor an opaque closure expresses well.
All three feed `CommandRegistry::arg_indices_for_role` **additively**, so a
consumer asks "which arguments carry role X" and gets the whole answer.

The IR lowering pass uses `ArgRole::Body` and `ArgRole::Expr` to decide which
arguments should be recursively lowered or parsed as expressions, and
`ArgRole::VarWrite` to extract variable definitions for dataflow analysis.
Two predicates on `ArgRole` itself — `carries_script()` and
`names_variable()` — answer the cross-cutting questions so no consumer
re-derives them.

#### ArgValue — completable values

`ArgValue` (`rust/tcl-registry/src/hover.rs`) describes a valid value for a
specific argument position, providing completion text and its version gate:

```rust
// rust/tcl-registry/src/hover.rs
pub struct ArgValue {
    pub value: &'static str,          // completion text (e.g. "length", "alnum")
    pub detail: &'static str,         // short description in the completion list
    pub min_tcl: Option<TclVersion>,  // lowest release accepting this value
    pub code: Option<i64>,            // canonical integer equivalent ("ok" → 0)
}
```

Argument values are declared in two places, both as
`&'static [(u8, &'static [ArgValue])]` keyed by 0-based argument index:

1. **`CommandSpec.arg_values`** — command-level positional values, flattened
   from per-form values since the completion consumer keys purely on
   positional index:

   ```rust
   arg_values: &[(2, &[
       ArgValue { value: "enable",  detail: "Enable event timing.",  ..ArgValue::DEFAULT },
       ArgValue { value: "disable", detail: "Disable event timing.", ..ArgValue::DEFAULT },
   ])],
   ```

2. **`SubCommand.arg_values`** — per-subcommand values, indexed *after* the
   subcommand word.  For example, `string is` has character-class values at
   arg index 0:

   ```rust
   SubCommand {
       name: "is",
       arg_values: &[(0, &[
           ArgValue { value: "alnum",
                      detail: "Any Unicode alphabet or digit character.",
                      ..ArgValue::DEFAULT },
           ArgValue { value: "integer",
                      detail: "Any valid integer of arbitrary size.",
                      ..ArgValue::DEFAULT },
           // …
       ])],
       ..SubCommand::DEFAULT
   }
   ```

`SubCommand::arg_values_at(index)` returns the slice for one index, and
`versioned_arg_values` carries the owning-package release ranges for
individual literal values when they differ from the subcommand's own
lifecycle.

#### HoverSnippet — documentation content

`HoverSnippet` (`rust/tcl-registry/src/hover.rs`) carries hover and
signature-help content derived from man pages or vendor documentation:

```rust
// rust/tcl-registry/src/hover.rs
pub struct HoverSnippet {
    pub summary: &'static str,                // one-line description
    pub synopsis: &'static [&'static str],    // invocation signatures
    pub snippet: &'static str,                // extended description
    pub source: &'static str,                 // attribution ("Tcl string(n)")
    pub examples: &'static str,               // code example
    pub return_value: &'static str,           // return value description
}
```

`HoverSnippet` appears on `CommandSpec.hover` and `SubCommand.hover`.
`HoverSnippet::brief(summary, synopsis, source)` builds the common case —
the three fields most specs fill — leaving the rest empty.

#### ArgTypeHint — expected types

`ArgTypeHint` (`rust/tcl-registry/src/hooks.rs`) declares what Tcl internal
representation (intrep) a command expects for a given argument:

```rust
// rust/tcl-registry/src/hooks.rs
pub struct ArgTypeHint {
    pub expected: Option<TclType>,   // expected type (None = any)
    pub shimmers: bool,              // true if the command forces conversion
    pub transparent_from: &'static [TclType],  // intreps read without converting
}
```

Type hints are declared via `SubCommand.arg_types` or
`CommandSpec.arg_types` — a `&'static [(u8, ArgTypeHint)]` of
`(index, hint)` pairs.  The type-inference pass uses these to detect shimmer
risks (diagnostic `O130`) and propagate types through the SSA graph.
`transparent_from` records the fast paths that read an operand in its current
intrep without installing `expected`, so no shimmer is reported: `string
length` is `expected: Some(TclType::String), shimmers: true, transparent_from:
&[TclType::ByteArray]`, matching `Tcl_GetCharLength`'s short-circuit.

Return types are declared via `SubCommand.return_type` or
`CommandSpec.return_type` — an `Option<TclType>`.  For example,
`string length` has `return_type: Some(TclType::Int)`.  What the command
writes into a *variable* is a separate fact, `var_write_typing:
VarWriteTyping`, because a destructuring writer (`lassign`, `scan`, `regexp`,
`gets`) returns one thing and writes another.

#### Structural keywords — variable-layout scaffolding

Commands like `if`, `try`, and `switch` have keyword-delimited structure
rather than fixed argument positions.  Their `then` / `elseif` / `else` and
`on` / `trap` / `finally` words carry `ArgRole::Keyword` at the positions the
C-Tcl-shaped clause walk puts them, so completion and the formatter read the
role directly rather than scanning argument *values* for those words.  The
distinction is observable: in `if {1} {a} else then` the trailing `then` sits
in the else-branch **body** slot, so it is a body word, not a keyword, and
only a positional answer gets that right.

#### Deprecation

Commands and subcommands can be marked as deprecated with a replacement:

- `CommandSpec.deprecated_replacement` — `Option<&'static str>`, the
  replacement command *name*.  There is no command type to reference: a
  command is a `spec()` module, so a replacement is always a plain name.
- `CommandSpec.deprecated_replacement_drop_in` — `bool`, whether the
  replacement accepts the deprecated command's argument list unchanged, so a
  quick fix may mechanically swap the command head (`client_addr` →
  `IP::client_addr`).  `false` for replacements that restructure arguments or
  are prose; those stay message-only.
- `SubCommand.deprecated_replacement` — per-subcommand replacement.
- `lifecycle: Lifecycle` on commands, subcommands, options, and argument
  values — `introduced` / `deprecated` / `retired` releases on the owning
  package's version axis, plus an optional `deprecation_fix:
  Option<DeprecationFixHook>`.  Consumers invoke the hook only while
  `deprecated` applies, so every versioned entity shares one quick-fix
  contract.

### Side effects and purity

The side-effect model lives in `rust/tcl-compiler/src/side_effects.rs` and
classifies what each command invocation reads and writes.

**Enums** describe the vocabulary:

```rust
// rust/tcl-compiler/src/side_effects.rs
pub enum SideEffectTarget {
    Variable, SessionTable, HttpHeader, HttpBody, HttpUri,
    ResponseCommit, PoolSelection, FileIo, LogIo, // …
}

pub enum StorageScope {
    ProcLocal, Namespace, Global, Upvar,           // Tcl-universal
    Event, Connection, Static, SessionTable,       // F5 iRules-specific
    Persistence, DataGroup,
    FileSystem, NetworkSocket, LogOutput,          // external I/O
    Unknown,
}

pub enum ConnectionSide {
    Client, Server, Both, Global, None,
}

pub enum StorageType {
    Scalar, List, Dict, Array, Unknown,
}
```

**Per-invocation facts** compose into `SideEffect` and
`CommandSideEffects`.  Note that the registry declares a *narrower*
`SideEffect` (`rust/tcl-registry/src/side_effects.rs`) — target, reads,
writes, connection side, dialects — and the compiler's classifier widens it
to the full record:

```rust
// rust/tcl-compiler/src/side_effects.rs
pub struct SideEffect {
    pub target: SideEffectTarget,        // what resource
    pub reads: bool,                     // does it read?
    pub writes: bool,                    // does it write?
    pub storage_type: StorageType,       // data shape
    pub scope: StorageScope,             // where it lives
    pub connection_side: ConnectionSide, // F5 proxy context
    pub namespace: Option<String>,       // Tcl or protocol namespace
    pub dialect: Option<String>,         // None = dialect-independent
    pub key: Option<String>,             // literal variable/header name
    pub subtable: Option<String>,        // F5 session-table subtable
}

pub struct CommandSideEffects {
    pub effects: Vec<SideEffect>,   // individual effects
    pub pure: bool,                 // no observable side effects
    pub deterministic: bool,        // same inputs → same outputs
    pub dynamic_barrier: bool,      // eval/uplevel — unknowable
    pub dialect: Option<String>,    // context this classification was made in
}
```

**Classification** — `classify_side_effects(registry, command, args, dialect,
callee_summary)` (`rust/tcl-compiler/src/side_effects.rs`) combines registry
declarations with the call's actual arguments:

1. Check the interprocedural `CalleeSummary` (for user-defined procs).
2. Check for dynamic barriers (`eval`, `uplevel`).
3. Resolve the subcommand and the matching form.
4. Read the `side_effects` slice from the resolved form, subcommand, or
   command spec.
5. Check the `Traits::PURE` bit and the subcommand-level `pure` / `mutator`
   flags.

**How purity propagates:**

- A command is pure when its spec sets `Traits::PURE` (e.g. `string`, `list`).
- A subcommand can override upward: `SubCommand.pure = true` makes
  `string length` pure even if the parent were not.
- A subcommand can also override downward: `SubCommand.mutator = true` makes
  `HTTP::header replace` impure even though `HTTP::header` itself carries
  `Traits::PURE` (its getter form reads without side effects).
- A `CommandForm` / `SubCommandForm` refines further, overriding the levels
  above it for that specific arity match.

The GVN optimiser uses purity to decide whether a command's result can be
cached (`Traits::CSE_CANDIDATE`), and the SCCP analysis uses it to infer
through pure calls without bailing out.  Purity alone is not sufficient for
call reuse: reuse also needs `result_stability`, closed `state_transitions`,
no relevant `world_effects`, and a site proof covering every
`dispatch_dependencies` entry.

### Taint analysis

Taint tracking determines whether values originate from untrusted input
(user-controlled HTTP headers, URI, query parameters, etc.).

**TaintColour** (`rust/tcl-registry/src/taint.rs`) is a `Flag` enum — colours compose
with `|` and the lattice join is their intersection (`&`):

```rust
// rust/tcl-registry/src/taint.rs
bitflags! {
    pub struct TaintColour: u32 {
        const TAINTED            = 1 << 0;  // base: value is attacker-controlled
        const PATH_PREFIXED      = 1 << 1;  // starts with "/" (HTTP::uri, HTTP::path)
        const NON_DASH_PREFIXED  = 1 << 2;  // provably starts with a non-"-" literal
        const CRLF_FREE          = 1 << 3;  // no CR/LF (header-injection safe)
        const SHELL_ATOM         = 1 << 4;  // no shell metachar splitting
        const LIST_CANONICAL     = 1 << 5;  // canonical Tcl list representation
        const REGEX_LITERAL      = 1 << 6;  // regex-escaped literal payload
        const PATH_NORMALISED    = 1 << 7;
        const PATH_BOUNDED       = 1 << 8;
        const HEADER_TOKEN_SAFE  = 1 << 9;
        const HTML_ESCAPED       = 1 << 10; // HTML-escaped text context
        const URL_ENCODED        = 1 << 11; // URL-encoded text context
        const IP_ADDRESS         = 1 << 12; // IPv4/IPv6 digits-dots-colons
        const PORT               = 1 << 13; // integer 0-65535
        const FQDN               = 1 << 14; // fully qualified domain name
        const PATH_JOINED        = 1 << 15;
        const CHANNEL            = 1 << 16; // I/O channel handle
    }
}
```

Colours represent *safety properties* of tainted data.  A value with
`TAINTED | IP_ADDRESS` is tainted but known to be a safe IP address
format, which may satisfy certain sinks (e.g. connecting to a backend).

**Sources and sinks are plain fields on the `CommandSpec` literal**, not a
separate hint object hung off a callback.  The relevant ones are:

| Field | Type | Meaning |
|-------|------|---------|
| `taint_source` | `Option<TaintColour>` | Colour this command's result carries as an origin of untrusted data.  `None` = not a source |
| `taint_transform` | `Option<TaintColour>` | Colour bits added to tainted output (a sanitiser) |
| `taint_output_sink` | `Option<&'static str>` | Output-sink diagnostic code (`"IRULE3001"` for XSS) |
| `taint_output_sink_subcommands` | `&'static [&'static str]` | Restricts the output sink to those subcommands; empty = every invocation |
| `taint_log_sink` | `Option<&'static str>` | Log-injection sink diagnostic code |
| `taint_network_sink_args` | `Option<&'static [u8]>` | Argument indices that are network sinks |
| `taint_code_sink_args` | `Option<&'static [u8]>` | Argument indices carrying the code-execution hazard; `None` = the whole tail reaches evaluation |
| `taint_sink_safe_colour` | `Option<TaintColour>` | Colour that suppresses the sink diagnostic |
| `taint_sink_gate` | `Option<fn(&[&str]) -> bool>` | A `false` result suppresses sink classification for that call (`subst -nocommands`) |
| `setter_constraints` | `&'static [SetterConstraint]` | Setter-form argument constraints (IRULE3101) |

**Example** — `HTTP::host`
(`rust/tcl-registry/src/commands/irules/http__host.rs`) is a taint source, one
line on its spec literal:

```rust
taint_source: Some(TaintColour::TAINTED),
```

**Example** — `HTTP::header`
(`rust/tcl-registry/src/commands/irules/http__header.rs`) is both a source and
a subcommand-restricted sink:

```rust
taint_source: Some(TaintColour::TAINTED),
taint_output_sink: Some("IRULE3002"),          // header injection
taint_output_sink_subcommands: &["insert", "replace"],
```

`SetterConstraint` is the one taint declaration with a struct of its own:

```rust
// rust/tcl-registry/src/taint.rs
pub struct SetterConstraint {
    pub arg_index: u8,                  // 0-based after the command name
    pub required_prefix: &'static str,  // literal prefix the argument must start with
    pub code: DiagCode,                 // e.g. "IRULE3101"
    pub message: &'static str,
}
```

`is_taint_source` and `taint_source_colour`
(`rust/tcl-registry/src/taint.rs`) resolve a call against all of this —
the `Traits::TAINT_SOURCE` / `Traits::UNNORMALISED_HTTP_GETTER` bits, a
subcommand's own `TAINT_SOURCE` bit (resolved prefix-aware so `chan g` cannot
dodge classification), and the registry's dialect-agnostic taint-source index.
`augment_source_colours` then adds the properties a colour implies: a
path-prefixed value also proves `NON_DASH_PREFIXED`; an IP / port / FQDN value
also proves `NON_DASH_PREFIXED`, `CRLF_FREE`, and `SHELL_ATOM`.

The taint engine (`rust/tcl-compiler/src/taint.rs`) propagates colours through
the SSA graph, and emits diagnostics (e.g. `IRULE3001` for XSS, `IRULE3002`
for header injection) when tainted data reaches a sink — classified by
`classify_sink` — without sufficient safety colours.

### Dialects

Dialects partition command availability across Tcl versions and tool
contexts.  The canonical profile names (`rust/tcl-dialect/src/dialect_set.rs`,
re-exported from `rust/tcl-registry/src/dialects.rs`) are a sorted
`&'static [&'static str]`:

```rust
// rust/tcl-dialect/src/dialect_set.rs
pub const KNOWN_DIALECTS: &[&str] = &[
    "bpf",
    "cadence-eda-tcl",
    "expect",
    "f5-bigip",                  // F5 BIG-IP config surface
    "f5-iapps",                  // F5 iApps
    "f5-irules",                 // F5 iRules
    "f5-tmsh",                   // F5 tmsh scripting
    "intel-quartus-eda-tcl",
    "mentor-eda-tcl",
    "synopsys-eda-tcl",
    "tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1",   // Tcl version dialects
    "xilinx-eda-tcl",
];
```

A spec's own gating is not a set of those strings but a `DialectSet` — a
`bitflags` set over `u64`, with composite constants combined by `union` /
`|`:

```rust
// rust/tcl-dialect/src/dialect_set.rs
bitflags! {
    pub struct DialectSet: u64 {
        const TCL84  = 1 << 0;
        const TCL85  = 1 << 1;
        const TCL86  = 1 << 2;
        const TCL90  = 1 << 3;
        const IRULES = 1 << 4;
        const IAPPS  = 1 << 5;
        const TK     = 1 << 6;
        const EXPECT = 1 << 7;
        const BPF    = 1 << 13;
        const TCL91  = 1 << 14;
        const TMSH   = 1 << 15;
        const BIGIP  = 1 << 16;

        const ALL_TCL     = /* TCL84 | TCL85 | TCL86 | TCL90 | TCL91 */;
        const TCL85_PLUS  = /* TCL85 | TCL86 | TCL90 | TCL91 */;
        const TCL86_PLUS  = /* TCL86 | TCL90 | TCL91 */;
        const TCL8X       = /* TCL84 | TCL85 | TCL86 */;
        const TCL90_PLUS  = /* TCL90 | TCL91 */;
        const TK_AND_TCL  = /* ALL_TCL | TK */;
    }
}
```

The EDA shells have no dialect bits of their own: they are modelled as a base
Tcl version plus `required_package`-gated command libraries.

Every `CommandSpec` has an optional `dialects: Option<DialectSet>` field:

- `dialects: None` → available in **all** dialects.
- `dialects: Some(DialectSet::IRULES)` → iRules-only command (e.g.
  `HTTP::host`, `pool`, `table`).
- `dialects: Some(DialectSet::ALL_TCL.union(DialectSet::IRULES))` → core Tcl
  plus iRules, the `string` case.

Subcommands, options, forms, and individual side effects each carry the same
`Option<DialectSet>`, overriding the parent's when `Some` and inheriting it
when `None`.  `DialectSet::is_valid_nested_dialects(child, parent)` is a
`const fn`, so an unreachable nesting can be rejected at build time rather
than by a test sweep.

### Events (iRules only)

In F5 iRules, commands are only valid in certain events (e.g. `HTTP::uri`
requires an HTTP profile and only works in HTTP events).  This is
modelled by `EventRequires` (`rust/tcl-registry/src/events.rs`):

```rust
// rust/tcl-registry/src/events.rs
pub struct EventRequires {
    pub client_side: bool,                    // needs client-side connection
    pub server_side: bool,                    // needs server-side connection
    pub transport: Option<&'static str>,      // "tcp" or "udp"
    pub profiles: &'static [&'static str],    // needs one of these profiles
    pub also_in: &'static [&'static str],     // always valid in these events
    pub init_only: bool,                      // only valid in RULE_INIT
    pub flow: bool,                           // needs an active traffic flow
    pub capability: Option<&'static str>,     // profile capability (e.g. "sni")
}
```

**Example** — `HTTP::host` requires TCP transport and an HTTP or FASTHTTP
profile:

```rust
event_requires: Some(EventRequires {
    client_side: false,
    server_side: false,
    transport: Some("tcp"),
    profiles: &["FASTHTTP", "HTTP"],
    also_in: &[],
    init_only: false,
    flow: false,
    capability: None,
}),
```

A command whose subforms have *different* event contracts declares
`event_requirement_forms: &'static [EventRequirementForm]`, each keyed on a
literal leading-argument prefix; the longest match wins and overrides
`event_requires`.

The validator matches `event_requires` against the event's `EventProps`
(`rust/tcl-registry/src/spec.rs`), which describes what each event provides
(client/server side, transport, implied profiles).  Mismatches produce
diagnostic `IRULE1001`.

`CommandSpec.excluded_events` lists events where a command is explicitly
forbidden (e.g. a command that crashes in `RULE_INIT`).

### How the infrastructure feeds the compiler

The registry metadata flows into every stage of the compilation pipeline:

1. **IR Lowering** — `lower_to_ir()` uses `arg_roles` to identify which
   arguments are bodies (`ArgRole::Body`), expressions (`ArgRole::Expr`), or
   variable names (`ArgRole::VarWrite`).  This drives recursive lowering of
   script bodies and expression parsing.

2. **CFG** — `Traits::CREATES_DYNAMIC_BARRIER` marks commands that defeat
   static analysis (e.g. `eval`, `uplevel`).  The lowering emits a
   `Statement::Barrier` for these.

3. **SSA/SCCP** — `Traits::PURE` commands can be inferred through without
   invalidating the lattice state.  Impure commands force variables to
   `LatticeValue::Overdefined`.

4. **GVN** — `Traits::CSE_CANDIDATE` and `Traits::PURE` determine whether a
   command's result is a static reuse candidate (common subexpression
   elimination); `result_stability`, `world_effects`, `state_transitions`,
   and `dispatch_dependencies` decide whether the reuse is actually sound.

5. **Codegen** — `codegen_hook` / `inline_codegen_hook` typed IDs on
   `SubCommand` or `CommandSpec` select specialised bytecode (e.g.
   `string length` → `strLen` opcode instead of a generic `invokeStk`).

6. **Taint engine** — `taint_source` / `taint_transform` / the
   `taint_*_sink*` fields mark sources and sinks; the taint lattice
   propagates `TaintColour` through the SSA graph.

7. **Diagnostics** — arity, option terminators, event requirements,
   deprecation, `analyser_hook`, and `literal_argument_validator` all
   produce diagnostics (`W101`, `W304`, `IRULE1001`, etc.).

---

## Example 1: `set x 42`

The simplest possible Tcl script — assign a constant to a variable.

### Source

```tcl
set x 42
```

### Stage 1 — Lexer → Token stream

The `TclLexer` scans character-by-character and produces a flat stream:

```
Token { kind: TokenType::Esc, span: Span { start: 0, end: 3 } }   // "set"
Token { kind: TokenType::Sep, span: Span { start: 3, end: 4 } }   // " "
Token { kind: TokenType::Esc, span: Span { start: 4, end: 5 } }   // "x"
Token { kind: TokenType::Sep, span: Span { start: 5, end: 6 } }   // " "
Token { kind: TokenType::Esc, span: Span { start: 6, end: 8 } }   // "42"
Token { kind: TokenType::Eof, span: Span { start: 8, end: 8 } }   // ""
```

The trailing comments are the text each span covers — a `Token` stores only
its `Span`, and the text is resolved through the `SourceMap` on demand.

Key observations:
- `set`, `x`, and `42` are all `Esc` (plain word fragments) — no variable
  substitution or braces involved.
- Whitespace becomes `Sep` tokens — they delimit words but carry no semantic
  value.

### Stage 2 — Segmenter → SegmentedCommand

The segmenter builds the red-green CST for the source and derives one
`SegmentedCommand` per command (split at `Eol`/`Eof` boundaries):

```
SegmentedCommand {
    span: Span { start: 0, end: 8 },
    argv: [Token { kind: TokenType::Esc, .. },   // "set"
           Token { kind: TokenType::Esc, .. },   // "x"
           Token { kind: TokenType::Esc, .. }],  // "42"
    texts: ["set", "x", "42"],
    single_token_word: [true, true, true],
    all_tokens: [/* all 6 tokens */],
}
```

- `texts[0] == "set"` → command name
- `texts[1] == "x"` → variable name argument
- `texts[2] == "42"` → value argument
- All words are single-token (no interpolation), so `single_token_word` is
  all `true` — this tells the lowerer the value is a compile-time constant.

### Stage 3 — IR Lowering → Statement::AssignConst

The lowerer pattern-matches `set` with two arguments where the second argument
is a single-token constant:

```
Module {
    top_level: Script { statements: [
        Statement::AssignConst {
            span: Span { start: 0, end: 8 },
            name: "x",
            value: "42",
        },
    ] },
    procedures: {},
}
```

Why `Statement::AssignConst` and not `Statement::AssignValue`?  Because
`"42"` is a single atomic token with no variable substitution — it's known
at compile time.

### Stage 4 — CFG → single basic block

With no control flow, the CFG is trivial:

```
Function {
    name: "::top",
    entry: entry_1,
    blocks: {
        entry_1: Block {
            name: "entry_1",
            statements: [Statement::AssignConst { name: "x", value: "42" }],
            terminator: None,
        },
        exit_2: Block {
            name: "exit_2",
            statements: [],
            terminator: None,
        },
    },
}
```

Blocks are keyed by `BlockId`, shown here by the display name
`Function::block_name` resolves each id to.  The builder creates an entry
block containing the statement, linked to an exit block via
`Terminator::Goto`.

### Stage 5 — SSA → x₀

With a single block and a single definition, SSA is trivial:

```
SsaFunction blocks:
  entry_1:
    phis: []
    statements: [
        SsaStatement {
            statement: Statement::AssignConst { name: "x", value: "42" },
            uses: {},
            defs: {x: 1},
        },
    ]
    entry_versions: {}
    exit_versions: {x: 1}
```

`uses`, `defs`, and the version maps key on the interned `Symbol`, shown here
by the display name `SsaFunction::var_name` resolves it to.

- `x` gets version 1 (its first definition): SSA value key `(x, 1)` (see [Glossary → SSA](#glossary)).
- No phi nodes — there is only one path through the program (see [Glossary → Phi node](#glossary)).
- `uses` is empty — `set x 42` doesn't read any variables.

### Stage 6 — Core analyses

**SCCP** (see [Glossary](#glossary))**:** `(x, 1)` → `LatticeValue(CONST, "42")` — provably constant.

**Type inference:** `(x, 1)` → `TypeLattice::of(TclType::Int)` — `"42"` is
a valid integer literal, so the intrep is INT.

**Liveness:** `(x, 1)` is dead if nothing reads it.

**Dead stores:** If `x` is never read, SCCP marks it as a dead store →
diagnostic `O109` (dead store elimination).

### Stage 7 — Bytecode (matches tclsh 9.0 identically)

```
  Literals:  0="x"  1="42"

  (0) push1 0       # push "x" onto stack
  (2) push1 1       # push "42" onto stack
  (4) storeStk      # pop name + value, store variable
  (5) done           # end of script
```

At top level, tclsh 9.0 uses `storeStk` (name-based variable storage via
the stack) because there is no local variable table.  Inside a `proc`,
this would become `storeScalar1` (LVT-indexed).

---

## Example 2: `set x 42; set y $x`

Variable assignment followed by a variable read — introduces `loadStk`.

### Source

```tcl
set x 42
set y $x
```

### Stage 1 — Lexer

```
Esc "set"   Sep " "   Esc "x"   Sep " "   Esc "42"   Eol "\n"
Esc "set"   Sep " "   Esc "y"   Sep " "   Var "x"    Eof ""
```

(each entry is a `Token` whose `kind` is the named `TokenType` variant and
whose `span` covers the quoted text)

Note the critical difference: `42` is `Esc` (plain text), but `$x` produces a
token whose `kind` is `TokenType::Var` spanning `x` — the `$` prefix is
consumed by the lexer's `content_offset`, and the spanned text is the bare
variable name.

### Stage 2 — Segmenter

Two `SegmentedCommand` values:

```
// Command 1: set x 42
SegmentedCommand {
    texts: ["set", "x", "42"],
    single_token_word: [true, true, true],   // all constant
}

// Command 2: set y $x
SegmentedCommand {
    texts: ["set", "y", "${x}"],             // Var token → "${x}" text
    single_token_word: [true, true, true],   // single token, but it's a Var
}
```

In command 2, `texts[2]` is `"${x}"` — the segmenter wraps `Var` tokens in
`${...}` form so the text reflects what the user wrote.

### Stage 3 — IR Lowering

```
Script { statements: [
    Statement::AssignConst { name: "x", value: "42" },
    Statement::AssignValue { name: "y", value: "${x}" },   // has variable reference
] }
```

The second `set` produces `Statement::AssignValue` (not
`Statement::AssignConst`) because the value `${x}` contains a variable
substitution that must be resolved at runtime.

### Stage 5 — SSA

```
  entry_1:
    SsaStatement { statement: Statement::AssignConst { name: "x", value: "42" },
                   uses: {}, defs: {x: 1} }
    SsaStatement { statement: Statement::AssignValue { name: "y", value: "${x}" },
                   uses: {x: 1}, defs: {y: 1} }
```

- `x₁ = "42"` — defined by the first `set`.
- `y₁` uses `x₁` — the SSA pass resolves `${x}` to version 1 of `x`.
- SCCP can prove `y₁` is also constant `"42"` (propagated from `x₁`).

### Stage 7 — Bytecode (matches tclsh 9.0)

```
  Literals:  0="x"  1="42"  2="y"

  (0)  push1 0       # "x"
  (2)  push1 1       # "42"
  (4)  storeStk      # x = "42"
  (5)  pop           # discard storeStk result
  (6)  push1 2       # "y"
  (8)  push1 0       # "x" (variable name to load)
  (10) loadStk       # push value of x
  (11) storeStk      # y = value of x
  (12) done
```

The `pop` at offset 5 discards the return value of the first `set` (Tcl
commands always return a value; intermediate results must be popped).

---

## Example 3: `expr {2 + 3}`

Compile-time constant folding — the expression is evaluated entirely by the
compiler.

### Source

```tcl
expr {2 + 3}
```

### Stage 3 — IR Lowering

The `expr` command with a braced body triggers expression parsing:

```
Script { statements: [
    Statement::ExprEval {
        span: Span { .. },
        expr: ExprNode::Binary {
            op: BinOp::Add,
            left: ExprNode::Literal { text: "2", start: 0, end: 1 },
            right: ExprNode::Literal { text: "3", start: 4, end: 5 },
        },
    },
] }
```

The expression parser produces a structured `ExprNode::Binary` node, not a raw
string.

### Stage 6 — SCCP

SCCP evaluates the expression: the two `ExprNode::Literal` operands `"2"` and
`"3"` fold to `CONST(5)`.  The compiler knows the result at compile time.

### Stage 7 — Bytecode (matches tclsh 9.0)

```
  Literals:  0="5"

  (0) push1 0       # "5" — folded result
  (2) done
```

The compiler folds `2 + 3` into the constant `5` at compile time — no
arithmetic opcodes are emitted.  This is identical to what `tclsh 9.0`
produces.

---

## Example 4: `expr {$a + $b}` (variables in expressions)

When expression operands are variables, the compiler emits inline arithmetic.

### Source

```tcl
set a 10
set b 20
expr {$a + $b}
```

### Stage 3 — IR Lowering

```
Script { statements: [
    Statement::AssignConst { name: "a", value: "10" },
    Statement::AssignConst { name: "b", value: "20" },
    Statement::ExprEval {
        expr: ExprNode::Binary {
            op: BinOp::Add,
            left: ExprNode::Var { text: "$a", name: "a", start: 0, end: 2 },
            right: ExprNode::Var { text: "$b", name: "b", start: 5, end: 7 },
        },
    },
] }
```

### Stage 5 — SSA

```
  a₁ = "10"   (CONST)
  b₁ = "20"   (CONST)
  Statement::ExprEval uses: {a: 1, b: 1}
```

SCCP propagates: `a₁ = 10`, `b₁ = 20`, so the expression result is
`CONST(30)`.

**Optimisation opportunity — O101 (fold constant integer expression):**
Since both operands are compile-time constants, the optimiser can suggest
replacing `expr {$a + $b}` with `30`.

### Stage 7 — Bytecode (matches tclsh 9.0)

```
  Literals:  0="a"  1="10"  2="b"  3="20"

  (0)  push1 0       # "a"
  (2)  push1 1       # "10"
  (4)  storeStk      # a = "10"
  (5)  pop
  (6)  push1 2       # "b"
  (8)  push1 3       # "20"
  (10) storeStk      # b = "20"
  (11) pop
  (12) push1 0       # "a"
  (14) loadStk       # push value of a
  (15) push1 2       # "b"
  (17) loadStk       # push value of b
  (18) add           # pop two operands, push sum
  (19) done
```

At the bytecode level, tclsh does not fold this — it emits `loadStk` +
`add` because the variables could theoretically be modified by traces.
The optimiser's O101 suggestion is a *diagnostic hint*, not a bytecode
transformation.

---

## Example 5: `if {$x} { set y 10 }`

The simplest conditional — introduces `Terminator::Branch` and forked
control flow.

### Source

```tcl
set x 1
if {$x} {
    set y 10
}
```

### Stage 3 — IR Lowering

```
Script { statements: [
    Statement::AssignConst { name: "x", value: "1" },
    Statement::If {
        span: Span { .. },
        clauses: [
            IfClause {
                condition: ExprNode::Var { text: "$x", name: "x", start: 0, end: 2 },
                condition_span: Span { .. },
                body: Script { statements: [
                    Statement::AssignConst { name: "y", value: "10" },
                ] },
                body_span: Span { .. },
            },
        ],
        else_body: None,
    },
] }
```

- `Statement::If` holds a `Vec` of `IfClause` values (one per `if`/`elseif`).
- The condition `{$x}` is parsed as `ExprNode::Var { name: "x", .. }`.
- No `else_body` for this example.

### Stage 4 — CFG decomposition

The `Statement::If` is decomposed into basic blocks:

```
  entry_1:
    statements: [Statement::AssignConst { name: "x", value: "1" }]
    terminator: Terminator::Branch {
        condition: ExprNode::Var { text: "$x", .. },
        true_target: if_then_3,
        false_target: if_next_4,
    }

  if_then_3:
    statements: [Statement::AssignConst { name: "y", value: "10" }]
    terminator: Terminator::Goto { target: if_end_2 }

  if_next_4:
    statements: []
    terminator: Terminator::Goto { target: if_end_2 }

  if_end_2:
    statements: []
    terminator: Terminator::Goto { target: exit_5 }
```

```
      entry_1
      ┌──────────────────────┐
      │ x = "1"              │
      │ branch($x)           │
      └──┬───────────────┬───┘
    true │               │ false
         ▼               ▼
   if_then_3        if_next_4
   ┌──────────┐     ┌────────┐
   │ y = "10" │     │ (empty)│
   └──┬───────┘     └──┬─────┘
      │                │
      ▼                ▼
      if_end_2 ────► exit_5
```

### Stage 5 — SSA

```
  entry_1:
    x₁ = "1"
    branch uses: {x: 1}

  if_then_3:
    y₁ = "10"

  if_end_2:
    (no phi nodes needed — y is only defined in one branch)
```

### Stage 6 — SCCP and constant branch detection

SCCP determines:
- `x₁ = CONST("1")` — `"1"` is truthy in Tcl.
- The branch condition is constant `true` → `if_next_4` is unreachable.

This produces a `ConstantBranch`:
```
ConstantBranch {
    block: "entry_1",
    condition: "$x",
    value: true,
    taken_target: "if_then_3",
    not_taken_target: "if_next_4",
}
```

**Optimisation opportunity — O112 (constant condition elimination):**
The condition `{$x}` is always true when `x` is `"1"`, so the `if` could
be eliminated, keeping only the body.

### Stage 7 — Bytecode (matches tclsh 9.0)

```
  Literals:  0="x"  1="1"  2="y"  3="10"  4=""

  (0)  push1 0       # "x"
  (2)  push1 1       # "1"
  (4)  storeStk      # x = "1"
  (5)  pop
  (6)  push1 0       # "x"
  (8)  loadStk       # push value of x
  (9)  nop           # alignment (tclsh artifact)
  (10) jumpFalse1 +9 # if false, jump to pc 19
  (12) push1 2       # "y"
  (14) push1 3       # "10"
  (16) storeStk      # y = "10"
  (17) jump1 +4      # skip to pc 21 (past empty-string push)
  (19) push1 4       # "" (if body not taken, result is empty string)
  (21) done
```

The `nop` at offset 9 is a tclsh artifact for instruction alignment.
The `push1 ""` at offset 19 provides the return value when the condition
is false (Tcl `if` returns the empty string if no branch is taken).

---

## Example 6: `if {1} { ... } else { ... }` (constant condition)

Demonstrates constant branch folding at the bytecode level.

### Source

```tcl
if {1} {
    set x 1
} else {
    set x 2
}
```

### Stage 3 — IR Lowering

```
Statement::If {
    clauses: [
        IfClause {
            condition: ExprNode::Literal { text: "1", start: 0, end: 1 },
            body: Script { statements: [Statement::AssignConst { name: "x", value: "1" }] },
        },
    ],
    else_body: Some(Script { statements: [Statement::AssignConst { name: "x", value: "2" }] }),
}
```

### Stage 4/5 — CFG and SCCP

SCCP immediately determines the `ExprNode::Literal` `"1"` is truthy → the
else branch is unreachable.  The constant branch detection marks `if_next`
as dead code.

### Stage 7 — Bytecode (matches tclsh 9.0)

```
  Literals:  0="x"  1="1"

  (0) push1 0       # "x"
  (2) push1 1       # "1"
  (4) storeStk      # x = "1"
  (5) done
```

The compiler (and tclsh) fold the constant condition entirely — the `else`
branch is eliminated, and only `set x 1` remains.  No `jumpFalse` or
branch instructions are emitted.

**Optimisation O112** would flag this pattern: "condition `{1}` is always
true; the else branch is unreachable dead code."

---

## Example 7: `if/elseif/else` chain

Multi-way branching with expression conditions.

### Source

```tcl
set x 5
if {$x < 0} {
    set sign -1
} elseif {$x > 0} {
    set sign 1
} else {
    set sign 0
}
```

### Stage 3 — IR Lowering

```
Statement::If {
    clauses: [
        IfClause {
            condition: ExprNode::Binary { op: BinOp::Lt,
                left: ExprNode::Var { text: "$x", name: "x", .. },
                right: ExprNode::Literal { text: "0", .. } },
            body: Script { statements: [Statement::AssignConst { name: "sign", value: "-1" }] },
        },
        IfClause {
            condition: ExprNode::Binary { op: BinOp::Gt,
                left: ExprNode::Var { text: "$x", name: "x", .. },
                right: ExprNode::Literal { text: "0", .. } },
            body: Script { statements: [Statement::AssignConst { name: "sign", value: "1" }] },
        },
    ],
    else_body: Some(Script { statements: [Statement::AssignConst { name: "sign", value: "0" }] }),
}
```

### Stage 4 — CFG decomposition

Each `elseif` clause chains to a new dispatch block:

```
      entry_1: x = "5"
      branch($x < 0)
      ┌────────┴────────┐
  true│                  │false
      ▼                  ▼
  if_then_3:         if_next_4:
  sign = "-1"        branch($x > 0)
      │              ┌────────┴────────┐
      │          true│                  │false
      │              ▼                  ▼
      │          if_then_5:         if_next_6:
      │          sign = "1"         sign = "0"
      │              │                  │
      ▼              ▼                  ▼
      └──────► if_end_2 ◄──────────────┘
```

### Stage 5 — SSA (phi nodes at merge)

```
  if_end_2:
    phi: sign₄ = phi(sign₁ from if_then_3,
                      sign₂ from if_then_5,
                      sign₃ from if_next_6)
```

Three definitions of `sign` merge at `if_end_2` — a phi node (see
[Glossary → Phi node](#glossary)) selects the correct version based on
which predecessor block executed.

SCCP determines `x₁ = CONST("5")`:
- `5 < 0` → false, so `if_then_3` is unreachable
- `5 > 0` → true, so `if_then_5` is taken
- Result: `sign` is `CONST("1")`

### Stage 7 — Bytecode (matches tclsh 9.0)

```
  Literals:  0="x"  1="5"  2="0"  3="sign"  4="-1"  5="1"

  (0)  push1 0       # "x"
  (2)  push1 1       # "5"
  (4)  storeStk      # x = "5"
  (5)  pop
  (6)  push1 0       # "x"
  (8)  loadStk       # load x
  (9)  push1 2       # "0"
  (11) lt            # x < 0 ?
  (12) jumpFalse1 +9 # if false, jump to pc 21
  (14) push1 3       # "sign"
  (16) push1 4       # "-1"
  (18) storeStk      # sign = -1
  (19) jump1 +22     # jump to done (pc 41)
  (21) push1 0       # "x"
  (23) loadStk       # load x
  (24) push1 2       # "0"
  (26) gt            # x > 0 ?
  (27) jumpFalse1 +9 # if false, jump to pc 36
  (29) push1 3       # "sign"
  (31) push1 5       # "1"
  (33) storeStk      # sign = 1
  (34) jump1 +7      # jump to done (pc 41)
  (36) push1 3       # "sign"
  (38) push1 2       # "0"
  (40) storeStk      # sign = 0
  (41) done
```

The `elseif` chain compiles to a cascade of `jumpFalse` instructions, each
skipping to the next condition test.  Each branch body ends with a `jump` to
the common exit point.

---

## Example 8: `while` loop

Introduces backward control flow edges and loop-condition-at-bottom layout.

### Source

```tcl
set i 0
while {$i < 5} {
    incr i
}
```

### Stage 3 — IR Lowering

```
Script { statements: [
    Statement::AssignConst { name: "i", value: "0" },
    Statement::While {
        condition: ExprNode::Binary { op: BinOp::Lt,
            left: ExprNode::Var { text: "$i", name: "i", .. },
            right: ExprNode::Literal { text: "5", .. } },
        body: Script { statements: [Statement::Incr { name: "i", .. }] },
    },
] }
```

- `Statement::While` has a structured `ExprNode::Binary` condition and a
  `Script` body.
- `Statement::Incr { name: "i", .. }` with `amount: None` means increment by 1.

### Stage 4 — CFG decomposition

```
  entry_1: i = "0"
      │
      ▼
  ┌──► while_header_3:
  │    branch($i < 5)
  │    ┌────────┴────────┐
  │ true│                 │false
  │    ▼                  ▼
  │  while_body_4:     while_end_5:
  │  incr i               │
  │    │                   ▼
  └────┘                 exit_6
```

The `while` decomposes into:
- A header block with the condition `Terminator::Branch`
- A body block that jumps back to the header (back edge)
- An exit block for when the condition is false

### Stage 5 — SSA with loop phi

```
  while_header_3:
    phi: i₂ = phi(i₁ from entry_1, i₃ from while_body_4)
    branch uses: {i: 2}

  while_body_4:
    Statement::Incr { name: "i", .. } → i₃ = i₂ + 1
```

The phi node at the loop header merges:
- `i₁ = 0` (initial value from entry)
- `i₃` (incremented value from the body)

SCCP cannot fold this to a constant (the value changes each iteration),
so `i₂` is `OVERDEFINED`.

### Stage 7 — Bytecode (matches tclsh 9.0)

```
  Literals:  0="i"  1="0"  2="5"  3=""

  (0)  push1 0       # "i"
  (2)  push1 1       # "0"
  (4)  storeStk      # i = "0"
  (5)  pop
  (6)  jump1 +7      # jump to condition test (pc 13)
  (8)  push1 0       # "i"         ◄── loop body start
  (10) incrStkImm +1 # incr i by 1
  (12) pop
  (13) push1 0       # "i"         ◄── condition test
  (15) loadStk       # load i
  (16) push1 2       # "5"
  (18) lt            # i < 5 ?
  (19) jumpTrue1 -11 # if true, jump back to pc 8 (body)
  (21) push1 3       # "" (loop result is empty string)
  (23) done
```

Key bytecode pattern: **condition-at-bottom layout**.  The initial `jump1`
at offset 6 skips over the body to the condition test.  The condition test
uses `jumpTrue1` with a *negative* offset to jump back to the body start.
This avoids an extra unconditional jump per iteration.

**Optimisation opportunity — O114 (incr idiom):**  If the loop body had
`set i [expr {$i + 1}]` instead of `incr i`, the optimiser would suggest
rewriting it to `incr i` (which compiles to the specialised `incrStkImm`
opcode).

---

## Example 9: `for` loop

The `for` loop adds init and step clauses around the while-style pattern.

### Source

```tcl
for {set i 0} {$i < 10} {incr i} {
    set x $i
}
```

### Stage 3 — IR Lowering

```
Statement::For {
    init: Script { statements: [Statement::AssignConst { name: "i", value: "0" }] },
    condition: ExprNode::Binary { op: BinOp::Lt,
        left: ExprNode::Var { text: "$i", name: "i", .. },
        right: ExprNode::Literal { text: "10", .. } },
    next: Script { statements: [Statement::Incr { name: "i", .. }] },
    body: Script { statements: [Statement::AssignValue { name: "x", value: "${i}" }] },
}
```

### Stage 4 — CFG decomposition

```
  entry_1: i = "0"  (init clause)
      │
      ▼
  ┌──► for_header_3:
  │    branch($i < 10)
  │    ┌────────┴────────┐
  │ true│                 │false
  │    ▼                  ▼
  │  for_body_4:       for_end_6:
  │  x = $i               │
  │    │                   ▼
  │    ▼                 exit_7
  │  for_step_5:
  │  incr i
  │    │
  └────┘
```

Unlike `while`, the `for` loop has a separate step block that runs after the
body, before looping back to the header.

### Stage 7 — Bytecode (matches tclsh 9.0)

```
  Literals:  0="i"  1="0"  2="x"  3="10"  4=""

  (0)  push1 0       # "i"
  (2)  push1 1       # "0"
  (4)  storeStk      # i = "0"     (init)
  (5)  pop
  (6)  jump1 +14     # jump to condition (pc 20)
  (8)  push1 2       # "x"         ◄── body start
  (10) push1 0       # "i"
  (12) loadStk       # load i
  (13) storeStk      # x = i
  (14) pop
  (15) push1 0       # "i"         ◄── step
  (17) incrStkImm +1 # incr i
  (19) pop
  (20) push1 0       # "i"         ◄── condition
  (22) loadStk       # load i
  (23) push1 3       # "10"
  (25) lt            # i < 10 ?
  (26) jumpTrue1 -18 # if true, back to pc 8
  (28) push1 4       # ""
  (30) done
```

Same condition-at-bottom layout as `while`, but with the step clause
(`incrStkImm`) placed between the body and the back-jump.

---

## Example 10: `foreach` (top-level)

At top level, `foreach` compiles as a generic `invokeStk` call rather than
an inlined loop.

### Source

```tcl
foreach item {a b c} {
    set x $item
}
```

### Stage 3 — IR Lowering

```
Statement::Foreach {
    iterators: [ForeachIterator { vars: ["item"], list_arg: "{a b c}", .. }],
    body: Script { statements: [Statement::AssignValue { name: "x", value: "${item}" }] },
    is_lmap: false,
}
```

### Stage 4 — CFG (top-level deferral)

At top level, `foreach` is **not** inlined into a loop CFG.  Instead, it is
emitted as an opaque `Statement::Call`:

```
Block {
    statements: [
        Statement::Call { command: "foreach",
                          args: ["item", "{a b c}", "\n    set x $item\n"],
                          defs: ["item"] },
    ],
}
```

This matches tclsh 9.0's behaviour: top-level `foreach` is compiled as a
generic `invokeStk` call to the `foreach` command.  Inside a `proc`, the
compiler would inline it with `foreach_start`/`foreach_step`/`foreach_end`
opcodes.

### Stage 7 — Bytecode (matches tclsh 9.0)

```
  Literals:  0="foreach"  1="item"  2="a b c"  3="\n    set x $item\n"

  (0)  push1 0       # "foreach"
  (2)  push1 1       # "item"
  (4)  push1 2       # "a b c"
  (6)  push1 3       # body script
  (8)  invokeStk1 4  # invoke foreach with 4 args
  (10) done
```

---

## Example 11: `proc` with expression body

Procedure definition shows the two-level compilation: the `proc` command
itself at top level, plus the procedure body compiled separately with
local variable table (LVT) optimisation.

### Source

```tcl
proc add {a b} {
    expr {$a + $b}
}
```

### Stage 3 — IR Lowering

```
Module {
    top_level: Script { statements: [] },   // proc def is extracted
    procedures: {
        "::add": Procedure {
            name: "add",
            qualified_name: "::add",
            params: ["a", "b"],
            body: Script { statements: [
                Statement::ExprEval {
                    expr: ExprNode::Binary { op: BinOp::Add,
                        left: ExprNode::Var { text: "$a", name: "a", .. },
                        right: ExprNode::Var { text: "$b", name: "b", .. } },
                },
            ] },
        },
    },
}
```

The procedure definition is extracted from `top_level` into
`Module::procedures`.  The top-level code emits the `proc` registration
as an `invokeStk` call.

### Stage 7 — Bytecode (matches tclsh 9.0)

**Top-level (proc registration):**
```
  Literals:  0="proc"  1="add"  2="{a b}"  3="{\n    expr {$a + $b}\n}"

  (0)  push1 0       # "proc"
  (2)  push1 1       # "add"
  (4)  push1 2       # "{a b}"
  (6)  push1 3       # body source
  (8)  invokeStk1 4  # proc add {a b} {...}
  (10) done
```

**Procedure body (::add):**
```
  LVT:  %v0="a"  %v1="b"

  (0) loadScalar1 %v0  # load param a from LVT slot 0
  (2) loadScalar1 %v1  # load param b from LVT slot 1
  (4) add              # a + b
  (5) done
```

Inside a proc, variables are accessed via `loadScalar1` using a
`LocalVarTable` (LVT) index — this is much faster than the `loadStk` name
lookup used at top level.  The `expr` body `{$a + $b}` is compiled inline
as `loadScalar1` + `loadScalar1` + `add`, not as an `exprStk` call.

---

## Example 12: Taint analysis — `HTTP::header` to `HTTP::respond` (subcommand flow and spec)

This example demonstrates taint analysis through iRules commands with
subcommand-level flow and spec metadata.  An HTTP header value (taint
source) flows through a `string tolower` sanitiser and into an HTTP
response body (taint sink), triggering an XSS warning.

### Source

```tcl
when HTTP_REQUEST {
    set host [HTTP::header value Host]
    set lower [string tolower $host]
    HTTP::respond 200 content "<h1>Welcome to $lower</h1>"
}
```

### Command registry — subcommand spec

The taint analysis relies on `CommandSpec` and `SubCommand` metadata from
the command registry.  Here is how each command contributes:

**`HTTP::header`** — a taint source at the command level, so every
invocation (including `HTTP::header value Host`) returns tainted data:

```rust
// rust/tcl-registry/src/commands/irules/http__header.rs (simplified)
const SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "value",
        arity: Arity::exact(1),
        detail: "Get first header value.",
        // reading a header has no side effects; no colour transform declared
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "replace",
        arity: Arity::new(1, 2),
        detail: "Replace header value.",
        mutator: true,              // writing a header IS a side effect
        ..SubCommand::DEFAULT
    },
    // …
];

pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "HTTP::header",
        traits: Traits::PURE
            .union(Traits::CSE_CANDIDATE)
            .union(Traits::DIAGRAM_ACTION),
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(1),
        subcommands: SUBCOMMANDS,
        taint_source: Some(TaintColour::TAINTED),   // return value is tainted
        taint_output_sink: Some("IRULE3002"),       // header injection
        taint_output_sink_subcommands: &["insert", "replace"],
        ..CommandSpec::DEFAULT
    }
}
```

**`HTTP::respond`** — the response body is a taint sink:

```rust
// rust/tcl-registry/src/commands/irules/http__respond.rs
// Tainted data in the response body → XSS / content injection.
taint_output_sink: Some("IRULE3001"),
// No `taint_output_sink_subcommands` → every invocation is a sink.
```

**`string tolower`** — a pure subcommand, but *not* a sanitiser:

```rust
// rust/tcl-registry/src/commands/tcl/string_.rs
SubCommand {
    name: "tolower",
    arity: Arity::new(1, 3),
    detail: "Convert to lower case.",
    pure: true,
    return_type: Some(TclType::String),
    // no `taint_transform` → does NOT strip taint
    ..SubCommand::DEFAULT
}
```

### Stage 3 — IR Lowering

```
Module {
    top_level: Script { statements: [] },
    procedures: {
        "::when::HTTP_REQUEST": Procedure {
            body: Script { statements: [
                Statement::AssignValue { name: "host", value: "[HTTP::header value Host]" },
                Statement::AssignValue { name: "lower", value: "[string tolower ${host}]" },
                Statement::Call { command: "HTTP::respond",
                                  args: ["200", "content",
                                         "<h1>Welcome to ${lower}</h1>"],
                                  defs: [], reads: ["lower"] },
            ] },
        },
    },
}
```

### Stage 4 — CFG

A single straight-line block (no control flow):

```
  entry_1:
    host = [HTTP::header value Host]
    lower = [string tolower $host]
    HTTP::respond 200 content "<h1>Welcome to $lower</h1>"
    → exit_2
```

### Stage 5 — SSA

```
  entry_1:
    SsaStatement { statement: Statement::AssignValue { name: "host", .. },
                   uses: {}, defs: {host: 1} }
    SsaStatement { statement: Statement::AssignValue { name: "lower", .. },
                   uses: {host: 1}, defs: {lower: 1} }
    SsaStatement { statement: Statement::Call { command: "HTTP::respond", .. },
                   uses: {lower: 1}, defs: {} }
```

### Taint propagation

The taint engine (`rust/tcl-compiler/src/optimiser/propagation.rs`)
walks the SSA graph and computes a `TaintLattice` for each SSA value key:

1. **`(host, 1)`** — the `[HTTP::header value Host]` command substitution
   is evaluated:
   - `_taint_source_colour("HTTP::header", ("value", "Host"))` looks up the
     `TaintHint` from the registry → returns `TaintColour::TAINTED`.
   - Result: `TaintLattice { colours: TaintColour::TAINTED }` —
     `is_tainted()` is true because `TAINTED` is a member of `colours`.

2. **`(lower, 1)`** — `[string tolower $host]`:
   - The argument `$host` has taint `(host, 1)` → `TAINTED`.
   - `_is_sanitiser("string", ("tolower", ...))` → `false` (case
     conversion does not sanitise).
   - `_derive_transform_colours("string", ("tolower", ...))` → no extra
     colours.
   - The command is pure, so taint flows through: result inherits from
     arguments.
   - Result: `TaintLattice { colours: TaintColour::TAINTED }`

3. **`HTTP::respond`** — the sink check:
   - `_classify_sink(Statement::Call { command: "HTTP::respond", .. })`
     queries the registry:
     `REGISTRY.classify_taint_sinks("HTTP::respond", None, dialect)`.
   - Returns `[("IRULE3001", "HTTP::respond")]` — the content body is an
     XSS-sensitive output sink.
   - `_stmt_var_arg_indexes(stmt, "lower")` → `(2,)` — `$lower` appears at
     arg index 2 (the content argument).
   - The taint of `(lower, 1)` is `TAINTED` with no mitigating colours
     (e.g. `HTML_ESCAPED` would suppress the warning).

### Taint warning emitted

```
TaintWarning {
    // the HTTP::respond line — line 3, columns 4..55
    span: Span { .. },
    variable: "lower",
    sink_command: "HTTP::respond",
    code: DiagCode::Irule3001,
    message: "Tainted variable $lower in HTTP response body (HTTP::respond); \
              risk of XSS or content injection",
}
```

### How to suppress the warning

Add `HTML::encode` before interpolation — this adds the `HTML_ESCAPED`
colour, which satisfies the IRULE3001 sink:

```tcl
when HTTP_REQUEST {
    set host [HTTP::header value Host]
    set safe [HTML::encode [string tolower $host]]
    HTTP::respond 200 content "<h1>Welcome to $safe</h1>"
}
```

Now `(safe, 1)` has
`TaintLattice { colours: TaintColour::TAINTED | TaintColour::HTML_ESCAPED }`.
The sink check sees `HTML_ESCAPED` is present → suppresses IRULE3001.

### Taint colour lattice at join points

If we add a branch:

```tcl
when HTTP_REQUEST {
    if {[HTTP::header exists Host]} {
        set val [HTTP::header value Host]
    } else {
        set val "unknown"
    }
    HTTP::respond 200 content $val
}
```

At the merge point after `if`:

```
  if_end:
    phi: val₃ = phi(val₁ from if_then, val₂ from if_else)
```

- `val₁` → `TaintLattice { colours: TaintColour::TAINTED }` (from HTTP::header)
- `val₂` → `TaintLattice { colours: TaintColour::empty() }` (constant "unknown")

`taint_join(val₁, val₂)`:
- Either operand tainted → result is tainted.
- Colours: only keep colours present in **both** tainted operands.
  Since `val₂` is untainted, it contributes the tainted operand's colours
  unchanged.
- Result: `TaintLattice { colours: TaintColour::TAINTED }`

The IRULE3001 warning fires on the `HTTP::respond` line.

---

## Example 13: ICIP — Interprocedural Constant Propagation (O103)

Demonstrates how the compiler evaluates a pure procedure call with constant
arguments at compile time and replaces it with the computed result.

### Source

```tcl
proc double {n} {
    expr {$n * 2}
}

set result [double 21]
puts $result
```

### Stage 3 — IR Lowering

```
Module {
    top_level: Script { statements: [
        Statement::AssignValue { name: "result", value: "[double 21]" },
        Statement::Call { command: "puts", args: ["${result}"] },
    ] },
    procedures: {
        "::double": Procedure {
            name: "double",
            qualified_name: "::double",
            params: ["n"],
            body: Script { statements: [
                Statement::ExprEval {
                    expr: ExprNode::Binary { op: BinOp::Mul,
                        left: ExprNode::Var { text: "$n", name: "n", .. },
                        right: ExprNode::Literal { text: "2", .. } },
                },
            ] },
        },
    },
}
```

### Interprocedural analysis

The `InterproceduralAnalysis` pass (`rust/tcl-compiler/src/interprocedural.rs`) builds summaries
for each procedure.  For `::double`:

1. The body is a single `expr {$n * 2}` in tail position.
2. SCCP within the proc body: parameter `n` is initially `UNKNOWN`.
3. The interprocedural solver calls `fold_static_proc_call("::double", ("21",))`:
   - Binds `n₁ = "21"` (constant).
   - Evaluates `$n * 2` → `21 * 2` → `CONST(42)`.
4. Result: the call `[double 21]` folds to the string `"42"`.

### Optimisation pass — O103

`optimise_static_proc_calls()` in
`rust/tcl-compiler/src/optimiser/propagation.rs`:

1. Encounters the `[double 21]` command substitution token.
2. Resolves `double` → qualified name `::double`.
3. Checks: `::double` is not in `ir_module.redefined_procedures`.
4. All arguments are static: `"21"` is a literal.
5. Calls `fold_static_proc_call(interproc, "::double", ("21",))`.
6. Gets back `"42"`.
7. Emits:

```
Optimisation {
    code: DiagCode::O103,
    message: "Fold static procedure call",
    span: Span { .. },   // [double 21] — line 4, columns 13..23
    replacement: "42",
}
```

After applying O103, the source becomes:

```tcl
set result 42
puts $result
```

This unlocks O100 (propagate `"42"` into `puts`) and potentially O109
(dead store elimination if `result` has no other uses).

---

## Example 14: Code Sinking — LCP (O125)

Demonstrates moving a loop-invariant assignment from before a conditional
into the specific branch that actually uses it.

### Source

```tcl
set msg "Request denied"
set code 403
if {[HTTP::uri] eq "/health"} {
    set code 200
    set msg "OK"
} else {
    HTTP::respond $code content $msg
}
```

### Problem

`set msg "Request denied"` and `set code 403` execute unconditionally, but
they are only used in the `else` branch.  In the `if` branch, both are
immediately overwritten.

### Optimisation pass — O125

`optimise_code_sinking()` in
`rust/tcl-compiler/src/optimiser/code_sinking.rs`:

1. **Sinkability check** (`_is_sinkable()`):
   `Statement::AssignConst { name: "msg", value: "Request denied" }` is
   sinkable — it is a simple constant assignment with no command
   substitutions.

2. **Variable reference scan** (`_stmt_uses_var()`): walks the
   `Statement::If` body recursively.  `$msg` appears only in the `else`
   body (`HTTP::respond ... $msg`), not in the `if` body (which defines a
   new `msg`).

3. **Deepest target** (`_find_deepest_sink_targets()`): the only use of the
   original `msg` is in the `else` branch → sink target is the else body.

4. **Emission** (`_emit_sinking_opts()`): emits a grouped pair of O125
   optimisations:

```
// Part 1: Comment out the original statement
Optimisation {
    code: DiagCode::O125,
    message: "Sink set msg \"Request denied\" into else body",
    span: Span { .. },   // set msg "Request denied" — line 0, columns 0..24
    replacement: "",
    group: Some(0),
}

// Part 2: Insert at the start of the else body
Optimisation {
    code: DiagCode::O125,
    message: "Insert sunk set msg \"Request denied\"",
    span: Span { .. },   // start of else body — line 6, column 0 (zero-width)
    replacement: "    set msg \"Request denied\"\n",
    group: Some(0),
}
```

After applying O125 to both statements:

```tcl
if {[HTTP::uri] eq "/health"} {
    set code 200
    set msg "OK"
} else {
    set msg "Request denied"
    set code 403
    HTTP::respond $code content $msg
}
```

The `set msg` and `set code` now only execute when they are actually
needed, avoiding wasted work on the `/health` path.

---

## Example 15: GVN/CSE — Redundant Computation (O105)

Demonstrates the Global Value Numbering pass detecting a pure command
invocation that is evaluated more than once.

### Source

```tcl
when HTTP_REQUEST {
    if {[HTTP::uri] starts_with "/api"} {
        pool api_pool
    }
    log local0. "Request to [HTTP::uri]"
}
```

### Problem

`HTTP::uri` is invoked twice — once in the `if` condition and once in the
`log` command.  It is a pure command (the URI does not change during a
single event), so the second call is redundant.

### GVN pass — O105

`find_redundant_computations()` in
`rust/tcl-compiler/src/gvn.rs`:

1. **Purity check** (`_is_pure_command()`): looks up `HTTP::uri` in the
   command registry → `CommandSpec.pure = true`, `cse_candidate = true`.

2. **Value numbering**: assigns a canonical `ExprKey` to each computation.
   Both `[HTTP::uri]` calls get the same key `("HTTP::uri",)`.

3. **Dominance check**: the first occurrence (in the `if` condition)
   dominates the second (in the `log` command) — every path to the `log`
   statement passes through the `if` condition first.

4. **Kill check**: no barriers or mutating commands between the two
   occurrences invalidate the value.

5. **Emission**:

```
RedundantComputation {
    span: Span { .. },        // second [HTTP::uri] — line 4, columns 28..40
    first_span: Span { .. },  // first [HTTP::uri]  — line 1, columns 8..20
    expression_text: "HTTP::uri",
    code: DiagCode::O105,
    message: "Redundant computation of [HTTP::uri]; result already \
              available from line 2",
}
```

The fix is to extract to a local variable:

```tcl
when HTTP_REQUEST {
    set uri [HTTP::uri]
    if {$uri starts_with "/api"} {
        pool api_pool
    }
    log local0. "Request to $uri"
}
```

---

## Example 16: DCE and DSE — Dead Code and Dead Store Elimination (O107/O108/O109)

Demonstrates the elimination passes that remove code whose results are
never used.

### Source

```tcl
proc compute {x} {
    set temp [expr {$x * 2}]
    set unused 99
    set result [expr {$temp + 1}]
    return $result
}
```

### SSA and liveness

```
  entry_1:
    temp₁ = expr {$x * 2}     uses: {x: 1}  defs: {temp: 1}
    unused₁ = 99               uses: {}       defs: {unused: 1}
    result₁ = expr {$temp + 1} uses: {temp: 1} defs: {result: 1}
    return $result              uses: {result: 1}
```

**Liveness analysis:**
- `result₁` is live (read by `return`).
- `temp₁` is live (read by the `result` expression).
- `unused₁` is dead — never read by any statement.

### Elimination passes

`optimise_elimination_passes()` in
`rust/tcl-compiler/src/optimiser/elimination.rs`:

**O109 — Dead Store Elimination:**
`unused₁` is assigned but never read.  The assignment `set unused 99` is
a dead store:

```
Optimisation {
    code: DiagCode::O109,
    message: "Dead store: unused is set but never read",
    span: Span { .. },   // set unused 99
    replacement: "",
}
```

**O107 — Dead Code Elimination (basic DCE):**
If a conditional branch is unreachable (e.g. dead code after `return`), the
entire block is flagged:

```tcl
proc example {} {
    return 1
    set x 42    ;# unreachable — O107
}
```

**O108 — Aggressive DCE (ADCE):**
Tracks statement-level liveness backwards from live roots (return values,
side-effecting calls).  A statement is dead if its defined values are never
used AND it has no side effects (checked via `_is_adce_removable_statement()`
which inspects `FunctionExecutionIntent` for command substitution purity).

---

## Example 17: Pattern Recognition — String Build Chains (O104) and Incr Idioms (O114)

### O104 — String Build Chain

Detects `set` + `append` sequences that build a string incrementally
and suggests `string cat` or direct concatenation.

```tcl
set result ""
append result "Hello "
append result $name
append result "!"
```

`optimise_string_build_chains()` recognises the `set` + `append` pattern →
suggests combining into `set result "Hello ${name}!"` (O104).

### O114 — Incr Idiom

Detects `set x [expr {$x + N}]` patterns and suggests `incr x N`,
which compiles to the specialised `incrStkImm` opcode:

```tcl
# Before
set count [expr {$count + 1}]

# After (O114)
incr count
```

`optimise_incr_idioms()` matches the pattern: the `set` target is the
same variable used in the expression, and the expression is an integer
addition.

---

## Example 18: Structure Elimination via SCCP (O112)

When SCCP proves a branch condition is constant, the entire control flow
structure can be replaced with just the taken branch's body.

```tcl
set debug 0
if {$debug} {
    puts "Debug: entering handler"
}
pool main_pool
```

SCCP determines `debug₁ = CONST("0")` → the `if` condition is always
false → the body is unreachable.

`optimise_structure_elimination()` in
`rust/tcl-compiler/src/optimiser/structure_elimination.rs`
replaces the entire `if {$debug} { ... }` block with nothing (O112),
and a grouped O109 removes the dead `set debug 0`:

```tcl
pool main_pool
```

---

## Example 19: Tail-Call Optimisation (O121/O122/O126)

### O121 — Tail-call suggestion

Detects self-recursive calls in tail position and suggests `tailcall`:

```tcl
proc factorial {n acc} {
    if {$n <= 1} {
        return $acc
    }
    factorial [expr {$n - 1}] [expr {$acc * $n}]
}
```

The recursive `factorial` call is the last expression in the proc →
suggest `tailcall factorial ...` (O121).

### O126 — Dead variable after tail position

When a variable is assigned but only consumed by a tail-position return
that can be eliminated, O126 removes the dead store.

---

## Example 20: Error recovery — unclosed bracket

Demonstrates how the parser handles malformed input by injecting virtual
tokens so downstream passes receive clean commands.

### Source (malformed)

```tcl
set x [string length "hello"
set y 42
```

The `[` on line 1 is never closed — `string length "hello"` runs to the
end of the line without a matching `]`.

### Stage 0 — Error recovery

`rust/tcl-compiler/src/segmenter.rs` runs a first-pass parse via
`segment_commands()`.  The segmenter detects that the `Cmd` token starting
at `[` is unterminated (the character after the `Cmd` text is not `]`).

**Heuristic — command-break detection** (`e201_at_command`):
The next non-blank line starts with `set` — a known command name.  This
signals that `]` should be inserted at the end of line 1.

`segment_with_recovery()` records the insertion as a *ghost* entry in its
`ghosts: BTreeMap<u32, u8>` (source offset → inserted byte), paired with the
`Diagnostic` the heuristic produced.  There is no distinct token type for the
insertion — the ghost byte is fed back into the re-lex:

```
// ghosts entry: 29u32 → b']'
//   offset 29 = end of "hello" on line 1; b']' = the missing delimiter
Diagnostic {
    code: DiagCode::E201,
    message: "missing close-bracket",
    span: Span { .. },   // line 0, column 8 (zero-width)
    severity: Severity::Error,
    fixes: [CodeFix {
        description: "Insert \"]\"",
        ..
    }],
}
```

**Re-parse with the ghost byte:** The lexer sees the injected `]` and
produces a clean `Cmd` token.  The second parse yields two well-formed
`SegmentedCommand` values:

```
// Command 1 (recovered):
SegmentedCommand { texts: ["set", "x", "[string length \"hello\"]"] }

// Command 2 (clean):
SegmentedCommand { texts: ["set", "y", "42"] }
```

Both downstream passes (IR lowering, CFG, SSA, codegen) proceed on the
clean parse.  The `E201` diagnostic is emitted to the editor so the user
sees the error.

### Error recovery heuristics

| Code | Condition | Heuristic |
|------|-----------|-----------|
| E201 | Missing `]` | `#` comment on next line, known command on next line, or `{` inside `[` |
| E202 | Missing `"` | Newline with known command on next non-blank line |
| E203 | Missing `}` | De-indented line starting with a known command |
| E204 | Extra chars after `}` | Lexer warning |
| E205 | Extra chars after `"` | Lexer warning |
| E206 | Missing `}` for `${var` | Lexer warning |

---

## Example 21: Expression parsing — braced vs unbraced

Shows how the Pratt parser handles Tcl `expr` bodies and how braced vs
unbraced expressions differ.

### Braced expression: `expr {$a + $b * 2}`

The braces protect the expression from Tcl substitution — the content is
passed verbatim to the expression parser.

**Tokenisation** (`tokenise_expr()`): produces `ExprToken` stream:

```
ExprToken { kind: ExprTokenType::Variable, text: "$a", .. }
ExprToken { kind: ExprTokenType::Operator, text: "+",  .. }
ExprToken { kind: ExprTokenType::Variable, text: "$b", .. }
ExprToken { kind: ExprTokenType::Operator, text: "*",  .. }
ExprToken { kind: ExprTokenType::Number,   text: "2",  .. }
```

**Pratt parsing** (`_PrattParser` in
`rust/tcl-syntax/src/expr/parser.rs`):

The parser uses binding powers to handle precedence:
- `*` has binding power (22, 23) — higher than `+` at (20, 21).
- So `$b * 2` binds tighter than `$a + ...`.

Result:

```
ExprNode::Binary {
    op: BinOp::Add,
    left: ExprNode::Var { text: "$a", name: "a", .. },
    right: ExprNode::Binary {
        op: BinOp::Mul,
        left: ExprNode::Var { text: "$b", name: "b", .. },
        right: ExprNode::Literal { text: "2", .. },
    },
}
```

### Unbraced expression: `expr $a + $b * 2`

Without braces, Tcl performs variable substitution *before* the
expression is compiled.  The segmenter sees multiple tokens:

```
Token(VAR, "a")  Token(ESC, "+")  Token(VAR, "b")  ...
```

These are concatenated into a single text `"${a} + ${b} * 2"`.
The expression parser receives a string with *already-substituted*
variable references, but since it cannot know the runtime values, it
falls back to:

```
ExprNode::Raw { text: "${a} + ${b} * 2" }
```

`ExprNode::Raw` is the fallback — the compiler cannot statically analyse the
expression, which is why diagnostic **W100** ("Unbraced expr body")
warns about this pattern.  Braced expressions enable compile-time
parsing, constant folding, and type inference.

### iRules extensions

The Pratt parser also handles iRules-specific operators at the same
precedence as their symbolic counterparts:

| iRules operator | Equivalent | Binding power |
|----------------|------------|--------------|
| `starts_with` | `eq` (prefix) | (14, 15) |
| `ends_with` | `eq` (suffix) | (14, 15) |
| `contains` | `eq` (substring) | (14, 15) |
| `matches_glob` | glob match | (14, 15) |
| `matches_regex` | regexp | (14, 15) |
| `and` / `or` | `&&` / `\|\|` | (6,7) / (4,5) |

---

## Example 22: Lowering dispatch — `arg_roles` and command classification

Shows how `_lower_command()` in
`rust/tcl-compiler/src/lowering/mod.rs` dispatches each command to
the appropriate IR node using registry metadata.

### Dispatch hierarchy

```
_lower_command(cmd)
    │
    ├─ Check lowering hook on CommandSpec → spec.lowering(lowerer, cmd)
    │   (e.g. set → lower_set(), incr → lower_incr())
    │
    ├─ match cmd_name:
    │   ├─ "proc"     → extract params, lower body, register Procedure
    │   ├─ "when"     → lower iRules event handler body
    │   ├─ "if"       → _lower_if() → Statement::If with IfClause list
    │   ├─ "for"      → _lower_for() → Statement::For (init, cond, step, body)
    │   ├─ "while"    → _lower_while() → Statement::While (cond, body)
    │   ├─ "foreach"  → _lower_foreach() → Statement::Foreach
    │   ├─ "catch"    → _lower_catch() → Statement::Catch
    │   ├─ "try"      → _lower_try() → Statement::Try with TryHandler
    │   ├─ "switch"   → _lower_switch() → Statement::Switch with SwitchArm
    │   ├─ eval/uplevel/upvar → Statement::Barrier (defeats static analysis)
    │   │
    │   └─ default (fallthrough):
    │       ├─ arg_indices_for_role(BODY) → Statement::Barrier (has body args)
    │       ├─ arg_indices_for_role(VAR_NAME) → Statement::Call with defs
    │       └─ else → Statement::Call (generic)
```

### Example: `lower_set()` — the `set` lowering hook

`set` has a registered lowering hook
(`rust/tcl-compiler/src/var_refs.rs`).
It pattern-matches on the second argument's token type:

| Token type of `args[1]` | IR node produced | Example |
|-------------------------|-----------------|---------|
| `Str` (braced string) | `Statement::AssignConst` | `set x {hello}` |
| `Esc` (decimal integer) | `Statement::AssignConst` | `set x 42` |
| `Cmd` wrapping `expr` | `Statement::AssignExpr` | `set x [expr {$a + 1}]` |
| `Var` or interpolated | `Statement::AssignValue` | `set x $y`, `set x "hi $name"` |
| 0 args (getter) | `Statement::Call` | `set x` (read variable) |

### Example: fallthrough with `arg_roles`

For a command like `regexp`:

```tcl
regexp {(\d+)} $input match submatch
```

The registry declares `ArgRole::VarWrite` at arg indices 2 and 3 (the
match variables).  The fallthrough path calls:

```
let var_indices = registry.arg_indices_for_role("regexp", args, ArgRole::VarWrite);
// → [2, 3]  (match, submatch)
```

This produces:

```
Statement::Call {
    command: "regexp",
    args: [r"(\d+)", "${input}", "match", "submatch"],
    defs: ["match", "submatch"],   // SSA tracks these as definitions
}
```

The `defs` list tells the SSA builder that `regexp` defines `match` and
`submatch`, so they get new SSA versions.

### Example: barrier commands

Commands in `_DYNAMIC_BARRIER_COMMANDS` (e.g. `eval`, `uplevel`, `upvar`)
always produce `Statement::Barrier`:

```tcl
eval $script
```

```
Statement::Barrier {
    span: Span { .. },
    reason: "dynamic command",
    command: "eval",
    args: ["${script}"],
}
```

`Statement::Barrier` tells all downstream passes: *stop reasoning about
variable state here* — the command can read/write any variable, define new
procedures, or modify the call stack.

---

## Example 23: Execution intent — command substitution classification

Shows how `FunctionExecutionIntent` classifies command substitutions
for use by ADCE, shimmer detection, and other passes.

### Source

```tcl
proc process {items} {
    set count [llength $items]
    set label [format "Total: %d" $count]
    set result [http::geturl $url]
    return $label
}
```

### Execution intent construction

`build_execution_intent()` in
`rust/tcl-compiler/src/execution_intent.rs` walks each
`Statement::AssignValue` in the CFG and parses the command substitution:

**`[llength $items]`:**

```
CommandSubstitutionIntent {
    command: "llength",
    args: ["$items"],
    arg_categories: [SubstitutionCategory::ScalarVar],
    side_effect: SideEffectClass::Pure,      // llength is pure
    escape: EscapeClass::NoEscape,           // no dynamic barriers
    shimmer_pressure: 1,                     // one var arg
}
```

**`[format "Total: %d" $count]`:**

```
CommandSubstitutionIntent {
    command: "format",
    args: ["\"Total: %d\"", "$count"],
    arg_categories: [SubstitutionCategory::Literal, SubstitutionCategory::ScalarVar],
    side_effect: SideEffectClass::Pure,
    escape: EscapeClass::NoEscape,
    shimmer_pressure: 1,
}
```

**`[http::geturl $url]`:**

```
CommandSubstitutionIntent {
    command: "http::geturl",
    args: ["$url"],
    arg_categories: [SubstitutionCategory::ScalarVar],
    side_effect: SideEffectClass::MaySideEffect,  // network I/O
    escape: EscapeClass::MayEscape,               // may throw
    shimmer_pressure: 1,
}
```

### How ADCE uses execution intent

`_is_adce_removable_statement()` checks the intent before removing a
"dead" assignment:

- `[llength $items]` → `Pure` + `NoEscape` → **safe to remove** if `count`
  is never read.
- `[http::geturl $url]` → `MaySideEffect` → **cannot remove** even if
  `result` is never read (the network call is observable).

---

## Example 24: Interprocedural analysis — summary construction

Shows how `InterproceduralAnalysis` builds `ProcSummary` objects that
describe each procedure's behaviour for cross-procedure optimisation.

### Source

```tcl
proc helper {x} {
    return [expr {$x * 2}]
}

proc main {a b} {
    set r [helper $a]
    puts $r
}
```

### Phase 1 — Local facts (`ProcLocalSummary`)

For each procedure, the interprocedural pass builds local facts by
walking the IR:

**`::helper`:**

```
ProcLocalSummary(
    qualified_name="::helper",
    params=("x",),
    arity=Arity(1, 1),
    calls=(),                        # no internal proc calls
    has_barrier=false,               # no eval/uplevel
    has_unknown_calls=false,
    writes_global=false,
    local_effect_reads=EffectRegion(0),   # reads only local params
    local_effect_writes=EffectRegion(0),  # writes only local var
    returns_constant=false,               # depends on param
    constant_return=None,
    return_depends_on_params=("x",),      # return value depends on x
    return_passthrough_param=None,
)
```

**`::main`:**

```
ProcLocalSummary(
    qualified_name="::main",
    params=("a", "b"),
    arity=Arity(2, 2),
    calls=("::helper",),            # calls helper
    has_barrier=false,
    has_unknown_calls=false,
    writes_global=false,
    local_effect_reads=EffectRegion(0),
    local_effect_writes=EffectRegion.LOG_IO,  # puts writes to output
)
```

### Phase 2 — Transitive closure

The solver iterates over the call graph to propagate effects:

1. `::helper` has no callees → its summary is final.
2. `::main` calls `::helper`:
   - `::helper` is pure → no additional effect reads/writes propagated.
   - `::main` calls `puts` → `LOG_IO` effect write.
   - `::main` is NOT pure (has `puts` side effect).

### Phase 3 — Constant folding eligibility

`::helper` meets the criteria for `can_fold_static_calls`:
- No barrier
- No unknown calls
- No global writes
- Return depends only on parameters
- Body is a single expression

When the optimiser encounters `[helper 21]` with a constant argument,
`fold_static_proc_call()` evaluates the body with `x₁ = 21` →
`21 * 2` = `42` (O103).

### Final `ProcSummary`

```
ProcSummary {
    qualified_name: "::helper",
    params: ["x"],
    arity: Arity { min: 1, max: 1, .. },
    calls: [],
    has_barrier: false,
    has_unknown_calls: false,
    writes_global: false,
    pure: true,
    effect_reads: EffectRegion::NONE,
    effect_writes: EffectRegion::NONE,
    returns_constant: false,
    constant_return: None,
    return_depends_on_params: ["x"],
    return_passthrough_param: None,
    can_fold_static_calls: true,
}
```

---

## Example 25: Connection scope — cross-event variable flow (iRules)

Shows how `ConnectionScope` analysis tracks variables that flow between
`when` event handlers in iRules.

### Source

```tcl
when CLIENT_ACCEPTED {
    set conn_start [clock seconds]
    set request_count 0
}

when HTTP_REQUEST {
    incr request_count
    log local0. "Request #$request_count on conn from $conn_start"
}
```

### Problem

In iRules, `when` event handlers share a connection-scoped variable
stack.  Variables set in `CLIENT_ACCEPTED` persist until the connection
closes, so `conn_start` and `request_count` in `HTTP_REQUEST` are
*not* read-before-set errors — they were defined in an earlier event.

Without connection scope analysis, the compiler would emit false
positives: `W103` (read before set) for `$request_count` and
`$conn_start` in `HTTP_REQUEST`.

### EventVarSummary construction

`extract_event_summary()` in
`rust/tcl-compiler/src/connection_scope.rs` walks
each event's SSA blocks:

**CLIENT_ACCEPTED:**

```
EventVarSummary {
    event: "CLIENT_ACCEPTED",
    defs: {"conn_start", "request_count"},
    uses_before_def: {},    // no version-0 reads
    unsets: {},
}
```

**HTTP_REQUEST:**

```
EventVarSummary {
    event: "HTTP_REQUEST",
    defs: {"request_count"},      // incr defines it
    uses_before_def: {"request_count", "conn_start"},  // version 0
    unsets: {},
}
```

### Cross-event set computation

`build_connection_scope()` compares every pair of events:

- `CLIENT_ACCEPTED` defines `{conn_start, request_count}`.
- `HTTP_REQUEST` uses-before-def `{request_count, conn_start}`.
- Intersection: `{conn_start, request_count}` — these flow across events.

```
ConnectionScope {
    summaries: {/* … */},
    cross_event_defs: {"conn_start", "request_count"},
    cross_event_imports: {"conn_start", "request_count"},
}
```

### Effect on diagnostics

The optimiser's `PassContext` receives `cross_event_vars` when processing
`HTTP_REQUEST`.  Dead store elimination (O109) and read-before-set
diagnostics check whether a variable is in `cross_event_vars` before
reporting — suppressing false positives for `conn_start` and
`request_count`.

---

## Example 26: Namespace resolution

Shows how qualified names are resolved throughout the pipeline.

### Source

```tcl
namespace eval mylib {
    proc helper {x} { expr {$x + 1} }
    proc compute {a} { helper $a }
}

mylib::compute 5
```

### `normalise_qualified_name()` — the core helper

`rust/tcl-syntax/src/naming.rs` provides the canonical form:

```
normalise_qualified_name("helper")       → "::helper"
normalise_qualified_name("::helper")     → "::helper"
normalise_qualified_name("mylib::helper") → "::mylib::helper"
normalise_qualified_name("::mylib::helper") → "::mylib::helper"

# Collapsed double-colons:
normalise_qualified_name("::::foo::::bar") → "::foo::bar"
```

### How namespace context propagates through lowering

`_lower_command()` carries a `namespace` parameter that tracks the
current namespace:

1. Top-level: `namespace="::"`
2. Inside `namespace eval mylib { ... }`:
   - `_join_namespace("::", "mylib")` → `"::mylib"`
   - `namespace="::mylib"` is passed to body lowering
3. `proc helper` inside `::mylib`:
   - `_qualify_proc_name("::mylib", "helper")` → `"::mylib::helper"`
4. `proc compute` inside `::mylib`:
   - `_qualify_proc_name("::mylib", "compute")` → `"::mylib::compute"`

### How interprocedural analysis resolves calls

Inside `::mylib::compute`, the call `helper $a` is unqualified.
`resolve_internal_call("helper", "::mylib::compute", known_procs)`:

1. Extract namespace parts from caller: `["mylib"]`
2. Try `::mylib::helper` → found in known procs → return it.

If not found in `::mylib`, it would try `::helper` (global namespace),
walking up the namespace hierarchy.

### Resulting IR module

```
Module {
    procedures: {
        "::mylib::helper": Procedure { name: "helper", .. },
        "::mylib::compute": Procedure { name: "compute", .. },
    },
}
```

---

## Example 27: Codegen internals — labels, LVT, linearisation, peephole

Traces the detailed bytecode generation process for a procedure with
control flow.

### Source

```tcl
proc abs {n} {
    if {$n < 0} {
        expr {-$n}
    } else {
        set n
    }
}
```

### Step 1 — LVT allocation

The `_Emitter` constructor creates a `LocalVarTable` from the
parameter list:

```
LocalVarTable::new(&["n"])
// LVT slots: %v0 = "n"
```

Inside a `proc`, all variable accesses use LVT-indexed instructions
(`loadScalar1 %v0`) instead of name-based stack operations (`loadStk`).

### Step 2 — Block linearisation (`_linearise()`)

The emitter performs a DFS traversal from the entry block, producing a
reverse post-order (RPO) that determines instruction layout:

```
DFS visit order: entry_1 → if_then_3 → if_end_2 → exit → if_else_4

RPO (reversed): entry_1, if_then_3, if_else_4, if_end_2, exit
```

The `if_then_3` (true branch) appears immediately after the condition
so `jumpFalse` skips *forward* to `if_else_4` — matching tclsh's
fall-through layout.

For loops, `_reorder_bottom_tested()` detects back-edges and moves the
loop body *before* the header, producing a condition-at-bottom layout:

```
Before (top-tested):  header → body → jump header
After  (bottom-tested): jump header → body → header (jumpTrue body)
```

### Step 3 — Instruction emission with labels

As the emitter walks blocks, it places labels and emits instructions:

```
_place_label("entry_1")        → label at instruction 0
  emit(LOAD_SCALAR1, %v0)     # load n
  emit(PUSH1, lit("0"))       # push "0"
  emit(LT)                    # n < 0
  emit(JUMP_FALSE4, "L_else") # → if_else_4

_place_label("if_then_3")
  emit(LOAD_SCALAR1, %v0)     # load n
  emit(UMINUS)                # negate
  emit(JUMP4, "L_end")        # → if_end_2

_place_label("L_else")        → if_else_4
  emit(LOAD_SCALAR1, %v0)     # just return n

_place_label("L_end")         → if_end_2
  emit(DONE)
```

### Step 4 — Jump size optimisation (`optimise_jumps()`)

`rust/tcl-bytecode/src/layout.rs` iterates up to 10
times, replacing 4-byte jumps with 1-byte jumps when the relative
offset fits in [-128, 127]:

```
Pass 1:
  JUMP_FALSE4 "L_else"  (offset: +12 bytes)
  → fits in 1 byte → JUMP_FALSE1 "L_else"

  JUMP4 "L_end"  (offset: +4 bytes)
  → fits in 1 byte → JUMP1 "L_end"
```

Shortening jumps changes instruction sizes, which changes offsets,
which may enable more shortenings — hence the iterative approach.

### Step 5 — Label resolution (`resolve_layout()`)

Final pass assigns concrete byte offsets:

```
label_offsets = resolve_layout(instrs, labels)
# {"entry_1": 0, "if_then_3": 8, "L_else": 14, "L_end": 16}
```

Jump operands are patched from label names to relative byte offsets.

### Step 6 — Peephole optimisation

`_PeepholeMixin` applies tclsh-matching rewrites:

1. **`_remove_trailing_pop()`**: The last statement's result stays on
   the stack for `done` to return.  Strip `pop; done` → `done`.

2. **`_fold_const_push_pop_nops()`**: Dead constant results (`push; pop`
   pairs from folded branches) become `nop; nop; nop` — matching tclsh's
   3-nop pattern for folded constants.

3. **`_dedup_push_literals()`**: After nop-folding, surviving `push`
   instructions may reference duplicate literal slots.  Deduplicate
   to match tclsh's literal table interning.

### Step 7 — Literal table construction

The `LiteralTable` interns strings as they are referenced:

```
LiteralTable entries:
  0 = "n"     (parameter name, also used in loadScalar1)
  1 = "0"     (comparison constant)
```

Strings are deduplicated: if `"n"` is referenced twice, both get
slot 0.

### Final bytecode (matches tclsh 9.0)

```
  LVT:  %v0="n"
  Literals:  0="0"

  (0)  loadScalar1 %v0  # load n
  (2)  push1 0          # "0"
  (4)  lt               # n < 0 ?
  (5)  jumpFalse1 +5    # jump to pc 10
  (7)  loadScalar1 %v0  # load n (then-body)
  (9)  uminus           # negate
  (10) jump1 +3         # jump to pc 13
  (12) loadScalar1 %v0  # load n (else-body)
  (14) done
```

---

## Example 28: Side-effects classification

Shows how `classify_side_effects()` builds structured
`CommandSideEffects` from registry metadata.

### Source

```tcl
when HTTP_REQUEST {
    set uri [HTTP::uri]
    HTTP::header replace Host "example.com"
    pool my_pool
    log local0. "Routing $uri"
}
```

### `classify_side_effects()` for each command

`rust/tcl-compiler/src/side_effects.rs`:

**`HTTP::uri` (getter form):**

```
CommandSideEffects {
    effects: [
        SideEffect {
            target: SideEffectTarget::HttpUri,
            reads: true,
            writes: false,
            storage_type: StorageType::Scalar,
            scope: StorageScope::Event,
            connection_side: ConnectionSide::Client,
        },
    ],
    pure: true,             // reading is side-effect-free
    deterministic: true,    // same result within one event
    dynamic_barrier: false,
}
```

**`HTTP::header replace Host "example.com"` (setter form):**

```
CommandSideEffects {
    effects: [
        SideEffect {
            target: SideEffectTarget::HttpHeader,
            reads: false,
            writes: true,                     // modifying a header
            storage_type: StorageType::Scalar,
            scope: StorageScope::Event,
            connection_side: ConnectionSide::Client,
            key: Some("Host"),                // literal header name
        },
    ],
    pure: false,            // writing is a side effect
    deterministic: false,
    dynamic_barrier: false,
}
```

**`pool my_pool`:**

```
CommandSideEffects {
    effects: [
        SideEffect {
            target: SideEffectTarget::PoolSelection,
            reads: false,
            writes: true,
            storage_type: StorageType::Scalar,
            scope: StorageScope::Connection,
            connection_side: ConnectionSide::Server,
        },
    ],
    pure: false,
    deterministic: false,
}
```

**`log local0. "Routing $uri"`:**

```
CommandSideEffects {
    effects: [
        SideEffect {
            target: SideEffectTarget::LogIo,
            reads: false,
            writes: true,
            storage_type: StorageType::Scalar,
            scope: StorageScope::Global,
            connection_side: ConnectionSide::None,
        },
    ],
    pure: false,
    deterministic: false,
}
```

### How classification resolves form and subcommand

The classification function follows this resolution order:

1. **Interprocedural summary** — if `callee_summary` is provided (for
   user-defined procs), use its `effect_reads`/`effect_writes` directly.

2. **Dynamic barriers** — `eval`, `uplevel` → `dynamic_barrier=true`,
   all effects unknown.

3. **Subcommand resolution** — for `HTTP::header replace`:
   - Look up `CommandSpec` for `HTTP::header`.
   - Find `SubCommand` for `replace` → `mutator=true`.
   - Read `side_effect_hints` from the subcommand.

4. **Form resolution** — for `HTTP::uri` (no args):
   - `CommandSpec.resolve_form(args)` matches the getter form
     (`arity=Arity(0, 0)`).
   - Getter form has `pure=true` and `reads=true` hints.

### How consumers use side effects

| Consumer | Uses |
|----------|------|
| **GVN/CSE** | `pure=true` → result can be cached (O105) |
| **ADCE** | `pure=true` + `NoEscape` → statement is removable |
| **Optimiser** | `pure=false` → cannot propagate across this command |
| **iRules flow** | `RESPONSE_LIFECYCLE` write → response-commit tracking |
| **Taint engine** | `pure=true` → taint flows through unchanged |

---

## Optimisation opportunities across examples

The following table summarises all optimisation passes the compiler can
detect, their triggers, and example patterns:

| Code | Name | Trigger | Example |
|------|------|---------|---------|
| O100 | Constant propagation | Variable has a known constant value | `set x 5; puts $x` → propagate `"5"` into `puts` |
| O101 | Fold constant expression | All `expr` operands are constants | `expr {2 + 3}` → `5` |
| O102 | Fold expr command substitution | `[expr {...}]` with constant result | `set x [expr {1}]` → `set x 1` |
| O103 | Interprocedural constant fold (ICIP) | Pure proc called with all-constant args | `[double 21]` → `42` (when `proc double {n} { expr {$n * 2} }`) |
| O104 | String build chain | `set` + `append` sequence detected | `set s ""; append s "a"; append s "b"` → `set s "ab"` |
| O105 | GVN/CSE redundancy | Same pure computation appears twice | `[HTTP::uri]` used twice → extract to variable |
| O107 | Dead code elimination (DCE) | Unreachable code after `return`/`break` | Code after `return` is dead |
| O108 | Aggressive DCE (ADCE) | Statement result never used, no side effects | Pure expression whose value is discarded |
| O109 | Dead store elimination (DSE) | Variable set but never read | `set x 42` with no use of `x` |
| O110 | Instruction combine (InstCombine) | Algebraic simplification opportunity | `expr {$x * 1}` → `expr {$x}` |
| O112 | Constant condition (SCCP structure elimination) | Branch condition is compile-time constant | `if {1} {...}` → body only |
| O113 | Strength reduction | Power/modulo with small constants | `expr {$x ** 2}` → `expr {$x * $x}` |
| O114 | Incr idiom | `set x [expr {$x + N}]` pattern | → `incr x N` (specialised `incrStkImm` opcode) |
| O115 | Nested expr unwrap | `expr {expr {…}}` double wrapping | `expr {expr {$a + $b}}` → `expr {$a + $b}` |
| O116 | List folding | `[list a b c]` with all-constant args | `[list a b c]` → `a b c` |
| O117 | String length zero-check | `[string length $s] == 0` | → `$s eq ""` (avoids length computation) |
| O118 | Lindex folding | `[lindex {a b c} N]` with constant list and index | `[lindex {a b c} 1]` → `b` |
| O119 | Multi-set packing | Multiple `set` commands with related values | Consecutive `set` calls packed into one operation |
| O120 | String compare eq/ne | `==`/`!=` on string-typed operands | `expr {$s == "foo"}` → `expr {$s eq "foo"}` |
| O121 | Tail-call detection | Self-recursive call in tail position | → suggest `tailcall` for TCO |
| O122 | Tail-recursion to loop | Fully tail-recursive proc | → rewrite as iterative `while` loop |
| O123 | Accumulator introduction | Non-tail recursion with associative op | → introduce accumulator parameter |
| O124 | Unused proc elimination | Proc defined but never called | Comment out unused `proc` (iRules only) |
| O125 | Code sinking (LCP) | Assignment used only in one branch | Move `set` into the branch that uses it |
| O126 | Dead store after tail position | Variable only used by eliminated tail expr | Remove the dead `set` |

---

## How diagnostics are calculated

The LSP server produces diagnostics in two phases — a fast synchronous
phase for immediate feedback and an expensive asynchronous phase for deep
analysis.  Understanding this architecture explains why some warnings
appear instantly and others arrive after a brief delay.

### Phase 1 — Basic diagnostics (fast, synchronous)

`get_basic_diagnostics()` in
`rust/tcl-compiler/src/analyser/diagnostics/` runs on every
keystroke and returns immediately.  It produces:

```
Source text
    │
    ▼
┌───────────────────────────────────────────────────┐
│ Semantic Analysis (analyse())                      │
│   → W100: Unbraced expr body                       │
│   → W101: Wrong number of arguments                │
│   → W102: Unknown command                          │
│   → W103: Variable read before set                 │
│   → W104: Unused variable                          │
│   → W200+: iRules event/command warnings           │
│   → W300+: Deprecation/style warnings              │
└───────────────────────────────┬───────────────────┘
                                │
                                ▼
┌───────────────────────────────────────────────────┐
│ Style Checks                                       │
│   → W111: Line exceeds configured length            │
│   → W112: Trailing whitespace                       │
│   → W115: Backslash-newline continuation in comment │
│   → W120: Command used without package require      │
└───────────────────────────────┬───────────────────┘
                                │
                                ▼
                        Basic diagnostics
                    (published immediately)
```

The semantic analyser (`analyse()`) runs over the AST and produces
diagnostics for syntax errors, arity violations, unknown commands,
unused variables, and read-before-set conditions.  Style checks scan
the raw source text for formatting issues.

### Phase 2 — Deep diagnostics (expensive, background thread)

`get_deep_diagnostics()` in
`rust/tcl-compiler/src/analyser/diagnostics/` runs in a
background thread via `asyncio.to_thread` to avoid blocking the editor.
It reuses the `CompilationUnit` from Phase 1 (shared IR, CFG, SSA,
and analysis results).

```
CompilationUnit (shared)
    │
    ├───► Optimiser (find_optimisations)
    │     → O100–O130: All optimisation suggestions
    │     Groups related edits (e.g. O100+O109 for propagate + dead store)
    │
    ├───► Shimmer detector (find_shimmer_warnings)
    │     → S100: Value accessed as incompatible type
    │     → S101: Implicit shimmer (int→string, etc.)
    │     → S102: Cross-command type conflict
    │
    ├───► Taint engine (find_taint_warnings)
    │     → T100: Dangerous code-execution sink
    │     → T101: Tainted output
    │     → T102: Option injection (tainted arg without --)
    │     → T103: Regex injection / ReDoS
    │     → T104: SSRF (network address sink)
    │     → T105: Cross-interpreter code injection
    │     → T106: Double-encoding (informational)
    │     → IRULE1007: Collect without release (side-aware, in iRules flow analysis)
    │     → IRULE1008: Release without collect (side-aware, in iRules flow analysis)
    │     → IRULE3001: XSS in HTTP response body
    │     → IRULE3002: Header/cookie injection
    │     → IRULE3003: Log injection
    │     → IRULE3004: Open redirect
    │
    ├───► iRules flow checker (find_irules_flow_warnings)
    │     → IRULE1005: *_DATA handler without matching collect
    │     → IRULE1006: payload access without collect
    │     → IRULE1201: HTTP command after respond/redirect
    │     → IRULE1202: Multiple respond/redirect on different branches
    │     → IRULE4004: Per-request set hoistable to connection scope
    │     → IRULE5002: drop/reject without event disable or return
    │     → IRULE5004: DNS::return without return
    │
    └───► GVN/CSE (find_redundant_computations)
          → O105: Redundant pure computation
          → O106: Loop-invariant computation (LICM)
```

### Async scheduling and cancellation

The `DiagnosticScheduler` in
`rust/tcl-lsp-server/src/lib.rs` manages the
lifecycle of deep diagnostic tasks:

```
  Document edit (version N)
      │
      ├─► Phase 1: get_basic_diagnostics()
      │     → publish basic diagnostics immediately
      │
      └─► DiagnosticScheduler.schedule(uri, version=N, ...)
            │
            ├─► Cancel any in-flight deep task for this URI
            │     (previous version is stale)
            │
            └─► asyncio.create_task(_run())
                  │
                  └─► asyncio.to_thread(deep_fn)    ← background thread
                        │
                        ▼
                    Deep diagnostics complete
                        │
                        ▼
                    publish_fn(uri, basic + deep, version=N)
                        │
                        ▼
                    Editor shows full diagnostic set
```

Key properties:
- **Cancellation**: if the user types another character while deep analysis
  is running, the stale task is cancelled and a new one starts.
- **Version tracking**: each task carries a document version; results are
  discarded if a newer version has been scheduled.
- **Merge**: the final published diagnostics are `basic + deep`, ensuring
  a consistent complete set.

### Suppression with `# noqa`

Any diagnostic can be suppressed with an inline `# noqa` comment:

```tcl
set x 42    ;# noqa: O109  — suppress dead store warning
eval $cmd   ;# noqa: *     — suppress ALL warnings on this line
```

The suppression map `suppressed_lines: HashMap<i32, HashSet<String>>` is built
during semantic analysis and checked by both Phase 1 and Phase 2 before
emitting any diagnostic.  `# noqa: *` suppresses all codes; `# noqa: O109`
suppresses only the specified code.

### Grouped optimisations

When the optimiser produces related edits (e.g. O100 propagates a constant
AND O109 removes the now-dead store), they share a `group` ID.  The
diagnostics publisher emits one primary diagnostic with the others as
`DiagnosticRelatedInformation`:

```
Primary: O100 "Propagate constant into expression" (+1 dead store eliminated)
  └─ Related: O109 "Dead store: x is set but never read"
```

The LSP client receives a single code action that applies all grouped
edits atomically, keeping the source consistent.

### End-to-end diagnostic flow for Example 12

For the taint example (`HTTP::header value Host` → `HTTP::respond`):

1. **Phase 1** (immediate): semantic analysis finds no syntax errors.
   Basic diagnostics are published with zero warnings.

2. **Phase 2** (background):
   - **Optimiser**: no optimisation opportunities found (the code is
     already efficient).
   - **Taint engine**:
     - `ensure_compilation_unit()` → reuses shared `CompilationUnit`.
     - `_solve_interprocedural_taints()` → propagates taint from
       `HTTP::header value Host` through `string tolower` to
       `HTTP::respond`.
     - `_find_taint_sinks()` → detects `IRULE3001` on the
       `HTTP::respond` line.
   - **GVN**: no redundant computations.

3. **Publish**: `basic_diags + deep_diags` → one `IRULE3001` warning
   at `DiagnosticSeverity.Warning` is published to the editor.

---

## Summary of data flow through the pipeline

```
Source text  ─────────────────────────────────────────────────────►  Bytecode
  "set x 42"                                                        push1/storeStk/done
       │                                                                 ▲
       ▼                                                                 │
  Token stream         SegmentedCommand  Statement::AssignConst      Instruction
  ┌──────────┐        ┌──────────────┐        ┌───────────┐        ┌───────────┐
  │ type:ESC │   ──►  │ texts:       │  ──►   │ name:"x"  │  ──►   │ op:PUSH1  │
  │ text:"set"│       │  ["set",     │        │ value:"42"│        │ operands: │
  │ start:0,0│        │   "x","42"]  │        │ span:...  │        │  (0,)     │
  │ end:0,3  │        │ single:      │        └───────────┘        └───────────┘
  └──────────┘        │  [T, T, T]   │              │                    ▲
                      └──────────────┘              │                    │
                                                    ▼                    │
                                                Block              FunctionAsm
                                              ┌──────────┐       ┌───────────┐
                                              │ stmts:   │       │ literals: │
                                              │  [Assign]│       │  LitTable │
                                              │ term:    │  ──►  │ lvt:      │
                                              │  Goto    │       │  LVTTable │
                                              └──────────┘       │ instrs:   │
                                                    │            │  [Instr]  │
                                                    ▼            └───────────┘
                                               SsaBlock                    ▲
                                              ┌───────────────┐            │
                                              │ phis: []      │            │
                                              │ stmts:        │      codegen_module()
                                              │  SsaStatement │  ──────────┘
                                              │ defs:         │
                                              │  {x: 1}       │
                                              └───────────────┘
```

Each stage transforms the data into a richer representation:
1. **Tokens** — flat character-level classification (`rust/tcl-lexer/src/tokens.rs`)
2. **SegmentedCommand** — word-level grouping with command boundaries (`rust/tcl-compiler/src/segmenter.rs`)
3. **IR nodes** — typed, structured command semantics (`rust/tcl-compiler/src/ir.rs`)
4. **CFG blocks** — explicit control flow with terminators (`rust/tcl-compiler/src/cfg.rs`)
5. **SSA** — variable versioning with phi nodes at merge points (`rust/tcl-compiler/src/ssa.rs`)
6. **FunctionUnit** — constant values, types, taints, def-use chains, per function; its `sccp: SccpResult` holds the SCCP lattice (`rust/tcl-compiler/src/compilation_unit.rs`, `rust/tcl-compiler/src/sccp.rs`).  The `FunctionAnalysis` aggregate sketched in [Stage 6](#stage-6--analysis-types-rusttcl-compilersrcanalysesrs) is declared but not on this path — see issue #1406
7. **Bytecode** — executable instruction stream with literal/variable tables (`rust/tcl-bytecode/src/format.rs`)
