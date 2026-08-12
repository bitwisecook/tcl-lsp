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
| **Basic block** | A straight-line sequence of IR statements with no branches except at the end.  Represented by `cfg::Block` (`rust/tcl-compiler/src/cfg.rs`). |
| **CFG** | Control Flow Graph — a directed graph of basic blocks connected by jumps and branches.  Built by `build_cfg()` (`rust/tcl-compiler/src/cfg_builder/mod.rs`). |
| **Dominator / idom** | Block A *dominates* block B if every path from the entry to B passes through A.  The *immediate dominator* (`idom`) is the closest dominator.  Stored in `SsaFunction::idom` (`rust/tcl-compiler/src/ssa.rs`). |
| **Dominance frontier** | The set of blocks where a variable's dominance "ends" — these are where phi nodes must be inserted.  Stored in `SsaFunction::dominance_frontier`. |
| **GVN** | Global Value Numbering — an optimisation that detects redundant computations by assigning a canonical identity to each expression.  See `rust/tcl-compiler/src/gvn.rs`. |
| **IR** | Intermediate Representation — a structured, typed representation of Tcl commands between parsing and code generation.  Defined in `rust/tcl-compiler/src/ir.rs`; the enum `ir::Statement` covers all statement kinds. |
| **Lattice** | A mathematical structure used in dataflow analysis where values flow from *bottom* (unknown) toward *top* (overdefined).  The SCCP value lattice is `LatticeValue` (`rust/tcl-compiler/src/analyses.rs`); the type lattice is `TypeLattice` (`rust/tcl-compiler/src/types.rs`). |
| **Liveness** | A dataflow analysis that determines which SSA values are "live" (may still be read) at each program point.  Results are in `FunctionAnalysis::live_in` / `live_out`. |
| **LVT** | Local Variable Table — maps variable names to integer slot indices for fast access inside procedures.  See `LocalVarTable` (`rust/tcl-bytecode/src/lib.rs`). |
| **Phi node (φ)** | An SSA construct placed at control flow merge points.  `φ(x₁, x₃)` means "use `x₁` if control arrived from predecessor 1, or `x₃` if from predecessor 2."  Represented by `ssa::Phi`. |
| **SCCP** | Sparse Conditional Constant Propagation — a combined constant propagation and unreachable-code analysis that runs over the SSA graph.  Implemented by `sccp()` (`rust/tcl-compiler/src/sccp.rs`). |
| **Shimmer** | Tcl's internal type coercion: when a value's string representation is reinterpreted as a different type (e.g. `"42"` read as an integer).  Tracked by `TypeKind::Shimmered` (`rust/tcl-compiler/src/types.rs`) and analysed in `rust/tcl-compiler/src/shimmer/`. |
| **SSA** | Static Single Assignment — a form where every variable is defined exactly once.  Multiple definitions of the same source variable get unique *version numbers* (e.g. `x₁`, `x₂`).  Built by `build_ssa()` (`rust/tcl-compiler/src/ssa.rs`). |
| **SSA value key** | A `(Symbol, Version)` pair that uniquely identifies one definition of a variable.  Type alias `ssa::ValueKey`; `Symbol` is an interned variable-name index, `Version` a `u32`. |
| **Taint analysis** | Tracks whether values originate from untrusted sources (user input).  Uses `TaintLattice` (`rust/tcl-compiler/src/taint.rs`). |
| **Taint colour** | A `bitflags` set describing safety properties of tainted data (e.g. `CRLF_FREE`, `URL_ENCODED`, `HTML_ESCAPED`).  Colours compose with `\|` and join by intersection (`&`) — only properties shared by all incoming paths survive.  Defined as `TaintColour` (`rust/tcl-registry/src/taint.rs`). |
| **Taint source** | A command whose return value introduces tainted data (e.g. `HTTP::host`, `HTTP::uri`).  Declared via `CommandSpec::taint_source` on the command's registry spec. |
| **Taint sink** | A dangerous argument position where tainted data can cause harm (XSS, header injection, SSRF).  Declared via the `taint_*_sink*` fields on `CommandSpec` and resolved by the taint engine. |
| **CSE** | Common Subexpression Elimination — detects when the same pure computation is evaluated more than once and suggests extracting it to a variable.  Part of the GVN pass, reported as `O105`.  See `rust/tcl-compiler/src/gvn.rs`. |
| **ICIP** | Interprocedural Constant/Inline Propagation — evaluates procedure calls with known constant arguments at compile time and replaces the call with the result.  Reported as `O103`, emitted by the `Propagation` pass (`rust/tcl-compiler/src/optimiser/propagation.rs`). |
| **LCP** | Loop Constant Propagation / Code Sinking — moves invariant assignments out of the hot path into the specific branch that uses them.  Reported as `O125`, emitted by `rust/tcl-compiler/src/optimiser/code_sinking.rs`. |
| **DCE** | Dead Code Elimination — removes code whose result is never used.  `O107` (unreachable blocks), `O108` (aggressive DCE tracking statement liveness), `O109` (dead store elimination).  All emitted by `rust/tcl-compiler/src/optimiser/elimination.rs`. |
| **InstCombine** | Instruction Combine — canonicalises and simplifies expressions by applying algebraic identities (e.g. `$x * 1` → `$x`, DeMorgan's law).  Reported as `O110`, emitted by `rust/tcl-compiler/src/optimiser/expr_simplify.rs`. |
| **CommandSpec** | The central metadata type for a Tcl command — describes its argument layout, purity, side effects, taint properties, event validity, and dialect membership.  See `rust/tcl-registry/src/spec.rs`. |
| **SubCommand** | An ensemble operation selected by the first argument (e.g. `string length`, `HTTP::header value`).  Each has its own arity, purity, return type, and hook IDs.  Also in `rust/tcl-registry/src/spec.rs`. |
| **FormSpec** | A named invocation form of a command — `Default`, `Getter` (reads state), or `Setter` (writes state) — carrying that form's synopsis and dialect gate.  See `rust/tcl-registry/src/hover.rs`. |

---

## Pipeline stage summary

Every Tcl source string passes through these stages.  The orchestrating
entry point is `CompilationUnit::build_for(source, registry,
defer_top_level)` in `rust/tcl-compiler/src/compilation_unit.rs`:

| # | Stage | Entry point | Produces | Home |
|---|-------|-------------|----------|------|
| 1 | Lexer | `Lexer::tokenise_all()` | `Vec<Token>` | `rust/tcl-lexer/src/lexer.rs` |
| 2 | Segmenter | `segment_commands()` | `Vec<SegmentedCommand>` | `rust/tcl-compiler/src/segmenter.rs` |
| 3 | IR lowering | `lower_to_ir()` | `ir::Module` | `rust/tcl-compiler/src/lowering/mod.rs` |
| 4 | CFG | `build_cfg()` / `build_cfg_function()` | `CfgModule` | `rust/tcl-compiler/src/cfg_builder/mod.rs` |
| 5 | SSA | `build_ssa()` | `SsaFunction` | `rust/tcl-compiler/src/ssa.rs` |
| 6 | Core analyses | `sccp()` + the liveness / type / taint passes | `FunctionAnalysis` | `rust/tcl-compiler/src/analyses.rs` |
| 7 | Codegen | `codegen_module()` | `ModuleAsm` | `rust/tcl-compiler/src/codegen/emitter/mod.rs` |

Stage 2 derives its command list from the canonical red-green CST built
by `rust/tcl-syntax`, not from a hand-rolled token loop.

Every stage below can be inspected for real with the compiler explorer:

```
cargo run -p tcl-cli --bin tcl -- explore FILE.tcl --show ir,cfg,ssa,asm --text
```

The `ir` / `cfg` / `ssa` / `asm` listings quoted in the examples that
follow are that command's output.

---

## Data structure reference

Before diving into examples, here are the key types that appear at each
stage.  All of them are Rust types in the `rust/` workspace; the crate
and module are named on each heading.

### Stage 1 — Lexer types (`rust/tcl-lexer/src/tokens.rs`)

```rust
pub enum TokenType {
    /// Plain string fragment, possibly containing escape sequences.
    Esc,
    /// Braced string `{…}` (the braces are stripped from the token text).
    Str,
    /// Command substitution `[…]` (the brackets are stripped).
    Cmd,
    /// Variable substitution `$name`, `${name}`, or `$arr(idx)`.
    Var,
    /// Run of intra-command whitespace separators (space, tab, etc.).
    Sep,
    /// End-of-line: newline or `;`.
    Eol,
    /// End-of-input sentinel.
    Eof,
    /// Comment from `#` to end of line.
    Comment,
    /// `{*}` argument-expansion prefix (Tcl 8.5+).
    Expand,
}

pub struct Token {
    /// Token kind.
    pub kind: TokenType,
    /// Byte range in the source.
    pub span: Span,
    /// Leading bytes of `span` that are delimiters rather than content
    /// (1 for most wrappers, 2 for `${…}`, 0 for bare words).
    pub content_offset: u8,
    /// True when the token was emitted inside a quoted-string context.
    pub in_quote: bool,
}
```

The key structural difference from a naive token type: **a `Token` carries
no text and no line/column of its own**, only a `Span` of byte offsets
(`rust/tcl-lexer/src/span.rs`, a packed `{ start: u32, end: u32 }`).
Text and positions are resolved on demand through a
`SourceMap` (`rust/tcl-lexer/src/source_map.rs`):

| Need | Call |
|---|---|
| The raw slice for a span | `SourceMap::text(span)` |
| A token's human-readable text, opening delimiter stripped | `SourceMap::token_text(tok)` |
| Line/character for an offset | `SourceMap::position_at(offset)` → `SourcePosition` |
| Both ends of a span at once | `SourceMap::range_positions(span)` |

- `Token::kind` distinguishes variables (`$x` → `Var`), braced strings
  (`{hello}` → `Str`), command substitutions (`[foo]` → `Cmd`), and plain
  word fragments (`set` → `Esc`).
- `SourcePosition` carries a 0-based `line`, a 0-based `character` as a
  `ByteCol` (bytes from the line start), and the absolute byte `offset`.
  The LSP-facing UTF-16 column is a *separate* type, `Utf16Position`, so a
  byte column can never be mistaken for an LSP column.

### Stage 2 — Segmenter types (`rust/tcl-compiler/src/segmenter.rs`)

> `segment_commands()` builds the canonical lossless **red-green concrete syntax
> tree** (`rust/tcl-syntax`, see [syntax-tree.md](compiler/syntax-tree.md))
> and derives the `SegmentedCommand` list from it.  The tree is the
> *backing*, not a different shape.

```rust
pub struct SegmentedCommand {
    /// Byte span covering the whole command.
    pub span: Span,
    /// Per-word representative tokens (one per argv entry).
    pub argv: Vec<Token>,
    /// Per-word reconstructed text.
    pub texts: Vec<String>,
    /// Ordered lexical fragments for every word — the lossless companion
    /// to the `argv` / `texts` parallel arrays.
    pub word_fragments: Vec<Vec<WordFragment>>,
    /// Whether each word is a single token.
    pub single_token_word: Vec<bool>,
    /// All tokens in the command (including separators).
    pub all_tokens: Vec<Token>,
    /// Whether the command is incomplete (unclosed delimiter).
    pub is_partial: bool,
    /// Which delimiter was left unclosed, when `is_partial`.
    pub partial_delimiter: Option<UnclosedDelimiter>,
    /// `{*}` expansion markers per word, if any word uses expansion.
    pub expand_word: Option<Vec<bool>>,
    /// Comment line(s) immediately preceding the command.
    pub preceding_comment: Option<String>,
}
```

- `texts[0]` is the command name, `texts[1..]` are the arguments;
  `SegmentedCommand::name()` returns the former directly.
- `single_token_word[i]` is `true` when word `i` is a single atomic token
  (no interpolation) — important for constant tracking downstream.
- `argv[i]` is the *representative* token of word `i`; multi-token words
  (e.g. `$prefix.txt`) are concatenated into `texts[i]`, and their full
  substitution order is preserved in `word_fragments[i]`.

### Stage 3 — IR types (`rust/tcl-compiler/src/ir.rs`)

IR is **one enum**, `ir::Statement`, with 17 variants — not a union of
separate node classes.  Every variant carries a `span: Span` for precise
diagnostic mapping.  The variants, in declaration order:

| Variant | Shape | Example source |
|---|---|---|
| `AssignConst` | `{ span, name, name_braced, value, value_span }` | `set a 1` |
| `AssignExpr` | `{ span, name, name_braced, expr, expr_base }` | `set x [expr {$a + 1}]` |
| `AssignValue` | `{ span, name, name_braced, value, value_needs_backsubst, tokens }` | `set x $y`, `set x "hi $name"` |
| `Incr` | `{ span, name, name_braced, amount, safe_on_uninit }` | `incr i`, `incr i 5` |
| `ExprEval` | `{ span, expr, expr_base }` | `expr {2 + 3}` |
| `Call` | `{ span, command, canonical_command, args, defs, reads, reads_own_defs, safe_on_uninit, tokens, foreach_groups }` | `puts $x`, `append s a` |
| `Return` | `{ span, value, expr, braced }` | `return $r` |
| `Barrier` | `{ span, reason, command, canonical_command, args, tokens }` | `eval $script` |
| `Block` | `{ span, body, namespace, tokens, error_context }` | an inlined passthrough body |
| `UpFrame` | `{ span, frame_shift, absolute, body, tokens, … }` | `uplevel 1 {…}` |
| `If` | `{ span, clauses, else_body, else_span }` | `if {…} {…} else {…}` |
| `For` | `{ span, init, condition, next, body, … }` | `for {…} {…} {…} {…}` |
| `While` | `{ span, condition, body, … }` | `while {…} {…}` |
| `Foreach` | `{ span, iterators, body, is_lmap, is_dict, … }` | `foreach x $l {…}` |
| `Catch` | — | `catch {…} err` |
| `Try` | — | `try {…} on error {…}` |
| `Switch` | — | `switch $x {…}` |

Supporting types in the same module:

```rust
/// A sequence of statements in execution order.
pub struct Script {
    pub statements: Vec<Statement>,
}

/// One `if` / `elseif` clause.
pub struct IfClause { /* condition, condition_base, body, spans */ }

/// A `foreach` / `lmap` iterator group: a variable list and its list argument.
pub struct ForeachIterator { /* var names + list arg */ }

/// A procedure definition.
pub struct Procedure {
    /// Short procedure name.
    pub name: String,
    /// Fully qualified name (e.g. `::ns::proc`).
    pub qualified_name: String,
    /// Parameter names.
    pub params: Vec<String>,
    /// Source span of the definition.
    pub span: Span,
    /// Procedure body.
    pub body: Script,
    /// Raw parameter list text.
    pub params_raw: String,
    /// Source text of the body (`None` for synthetic procs like `when`).
    pub body_source: Option<String>,
    /// Whether defined inside `namespace eval`.
    pub namespace_scoped: bool,
    /// BIG-IP handler priority (0..2^32-1, default 500).
    pub base_priority: u32,
}

/// A whole lowered module.
pub struct Module {
    /// The source text this module was lowered from — spans index into it.
    pub source: String,
    /// Top-level script (code outside any procedure).
    pub top_level: Script,
    /// Named procedures, keyed by qualified name.
    pub procedures: HashMap<String, Procedure>,
    /// Named methods, keyed by `class::method`.
    pub methods: HashMap<String, MethodDef>,
    /// Synthetic body units — `apply` lambdas and `namespace eval` bodies,
    /// lowered so the analysis pipeline reaches inside them, but never
    /// emitted as callable procs.
    pub body_units: HashMap<String, Procedure>,
    /// Procedure names defined more than once.
    pub redefined_procedures: HashSet<String>,
    // … plus `lambda_body_units`, `redefined_methods`, and OO evidence fields
}
```

- `Statement::Barrier` marks commands (`eval`, `uplevel`, `upvar`) whose
  side effects defeat static analysis — no constant propagation or
  dead-store reasoning can cross them.
- Expression conditions are parsed into `ExprNode` AST trees at lowering
  time.
- `Statement::Call` carries **both** `command` (the source-surface
  spelling, so a diagnostic can quote what the user typed) and
  `canonical_command` (the registry-resolved name, populated when an
  alias resolves).  Dispatch sites use
  `Statement::canonical_command_or_source()` so a `None` canonical falls
  back to the source spelling.

### Expression AST (`rust/tcl-syntax/src/expr/ast.rs`)

```rust
pub enum ExprNode {
    /// Integer, float, or boolean literal.
    Literal { text: String, start: ExprOffset, end: ExprOffset },
    /// Quoted or braced string literal (`"..."` or `{...}`).
    String { text: String, start: ExprOffset, end: ExprOffset },
    /// Variable reference (`$var`, `${var}`, `$arr(idx)`).
    Var { text: String, name: String, start: ExprOffset, end: ExprOffset },
    /// Command substitution `[cmd ...]` — opaque boundary.
    Command { text: String, start: ExprOffset, end: ExprOffset },
    /// Binary operator application.
    Binary { op: BinOp, left: Box<ExprNode>, right: Box<ExprNode> },
    /// Unary operator application.
    Unary { op: UnaryOp, operand: Box<ExprNode> },
    /// Ternary conditional `cond ? a : b`.
    Ternary {
        condition: Box<ExprNode>,
        true_branch: Box<ExprNode>,
        false_branch: Box<ExprNode>,
    },
    /// Math function call: `sin($x)`, `int($y)`, `max($a, $b)`.
    Call { function: String, args: Vec<ExprNode>, start: ExprOffset, end: ExprOffset },
    /// Fallback: unparseable expression preserved as raw text.
    Raw { text: String },
}
```

`BinOp` covers arithmetic (`Add`, `Sub`, `Mul`, `Div`, `Mod`, `Pow`),
shifts (`LShift`, `RShift`), bitwise (`BitAnd`, `BitOr`, `BitXor`),
logical (`And`, `Or`), numeric comparison (`Eq`, `Ne`, `Lt`, `Le`, `Gt`,
`Ge`), string comparison (`StrEq`, `StrNe`, `StrLt`, `StrLe`, `StrGt`,
`StrGe`), list membership (`In`, `Ni`), the iRules word-logical operators
(`WordAnd`, `WordOr`), and the iRules string operators (`Contains`,
`StartsWith`, `EndsWith`, `StrEquals`, `MatchesGlob`, `MatchesRegex`).
`UnaryOp` is `Neg`, `Pos`, `BitNot`, `Not`, and the iRules `WordNot`.
`BinOp::as_str()` / `UnaryOp::as_str()` render each back to its source
spelling.

Two helper methods on `ExprNode` do most of the downstream work:
`vars()` collects every variable name in the tree, and
`function_calls()` returns every math-function application as
`(name, offset, arg_count)` so a consumer can map it back to the
`::tcl::mathfunc::` command it dispatches.

### Stage 4 — CFG types (`rust/tcl-compiler/src/cfg.rs`)

```rust
/// Interned block identifier — blocks are addressed by id, not by name.
pub struct BlockId(pub u32);

pub enum Terminator {
    /// Unconditional jump.
    Goto { target: BlockId, span: Option<Span> },
    /// Conditional jump.
    Branch {
        condition: ExprNode,
        true_target: BlockId,
        false_target: BlockId,
        span: Option<Span>,
        condition_base: Option<u32>,
    },
    /// Function exit.
    Return {
        value: Option<String>,
        span: Option<Span>,
        expr: Option<ExprNode>,
        braced: bool,
    },
}

pub struct Block {
    pub name: String,
    /// Straight-line IR statements.
    pub statements: Vec<Statement>,
    /// Exactly one per block once the builder has finished.
    pub terminator: Option<Terminator>,
}

pub struct Function {
    pub name: String,
    pub entry: BlockId,
    pub blocks: HashMap<BlockId, Block>,
    pub loop_nodes: HashMap<BlockId, LoopNode>,
    pub exception_edges: Vec<(BlockId, BlockId)>,
    pub inline_body_error_sites: Vec<InlineBodyErrorSite>,
    pub caller_frame_barrier: DynamicNameBarrier,
    // …
}

pub struct CfgModule {
    pub top_level: Function,
    pub procedures: HashMap<String, Function>,
}
```

`Terminator::successors()` returns the outgoing `BlockId`s for any
terminator kind, which is what the dataflow passes walk.

### Stage 5 — SSA types (`rust/tcl-compiler/src/ssa.rs`)

```rust
/// Each definition gets a unique version.
pub type Version = u32;
/// Interned variable-name index.
pub struct Symbol(pub u32);
/// A unique SSA value: (variable, version).
pub type ValueKey = (Symbol, Version);

pub struct Phi {
    pub name: Symbol,
    pub version: Version,
    pub incoming: HashMap<BlockId, Version>,
}

pub struct SsaStatement {
    /// The original IR statement.
    pub statement: Statement,
    /// Variables read → their versions.
    pub uses: HashMap<Symbol, Version>,
    /// Variables written → new versions.
    pub defs: HashMap<Symbol, Version>,
    /// Variables this statement *may* define (dynamic names, barriers).
    pub may_defs: HashSet<Symbol>,
    /// Uses that appear only inside a quoted (unsubstituted) word.
    pub quoted_uses: HashSet<Symbol>,
}

pub struct SsaBlock {
    pub name: String,
    /// Phi nodes at merge points.
    pub phis: Vec<Phi>,
    pub statements: Vec<SsaStatement>,
    pub entry_versions: HashMap<Symbol, Version>,
    pub exit_versions: HashMap<Symbol, Version>,
}

pub struct SsaFunction {
    pub name: String,
    pub entry: BlockId,
    pub blocks: HashMap<BlockId, SsaBlock>,
    /// Immediate dominator tree (see Glossary).
    pub idom: HashMap<BlockId, Option<BlockId>>,
    pub dominance_frontier: HashMap<BlockId, Vec<BlockId>>,
    pub dominator_tree: HashMap<BlockId, Vec<BlockId>>,
    // … plus the private block-name / variable-name interners
}
```

Variable names are **interned**: `SsaFunction` holds the name table
privately and hands out `Symbol` indices, so the hot dataflow maps are
keyed by two integers rather than by strings.  `may_defs` and
`quoted_uses` are the two facts a naive `(uses, defs)` model cannot
express — a barrier that might write anything, and a name that appears in
a word Tcl will not substitute.

### Stage 6 — Analysis types (`rust/tcl-compiler/src/analyses.rs`)

```rust
pub enum LatticeKind {
    /// Not yet analysed (bottom).
    Unknown,
    /// Provably one constant.
    Const,
    /// Provably one of a small set of constants.
    ConstSet,
    /// Multiple possible values (top).
    Overdefined,
}

pub enum LatticeValue {
    Unknown,
    Const(ConstValue),
    ConstSet(Vec<ConstValue>),
    Overdefined,
}

pub enum ConstValue {
    Int(i64),
    Float(f64),
    Bool(bool),
    String(String),
}

pub struct FunctionAnalysis {
    /// See Glossary → Liveness.
    pub live_in: HashMap<String, HashSet<ValueKey>>,
    pub live_out: HashMap<String, HashSet<ValueKey>>,
    pub dead_stores: Vec<DeadStore>,
    pub unreachable_blocks: HashSet<String>,
    pub constant_branches: Vec<ConstantBranch>,
    /// SCCP results (see Glossary → SCCP).
    pub values: HashMap<ValueKey, LatticeValue>,
    /// Type inference results.
    pub types: HashMap<ValueKey, TypeLattice>,
    pub read_before_set: Vec<ReadBeforeSet>,
    pub unused_variables: Vec<UnusedVariable>,
    pub unused_params: Vec<String>,
}
```

`LatticeKind::ConstSet` is the notable addition over a textbook SCCP
lattice: a value that is provably one of a *bounded* set of constants
(e.g. the loop variable of a `foreach` over a literal list) stays more
precise than `Overdefined`.  `LatticeValue::constset()` collapses to
`Overdefined` once the set exceeds `MAX_CONSTSET_SIZE`.

The finding records — `ConstantBranch`, `DeadStore`, `ReadBeforeSet`,
`UnusedVariable` — all identify a site as `{ block: String,
statement_index: usize, … }` (a `ConstantBranch` instead names its
condition text, its value, and both targets).

#### Type lattice (`rust/tcl-compiler/src/types.rs`)

The type lattice is **shape**-based, not a single coarse type tag:

```rust
/// A structural type shape.
pub enum TypeShape {
    String,
    Int,
    Bignum,
    Double,
    Boolean,
    Numeric,
    ByteArray,
    /// A list, with what is known about its elements.
    List(Elements),
    /// A dict, with what is known about its values.
    Dict(Elements),
    /// A TclOO object of a (possibly unknown) class.
    Object(Option<Box<str>>),
    Channel,
}

/// What is known about a container's elements.
pub enum Elements {
    Unknown,
    /// Every element has this shape.
    Uniform(Box<TypeShape>),
    /// Element `i` has shape `[i]` (`None` = unknown at that position).
    Exact(Box<[Option<TypeShape>]>),
}

pub enum TypeKind {
    /// Not yet analysed (bottom).
    Unknown,
    /// Concrete type determined.
    Known,
    /// Forced type change detected (see Glossary → Shimmer).
    Shimmered,
    /// Multiple incompatible types (top).
    Overdefined,
}

/// The lattice element itself: bottom, a bounded union of shapes, or top.
pub struct TypeLattice { /* private repr: Unknown | Union(ShapeSet) | Overdefined */ }
```

`TypeShape::coarse()` projects a shape down to the registry-level
`tcl_registry::types::TclType` (`String`, `Int`, `Double`, `Boolean`,
`List`, `Dict`, `ByteArray`, `Numeric`, `Object`, `Channel`) — that
coarse enum is what a `CommandSpec` declares in `return_type` and
`arg_types`.  `type_join()` is the lattice join; the union is bounded at
`MAX_TYPE_UNION` shapes before collapsing to `Overdefined`.

### Stage 7 — Codegen types (`rust/tcl-bytecode/src/lib.rs`)

```rust
/// The Tcl bytecode opcode set. Variants are SCREAMING_CASE so they read
/// as the C Tcl opcode names; `Op::mnemonic()` renders each to the
/// lower-camel spelling tclsh's disassembler prints (`PUSH1` → `push1`).
pub enum Op {
    PUSH1, PUSH4, POP, DUP,
    LOAD_SCALAR1, LOAD_SCALAR4, STORE_SCALAR1, STORE_SCALAR4,
    INCR_SCALAR1, INCR_SCALAR1_IMM,
    INVOKE_STK1, INVOKE_STK4, EVAL_STK, EXPR_STK,
    JUMP1, JUMP4, JUMP_TRUE1, JUMP_TRUE4, JUMP_FALSE1,
    // …
}

pub enum Operand {
    /// Immediate / literal-table index.
    Imm(i32),
    /// Unresolved label reference, patched by the layout pass.
    Label(String),
}

pub struct Instruction {
    pub op: Op,
    pub operands: Vec<Operand>,
    pub comment: String,
    /// Filled by the layout pass; -1 until then.
    pub offset: i32,
    /// Source provenance for `errorInfo` reconstruction.
    pub source_line: u32,
    pub source_cmd_text: String,
    pub source_span: Option<Span>,
    // … plus jump_table / foreach_vars / dict_vars specialisation payloads
}

/// Intern pool: string → object-array index.
pub struct LiteralTable { /* entries + index */ }
/// LVT: variable name → slot index (see Glossary).
pub struct LocalVarTable { /* … */ }

pub struct FunctionAsm {
    pub name: String,
    pub literals: LiteralTable,
    pub lvt: LocalVarTable,
    pub instructions: Vec<Instruction>,
    /// Label → byte offset.
    pub labels: HashMap<String, usize>,
    pub loop_targets: HashMap<usize, (Option<i32>, Option<i32>)>,
    pub body_base_line: u32,
    pub error_regions: Vec<ErrorRegion>,
}

pub struct ModuleAsm {
    pub top_level: FunctionAsm,
    pub procedures: HashMap<String, FunctionAsm>,
}
```

The emitter that produces these lives in
`rust/tcl-compiler/src/codegen/`; `codegen_module()` in
`codegen/emitter/mod.rs` is its entry point.  The bytecode VM that
executes them is `rust/tcl-vm`.

### Orchestration (`rust/tcl-compiler/src/compilation_unit.rs`)

```rust
pub struct FunctionUnit {
    pub name: String,
    pub cfg: CfgFunction,
    pub ssa: SsaFunction,
    pub def_use: Arc<DefUseResult>,
    pub sccp: SccpResult,
    pub types: Arc<HashMap<ValueKey, TypeLattice>>,
    pub return_type: TypeLattice,
    pub taints: Arc<HashMap<ValueKey, TaintLattice>>,
    // …
}

pub struct CompilationUnit {
    pub source: String,
    pub ir_module: IrModule,
    pub cfg_module: CfgModule,
    pub top_level: FunctionUnit,
    pub procedures: HashMap<String, FunctionUnit>,
    pub methods: HashMap<String, FunctionUnit>,
    pub body_units: HashMap<String, FunctionUnit>,
    pub interproc: Option<InterproceduralAnalysis>,
    pub connection_scope: Option<ConnectionScope>,
}
```

Built by `CompilationUnit::build_for(source, registry, defer_top_level)`.
Note that the per-function analysis results are *not* one bundled
`FunctionAnalysis` field: SCCP (`sccp`), def-use (`def_use`), types, and
taint are separate, individually `Arc`-shared so consumers that only need
one do not clone the rest.  TclOO methods and synthetic body units get
their own `FunctionUnit`s alongside real procedures, so the whole
analysis pipeline reaches inside them.

---

## Command infrastructure

The compiler's view of every Tcl command — its argument layout, purity,
side effects, taint properties, event validity, and dialect membership —
comes from the **command registry**.  This section explains each layer
of that infrastructure and how the pieces connect.

### Overview

```
┌────────────────────────────────────────────────────────────────────┐
│                     CommandRegistry                                │
│                                                                    │
│   rust/tcl-registry/src/commands/                                  │
│   ┌──────┐ ┌────────┐ ┌───────┐ ┌────┐ ┌────────┐ ┌─────┐ ┌─────┐ │
│   │ tcl/ │ │ irules/│ │ iapps/│ │ tk/│ │ tcllib/│ │itcl/│ │ bpf/│ │
│   └──┬───┘ └───┬────┘ └───┬───┘ └─┬──┘ └───┬────┘ └──┬──┘ └──┬──┘ │
│      │         │          │       │        │         │       │     │
│      └─────────┴──────────┴───────┴────────┴─────────┴───────┘     │
│                            │                                       │
│                            ▼                                       │
│                     CommandSpec                                    │
│     ┌────────────────────────────────────────────────┐            │
│     │ name, traits, dialects, arity, arg_roles,      │            │
│     │ forms, subcommands, options, side_effects,     │            │
│     │ event_requires, taint_*, lifecycle, hooks …    │            │
│     └────────────────────────────────────────────────┘            │
│         │          │           │            │                      │
│         ▼          ▼           ▼            ▼                      │
│     FormSpec   SubCommand   SideEffect   EventRequires             │
│                                                                    │
└────────────────────────────────────────────────────────────────────┘
```

(Plus `argparse/`, `expect/`, `sdc_base/`, `stdlib/`, `ticklecharts/`,
and the five `eda_*/` vendor library packs.)

Every command is **one Rust file** under
`rust/tcl-registry/src/commands/<dialect>/`, exporting a single
`pub const fn spec() -> CommandSpec`.  There is no decorator and no
import-time registration: the per-dialect `mod.rs` lists its commands,
and `CommandRegistry::build_default()` walks those lists into the
`by_name` lookup table.  Built registries are cached —
`tcl_registry::cache::default_registry()` and
`registry_for_dialect(dialect)` hand out `&'static CommandRegistry`.

Because the whole spec is a `const fn` returning `&'static` data, adding a
command costs no runtime allocation and the tables are shareable across
threads without locking.

### Defining a command

```rust
//! `SSL::sni` iRules command.
use crate::prelude::*;

pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "SSL::sni",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet { /* summary, synopsis, examples, … */ }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "SSL::sni <name | required>",
            dialects: None,
        }],
        subcommands: SUBCOMMANDS,
        side_effects: &[SideEffect {
            target: SideEffectTarget::SslState,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Both,
            dialects: None,
        }],
        taint_source: Some(TaintColour::TAINTED.union(TaintColour::FQDN)),
        ..CommandSpec::DEFAULT
    }
}
```

The `..CommandSpec::DEFAULT` tail is the load-bearing idiom: `CommandSpec`
has well over a hundred fields, and a spec names only the ones it
actually uses.

**Dialect membership** is a `DialectSet` bitflag rather than a set of
strings — `DialectSet::IRULES`, `DialectSet::ALL_TCL`,
`DialectSet::TCL86_PLUS`, and so on
(`rust/tcl-dialect/src/dialect_set.rs`).  `dialects: None` used to mean
"universal", but universal specs were eliminated registry-wide: every
command now carries an explicit group, with the `IRULES` bit present
if and only if iRules enables it.

### FormSpec — invocation forms

A command can declare several named invocation forms — typically a getter
(reads state) and a setter (writes state):

```rust
pub enum FormKind { Default, Getter, Setter }

pub struct FormSpec {
    pub kind: FormKind,
    /// Human-readable signature.
    pub synopsis: &'static str,
    /// Dialect gate; `None` inherits the parent command's.
    pub dialects: Option<DialectSet>,
}
```

Note what a Rust `FormSpec` is **not**: it carries no per-form arity,
options, purity, or side-effect list.  Those all live on the
`CommandSpec` / `SubCommand` itself.  A `FormSpec` is documentation and a
dialect gate — the thing that varies *by form* and is consumed
programmatically is `CommandSpec::command_forms` (`&[CommandForm]`) and
`SubCommand::subcommand_forms`.

### Arity — argument count constraints

```rust
pub struct Arity {
    /// Minimum args (after the command name).
    pub min: u16,
    /// Maximum args; `Arity::UNLIMITED` is `u16::MAX`.
    pub max: u16,
    /// Accepted counts step by this from `min` (e.g. 2 for pair-taking forms).
    pub step: u16,
    /// One extra exact count accepted outside the `min`/`max`/`step` pattern.
    pub also_exact: Option<u16>,
}
```

`step` and `also_exact` are the two additions over a plain min/max: they
express "takes key/value pairs" and "…or the one-argument shorthand"
without a bespoke validator.  Arity violations produce **`E002`** (too
few arguments) and **`E003`** (too many); a count that is in range but
the wrong *shape* — an odd `dict create` tail, an unpaired `foreach`
list — produces **`E005`**.

### SubCommand — ensemble commands

Commands like `string`, `dict`, `info`, and `HTTP::header` use
subcommands (the first argument selects the operation).  `SubCommand`
mirrors much of `CommandSpec`, including its own arity, argument roles,
options, hooks, side effects, and lifecycle.  The fields most often set:

```rust
pub struct SubCommand {
    pub name: &'static str,
    pub traits: Traits,
    /// Arg count for this subcommand (excluding the subcommand word).
    pub arity: Arity,
    pub detail: &'static str,
    pub synopsis: &'static str,
    pub hover: Option<HoverSnippet>,
    pub arg_roles: &'static [(u8, ArgRole)],
    pub arg_role_resolver: Option<ArgRoleResolver>,
    pub arg_types: &'static [(u8, ArgTypeHint)],
    pub return_type: Option<TclType>,
    /// No side effects.
    pub pure: bool,
    /// Modifies state.
    pub mutator: bool,
    pub options: &'static [OptionSpec],
    pub arg_values: &'static [(u8, &'static [ArgValue])],
    /// `None` inherits the parent command's dialects.
    pub dialects: Option<DialectSet>,
    pub side_effects: &'static [SideEffect],
    /// Typed hook IDs — dispatched on, never called through a stored closure.
    pub lowering_hook: Option<LoweringHookId>,
    pub codegen_hook: Option<CodegenHookId>,
    pub analyser_hook: Option<AnalyserHookId>,
    pub const_fold: Option<ConstFoldFn>,
    pub taint_transform: Option<TaintColour>,
    // … ~60 fields in total; `..SubCommand::DEFAULT` fills the rest
}
```

**Example** — `SSL::sni name` has `arity: Arity::exact(0)`, `pure: true`,
and a read-only `SideEffectTarget::SslState` effect.  Each subcommand
invocation is arity-checked independently.

The **hooks are typed IDs, not function pointers to arbitrary
closures**: `LoweringHookId`, `CodegenHookId`, and `AnalyserHookId` are
enums the consumer matches on.  That is what keeps the spec `const` and
keeps per-command knowledge in the registry rather than in a walker's
`match cmd_name`.

### OptionSpec and option constraints

Commands that accept `-flag` switches declare them via `OptionSpec`:

```rust
pub struct OptionSpec {
    /// e.g. `"-nocase"`, `"-length"`.
    pub name: &'static str,
    /// Whether the option takes a value, and of what kind.
    pub value: OptionValue,
    /// Completion description.
    pub detail: &'static str,
    /// `None` inherits the parent's dialects.
    pub dialects: Option<DialectSet>,
    pub aliases: &'static [&'static str],
    pub lifecycle: Lifecycle,
}
```

`OptionValue::flag()` is the no-value form; a value-taking option
describes its value instead of setting a bare `takes_value: bool`.
Relationships between options — mutual exclusion, requires-another — are
declared separately as `CommandSpec::option_constraints`
(`&[OptionConstraint]`).

**Option terminators** (`--`) prevent a dynamic argument from being
mistaken for a flag.  The `ArgRole::OptionTerminator` role marks the
`--` position, and the `W304` check uses the command's declared option
set to decide whether a dynamic argument is at risk.

When a command like `string match` receives a dynamic pattern (`$pat`)
without `--`, the checker emits `W304` because `$pat` could start with
`-` and be misinterpreted as the `-nocase` flag:

```tcl
# W304: use -- before dynamic pattern
string match $pat $str        ;# risky: $pat could be "-nocase"
string match -- $pat $str     ;# safe:  -- terminates option scanning
```

### Validation

Validation is layered.  There is **no `ValidationSpec` wrapper** — each
layer reads a field on the spec directly:

1. **Arity** — `CommandSpec::arity` sets the overall argument count and
   each `SubCommand` has its own.  Violations produce `E002` (too few) /
   `E003` (too many), and an in-range but wrong-shaped count produces
   `E005`.
2. **Option terminator** — a dynamic argument in an option-scanning
   position without `--` produces `W304`.
3. **Option constraints** — `CommandSpec::option_constraints`
   (`&[OptionConstraint]`) declares mutual exclusion and
   requires-another relationships between options.
4. **Literal argument validation** —
   `CommandSpec::literal_argument_validator`
   (`Option<LiteralArgumentValidator>`) and
   `clause_shape_check` (`Option<ClauseShapeChecker>`) are the two typed
   hooks for checks a field cannot express, feeding `W127` / `W141` and
   `E004` / `E005` respectively.
5. **Event validity** — `event_requires`, `event_requirement_forms`, and
   `excluded_events` are checked against the active event context (see
   [Events](#events-irules-only) below), producing `IRULE1001`.
6. **Lifecycle** — `Lifecycle` on the command, subcommand, option, or
   argument value gates it against the resolved version, producing
   `W135`–`W139` and `W144`.

### Argument processing — roles, values, and types

Beyond arity and options, the registry describes the *semantic role* of
each argument position, what values are valid there, what type the
command expects, and what hover/completion information to present.

#### ArgRole — what each argument means

`ArgRole` (`rust/tcl-registry/src/arg_role.rs`) classifies how the
compiler should treat each argument position:

```rust
pub enum ArgRole {
    /// Tcl script body — recursively lowered into IR.
    Body,
    /// Expression — parsed into an `ExprNode` AST.
    Expr,
    /// Variable name written by the command (`set`, `incr`).
    VarWrite,
    /// Variable name read without modification (`info exists`).
    VarRead,
    /// A `foreach`-style variable list.
    LoopVarList,
    /// Procedure parameter list (`proc`).
    ParamList,
    /// Symbolic name (proc name, namespace name).
    Name,
    /// Pattern or regex argument.
    Pattern,
    /// A switch/flag argument.
    Option,
    /// Generic value (default for unlisted positions).
    Value,
    /// The subcommand word (`length` in `string length`).
    Subcommand,
    /// The `--` terminator.
    OptionTerminator,
    /// A `format`-family format string.
    FormatString,
    /// A `scan`-family format string.
    ScanFormat,
    /// Channel identifier (`stdout`, a channel id).
    Channel,
    /// List/string index expression.
    Index,
    /// A structural keyword (`elseif`, `on`, `finally`).
    Keyword,
    /// A command prefix that will be invoked with appended arguments.
    CommandPrefix,
    /// A command name.
    CommandName,
    /// A word that may or may not be a command name — probe, do not assume.
    CommandNameProbe,
    /// An `apply` lambda literal.
    LambdaLiteral,
}
```

Note the naming: the write/read pair is **`VarWrite` / `VarRead`**, not
`VAR_NAME` / `VAR_READ`.  The four roles with no counterpart in the
older model — `CommandPrefix`, `CommandName`, `CommandNameProbe`, and
`LambdaLiteral` — are what let callback-taking commands
(`trace add`, `lsort -command`, `after`, `apply`) be described as data
rather than special-cased in a walker.

Roles are declared as `CommandSpec::arg_roles` or
`SubCommand::arg_roles`, both `&'static [(u8, ArgRole)]` — a sorted
slice of `(arg_index, role)` pairs, 0-based after the command name,
rather than a map.

For variable-layout commands like `if`, `try`, and `switch` (where
argument structure depends on the actual arguments), an
`arg_role_resolver` inspects the real argument list:

```rust
pub type ArgRoleResolver = fn(args: &[&str]) -> Vec<(u8, ArgRole)>;
```

It is a plain `fn` pointer, not a closure, so the whole spec stays
`const`.  The dynamic resolver takes priority over the static
`arg_roles`, which in turn takes priority over the legacy
`assigns_variable_at: Option<u8>` shorthand.

The IR lowering pass uses `ArgRole::Body` and `ArgRole::Expr` to decide
which arguments should be recursively lowered or parsed as expressions,
and `ArgRole::VarWrite` to extract variable definitions for dataflow
analysis.

#### ArgValue — completable values

`ArgValue` (`rust/tcl-registry/src/hover.rs`) describes a valid value for
a specific argument position, providing completion text and hover
documentation:

```rust
pub struct ArgValue {
    /// Completion text (e.g. `"length"`, `"alnum"`).
    pub value: &'static str,
    /// Short description shown in the completion list.
    pub detail: &'static str,
    /// Minimum Tcl version this value exists at, if it is version-gated.
    pub min_tcl: Option<tcl_dialect::TclVersion>,
    /// Numeric code, for value sets that carry one.
    pub code: Option<i64>,
}
```

Argument values are declared in two places, both as
`&'static [(u8, &'static [ArgValue])]` — arg index to its value set:

1. **`CommandSpec::arg_values`** — for the command's own positions.
2. **`SubCommand::arg_values`** — per-subcommand completable values;
   e.g. `string is` has its character-class values at index 0.

`SubCommand::versioned_arg_values` (`&[VersionedArgValue]`) covers value
sets whose membership changes across Tcl releases, and
`arg_values_accept_prefix` marks a set where unambiguous prefixes are
accepted.  A value outside a closed set produces **`W127`**; one that
needs a newer Tcl than the active dialect produces **`W137`**.

#### HoverSnippet — documentation content

`HoverSnippet` (`rust/tcl-registry/src/hover.rs`) carries hover and
signature-help content derived from man pages or vendor documentation:

```rust
pub struct HoverSnippet {
    /// One-line description.
    pub summary: &'static str,
    /// Invocation signatures.
    pub synopsis: &'static [&'static str],
    /// Extended description (used by signature help).
    pub snippet: &'static str,
    /// Attribution (e.g. a man-page name or a clouddocs URL).
    pub source: &'static str,
    /// Code example.
    pub examples: &'static str,
    /// Return-value description.
    pub return_value: &'static str,
}
```

`HoverSnippet::brief(summary, synopsis, source)` is the `const`
constructor for the common case.  Snippets appear on `CommandSpec::hover`
and `SubCommand::hover`; the LSP hover provider
(`rust/tcl-lsp-core/src/hover.rs`) renders them.

#### ArgTypeHint — expected types

`ArgTypeHint` (`rust/tcl-registry/src/hooks.rs`) declares what Tcl
internal representation (intrep) a command expects for a given argument:

```rust
pub struct ArgTypeHint {
    /// Expected type (`None` = any).
    pub expected: Option<TclType>,
    /// Whether the command forces a conversion.
    pub shimmers: bool,
    /// Types this argument passes through without forcing a conversion.
    pub transparent_from: &'static [TclType],
}
```

Type hints are declared as `CommandSpec::arg_types` /
`SubCommand::arg_types`, both `&'static [(u8, ArgTypeHint)]`.  The type
inference pass uses these to detect shimmer risks — the **`S100`–`S103`**
and **`S110`** family — and to propagate types through the SSA graph.
`transparent_from` is the field that stops a false positive when a
command accepts a type without re-representing it.

Return types are declared as `CommandSpec::return_type` /
`SubCommand::return_type`, an `Option<TclType>`; richer shape knowledge
goes in `return_elements` (`Option<ReturnElements>`).

#### Keyword completion — variable-layout scaffolding

Commands like `if`, `try`, and `switch` have keyword-delimited structure
rather than fixed argument positions.  Their structural words are
described by `ArgRole::Keyword` positions plus a
`CommandSpec::completion` descriptor
(`Option<CompletionDescriptor>`, `rust/tcl-registry/src/completion.rs`),
which the LSP completion provider consumes to suggest `elseif`, `else`,
`on`, `finally`, and so on based on what has been typed so far.

#### Deprecation and lifecycle

Version and deprecation knowledge is a single `Lifecycle` value
(`rust/tcl-registry/src/lifecycle.rs`) on `CommandSpec`, `SubCommand`,
`OptionSpec`, `ProfileSpec`, and `EventProps` alike — introducing,
deprecating, and retiring releases on the relevant version axis.  It
drives `W139` (retired), `W144` (deprecated), `W135`/`W136` (needs a
newer package), and `W137`/`W138` (needs a newer Tcl).

Straight replacements are named separately:

- `CommandSpec::deprecated_replacement: Option<&'static str>` — the
  replacement command name.
- `CommandSpec::deprecated_replacement_drop_in: bool` — whether the
  replacement is a literal drop-in, which is what lets the LSP offer an
  automatic code action rather than only a message.

### Side effects and purity

The side-effect model has **two halves**, and the distinction matters:

- **Declared** effects live on the registry spec —
  `tcl_registry::side_effects::SideEffect`, a `&'static` record of what
  the command *can* touch.
- **Classified** effects are per-invocation —
  `tcl_compiler::side_effects::{SideEffect, CommandSideEffects}`, built
  from the declaration plus this call's actual arguments.

The registry-side record is deliberately minimal:

```rust
// rust/tcl-registry/src/side_effects.rs
pub struct SideEffect {
    pub target: SideEffectTarget,
    pub reads: bool,
    pub writes: bool,
    pub connection_side: ConnectionSide,
    /// Dialect gate; `None` inherits the command's.
    pub dialects: Option<DialectSet>,
}
```

`SideEffectTarget` has 42 variants covering the Tcl-universal resources
(`Variable`, `FileIo`, `LogIo`, `NetworkIo`, `ChannelIo`, `Process`,
`ProcDefinition`, `NamespaceState`, `InterpState`) and the F5 ones
(`HttpHeader`, `HttpBody`, `HttpUri`, `HttpCookie`, `SslState`,
`TcpState`, `PoolSelection`, `SnatSelection`, `SessionTable`,
`PersistenceTable`, `DataGroup`, `AsmState`, `ApmState`, …), plus
`Unknown`.  `SideEffectTarget::as_str()` renders each to a stable
kebab-case name.  `ConnectionSide` is `None`, `Client`, `Server`, `Both`,
`Global`.

The compiler-side record adds everything that can only be known at a call
site:

```rust
// rust/tcl-compiler/src/side_effects.rs
pub struct SideEffect {
    pub target: SideEffectTarget,
    pub reads: bool,
    pub writes: bool,
    /// Data shape: Scalar / List / Dict / Array / Unknown.
    pub storage_type: StorageType,
    /// Where it lives.
    pub scope: StorageScope,
    /// F5 proxy context.
    pub connection_side: ConnectionSide,
    /// Enclosing namespace, when known.
    pub namespace: Option<String>,
    pub dialect: Option<String>,
    /// Literal variable / header / table name.
    pub key: Option<String>,
    /// Literal `table`/`session` subtable, when given.
    pub subtable: Option<String>,
}

pub struct CommandSideEffects {
    pub effects: Vec<SideEffect>,
    /// No observable side effects.
    pub pure: bool,
    /// Same inputs → same outputs.
    pub deterministic: bool,
    /// `eval` / `uplevel` — unknowable.
    pub dynamic_barrier: bool,
    pub dialect: Option<String>,
}

pub enum StorageScope {
    // Tcl-universal
    ProcLocal, Namespace, Global, Upvar,
    // F5 iRules-specific
    Event, Connection, Static, SessionTable, Persistence, DataGroup,
    // host resources
    FileSystem, NetworkSocket, LogOutput,
    Unknown,
}
```

**Classification** —
`classify_side_effects(registry, command, args, dialect, callee_summary)`
combines registry data with the real arguments:

1. If a `callee_summary` is supplied (a user-defined proc), classify from
   the interprocedural summary and stop.
2. Unknown command → a conservative unknown-write fallback.
3. Resolve the subcommand and read its `side_effects`, falling back to
   the command's.
4. Apply the command-level or subcommand-level `pure` / `mutator` flags.
5. Bind literal argument values into `key` / `subtable` where the spec
   says which argument names the resource.

**How purity propagates:**

- Purity is a `SubCommand::pure` / `SubCommand::mutator` pair plus
  `Traits` on the parent `CommandSpec` — the two flags are per-subcommand
  precisely so `HTTP::header value` can be pure while
  `HTTP::header replace` is a mutator.
- `target_to_region(target, scope)` projects an effect onto an
  `EffectRegion` bitset, which is the granularity the interprocedural
  summaries and the optimiser actually compare.

The GVN optimiser uses purity to decide whether a command's result can be
cached (`gvn::is_pure_command` / `is_pure_with_procs`), and SCCP uses it
to infer through pure calls without bailing out.

### Taint analysis

Taint tracking determines whether values originate from untrusted input
(user-controlled HTTP headers, URI, query parameters, etc.).

**TaintColour** (`rust/tcl-registry/src/taint.rs`) is a `bitflags` set —
colours compose with `|` and the lattice join is their intersection
(`&`):

```rust
bitflags! {
    pub struct TaintColour: u32 {
        /// Base: the value comes from untrusted input.
        const TAINTED            = 1 << 0;
        /// Starts with `/` (`HTTP::uri`, `HTTP::path`).
        const PATH_PREFIXED      = 1 << 1;
        const NON_DASH_PREFIXED  = 1 << 2;
        /// No CR/LF characters — header-injection safe.
        const CRLF_FREE          = 1 << 3;
        const SHELL_ATOM         = 1 << 4;
        /// Canonical Tcl list — safe for list operations.
        const LIST_CANONICAL     = 1 << 5;
        const REGEX_LITERAL      = 1 << 6;
        const PATH_NORMALISED    = 1 << 7;
        const PATH_BOUNDED       = 1 << 8;
        const HEADER_TOKEN_SAFE  = 1 << 9;
        const HTML_ESCAPED       = 1 << 10;
        const URL_ENCODED        = 1 << 11;
        const IP_ADDRESS         = 1 << 12;
        const PORT               = 1 << 13;
        const FQDN               = 1 << 14;
        const PATH_JOINED        = 1 << 15;
        const CHANNEL            = 1 << 16;
    }
}
```

Colours represent *safety properties* of tainted data.  A value with
`TAINTED | IP_ADDRESS` is tainted but known to be a safe IP-address
format, which may satisfy certain sinks (e.g. connecting to a backend).

There is **no `TaintHint` type**.  Taint metadata is a set of flat fields
directly on `CommandSpec`:

| Field | Meaning |
|---|---|
| `taint_source: Option<TaintColour>` | the return value is tainted, with these colours |
| `taint_transform: Option<TaintColour>` | colours this command *adds* to its input (a sanitiser) |
| `taint_output_sink: Option<&'static str>` | an output sink, named by its diagnostic |
| `taint_output_sink_subcommands: &'static [&'static str]` | restrict the sink to these subcommands |
| `taint_log_sink: Option<&'static str>` | a log sink (log injection) |
| `taint_network_sink_args: Option<&'static [u8]>` | argument indices that are network-address sinks (SSRF) |
| `taint_code_sink_args: Option<&'static [u8]>` | argument indices that are code-execution sinks |
| `taint_interp_eval_subcommands: &'static [&'static str]` | cross-interpreter eval subcommands |
| `taint_sink_safe_colour: Option<TaintColour>` | the colour that satisfies this sink |
| `taint_sink_gate: Option<fn(&[&str]) -> bool>` | a predicate that decides whether this call is a sink at all |
| `taint_double_encode_colour: Option<TaintColour>` | drives the `T106` double-encoding check |
| `setter_constraints: &'static [SetterConstraint]` | e.g. "must start with `/`" |

**Example** — `SSL::sni` is a taint source that also proves its result is
a domain name:

```rust
taint_source: Some(TaintColour::TAINTED.union(TaintColour::FQDN)),
```

`SetterConstraint` names the argument index, the required prefix, the
`DiagCode`, and the message, so the "`HTTP::uri` must start with `/`"
check (`IRULE3101`) is data rather than a hardcoded rule.

The taint engine (`rust/tcl-compiler/src/taint.rs` plus
`taint_interproc.rs`) propagates colours through the SSA graph as a
`TaintLattice` — a single `colours: TaintColour` field, where `TAINTED`
membership is "may be tainted" (a may-analysis) and every other colour is
a must-analysis surviving only when present on every incoming edge.  It
emits `T100`–`T106` for the Tcl-universal sinks and `IRULE3001`–`3004`
for the F5 ones when tainted data reaches a sink without sufficient
safety colours.

### Dialects

Dialects partition command availability across Tcl versions and tool
contexts.  A dialect *set* is a bitflag rather than a set of strings
(`rust/tcl-dialect/src/dialect_set.rs`):

```rust
bitflags! {
    pub struct DialectSet: u64 {
        const TCL84     = 1 << 0;
        const TCL85     = 1 << 1;
        const TCL86     = 1 << 2;
        const TCL90     = 1 << 3;
        const IRULES    = 1 << 4;
        const IAPPS     = 1 << 5;
        const TK        = 1 << 6;
        const EXPECT    = 1 << 7;
        const BPF       = 1 << 13;
        const TCL91     = 1 << 14;
        const TMSH      = 1 << 15;
        const BIGIP     = 1 << 16;

        // Convenience unions
        const ALL_TCL     = /* TCL84 | TCL85 | TCL86 | TCL90 | TCL91 */;
        const TCL85_PLUS  = /* … */;
        const TCL86_PLUS  = /* … */;
        const TCL8X       = /* TCL84 | TCL85 | TCL86 */;
        const TCL90_PLUS  = /* TCL90 | TCL91 */;
        const TK_AND_TCL  = /* ALL_TCL | TK */;
    }
}
```

Bits 8–12 are **free**: they used to be the Synopsys / Cadence / Xilinx /
Quartus / Mentor EDA vendor bits, and were retired when EDA shells moved
to "a base Tcl version plus `required_package`-gated command libraries"
(see [eda-library-packages.md](eda-library-packages.md)).  The
`KNOWN_DIALECTS` *name* list still carries the EDA names for profile
selection, but they no longer have their own spec-visibility bits.

Every `CommandSpec` has a `dialects: Option<DialectSet>` field:

- `dialects: Some(DialectSet::IRULES)` → iRules-only command
  (e.g. `HTTP::host`, `pool`, `table`).
- `dialects: Some(DialectSet::TCL86_PLUS)` → present from Tcl 8.6 on.

A universal `dialects: None` was **eliminated registry-wide**: every
command now carries an explicit group, with the `IRULES` bit present if
and only if iRules enables it.  That is what makes a K36322151-banned
command such as `exec` simply `ALL_TCL`, never intersecting the bare
`IRULES` mask, without a negative exclusion list.  Subcommands and
options keep `dialects: None` meaning "inherit the parent's".

Lookup is `CommandRegistry::get_for_dialect(name, dialect)`, which
applies the most-specific-spec rule when several specs share a name.
Per-version presence within a dialect is `Lifecycle`, not a separate
status enum — there is no `DialectStatus` type.

### Events (iRules only)

In F5 iRules, commands are only valid in certain events (e.g. `HTTP::uri`
requires an HTTP profile and only works in HTTP events).  This is
modelled by `EventRequires` (`rust/tcl-registry/src/events.rs`):

```rust
pub struct EventRequires {
    /// Requires client side.
    pub client_side: bool,
    /// Requires server side.
    pub server_side: bool,
    /// Required transport (`"tcp"` or `"udp"`).
    pub transport: Option<&'static str>,
    /// Required profile types.
    pub profiles: &'static [&'static str],
    /// Events where the command is unconditionally valid.
    pub also_in: &'static [&'static str],
    /// Only valid in `RULE_INIT`.
    pub init_only: bool,
    /// Requires active traffic flow.
    pub flow: bool,
    /// Required profile capability (e.g. `"sni"`) — declared, not yet consumed.
    pub capability: Option<&'static str>,
}
```

**Example** — `ASM::is_authenticated` requires an ASM profile:

```rust
event_requires: Some(EventRequires {
    client_side: false,
    server_side: false,
    transport: None,
    profiles: &["ASM"],
    also_in: &[],
    init_only: false,
    flow: false,
    capability: None,
}),
```

`tcl_registry::events::event_satisfies(props, requires, event_name,
profiles)` matches these against the event's `EventProps`, which
describes what each event provides (client/server side, transport,
implied profiles, whether there is an active flow).  Mismatches produce
**`IRULE1001`**, via
`CommandRegistry::is_irules_command_legal_in_event`; the inverse query,
"every command legal here", is `valid_irules_commands_for_event`.

A few commands have subforms with different event contracts —
`FIX::tag get` reads a live message while `FIX::tag map set` configures
a mapping.  `CommandSpec::event_requirement_forms`
(`&[EventRequirementForm]`) overrides the top-level contract for a
matching literal argument prefix, longest match winning, so a consumer
selects the right contract from the call words without knowing the
command by name.

`CommandSpec::excluded_events` lists events where a command is explicitly
forbidden (e.g. a command that crashes in `RULE_INIT`).

### How the infrastructure feeds the compiler

The registry metadata flows into every stage of the compilation pipeline:

1. **IR Lowering** — `lower_to_ir()` uses `arg_roles` /
   `arg_role_resolver` to identify which arguments are bodies
   (`ArgRole::Body`), expressions (`ArgRole::Expr`), or variable names
   (`ArgRole::VarWrite`).  This drives recursive lowering of script
   bodies and expression parsing.  A `lowering_hook: Option<LoweringHookId>`
   selects a per-command specialisation.

2. **CFG** — commands whose effects defeat static analysis (`eval`,
   `uplevel`, `upvar`) lower to `Statement::Barrier`, which the CFG
   builder carries through as an opaque statement.

3. **SSA/SCCP** — `pure` commands can be inferred through without
   invalidating the lattice state.  Impure commands force values to
   `LatticeValue::Overdefined`.

4. **GVN** — purity (`gvn::is_pure_command`) plus the declared
   `EffectRegion` determine whether a command's result can be cached and
   reused (common subexpression elimination, `O105`).

5. **Codegen** — `codegen_hook` / `inline_codegen_hook` on `SubCommand`
   or `CommandSpec` select specialised bytecode (e.g. a `string length`
   opcode instead of a generic `invokeStk`).

6. **Taint engine** — `taint_source` / `taint_transform` / the
   `taint_*_sink*` fields mark sources and sinks; the taint lattice
   propagates `TaintColour` through the SSA graph.

7. **Diagnostics** — arity (`E002`/`E003`/`E005`), option terminators
   (`W304`), argument value sets (`W127`), event requirements
   (`IRULE1001`), and lifecycle (`W139`/`W144`) all read the spec.

---

## Example 1: `set x 42`

The simplest possible Tcl script — assign a constant to a variable.

### Source

```tcl
set x 42
```

### Stage 1 — Lexer → Token stream

The lexer scans byte-by-byte and produces a flat stream.  Rendering each
token as `kind span → text` (the text resolved through the `SourceMap`):

```
Esc  0..3  → "set"
Sep  3..4  → " "
Esc  4..5  → "x"
Sep  5..6  → " "
Esc  6..8  → "42"
Eof  8..8  → ""
```

Key observations:
- `set`, `x`, and `42` are all `Esc` (plain word fragments) — no variable
  substitution or braces involved.
- Whitespace becomes `Sep` tokens — they delimit words but carry no semantic
  value.
- The tokens hold only spans; `"set"` is a slice of the source, not an
  owned `String`.

### Stage 2 — Segmenter → SegmentedCommand

The segmenter builds the red-green CST for the source and derives one
`SegmentedCommand` per command (split at `Eol`/`Eof` boundaries):

```rust
SegmentedCommand {
    span: Span::new(0, 8),
    argv: vec![/* the Esc tokens for `set`, `x`, `42` */],
    texts: vec!["set".into(), "x".into(), "42".into()],
    single_token_word: vec![true, true, true],
    all_tokens: vec![/* all six tokens, separators included */],
    is_partial: false,
    ..
}
```

- `texts[0] == "set"` → command name (`SegmentedCommand::name()`)
- `texts[1] == "x"` → variable name argument
- `texts[2] == "42"` → value argument
- All words are single-token (no interpolation), so `single_token_word` is
  all `true` — this tells the lowerer the value is a compile-time constant.

### Stage 3 — IR Lowering → `Statement::AssignConst`

The lowerer's `set` hook matches two arguments where the second is a
single-token constant.  `tcl explore set.tcl --show ir --text`:

```
=== ir ===
└── top-level
    └── assign-const x = 42
        · kind: IRAssignConst
        · summary: assign-const x = 42
        · range: 1:1  (0…8)
```

As a Rust value that is:

```rust
Statement::AssignConst {
    span: Span::new(0, 8),
    name: "x".into(),
    name_braced: false,
    value: "42".into(),
    value_span: Some(Span::new(6, 8)),
}
```

Why `AssignConst` and not `AssignValue`?  Because `"42"` is a single
atomic token with no variable substitution — it is known at compile time.

(The explorer labels nodes with the `IR*` names from
`tcl_explorer::formatters::stmt_kind`, which map one-to-one onto the
`Statement` variants: `IRAssignConst` ⇔ `Statement::AssignConst`, and so
on.)

### Stage 4 — CFG → single basic block

With no control flow, the CFG is trivial —
`tcl explore set.tcl --show cfg --text`:

```
=== cfg ===
└── function ::top (entry=entry_1, 2 blocks)
    ├── block entry_1 [entry]
    │   ├── assign-const x = 42
    │   │   · summary: assign-const x = 42
    │   │   · range: 1:1  (0…8)
    │   └── term goto exit_2
    │       · range: ?
    └── block exit_2
        └── term <none>
            · range: ?
```

The builder creates an entry block containing the statement, linked to an
exit block via `Terminator::Goto`.  The exit block's terminator is `None`
— it is where control leaves the function.

### Stage 5 — SSA → x₁

With a single block and a single definition, SSA is trivial —
`tcl explore set.tcl --show ssa --text`:

```
=== ssa ===
└── function ::top (entry=entry_1, 2 blocks)
    ├── block entry_1 [entry]
    │   ├── assign-const x = 42
    │   │   · summary: assign-const x = 42
    │   │   · range: 1:1  (0…8)
    │   │   · uses: {}
    │   │   · defs: {x#1=const(42):int}
    │   └── term goto exit_2
    ├── block exit_2
    │   └── term <none>
    └── analysis
        ├── dead store entry_1 x#1
        │   · block: entry_1
        │   · stmt index: 0
        └── x#1: int
            · value: x#1
            · type: int
```

- `x` gets version 1 (its first definition): SSA value key `x#1`, i.e. the
  `ValueKey` pair `(Symbol(x), 1)` (see [Glossary → SSA](#glossary)).
- No phi nodes — there is only one path through the program (see
  [Glossary → Phi node](#glossary)).
- `uses` is empty — `set x 42` doesn't read any variables.

### Stage 6 — Core analyses

Note that the explorer prints all of these inline under `analysis`, as
seen above.

**SCCP** (see [Glossary](#glossary))**:** `x#1` →
`LatticeValue::Const(ConstValue::Int(42))` — provably constant.  The
explorer renders it `const(42):int`.

**Type inference:** `x#1` → a `TypeLattice` holding `TypeShape::Int` —
`"42"` is a valid integer literal, so the intrep is `Int`.

**Liveness:** `x#1` is dead here, because nothing reads it.

**Dead stores:** the `dead store entry_1 x#1` finding is a
`DeadStore { block: "entry_1", statement_index: 0, variable: "x",
version: 1 }`, which the elimination pass turns into diagnostic
**`O109`**.

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

Note the critical difference: `42` is `Esc` (plain text), but `$x` produces
a `Var` token whose *text* is the bare variable name — the token's
`content_offset` is 1, so `SourceMap::token_text` strips the leading `$`
and yields `"x"`.

### Stage 2 — Segmenter

Two `SegmentedCommand` values:

```rust
// Command 1: set x 42
SegmentedCommand {
    texts: vec!["set".into(), "x".into(), "42".into()],
    single_token_word: vec![true, true, true],   // all constant
    ..
}

// Command 2: set y $x
SegmentedCommand {
    texts: vec!["set".into(), "y".into(), "${x}".into()],  // Var → "${x}"
    single_token_word: vec![true, true, true],   // single token, but a Var
    ..
}
```

In command 2, `texts[2]` is `"${x}"` — the segmenter renders `Var` tokens
in canonical `${…}` form so a downstream consumer can tell a substitution
from a literal without re-inspecting the token kind.

### Stage 3 — IR Lowering

```
=== ir ===
└── top-level
    ├── assign-const x = 42
    │   · kind: IRAssignConst
    │   · range: 1:1  (0…8)
    └── assign-value y = ${x}
        · kind: IRAssignValue
        · range: 2:1  (9…17)
```

The second `set` produces `Statement::AssignValue` (not `AssignConst`)
because the value `${x}` contains a variable substitution that must be
resolved at run time.

### Stage 5 — SSA

```
=== ssa ===
└── function ::top (entry=entry_1, 2 blocks)
    ├── block entry_1 [entry]
    │   ├── assign-const x = 42
    │   │   · uses: {}
    │   │   · defs: {x#1=const(42):int}
    │   ├── assign-value y = ${x}
    │   │   · uses: {x#1=const(42):int}
    │   │   · defs: {y#1=const(42):int}
    │   └── term goto exit_2
    └── analysis
        ├── dead store entry_1 y#1
        │   · stmt index: 1
        ├── x#1: int
        └── y#1: int
```

- `x#1 = "42"` — defined by the first `set`.
- `y#1` uses `x#1` — the SSA pass resolves `${x}` to version 1 of `x`.
- SCCP proves `y#1` is also constant `42`, propagated from `x#1` — the
  explorer prints the lattice value inline on both the use and the def.

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
=== ir ===
└── top-level
    └── IRExprEval
        · kind: IRExprEval
        · range: 1:1  (0…12)
```

The statement is `Statement::ExprEval`, and its `expr` field holds a
structured tree, not a raw string:

```rust
Statement::ExprEval {
    span: Span::new(0, 12),
    expr: ExprNode::Binary {
        op: BinOp::Add,
        left: Box::new(ExprNode::Literal { text: "2".into(), start: 0, end: 1 }),
        right: Box::new(ExprNode::Literal { text: "3".into(), start: 4, end: 5 }),
    },
    expr_base: Some(6),
}
```

`expr_base` is the absolute source offset the expression text starts at,
so the offsets inside the tree (which are relative to the expression
text) can be mapped back to the file.

### Stage 6 — SCCP

SCCP evaluates the expression: `Literal("2") + Literal("3")` →
`LatticeValue::Const(ConstValue::Int(5))`.  The compiler knows the result
at compile time.

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
=== ir ===
└── top-level
    ├── assign-const a = 10
    │   · kind: IRAssignConst
    │   · range: 1:1  (0…8)
    ├── assign-const b = 20
    │   · kind: IRAssignConst
    │   · range: 2:1  (9…17)
    └── IRExprEval
        · kind: IRExprEval
        · range: 3:1  (18…32)
```

The `ExprEval`'s tree this time has variable leaves:

```rust
ExprNode::Binary {
    op: BinOp::Add,
    left:  Box::new(ExprNode::Var { text: "$a".into(), name: "a".into(), start: 0, end: 2 }),
    right: Box::new(ExprNode::Var { text: "$b".into(), name: "b".into(), start: 5, end: 7 }),
}
```

`ExprNode::vars()` on this tree returns `{"a", "b"}` — that is how the
SSA builder learns which versions the statement uses without re-scanning
the text.

### Stage 5 — SSA

```
  a#1 = const(10):int
  b#1 = const(20):int
  IRExprEval  uses: {a#1, b#1}
```

SCCP propagates: `a#1 = 10`, `b#1 = 20`, so the expression result is
`Const(Int(30))`.

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

The simplest conditional — introduces `CFGBranch` and forked control flow.

### Source

```tcl
set x 1
if {$x} {
    set y 10
}
```

### Stage 3 — IR Lowering

```
=== ir ===
└── top-level
    ├── assign-const x = 1
    │   · kind: IRAssignConst
    │   · range: 1:1  (0…7)
    └── if (1 clause(s))
        · kind: IRIf
        · summary: if (1 clause(s))
        · range: 2:1  (8…32)
        └── clause 1: $x:
            └── assign-const y = 10
                · kind: IRAssignConst
                · range: 3:5  (22…30)
```

- `Statement::If` holds a `Vec<IfClause>` (one per `if` / `elseif`).
- The condition `{$x}` is parsed as `ExprNode::Var { name: "x", .. }`.
- No `else_body` for this example — the explorer's summary would read
  `if (1 clause(s), else)` if there were one.

### Stage 4 — CFG decomposition

The `If` statement is decomposed into basic blocks —
`tcl explore --show cfg --text`:

```
=== cfg ===
└── function ::top (entry=entry_1, 5 blocks)
    ├── block entry_1 [entry]
    │   ├── assign-const x = 1
    │   │   · range: 1:1  (0…7)
    │   └── term branch $x → if_then_3/if_next_4
    │       · range: 2:4  (11…15)
    ├── block if_end_2
    │   └── term goto exit_5
    ├── block if_then_3
    │   ├── assign-const y = 10
    │   │   · range: 3:5  (22…30)
    │   └── term goto if_end_2
    ├── block if_next_4
    │   └── term goto if_end_2
    └── block exit_5
        └── term <none>
```

`term branch $x → if_then_3/if_next_4` is a
`Terminator::Branch { condition, true_target, false_target, .. }`; the
plain `term goto …` lines are `Terminator::Goto`.

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
=== ssa ===
    ├── block entry_1 [entry]
    │   ├── assign-const x = 1
    │   │   · defs: {x#1=const(1):int}
    │   └── term branch $x → if_then_3/if_next_4
    ├── block if_then_3
    │   ├── assign-const y = 10
    │   │   · defs: {y#1=const(10):int}
    │   └── term goto if_end_2
    ├── block if_next_4 [unreachable]
    │   └── term goto if_end_2
```

No phi nodes are needed — `y` is only defined in one branch.

### Stage 6 — SCCP and constant branch detection

SCCP determines:
- `x#1 = const(1)` — `"1"` is truthy in Tcl.
- The branch condition is constant `true` → `if_next_4` is unreachable
  (the explorer tags the block `[unreachable]`).

The explorer's `analysis` section shows both findings:

```
    └── analysis
        ├── const branch entry_1: always True
        │   · condition: $x
        │   · take: if_then_3
        ├── dead store if_then_3 y#1
        │   · block: if_then_3
        │   · stmt index: 0
        ├── unreachable: if_next_4
        │   · blocks: if_next_4
        ├── x#1: int
        └── y#1: int
```

The first is a `ConstantBranch { block: "entry_1", condition: "$x",
value: true, taken_target: "if_then_3", not_taken_target: "if_next_4" }`.

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
=== ir ===
└── top-level
    └── if (1 clause(s), else)
        · kind: IRIf
        ├── clause 1: 1:
        │   └── assign-const x = 1
        └── else:
            └── assign-const x = 2
```

The clause's condition is `ExprNode::Literal { text: "1", .. }` and
`Statement::If::else_body` is `Some(Script { … })`.

### Stage 4/5 — CFG and SCCP

SCCP immediately determines `ExprLiteral("1")` is truthy → the else branch
is unreachable.  The constant branch detection marks `if_next` as dead code.

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
=== ir ===
└── top-level
    ├── assign-const x = 5
    │   · kind: IRAssignConst
    │   · range: 1:1  (0…7)
    └── if (2 clause(s), else)
        · kind: IRIf
        · range: 2:1  (8…98)
        ├── clause 1: $x < 0:
        │   └── assign-const sign = -1
        │       · range: 3:5  (26…37)
        ├── clause 2: $x > 0:
        │   └── assign-const sign = 1
        │       · range: 5:5  (62…72)
        └── else:
            └── assign-const sign = 0
                · range: 7:5  (86…96)
```

Each clause's condition is an `ExprNode::Binary { op: BinOp::Lt, .. }` /
`{ op: BinOp::Gt, .. }` over an `ExprNode::Var` and an
`ExprNode::Literal`; the explorer prints the clause header as the
condition's rendered source text.

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

Phi insertion is **pruned**: a phi is placed only where a later use
actually needs the merged value.  With the script exactly as written
above, nothing reads `sign` afterwards, so `if_end_2` carries no phi —
just three separate definitions (`sign#1` in `if_then_3`, `sign#2` in
`if_then_5`, `sign#3` in `if_next_6`) that die at the merge.

Add a `puts $sign` after the `if`, and the phi appears:

```
    ├── block if_end_2
    │   ├── phi sign#1 ← if_next_6:sign#4, if_then_3:sign#2, if_then_5:sign#3
    │   │   · incoming: if_next_6:sign#4, if_then_3:sign#2, if_then_5:sign#3
```

Three definitions of `sign` merge at `if_end_2` — the phi node (see
[Glossary → Phi node](#glossary)) selects the correct version based on
which predecessor block executed.  It is a `ssa::Phi { name, version,
incoming: HashMap<BlockId, Version> }`; the explorer renders the map as
`predecessor:variable#version` pairs.

SCCP determines `x#1 = const(5)`:
- `5 < 0` → false, so `if_then_3` is unreachable
- `5 > 0` → true, so `if_then_5` is taken
- Result: `sign` is `const(1)`

The explorer's `analysis` section names both folded branches:

```
    └── analysis
        ├── const branch entry_1: always False
        │   · condition: $x < 0
        │   · take: if_next_4
        ├── const branch if_next_4: always True
        │   · condition: $x > 0
        │   · take: if_then_5
        ├── unreachable: if_next_6, if_then_3
```

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
=== ir ===
└── top-level
    ├── assign-const i = 0
    │   · kind: IRAssignConst
    │   · range: 1:1  (0…7)
    └── IRWhile
        · kind: IRWhile
        · range: 2:1  (8…37)
```

- `Statement::While` has a structured `ExprNode` condition and a `Script`
  body.  (The explorer's IR view prints the statement kind for `While`
  rather than expanding its sub-scripts; the CFG view below is where the
  body becomes visible.)
- `Statement::Incr { name: "i", amount: None, .. }` means increment by 1.

### Stage 4 — CFG decomposition

```
=== cfg ===
└── function ::top (entry=entry_1, 5 blocks)
    ├── block entry_1 [entry]
    │   ├── assign-const i = 0
    │   └── term goto while_header_2
    ├── block while_header_2
    │   └── term branch $i < 5 → while_body_3/while_end_4
    ├── block while_body_3
    │   ├── incr i
    │   └── term goto while_header_2
    ├── block while_end_4
    │   └── term goto exit_5
    └── block exit_5
        └── term <none>
```

```
  entry_1: i = "0"
      │
      ▼
  ┌──► while_header_2:
  │    branch($i < 5)
  │    ┌────────┴────────┐
  │ true│                 │false
  │    ▼                  ▼
  │  while_body_3:     while_end_4:
  │  incr i               │
  │    │                   ▼
  └────┘                 exit_5
```

The `while` decomposes into:
- A header block with the condition `Terminator::Branch`
- A body block that jumps back to the header (back edge)
- An exit block for when the condition is false

### Stage 5 — SSA with loop phi

```
=== ssa ===
    ├── block entry_1 [entry]
    │   ├── assign-const i = 0
    │   │   · defs: {i#1=const(0):int}
    │   └── term goto while_header_2
    ├── block while_header_2
    │   ├── phi i#2 ← entry_1:i#1, while_body_3:i#3
    │   │   · type: int
    │   │   · incoming: entry_1:i#1, while_body_3:i#3
    │   └── term branch $i < 5 → while_body_3/while_end_4
    ├── block while_body_3
    │   ├── incr i
    │   │   · uses: {i#2=overdefined:int}
    │   │   · defs: {i#3=overdefined:int}
    │   └── term goto while_header_2
```

The phi node at the loop header merges:
- `i#1 = 0` (initial value from entry)
- `i#3` (incremented value from the body)

SCCP cannot fold this to a constant (the value changes each iteration),
so `i#2` is `LatticeValue::Overdefined` — but the **type** stays `int`,
which is the point of keeping the value and type lattices separate.

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
=== ir ===
└── top-level
    └── for ($i < 10)
        · kind: IRFor
        · summary: for ($i < 10)
        · range: 1:1  (0…49)
        ├── init:
        │   └── assign-const i = 0
        │       · range: 1:6  (5…12)
        ├── condition: $i < 10:
        ├── next:
        │   └── incr i
        │       · range: 1:26  (25…31)
        └── body:
            └── assign-value x = ${i}
                · range: 2:5  (39…47)
```

Unlike `While`, the explorer expands `Statement::For` in the IR view,
because each of its four sub-parts (`init`, `condition`, `next`, `body`)
is a separately-lowered field on the variant.

### Stage 4 — CFG decomposition

```
=== cfg ===
└── function ::top (entry=entry_1, 6 blocks)
    ├── block entry_1 [entry]
    │   ├── assign-const i = 0
    │   └── term goto for_header_2
    ├── block for_header_2
    │   └── term branch 1 → for_body_3/for_end_5
    ├── block for_body_3
    │   ├── assign-value x = ${i}
    │   └── term goto for_step_4
    ├── block for_step_4
    │   ├── incr i
    │   └── term branch $i < 10 → for_body_3/for_end_5
    ├── block for_end_5
    │   └── term goto exit_6
    └── block exit_6
        └── term <none>
```

```
  entry_1: i = "0"  (init clause)
      │
      ▼
    for_header_2:
    branch(1)              ← first-iteration entry: the guard is already
    ┌────────┴────────┐      known true when the init ran
true│                  │false
    ▼                  ▼
  for_body_3:       for_end_5:
  x = $i               │
    │                   ▼
    ▼                 exit_6
  for_step_4:
  incr i
  branch($i < 10) ──true──► for_body_3
    │ false
    ▼
  for_end_5
```

Unlike `while`, the `for` loop has a separate step block that runs after
the body.  Note that the *real* shape is bottom-tested: the header's
branch is the trivially-true first-iteration guard, and the live loop
test lives at the end of the step block, branching straight back to the
body.  That is the same condition-at-bottom layout the bytecode uses,
built at the CFG level rather than only in codegen.

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

`foreach` is inlined into a real loop CFG and emits the specialised
`foreach_start` / `foreach_step` / `foreach_end` opcodes — at top level
as well as inside a `proc`.

### Source

```tcl
foreach item {a b c} {
    set x $item
}
```

### Stage 3 — IR Lowering

```
=== ir ===
└── top-level
    └── IRForeach
        · kind: IRForeach
        · range: 1:1  (0…40)
```

The statement is `Statement::Foreach { iterators, body, is_lmap: false,
is_dict: false, .. }`, where `iterators` is a `Vec<ForeachIterator>`
pairing each variable list with its list argument.

### Stage 4 — CFG

The loop is decomposed the same way `while` and `for` are, with the
addition of a **latch** block that drives the iterator:

```
=== cfg ===
└── function ::top (entry=entry_1, 6 blocks)
    ├── block entry_1 [entry]
    │   └── term goto foreach_header_2
    ├── block foreach_header_2
    │   └── term branch 1 → foreach_body_3/foreach_end_4
    ├── block foreach_body_3
    │   ├── call foreach a b c
    │   │   · summary: call foreach a b c
    │   │   · range: 1:1  (0…40)
    │   ├── assign-value x = ${item}
    │   │   · range: 2:5  (27…38)
    │   └── term goto foreach_latch_5
    ├── block foreach_end_4
    │   └── term goto exit_6
    └── block exit_6
```

The synthetic `call foreach a b c` at the head of the body block is the
iterator-binding header the CFG builder inserts: it is a
`Statement::Call` whose `defs` are the loop variables and whose
`foreach_groups` field records how many variables belong to each
iterator group, so codegen can reconstruct the original
`var-list` ↔ `list-arg` pairing.

### Stage 7 — Bytecode

```
  ByteCode ::top, 11 instructions, 19 bytes, 4 literals, 0 variables
    Literals:
      0: "a b c"
      1: "x"
      2: "item"
      3: ""

    # entry_1:
    # foreach_header_2:
      (0)  push1 0          # "a b c"
      (2)  foreach_start 0
    # foreach_body_3:
      (7)  push1 1          # "x"
      (9)  push1 2          # "item"
      (11) loadStk
      (12) storeStk
    # cmd_end_2:
      (13) pop
    # foreach_continue_0:
      (14) foreach_step
    # foreach_break_1:
      (15) foreach_end
    # foreach_end_4:
      (16) push1 3          # ""
    # exit_5:
      (18) done
```

Note that the literal table holds only the *list* (`"a b c"`), the
variable names, and the empty-string loop result — the body never becomes
a literal, because it is compiled inline rather than passed to a generic
`invokeStk` call to the `foreach` command.  `foreach_continue_0` and
`foreach_break_1` are the labels `continue` and `break` inside the body
would jump to.

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
=== ir ===
├── top-level
│   └── call proc add a b \n    expr {$a + ...
│       · kind: IRCall
│       · range: 1:1  (0…37)
└── ::add {a b}
    · range: 1:1  (0…37)
    └── IRExprEval
        · kind: IRExprEval
        · range: 2:5  (21…35)
```

The procedure body is lifted into `Module::procedures` under the key
`::add`, as a `Procedure { name: "add", qualified_name: "::add", params:
vec!["a", "b"], body, .. }`.  Note that the `proc` command itself
**stays** in `top_level` as a `Statement::Call` — the definition is
copied out for analysis, not removed, because the registration is a real
runtime effect that must still be emitted.

### Stage 7 — Bytecode

**Top-level (proc registration):**
```
  ByteCode ::top, 6 instructions, 11 bytes, 4 literals, 0 variables
    Literals:
      0: "proc"
      1: "add"
      2: "a b"
      3: "\n    expr {$a + $b}\n"

    # entry_1:
      (0)  push1 0          # "proc"
      (2)  push1 1          # "add"
      (4)  push1 2          # "a b"
      (6)  push1 3          # "\n    expr {$a + $b}\n"
      (8)  invokeStk1 4     # proc
    # exit_2:
      (10) done
```

The parameter list and body literals are the **brace contents**
(`a b`, not `{a b}`) — the braces are word delimiters the lexer strips,
not part of the value.

**Procedure body (`::add`):**
```
  ByteCode ::add, 4 instructions, 6 bytes, 0 literals, 2 variables
    Local variables:
      %v0: "a"
      %v1: "b"

    # entry_1:
      (0) loadScalar1 %v0   # var "a"
      (2) loadScalar1 %v1   # var "b"
      (4) add
    # exit_2:
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

**`HTTP::header`** — the whole command is a taint source, and its
mutating subcommands are a header-injection sink
(`rust/tcl-registry/src/commands/irules/http__header.rs`):

```rust
const SUBCOMMANDS: &[SubCommand] = &[
    // …
    SubCommand {
        name: "replace",
        arity: Arity::new(1, 2),
        detail: "Replace header value.",
        synopsis: "HTTP::header replace <name> ?value?",
        mutator: true,                 // writing a header IS a side effect
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "value",
        arity: Arity::exact(1),
        detail: "Get first header value.",
        synopsis: "HTTP::header value <name>",
        ..SubCommand::DEFAULT           // pure by inheritance: Traits::PURE
    },
];

pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "HTTP::header",
        traits: Traits::PURE
            .union(Traits::CSE_CANDIDATE)
            .union(Traits::DIAGRAM_ACTION),
        // `HTTP::header insert|replace` with tainted data →
        // header injection (IRULE3002).
        taint_output_sink: Some("IRULE3002"),
        taint_output_sink_subcommands: &["insert", "replace"],
        event_requires: Some(EventRequires {
            transport: Some("tcp"),
            profiles: &["FASTHTTP", "HTTP"],
            also_in: &["MR_EGRESS", "MR_INGRESS", "SERVER_CONNECTED"],
            ..
        }),
        side_effects: &[SideEffect {
            target: SideEffectTarget::HttpHeader,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::Both,
            dialects: None,
        }],
        taint_source: Some(TaintColour::TAINTED),
        ..CommandSpec::DEFAULT
    }
}
```

Two things worth noticing.  First, `taint_source` is declared **on the
command**, not per-subcommand — the engine narrows it by call shape
rather than the spec enumerating source subcommands.  Second,
`taint_output_sink` and `taint_output_sink_subcommands` are a pair: the
sink applies only to `insert` and `replace`.

**`HTTP::respond`** — the response body is an XSS sink, declared as a
single field on its spec
(`rust/tcl-registry/src/commands/irules/http__respond.rs`):

```rust
taint_output_sink: Some("IRULE3001"),
```

With no `taint_output_sink_subcommands` list, the sink covers every
invocation.

**`string tolower`** — a pure command, but *not* a sanitiser
(`rust/tcl-registry/src/commands/tcl/string_.rs`):

```rust
SubCommand {
    name: "tolower",
    byte_array_effect: ByteArrayEffect::CaseFolds,
    arity: Arity::new(1, 3),
    detail: "Convert to lower case.",
    synopsis: "string tolower string ?first? ?last?",
    pure: true,
    return_type: Some(TclType::String),
    const_fold: Some(fold_tolower),
    ..SubCommand::DEFAULT
}
```

It sets no `taint_transform`, so it adds no mitigating colour — case
conversion does not sanitise anything.  Contrast a real sanitiser, which
declares `taint_transform: Some(TaintColour::HTML_ESCAPED)` (or
`URL_ENCODED`, `CRLF_FREE`, …).

### Stage 3 — IR Lowering

The `when HTTP_REQUEST { … }` handler is lowered as a synthetic
`Procedure`, so the whole analysis pipeline reaches inside it:

```
  ::when::HTTP_REQUEST
    assign-value host  = [HTTP::header value Host]
    assign-value lower = [string tolower ${host}]
    call HTTP::respond 200 content <h1>Welcome to ${lower}</h1>
```

The third statement is a `Statement::Call { command: "HTTP::respond",
args, reads: vec!["lower"], .. }`.

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
    assign-value host   uses: {}          defs: {host#1}
    assign-value lower  uses: {host#1}    defs: {lower#1}
    call HTTP::respond  uses: {lower#1}   defs: {}
```

### Taint propagation

The taint engine (`rust/tcl-compiler/src/taint.rs`, with the cross-proc
half in `taint_interproc.rs`) walks the SSA graph and computes a
`TaintLattice` for each `ValueKey`:

1. **`host#1`** — the `[HTTP::header value Host]` command substitution is
   evaluated:
   - the registry's `CommandSpec::taint_source` for `HTTP::header` is
     `Some(TaintColour::TAINTED)`.
   - Result: `TaintLattice { colours: TaintColour::TAINTED }`

2. **`lower#1`** — `[string tolower $host]`:
   - the argument `$host` carries `host#1`'s lattice → `TAINTED`.
   - `string tolower` sets no `taint_transform`, so no mitigating colour
     is added.
   - The subcommand is `pure`, so taint flows through: the result
     inherits from its arguments.
   - Result: `TaintLattice { colours: TaintColour::TAINTED }`

3. **`HTTP::respond`** — the sink check:
   - the spec's `taint_output_sink` is `Some("IRULE3001")` — the response
     body is an XSS-sensitive output sink.
   - `$lower` appears in the content argument.
   - `lower#1` is `TAINTED` with no mitigating colours; had it carried
     `HTML_ESCAPED`, the check would pass.

### Taint warning emitted

```rust
TaintWarning {
    span: /* the HTTP::respond command's span */,
    variable: "lower".into(),
    sink_command: "HTTP::respond".into(),
    code: DiagCode::Irule3001,
    message: "Tainted variable $lower in HTTP response body (HTTP::respond); \
              risk of XSS or content injection".into(),
    replacement: None,
    fixes: vec![/* offered code actions, e.g. wrap in an escaper */],
}
```

`TaintWarning::fixes` is the field with no counterpart in a
message-only warning: the engine can attach concrete `CodeFix` edits, so
the LSP offers "wrap this in `HTML::encode`" rather than only reporting
the risk.

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

Now `("safe", 1)` has `TaintLattice(tainted=True, colour=TAINTED | HTML_ESCAPED)`.
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

- `val₁` → `TaintLattice(tainted=True, colour=TAINTED)` (from HTTP::header)
- `val₂` → `TaintLattice(tainted=False)` (constant "unknown")

`taint_join(val₁, val₂)`:
- Either operand tainted → result is tainted.
- Colours: only keep colours present in **both** tainted operands.
  Since `val₂` is untainted, it contributes the tainted operand's colours
  unchanged.
- Result: `TaintLattice(tainted=True, colour=TAINTED)`

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
=== ir ===
├── top-level
│   ├── call proc double n \n    expr {$n * ...
│   ├── assign-value result = [double 21]
│   └── call puts ${result}
└── ::double {n}
    └── IRExprEval
```

`Module::procedures` gains a `Procedure { name: "double",
qualified_name: "::double", params: vec!["n"], .. }` whose body holds the
single `Statement::ExprEval` for `expr {$n * 2}`.

### Interprocedural analysis

`InterproceduralAnalysis`
(`rust/tcl-compiler/src/interprocedural.rs`) builds a `ProcSummary` for
each procedure.  For `::double`:

1. The body is a single `expr {$n * 2}` in tail position.
2. SCCP within the proc body: parameter `n` starts `LatticeValue::Unknown`.
3. Folding a call binds `n#1 = 21` and evaluates `$n * 2` →
   `Const(Int(42))`.
4. Result: the call `[double 21]` folds to the string `"42"`.

The eligibility gate is `ProcSummary::can_fold_static_calls`, computed in
the closure phase below.

### Optimisation pass — O103

The `PassId::Propagation` pass
(`rust/tcl-compiler/src/optimiser/propagation.rs`) is the one that emits
O103, alongside O100 constant-var-ref, O100 string-interpolation, O102
load forwarding, and O100 return-terminator folding:

1. Encounters the `[double 21]` command substitution.
2. Resolves `double` → the qualified name `::double`, using
   `resolve_internal_call` (which is `tcl_syntax::naming`'s canonical
   resolution over the unit's proc table).
3. Checks that `::double` is not in `Module::redefined_procedures`.
4. Checks all arguments are static: `"21"` is a literal.
5. Folds the body with the argument bound.
6. Gets back `"42"`.
7. Emits:

`tcl explore --show opt --text` shows exactly that:

```
=== opt ===
└── O103 Fold pure-proc call to '::double' to its constant return → 42
    · code: O103
    · message: Fold pure-proc call to '::double' to its constant return
    · replacement: 42
    · range: 5:12  (50…61)
```

As a Rust value:

```rust
Optimisation {
    code: DiagCode::O103,
    message: "Fold pure-proc call to '::double' to its constant return".into(),
    span: /* 50..61 — the span of `[double 21]` */,
    replacement: "42".into(),
    group: None,
    hint_only: false,
}
```

`Optimisation::hint_only` is the field that distinguishes a suggestion
the LSP can apply as an edit from one it can only describe — a pass falls
back to `hint_only: true` when it cannot compute a safe replacement span.
`group` links edits that must be applied together (see O125 below).

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

`PassId::CodeSinking` → `rust/tcl-compiler/src/optimiser/code_sinking.rs`:

1. **Sinkability check**: `Statement::AssignConst { name: "code", value:
   "403", .. }` is sinkable — a simple constant assignment with no command
   substitutions.
2. **Variable reference scan**: walks the `Statement::If` bodies
   recursively.  `$code` appears only in the `else` body
   (`HTTP::respond $code …`), not in the `if` body (which defines a new
   `code`).
3. **Deepest target**: the only use of the original `code` is in the
   `else` branch → the sink target is the else body.
4. **Emission**: a rewrite that prepends the sunk statement into the
   target body.

`tcl explore --show opt --text` on the source above:

```
=== opt ===
└── O125 Sink 'code' into branch — prepend in target body → set code 403; HTTP::respond $code content $msg
    · code: O125
    · message: Sink 'code' into branch — prepend in target body
    · replacement: set code 403; HTTP::respond $code content $msg
    · range: 7:5  (117…149)
```

Note the shape: this is **one** `Optimisation` whose `replacement`
rewrites the whole target statement, not a delete/insert pair sharing a
`group`.  The `group: Option<u32>` field exists for passes that genuinely
need paired edits (allocated by `PassContext::alloc_group`); code sinking
uses it only when it cannot express the move as a single span rewrite,
and falls back to `hint_only: true` when the original `set`'s span is
local-offset rather than file-absolute.

The `optimiserPasses` view shows which pass produced what, including the
O102 load-forwarding rewrite the propagation pass finds on the same
source:

```
=== optimiserPasses ===
├── Copy / constant propagation (1)
│   · pass: propagation
│   └── O102 Forward literal load of 'code' from its single reaching definition → 403
│       · range: 7:19  (131…136)
├── Constant-branch folding (0)
│   · pass: branch_folding
…
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

`rust/tcl-compiler/src/gvn.rs`.  Unlike the O1xx rewrites, GVN is a
separate analysis rather than a `PassId` in `run_passes`, so it does not
appear in the explorer's `optimiserPasses` breakdown:

1. **Purity check** (`gvn::is_pure_command` / `is_pure_with_procs`):
   looks up `HTTP::uri` in the command registry and checks its `Traits`
   for `PURE` and `CSE_CANDIDATE`, and its declared `EffectRegion` writes
   for `EffectRegion::NONE`.
2. **Value numbering**: `gvn::build_call_key` assigns a canonical
   `ExprKey` (a `Vec<String>` beginning `["call", command, …canonicalised
   args]`) to each computation.  Both `[HTTP::uri]` calls get the same
   key `["call", "HTTP::uri"]`.
3. **Dominance check**: the first occurrence (in the `if` condition)
   dominates the second (in the `log` command) — every path to the `log`
   statement passes through the `if` condition first.  The dominator tree
   comes from `SsaFunction::dominator_tree`.
4. **Kill check**: no barriers or mutating commands between the two
   occurrences invalidate the value.
5. **Emission**:

```rust
RedundantComputation {
    span: /* the second [HTTP::uri] */,
    first_span: /* the first [HTTP::uri] */,
    expression_text: "HTTP::uri".into(),
    code: DiagCode::O105,
    message: gvn::full_redundancy_message("HTTP::uri"),
}
```

`gvn` distinguishes three message shapes for the same finding —
`full_redundancy_message`, `partial_redundancy_message` (the value is
available on some but not all incoming paths), and
`loop_invariant_message` (the `O106` LICM case).  Which one is used is
what tells the reader whether extracting the value is unconditionally
safe.

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
    temp#1   = expr {$x * 2}     uses: {x#1}     defs: {temp#1}
    unused#1 = 99                uses: {}        defs: {unused#1}
    result#1 = expr {$temp + 1}  uses: {temp#1}  defs: {result#1}
    return $result               uses: {result#1}
```

**Liveness analysis:**
- `result#1` is live (read by `return`).
- `temp#1` is live (read by the `result` expression).
- `unused#1` is dead — never read by any statement.

### Elimination passes

`PassId::Elimination` → `rust/tcl-compiler/src/optimiser/elimination.rs`,
which covers four codes: O107 unreachable blocks, O108 ADCE, O109 dead
stores, and O126 unused-variable assignments.

On this source it emits O126 — the assignment defines a variable no one
ever reads:

```
=== opt ===
└── O126 Remove unused variable assignment → 
    · code: O126
    · message: Remove unused variable assignment
    · replacement: 
    · range: 3:5  (52…65)
```

Note the empty `replacement` — that is how a removal is expressed:
rewrite the statement's span to nothing.

**O109 — Dead Store Elimination** is the sibling case: a variable that
*is* read somewhere, but whose particular store is overwritten before any
read reaches it.  It comes from the `FunctionAnalysis::dead_stores`
findings (`DeadStore { block, statement_index, variable, version }`).

**O107 — unreachable code:**
If a block is unreachable (e.g. code after `return`), it is flagged:

```tcl
proc example {} {
    return 1
    set x 42    ;# unreachable — O107
}
```

**O108 — Aggressive DCE (ADCE):**
Tracks statement-level liveness backwards from live roots (return values,
side-effecting calls).  A statement is dead if its defined values are
never used **and** it has no side effects — the second half checked
against `FunctionExecutionIntent`, so a "dead" assignment whose
right-hand side is `[http::geturl …]` survives (see Example 23).

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

`PassId::PatternRecognition`
(`rust/tcl-compiler/src/optimiser/pattern_recognition.rs`) recognises the
`set` + `append` pattern and suggests combining into
`set result "Hello ${name}!"` (O104).  The same pass covers `O130`, the
`lappend` equivalent.

### O114 — Incr Idiom

Detects `set x [expr {$x + N}]` patterns and suggests `incr x N`,
which compiles to the specialised `incrStkImm` opcode:

```tcl
# Before
set count [expr {$count + 1}]

# After (O114)
incr count
```

The same `PassId::PatternRecognition` pass matches this: the `set` target
is the same variable used in the expression, and the expression is an
integer addition.  It also emits `O119`, the multi-set packing hint.

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

SCCP determines `debug#1 = const(0)` → the `if` condition is always
false → the body is unreachable.

`PassId::StructureElimination`
(`rust/tcl-compiler/src/optimiser/structure_elimination.rs`) replaces the
entire `if {$debug} { … }` block with nothing:

```
=== opt ===
└── O112 Eliminate dead if (all conditions are always false) → 
    · code: O112
    · message: Eliminate dead if (all conditions are always false)
    · replacement: 
    · range: 2:1  (12…62)
```

The `optimiserPasses` view shows the branch-folding pass also proving the
condition constant on the way (`O101 Fold constant expression → {0}`).
Applying O112 leaves:

```tcl
set debug 0
pool main_pool
```

The now-dead `set debug 0` is a separate finding from the elimination
pass (`O126` / `O109`) on the next round, not a grouped part of the O112
rewrite — running the optimiser at the `aggressive` profile, which
re-runs `full` to a fixpoint, is what collapses both in one go.

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
suggest `tailcall factorial …` (O121).  `PassId::TailCall`
(`rust/tcl-compiler/src/optimiser/tail_call.rs`) emits this, in both a
bare and a return-substitution variant, plus `O122` (the
loop-conversion hint) and `O123` (the accumulator-candidate hint).

### O126 — unused variable assignment

When a variable is assigned but the eliminated tail expression was its
only consumer, the assignment becomes unused and `O126` removes it.  That
code comes from `PassId::Elimination`, not from the tail-call pass — the
two cooperate across optimiser rounds rather than in one rewrite.

---

## Example 20: Error recovery — unclosed bracket

Demonstrates how the parser handles malformed input by inserting
zero-width **ghost tokens** so downstream passes receive clean commands.

### Source (malformed)

```tcl
set x [string length "hello"
set y 42
```

The `[` on line 1 is never closed — `string length "hello"` runs to the
end of the line without a matching `]`.

### Stage 0 — Error recovery

`segment_with_recovery(source, config, known)`
(`rust/tcl-compiler/src/segmenter.rs`) drives this.  It parses once
plainly, then runs
`analyser::syntax_checks::unterminated_bracket_diagnostics` over each
command.  That check owns the E201 heuristics: the next non-blank line
starts with `set`, a **known command**, which signals that `]` belongs at
the end of line 1.

`known` is a `RecoveryKnownCommands` — the active registry's names plus
every proc, class, and alias the document itself defines — so a break
just before a call to a user-defined proc recovers as readily as one
before a builtin.

Recovery is expressed as a **ghost map**, `BTreeMap<u32, u8>` from source
offset to the byte to pretend is there:

```rust
ghosts.insert(/* offset just past "hello" */, b']');
```

**Re-parse with the ghosts:**
`parsing::syntax::build::build_document_with_ghosts(source, config,
ghosts)` re-lexes the *unmodified* source with the ghost bytes applied,
and `segment::segments_from_document` derives the command list from the
resulting CST.  The loop repeats — a fresh diagnostic pass can reveal
another break — bounded at `MAX_GHOST_RECOVERY_PASSES` (32).  The second
parse yields two well-formed `SegmentedCommand` values:

```rust
// Command 1 (recovered):
SegmentedCommand { texts: vec!["set", "x", "[string length \"hello\"]"], .. }

// Command 2 (clean):
SegmentedCommand { texts: vec!["set", "y", "42"], .. }
```

Because the ghosts are applied at *lex* time rather than by editing the
source, every span downstream still points into the real file — the
recovered command's range covers what the user actually typed.

Both downstream stages (IR lowering, CFG, SSA, codegen) proceed on the
clean parse.  The `E201` diagnostic, with its "insert `]`" `CodeFix`, is
returned alongside the commands and published to the editor.

### The other recovery path — tail re-segmentation

When no ghost applies but a token still runs to EOF,
`segment_commands_with_recovery` takes over.  It:

1. Finds the suspicious EOF-reaching token (`find_suspicious_token`) — a
   `Cmd` token immediately, or a `Str` / `Esc` token that spans at least
   `RECOVERY_LINE_THRESHOLD` (3) lines.
2. Marks the last command `is_partial = true` and records
   `partial_delimiter: Some(UnclosedDelimiter::{Brace|Bracket|Quote})`,
   mapped from the token kind (`Str` → `Brace`, `Cmd` → `Bracket`,
   `Esc` → `Quote`).
3. Scans the swallowed text for the first line whose first word is a
   known command (`find_recovery_offset`), and re-segments the source
   from there as a fresh command stream.

`UnclosedDelimiter::missing_message()` supplies the `E200` text —
"missing close-brace", "missing close-bracket", or `missing "`.

### Error recovery diagnostics

| Code | Meaning |
|------|---------|
| E200 | Unterminated command — the parser could not tell where it ends (missing `]` / `"` / `}`) |
| E201 | Unterminated command substitution — missing close bracket `]` |
| E202 | Unterminated double-quoted string — missing closing `"` |
| E203 | Unterminated braced word — missing closing `}` |
| E204 | Extra characters after the close brace of a `${name}` reference |
| E205 | Extra characters after the close quote in a variable name |
| E206 | Missing close brace for a `${name}` variable reference |

E204–E206 are lexer warnings lifted by the analyser rather than
segmenter-driven recoveries.

---

## Example 21: Expression parsing — braced vs unbraced

Shows how the Pratt parser handles Tcl `expr` bodies and how braced vs
unbraced expressions differ.

### Braced expression: `expr {$a + $b * 2}`

The braces protect the expression from Tcl substitution — the content is
passed verbatim to the expression parser.

**Tokenisation** — `tcl_lexer::tokenise_expr(source, dialect)` produces a
`Vec<ExprToken>`, each `{ kind: ExprTokenType, text: String, start: u32,
… }`:

```
ExprToken { kind: Variable, text: "$a" }
ExprToken { kind: Operator, text: "+"  }
ExprToken { kind: Variable, text: "$b" }
ExprToken { kind: Operator, text: "*"  }
ExprToken { kind: Number,   text: "2"  }
```

`ExprTokenType` is `Number`, `String`, `Variable`, `Command`, `Operator`,
`ParenOpen`, `ParenClose`, `Comma`, `Function`, `Bool`, `TernaryQ`,
`TernaryC`, `Whitespace`, `Eof`.  The dialect argument is what admits the
iRules word operators — they lex as `Operator` only under the iRules
profile.

**Pratt parsing** — `tcl_syntax::expr::parser::parse_expr(source,
dialect)` (with `parse_expr_cached` for the memoised variant).  The
parser uses `binary_bp(op_text) -> Option<(u8, u8)>` binding powers,
where left-associative operators get `right_bp = left_bp + 1` and
right-associative ones get `right_bp = left_bp`:

- `*` has binding power `(22, 23)` — higher than `+` at `(20, 21)`.
- So `$b * 2` binds tighter than `$a + …`.

Result:

```rust
ExprNode::Binary {
    op: BinOp::Add,
    left: Box::new(ExprNode::Var { text: "$a".into(), name: "a".into(), .. }),
    right: Box::new(ExprNode::Binary {
        op: BinOp::Mul,
        left: Box::new(ExprNode::Var { text: "$b".into(), name: "b".into(), .. }),
        right: Box::new(ExprNode::Literal { text: "2".into(), .. }),
    }),
}
```

### Unbraced expression: `expr $a + $b * 2`

Without braces, Tcl performs variable substitution *before* the
expression is compiled.  The segmenter sees multiple tokens:

```
Var "a"   Esc "+"   Var "b"   …
```

These are concatenated into a single text `"${a} + ${b} * 2"`.
The expression parser receives a string with *already-substituted*
variable references, but since it cannot know the runtime values, it
falls back to:

```rust
ExprNode::Raw { text: "${a} + ${b} * 2".into() }
```

`ExprNode::Raw` is the universal fallback — `parse_expr` returns it on
*any* parse error, so the pipeline never panics on a malformed
expression.  Every consumer must treat `Raw` as "give up".  This is why
diagnostic **`W100`** ("unbraced expression argument") warns about the
pattern: braced expressions enable compile-time parsing, constant
folding, and type inference; unbraced ones also risk double
substitution, which is why `W100` escalates to an error when the
argument provably contains a substitution.

### iRules extensions

The Pratt parser handles the iRules word operators at the same binding
powers as their symbolic counterparts (verbatim from `binary_bp`):

| iRules operator | `BinOp` variant | Binding power |
|-----------------|-----------------|---------------|
| `contains`      | `Contains`      | (14, 15) |
| `starts_with`   | `StartsWith`    | (14, 15) |
| `ends_with`     | `EndsWith`      | (14, 15) |
| `equals`        | `StrEquals`     | (14, 15) |
| `matches_glob`  | `MatchesGlob`   | (14, 15) |
| `matches_regex` | `MatchesRegex`  | (14, 15) |
| `and`           | `WordAnd`       | (6, 7)   |
| `or`            | `WordOr`        | (4, 5)   |
| `not` (unary)   | `UnaryOp::WordNot` | — |

(14, 15) is the `== != eq ne` tier, so all six string operators sit
exactly where `eq` does; `and` / `or` sit exactly where `&&` / `||` do.

---

## Example 22: Lowering dispatch — `arg_roles` and command classification

Shows how `Lowerer::lower_command` in
`rust/tcl-compiler/src/lowering/mod.rs` dispatches each command to the
appropriate IR statement using registry metadata.  **There is no
`match cmd_name` ladder**: dispatch is on a typed `LoweringHookId` the
registry stamps onto the spec.

### Dispatch hierarchy

```
lower_command(cmd, namespace)
    │
    ├─ try_dispatch_structured_hook(cmd_name, seg, namespace)
    │     │
    │     ├─ registry.resolve_invocation(cmd_name, args, dialect)
    │     │     → canonical resolution, so an alias or a dialect-specific
    │     │       variant dispatches correctly
    │     │
    │     └─ match resolved.semantics.lowering_hook:
    │           LoweringHookId::If         → lower_if()         → Statement::If
    │           LoweringHookId::For        → lower_for()        → Statement::For
    │           LoweringHookId::While      → lower_while()      → Statement::While
    │           LoweringHookId::Foreach    → lower_foreach()    → Statement::Foreach
    │           LoweringHookId::Switch     → lower_switch()     → Statement::Switch
    │           LoweringHookId::Catch      → lower_catch()      → Statement::Catch
    │           LoweringHookId::Try        → lower_try()        → Statement::Try
    │           LoweringHookId::Set        → the `set` hook (table below)
    │           LoweringHookId::Incr       → Statement::Incr
    │           LoweringHookId::Proc       → lower_proc()       → a Procedure
    │           LoweringHookId::When       → an iRules event-handler Procedure
    │           LoweringHookId::NamespaceEval → a body unit
    │           LoweringHookId::Uplevel    → Statement::UpFrame, or fall through
    │           LoweringHookId::Eval       → Statement::Block,   or fall through
    │           LoweringHookId::Apply      → a lambda body unit + a barrier
    │           …                             (27 hook IDs in total)
    │
    └─ lower_default(seg, namespace)
          ├─ a body-role argument that could not be lowered → Statement::Barrier
          ├─ ArgRole::VarWrite positions                    → Statement::Call with defs
          └─ otherwise                                      → Statement::Call
```

Two properties fall out of dispatching on the hook ID rather than the
name.  A spec that aliases an existing form dispatches correctly the
moment its `lowering_hook` is stamped, with no walker change; and the
hook ID is the canonical key the audit, LSP, and compiler-explorer
surfaces consume.

**Fall-through is the safety mechanism.**  Hooks whose form has a shape
precondition (`Proc`, `NamespaceEval`, `Foreach`, `Lmap`, `Dict`, `When`,
`ForeachLine`) return `None` when the precondition fails, so
`lower_default` catches the call rather than the specialised lowerer
crashing on it.  `Uplevel` and `Eval` do the same by value: only a
brace-literal body is specialised, and a dynamic body
(`uplevel 1 $body`, `eval [cmd]`) falls back to the generic
`Call` / `Barrier` that carries the unresolved arguments.

### Example: the `set` lowering hook

`set` carries `lowering_hook: Some(LoweringHookId::Set)`.  The hook
pattern-matches on the second argument's token kind:

| Token kind of `args[1]` | Statement produced | Example |
|-------------------------|--------------------|---------|
| `Str` (braced string) | `AssignConst` | `set x {hello}` |
| `Esc` (decimal integer) | `AssignConst` | `set x 42` |
| `Cmd` wrapping `expr` | `AssignExpr` | `set x [expr {$a + 1}]` |
| `Var` or interpolated | `AssignValue` | `set x $y`, `set x "hi $name"` |
| 0 args (getter) | `Call` | `set x` (read variable) |

A braced name word also sets `name_braced: true` on the produced
statement, which is what stops `set {a($x)} v` from having its key
substituted.

### Example: fallthrough with `arg_roles`

For a command like `regexp`:

```tcl
regexp {(\d+)} $input match submatch
```

The registry declares `ArgRole::VarWrite` at arg indices 2 and 3 (the
match variables).  `lower_default` reads those positions off the resolved
spec and produces:

```rust
Statement::Call {
    span: /* … */,
    command: "regexp".into(),
    canonical_command: Some("::regexp".into()),
    args: vec![r"(\d+)".into(), "${input}".into(), "match".into(), "submatch".into()],
    defs: vec!["match".into(), "submatch".into()],   // SSA tracks these as definitions
    reads: vec![],
    reads_own_defs: false,
    safe_on_uninit: false,
    tokens: /* … */,
    foreach_groups: None,
}
```

The `defs` vector tells the SSA builder that `regexp` defines `match` and
`submatch`, so they get new SSA versions.  Nothing in the lowerer knows
the name `regexp` — the whole behaviour is the spec's `arg_roles`.

### Example: barrier commands

A command whose effects cannot be modelled — a dynamic `eval` /
`uplevel` / `upvar` — produces `Statement::Barrier`:

```tcl
eval $script
```

```rust
Statement::Barrier {
    span: /* … */,
    reason: /* human-readable, e.g. "dynamic command" */,
    command: "eval".into(),
    canonical_command: Some("::eval".into()),
    args: vec!["${script}".into()],
    tokens: /* … */,
}
```

`Barrier` tells all downstream passes: *stop reasoning about variable
state here* — the command can read or write any variable, define new
procedures, or modify the call stack.  Note that this is the *fallback*:
`eval {literal body}` is instead lowered to `Statement::Block`, whose
body is analysed normally, and `uplevel 1 {literal body}` to
`Statement::UpFrame`.  The barrier is only reached when the body is
genuinely unknowable.

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

`build_function_execution_intent()` in
`rust/tcl-compiler/src/execution_intent.rs` walks each
`Statement::AssignValue` in the CFG, parses the command substitution, and
classifies it through `side_effects::classify_side_effects`:

**`[llength $items]`:**

```rust
CommandSubstitutionIntent {
    command: "llength".into(),
    args: vec!["$items".into()],
    arg_categories: vec![SubstitutionCategory::ScalarVar],
    side_effect: SideEffectClass::Pure,      // llength is pure
    escape: EscapeClass::NoEscape,           // no dynamic barriers
    shimmer_pressure: 1,                     // one var arg
    invocation_shape: InvocationShape::CommandSubstitution,
}
```

**`[format "Total: %d" $count]`:**

```rust
CommandSubstitutionIntent {
    command: "format".into(),
    args: vec!["\"Total: %d\"".into(), "$count".into()],
    arg_categories: vec![SubstitutionCategory::Literal, SubstitutionCategory::ScalarVar],
    side_effect: SideEffectClass::Pure,
    escape: EscapeClass::NoEscape,
    shimmer_pressure: 1,
    invocation_shape: InvocationShape::CommandSubstitution,
}
```

**`[http::geturl $url]`:**

```rust
CommandSubstitutionIntent {
    command: "http::geturl".into(),
    args: vec!["$url".into()],
    arg_categories: vec![SubstitutionCategory::ScalarVar],
    side_effect: SideEffectClass::MaySideEffect,  // network I/O
    escape: EscapeClass::MayEscape,               // may throw
    shimmer_pressure: 1,
    invocation_shape: InvocationShape::CommandSubstitution,
}
```

`SubstitutionCategory` is `Literal`, `ScalarVar`, `ArrayVar`,
`NestedCommand`, or `Mixed`; `SideEffectClass` is `Pure` or
`MaySideEffect`; `EscapeClass` is `NoEscape` or `MayEscape`.  All three
are deliberately two- or five-valued: the point is a cheap conservative
tag, not a second effect model.

The results are collected into a
`FunctionExecutionIntent { command_substitutions: HashMap<StatementKey,
CommandSubstitutionIntent> }`, keyed by `StatementKey = (String, usize)`
— the block name and statement index — and looked up with
`FunctionExecutionIntent::intent_for(block, stmt_idx)`.

### How ADCE uses execution intent

The elimination pass checks the intent before removing a "dead"
assignment:

- `[llength $items]` → `Pure` + `NoEscape` → **safe to remove** if
  `count` is never read.
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

### Phase 1 — Local facts

`build_interprocedural_analysis(ir_module, registry, dialect,
object_types, …)` in `rust/tcl-compiler/src/interprocedural.rs` walks the
IR and seeds one `ProcSummary` per procedure with purely *local* facts —
what the body itself does, before any callee is considered.  There is no
separate `ProcLocalSummary` type in Rust; the same `ProcSummary` struct
is filled in place and then refined.

For `::helper`, the local seed is: no internal proc calls, no barrier, no
global writes, and a return value that depends on the parameter `x`.
For `::main`: `calls` contains `::helper`, and `puts` contributes a
log-output effect write.

Effects are compared as a coarse bitset rather than the full structured
model — `EffectRegion` in
`rust/tcl-compiler/src/side_effects.rs`:

```rust
bitflags! {
    pub struct EffectRegion: u32 {
        /// No region.
        const NONE               = 0;
        /// Any HTTP state (header, body, status, URI, cookie, method, HTTP/2).
        const HTTP_STATE         = 1 << 0;
        /// Response lifecycle (commit / redirect / respond).
        const RESPONSE_LIFECYCLE = 1 << 1;
        /// Global or namespace-scoped variable state.
        const GLOBAL_STATE       = 1 << 2;
        /// Catch-all for unknown effects.
        const UNKNOWN_STATE      = 1 << 3;
    }
}
```

Four bits, deliberately.  The structured `SideEffectTarget` model is the
authoritative one; `EffectRegion` exists purely so GVN and the
interprocedural solver can do kill checks with a bitwise AND.
`target_to_region(target, scope)` is the projection.

### Phase 2 — Transitive closure

The solver iterates over the call graph to propagate effects:

1. `::helper` has no callees → its summary is final.
2. `::main` calls `::helper`:
   - `::helper` is pure → no additional effect reads/writes propagated.
   - `::main` calls `puts` → an effect write.
   - `::main` is therefore NOT pure.

`ProcSummary` keeps `calls` (the transitive set) and `direct_calls`
(the immediate ones) separately, so a consumer can ask either question.

### Phase 3 — Constant folding eligibility

`::helper` meets the criteria for `can_fold_static_calls`:
- No barrier (`has_barrier: false`)
- No unknown calls (`has_unknown_calls: false`)
- No global writes (`writes_global: false`)
- Return depends only on parameters
- Body is a single expression

When the optimiser encounters `[helper 21]` with a constant argument, it
evaluates the body with `x#1 = 21` → `21 * 2` = `42`, and emits **O103**.

### Final `ProcSummary`

```rust
ProcSummary {
    qualified_name: "::helper".into(),
    params: vec!["x".into()],
    arity: Arity::exact(1),
    calls: vec![],
    direct_calls: vec![],
    has_barrier: false,
    has_unknown_calls: false,
    writes_global: false,
    pure: true,
    effect_reads: EffectRegion::NONE,
    effect_writes: EffectRegion::NONE,
    returns_constant: false,
    constant_return: None,
    return_depends_on_params: vec!["x".into()],
    return_passthrough_param: None,
    can_fold_static_calls: true,
    param_traits: /* HashMap<String, HashSet<ProcArgTrait>> */,
}
```

`constant_return` is an `Option<ConstantReturn>` —
`Int(i64)` / `Float(f64)` / `Bool(bool)` / `Str(String)` — so a proc that
*always* returns the same value can be folded without evaluating its body
at all.  `param_traits` records what each parameter is used *as* (a
`HashSet<ProcArgTrait>` per name), which is what lets a caller's argument
be type-checked against the callee.

TclOO methods get a `MethodSummary` instead, which wraps a `ProcSummary`
as `base` and adds `class_name`, `method_kind`, the instance variables
read and written, `calls_my`, and `calls_next`.

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

Without connection-scope analysis, the compiler would emit false
positives: **`W210`** (variable read before set) for `$request_count` and
`$conn_start` in `HTTP_REQUEST`, and **`W211`** (variable set but never
used) for both in `CLIENT_ACCEPTED`.

### EventVarSummary construction

`extract_event_summary(event, function_unit)` in
`rust/tcl-compiler/src/connection_scope.rs` walks each event's SSA
blocks:

**CLIENT_ACCEPTED:**

```rust
EventVarSummary {
    event: "CLIENT_ACCEPTED".into(),
    defs: HashSet::from(["conn_start".into(), "request_count".into()]),
    uses_before_def: HashSet::new(),   // no version-0 reads
    unsets: HashSet::new(),
}
```

**HTTP_REQUEST:**

```rust
EventVarSummary {
    event: "HTTP_REQUEST".into(),
    defs: HashSet::from(["request_count".into()]),   // incr defines it
    uses_before_def: HashSet::from([
        "request_count".into(),
        "conn_start".into(),
    ]),                                              // version 0
    unsets: HashSet::new(),
}
```

### Cross-event set computation

`build_connection_scope(when_procedures, …)` takes the subset of
`CompilationUnit::procedures` whose qualified names start with
`::when::` and compares events:

- `CLIENT_ACCEPTED` defines `{conn_start, request_count}`.
- `HTTP_REQUEST` uses-before-def `{request_count, conn_start}`.
- Intersection: `{conn_start, request_count}` — these flow across events.

```rust
ConnectionScope {
    summaries: /* one EventVarSummary per event */,
    cross_event_defs: HashSet::from(["conn_start".into(), "request_count".into()]),
    cross_event_imports: HashSet::from(["conn_start".into(), "request_count".into()]),
    racy_static_defs: HashSet::new(),
}
```

The two cross-event sets are deliberately separate, because they suppress
different diagnostics from opposite ends: `cross_event_defs` is the
**producer** side (suppressing dead-store / unused-variable findings in
`CLIENT_ACCEPTED`), `cross_event_imports` the **consumer** side
(suppressing `W210` in `HTTP_REQUEST`).

`racy_static_defs` is the fourth field with no counterpart in the
suppression story: `static::` variables written outside `RULE_INIT` and
read in another event, which *raise* a finding — **`IRULE4005`** — rather
than suppressing one.

### Effect on diagnostics

The result is cached on `CompilationUnit::connection_scope` and consumed
by the optimiser's `PassContext` and the analyser's variable checks.
Dead-store elimination (`O109`), unused-variable (`W211`), and
read-before-set (`W210`) all consult the cross-event sets before
reporting — suppressing the false positives for `conn_start` and
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

`tcl_syntax::naming` (`rust/tcl-syntax/src/naming.rs`) provides the
canonical form.  From its own doc examples:

```rust
assert_eq!(normalise_qualified_name("foo"),      "::foo");
assert_eq!(normalise_qualified_name("ns::bar"),  "::ns::bar");
assert_eq!(normalise_qualified_name("::baz"),    "::baz");
assert_eq!(normalise_qualified_name(""),         "");
assert_eq!(normalise_qualified_name("::::x"),    "::x");   // colon runs collapse
assert_eq!(normalise_qualified_name("::"),       "::");
```

Its sibling `qualify(prefix, name)` is the join used everywhere a
namespace prefix meets a written name, and it encodes the rule that
matters: an **absolute** `name` ignores the prefix entirely —
`qualify("::ns", "::other::C")` is `::other::C`, never re-prefixed.

### How namespace context propagates through lowering

Every lowering entry point carries a `namespace: &str` parameter tracking
the current namespace:

1. Top level: `namespace = "::"`.
2. Inside `namespace eval mylib { … }`: the `LoweringHookId::NamespaceEval`
   hook computes `::mylib` and threads it into body lowering; the body is
   also recorded as a *body unit* so analysis reaches inside it.
3. `proc helper` inside `::mylib` → `Procedure { name: "helper",
   qualified_name: "::mylib::helper", namespace_scoped: true, .. }`.
4. `proc compute` inside `::mylib` → `::mylib::compute`.

`Statement::Block` also carries the fully-qualified `namespace` its body
was lowered in, so an inlined `eval` body still resolves bare calls
against the right namespace.

### How call resolution works

Inside `::mylib::compute`, the call `helper $a` is unqualified.  There is
**one canonical algorithm** for resolving it, shared by the analyser, the
optimiser, the bytecode VM, and the WASM runtime — see
[command-resolution.md](contracts/command-resolution.md) for the full
contract and its conformance-vector gate.  The helpers live in
`tcl_syntax::naming`:

| Helper | Role |
|---|---|
| `command_resolution_candidates(ns, path, name)` | the candidate list in priority order |
| `bareword_resolution_candidates(ns, name)` | the path-free wrapper |
| `resolve_command_with(ns, path, name, exists)` | the full rule: first candidate for which `exists` is true |

For `helper` from `::mylib::compute`, the candidates are `::mylib::helper`
then `::helper`, and the first that *exists as a command* wins.  The
optimiser's `resolve_internal_call` is a thin wrapper over
`resolve_command_with` against the unit's proc table.

The rule is call-time, not lexical: a candidate defined later in the file
still wins, which is why the analyser settles call sites in a
post-walk pass rather than as it goes.

### Resulting IR module

```rust
Module {
    procedures: HashMap::from([
        ("::mylib::helper".into(),  Procedure { name: "helper".into(),  .. }),
        ("::mylib::compute".into(), Procedure { name: "compute".into(), .. }),
    ]),
    ..
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

The emitter builds a `LocalVarTable` from the parameter list, so the
proc's LVT is:

```
  Local variables:
    %v0: "n"
```

Inside a `proc`, all variable accesses use LVT-indexed instructions
(`loadScalar1 %v0`) instead of name-based stack operations (`loadStk`).

### Step 2 — Block linearisation (`linearise()`)

`codegen::emitter::ordering::linearise(cfg)` performs a DFS traversal
from the entry block, producing a reverse post-order that determines
instruction layout.  The true branch is placed immediately after the
condition, so `jumpFalse` skips *forward* to the else block — matching
tclsh's fall-through layout.

For loops, `ordering::reorder_bottom_tested(cfg, order)` detects
back-edges and moves the loop body *before* the header, producing a
condition-at-bottom layout:

```
Before (top-tested):    header → body → jump header
After  (bottom-tested): jump header → body → header (jumpTrue body)
```

### Step 3 — Instruction emission with labels

As the emitter walks blocks in that order, it places labels and emits
instructions.  Jump targets start as `Operand::Label(String)` — the block
name — because the byte offset is not known until layout:

```
label "entry_1"
  LOAD_SCALAR1 %v0        # load n
  PUSH1 <literal "0">
  LT                      # n < 0
  JUMP_FALSE4 Label("if_next_4")

label "if_then_3"
  LOAD_SCALAR1 %v0
  UMINUS
  JUMP4 Label("if_end_2")

label "if_next_4"
  …                       # the else body

label "if_end_2"
  DONE
```

### Step 4 — Jump size optimisation (`optimise_jumps()`)

`tcl_bytecode::layout::optimise_jumps(instrs, labels, max_iters)`,
called with `max_iters = 10`, replaces 4-byte jumps with 1-byte jumps
when the relative offset fits in a signed byte.  It walks a
`&[(Op, Op)]` table of wide→narrow pairs (`JUMP4` → `JUMP1`,
`JUMP_TRUE4` → `JUMP_TRUE1`, `JUMP_FALSE4` → `JUMP_FALSE1`, …).

Shortening jumps changes instruction sizes, which changes offsets,
which may enable more shortenings — hence the bounded iteration.

### Step 5 — Label resolution (`resolve_layout()`)

`tcl_bytecode::layout::resolve_layout(instrs, labels) ->
HashMap<String, usize>` walks the instruction list assigning each
`Instruction::offset` in turn, and returns the label→byte-offset map.
`Operand::Label` operands are then patched to the relative byte offsets.

### Step 6 — Peephole optimisation

`rust/tcl-compiler/src/codegen/peephole.rs` applies tclsh-matching
rewrites:

1. **`remove_trailing_pop()`** — the last statement's result stays on the
   stack for `done` to return.  Strip `pop; done` → `done`.
2. **`fold_const_push_pop_nops()`** — dead constant results (`push; pop`
   pairs from folded branches) become `nop` runs, matching tclsh's
   folded-constant pattern.
3. **`dedup_push_literals()`** — after nop-folding, surviving `push`
   instructions may reference duplicate literal slots; deduplicate to
   match tclsh's literal-table interning.
4. **`fold_tail_return_to_done()`**, **`strip_unused_start_cmd()`**,
   **`fixup_top_level_start_cmd()`**, **`strip_nodedup_tags()`** — the
   remaining tclsh-parity fixups.

### Step 7 — Literal table construction

`LiteralTable::intern(value)` returns an existing slot for a repeated
string and allocates a new one otherwise, so `"n"` referenced twice gets
one slot.  (`LiteralTable::register` is the deliberate escape hatch: it
always allocates, for the cases where tclsh itself emits a duplicate
slot.)

### Final bytecode

```
::abs
  ByteCode ::abs, 13 instructions, 37 bytes, 3 literals, 1 variables
    Literals:
      0: "0"
      1: "set"
      2: "n"
    Local variables:
      %v0: "n"
    Instructions:
    # entry_1:
      (0)  loadScalar1 %v0    # var "n"
      (2)  push1 0            # "0"
      (4)  lt
      (5)  jumpFalse1 +16     # pc 21
    # if_then_3:
      (7)  startCommand +12 1 # next cmd at pc 19, 1 cmds start here
      (16) loadScalar1 %v0    # var "n"
      (18) uminus
    # cmd_end_0:
      (19) jump1 +17          # pc 36
    # if_next_4:
      (21) startCommand +15 1 # next cmd at pc 36, 1 cmds start here
      (30) push1 1            # "set"
      (32) push1 2            # "n"
      (34) invokeStk1 2       # set
    # cmd_end_1:
    # exit_5:
    # if_end_2:
      (36) done
```

Two things this shows that a hand-written sketch would not.  The
`startCommand` instructions carry the per-command error context tclsh
uses to build `errorInfo` — they are why the offsets jump by more than
the visible operands account for.  And the `else` body `set n` is *not*
specialised: a one-argument `set` is a variable **read**, and the
compiler emits a generic `invokeStk1` for it rather than an LVT load,
because the read must go through the same name resolution `set` itself
would use.

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

`classify_side_effects(registry, command, args, dialect,
callee_summary)` in `rust/tcl-compiler/src/side_effects.rs`.  Each
command's *declared* effect comes straight from its spec; the classifier
then fills in the per-call fields (`scope`, `storage_type`, `key`,
`subtable`, `namespace`) the declaration cannot know.

**`HTTP::uri`** — declared on the spec as:

```rust
side_effects: &[SideEffect {
    target: SideEffectTarget::HttpUri,
    reads: true,
    writes: false,
    connection_side: ConnectionSide::Both,
    dialects: None,
}],
taint_source: Some(TaintColour::TAINTED.union(TaintColour::PATH_PREFIXED)),
```

Classified, it becomes a `CommandSideEffects` with that one read effect,
`pure: true` (reading is side-effect-free) and `deterministic: true`
(the same result within one event).  Note the `taint_source` colours:
`HTTP::uri` is tainted *but* provably starts with `/`, which is what lets
a path-prefix sink accept it.

**`HTTP::header replace Host "example.com"`** — the parent spec declares
a read/write `SideEffectTarget::HttpHeader`; the `replace` subcommand
narrows it:

```rust
SubCommand {
    name: "replace",
    arity: Arity::new(1, 2),
    detail: "Replace header value.",
    synopsis: "HTTP::header replace <name> ?<string>?",
    mutator: true,
    credential_arg: Some(2),
    sensitive_headers: &[
        "authorization", "proxy-authorization",
        "x-api-key", "x-auth-token", "x-secret",
    ],
    ..SubCommand::DEFAULT
}
```

`mutator: true` makes the classified result `pure: false`, and the
classifier binds `key: Some("Host")` from the literal argument.
`credential_arg` / `sensitive_headers` are the credential-exposure facts
that only make sense on the writing subcommand — they are what let a
check warn about `HTTP::header replace Authorization $token` without the
checker knowing any header names itself.

**`pool my_pool`:**

```rust
side_effects: &[SideEffect {
    target: SideEffectTarget::PoolSelection,
    reads: false,
    writes: true,
    connection_side: ConnectionSide::Server,
    dialects: None,
}],
```

**`log local0. "Routing $uri"`:**

```rust
side_effects: &[SideEffect {
    target: SideEffectTarget::LogIo,
    reads: false,
    writes: true,
    connection_side: ConnectionSide::Both,
    dialects: None,
}],
```

Both classify to `pure: false`, `deterministic: false`.

### How classification resolves form and subcommand

`classify_side_effects` follows this order:

1. **Interprocedural summary** — if `callee_summary` is `Some` (a
   user-defined proc), classify from it and return.
2. **Unknown command** — fall back to a conservative unknown-write.
3. **Subcommand resolution** — for `HTTP::header replace`: look up the
   `CommandSpec`, find the `SubCommand` named `replace`, take its
   `mutator` flag and its `side_effects` (falling back to the parent's).
4. **Command level** — for `HTTP::uri` with no arguments, use the
   command's own `side_effects` and its `Traits` purity.
5. **Argument binding** — bind literal values into `key` / `subtable`
   where the spec identifies which argument names the resource.

There is no separate "resolve the `FormSpec` by arity" step: a Rust
`FormSpec` carries no arity or effects, so getter/setter distinctions
that matter to analysis live on the subcommand or on
`CommandSpec::command_forms`.

### How consumers use side effects

| Consumer | Uses |
|----------|------|
| **GVN/CSE** | pure + `EffectRegion::NONE` writes → result can be cached (O105) |
| **ADCE** | `SideEffectClass::Pure` + `EscapeClass::NoEscape` → statement is removable |
| **Optimiser** | impure → cannot propagate across this command |
| **iRules flow** | `EffectRegion::RESPONSE_LIFECYCLE` write → response-commit tracking (IRULE1201/1202) |
| **Taint engine** | pure → taint flows through unchanged |

---

## Optimisation opportunities across examples

The following table summarises all optimisation passes the compiler can
detect, their triggers, and example patterns:

The authoritative inventory is the generated
[`docs/generated/optimisation_codes.md`](../generated/optimisation_codes.md),
which also records each code's category and which profiles enable it.

| Code | Name | Trigger | Example |
|------|------|---------|---------|
| O100 | Constant propagation | Variable has a known constant value | `set x 5; puts $x` → propagate `"5"` into `puts` |
| O101 | Fold constant expression | All `expr` operands are constants | `expr {2 + 3}` → `5` |
| O102 | Load forwarding | A variable has a single reaching literal definition | forward that literal to each use site |
| O103 | Interprocedural constant fold (ICIP) | Pure proc called with all-constant args | `[double 21]` → `42` (when `proc double {n} { expr {$n * 2} }`) |
| O104 | String build chain | Static `set` + `append` sequence detected | `set s ""; append s "a"; append s "b"` → `set s "ab"` |
| O105 | GVN/CSE redundancy | Same pure computation appears twice | `[HTTP::uri]` used twice → extract to variable |
| O106 | Loop-invariant hoisting (LICM) | A computation inside a loop does not vary per iteration | hoist it above the loop |
| O107 | Unreachable-code elimination | A block is unreachable | code after `return` is dead |
| O108 | Aggressive DCE (ADCE) | Statement result never used, no side effects | Pure expression whose value is discarded |
| O109 | Dead store elimination (DSE) | A store is overwritten before any read reaches it | the first `set x` of two in a row |
| O110 | Instruction combine (InstCombine) | Algebraic simplification opportunity | `expr {$x * 1}` → `expr {$x}` |
| O111 | Brace-expression performance hint | Unbraced `expr` body (paired with W100) | `expr $a + $b` → `expr {$a + $b}` |
| O112 | Constant condition (SCCP structure elimination) | Branch condition is compile-time constant | `if {1} {...}` → body only |
| O113 | Strength reduction | Power/modulo with small constants | `expr {$x ** 2}` → `expr {$x * $x}`; `$x % 8` → `$x & 7` |
| O114 | Incr idiom | `set x [expr {$x + N}]` pattern | → `incr x N` (specialised `incrStkImm` opcode) |
| O115 | Nested expr unwrap | `expr {expr {…}}` double wrapping | `expr {expr {$a + $b}}` → `expr {$a + $b}` |
| O116 | List folding | `[list a b c]` with all-constant args | `[list a b c]` → `a b c` |
| O117 | String length zero-check | `[string length $s] == 0` | → `$s eq ""` (avoids length computation) |
| O118 | Lindex folding | `[lindex {a b c} N]` with constant list and index | `[lindex {a b c} 1]` → `b` |
| O119 | Multi-set packing | Consecutive `set` literals | pack into `lassign` / `foreach` |
| O120 | String compare eq/ne | `==`/`!=` on string-typed operands | `expr {$s == "foo"}` → `expr {$s eq "foo"}` |
| O121 | Tail-call detection | Self-recursive call in tail position | → suggest `tailcall` for TCO |
| O122 | Tail-recursion to loop | Fully tail-recursive proc | → rewrite as iterative `while` loop |
| O123 | Accumulator introduction | Non-tail recursion with associative op | → introduce accumulator parameter (hint only) |
| O124 | Unused proc elimination | Proc defined but never called | Comment out unused `proc` (iRules only) |
| O125 | Code sinking (LCP) | Assignment used only in one branch | Move `set` into the deepest decision block that uses it |
| O126 | Unused variable assignment | A variable is assigned but never read | Remove the `set` |
| O127 | Single-use inline | A variable is assigned then read exactly once | Fold the `set` into its use site |
| O128 | End-relative index | `[expr {[llength $L] - N}]` used as an index | → `end-(N-1)` |
| O129 | Pure-builtin fold | A pure builtin substitution with constant args | `[string length "abc"]` → `3` |
| O130 | Lappend build chain | Static `lappend` chain | fold into a single assignment |

**Profiles.**  `off` disables everything; `readability`, `standard`, and
`full` enable progressively more passes in a single pass; `aggressive` is
`full` re-run to a fixpoint (up to 5 iterations).  The default editor
profile is `readability`; explicit actions (CLI, chat, MCP) default to
`full`.

---

## How diagnostics are calculated

The native LSP server (`rust/tcl-lsp-server`) publishes diagnostics in
**two tiers**, and — this is the part that most often surprises — the
fast tier is usually never sent at all.

The deep pass is started first and **raced against a 40 ms budget**
(`DIAGNOSTICS_FAST_TIER_BUDGET`).  If the whole pipeline finishes inside
that budget, the deep publish is the one and only publish, so a small or
warm file costs a single round-trip.  Only when the budget elapses with
the deep pass still running does the server publish the reduced fast
tier, then supersede it when the deep pass lands.  A size floor gates
this too: a document below a minimum line count never gets a fast tier,
because its wall-clock is dominated by one-time warm-up rather than
per-file work.

Everything is salsa-memoised and currency-guarded by document version, so
a superseding edit can never let a stale tier land after a fresh one.

### Tier 1 — the fast tier

`publish_fast_tier` filters the per-file analyser diagnostics through
`is_fast_tier(code)`, which is defined as **`!code.refined_by_workspace()`**
— and `refined_by_workspace` is exactly `{W120, W123}`:

```rust
const fn is_fast_tier(code: DiagCode) -> bool {
    !code.refined_by_workspace()
}
```

That is the whole rule.  The fast tier is not "cheap checks only"; it is
"every finding a later workspace pass cannot **retract**".  `W120`
(command used without a `package require`) and `W123` (unresolved
command) are held back precisely because a cross-file pass can discover
the definition and withdraw them — showing them early would flicker.

Alongside those, the fast tier carries the pure source-style lints
(`lift_source_style_diagnostics`: `W111` line length, `W112` trailing
whitespace, `W115` backslash-newline in a comment, `W118` inconsistent
line endings, and the decode-report findings `W107`/`W109`).  Both halves
are lifted off the event loop with `spawn_blocking`.

The fast tier is delivered **push-only** and never primes the pull-diagnostics
cache, so a pull-mode client is never served the reduced set.

### Tier 2 — the deep tier

`run_deep_diagnostics` runs **three independent whole-file analyses
concurrently** and joins them:

```
tokio::join!(
    base,                       // the per-file analyser walk (a Shared future,
                                //   the same one the fast tier awaited)
    compute_compiler_diags(…),  // compiler / optimiser checks, via salsa
    compute_project_diags(…),   // cross-file / project resolution
)
        │
        ▼
  W120 / W123 workspace refinement
  (source-graph inheritance + cross-file call settlement)
        │
        ▼
  diagnostic lifts → one authoritative, currency-guarded publish
```

`compute_compiler_diags` calls the salsa query
`tcl_lsp_db::compiler_check_diagnostics`, which returns a
`CompilerDiagnostics { checks, optimisations }` built over the memoised
`compilation_unit` and `proc_taint_solve` queries.  An unchanged
procedure contributes neither a re-solve nor a re-check.

What the deep tier adds over the fast one:

| Source | Codes |
|---|---|
| Optimiser (`run_passes`) | `O100`–`O130` |
| GVN / CSE | `O105` redundant pure computation, `O106` loop-invariant (LICM) |
| Shimmer detector | `S100`–`S103`, `S110` |
| Taint engine | `T100`–`T106`; `IRULE3001`–`3004`, `IRULE3101`–`3103` |
| iRules flow checks | `IRULE1005`–`IRULE1008` (collect / release / payload pairing), `IRULE1201`/`1202` (respond-then-use) |
| iRules variable/style checks | `IRULE4001`–`4005`, `IRULE5001`–`5007`, `IRULE6001` |
| Workspace refinement | the withheld `W120` / `W123`, now settled |

The concurrency has one deliberate cost: fail-fast on a base-analysis
cancellation is given up — the compiler and cross-file passes may do a
little wasted work before observing the same cancellation.

**Degradation is explicit.**  A salsa *cancellation* returns "not
settled", so the next edit retries.  A deterministic worker *panic* in a
secondary pass degrades that pass to its empty fallback and **still
publishes** the deep tier — because the fast tier may already have
replaced the client's complete set with its reduced subset, and returning
early would strand that reduced set as the terminal state.

### Suppression with `# noqa`

Any diagnostic can be suppressed with an inline `# noqa` comment:

```tcl
set x 42    ;# noqa: O109  — suppress dead store warning
eval $cmd   ;# noqa: *     — suppress ALL warnings on this line
```

The suppression map is `AnalysisResult::suppressed_lines:
HashMap<i32, HashSet<String>>`, built during semantic analysis
(`parse_noqa_line_suppressions` plus `apply_preceding_noqa` for
directives in a preceding comment block) and checked by both tiers before
emitting any diagnostic.  `# noqa: *` suppresses all codes;
`# noqa: O109` suppresses only the named code.

### Grouped optimisations

When the optimiser produces related edits (e.g. O100 propagates a constant
and O109 removes the now-dead store), they share an
`Optimisation::group: Option<u32>` allocated by
`PassContext::alloc_group`.  The publisher emits one primary diagnostic
with the others as related information:

```
Primary: O100 "Propagate constant into expression" (+1 dead store eliminated)
  └─ Related: O109 "Dead store: x is set but never read"
```

The LSP client receives a single code action that applies all grouped
edits atomically, keeping the source consistent.

### End-to-end diagnostic flow for Example 12

For the taint example (`HTTP::header value Host` → `HTTP::respond`):

1. The deep pass starts immediately; the per-file analyser walk finds no
   syntax errors, so if the 40 ms budget elapses first, the fast tier
   publishes an empty set.
2. **Deep tier**, running concurrently:
   - **Optimiser**: no rewrites (the code is already efficient).
   - **Taint engine**: the memoised `proc_taint_solve` propagates taint
     from `HTTP::header`'s `taint_source` through `string tolower`
     (which declares no `taint_transform`, so no mitigating colour is
     added) to `HTTP::respond`, whose `taint_output_sink` is
     `IRULE3001`.
   - **Cross-file**: nothing to settle.
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
  Token stream         SegmentedCommand    Statement::AssignConst    Instruction
  ┌──────────┐        ┌──────────────┐        ┌───────────┐        ┌───────────┐
  │ kind:Esc │   ──►  │ texts:       │  ──►   │ name:"x"  │  ──►   │ op:PUSH1  │
  │ span:0..3│        │  ["set",     │        │ value:"42"│        │ operands: │
  │ content_ │        │   "x","42"]  │        │ span:0..8 │        │  [Imm(0)] │
  │  offset:0│        │ single:      │        └───────────┘        └───────────┘
  └──────────┘        │  [T, T, T]   │              │                    ▲
                      └──────────────┘              │                    │
                                                    ▼                    │
                                              cfg::Block          FunctionAsm
                                              ┌──────────┐       ┌───────────┐
                                              │ stmts:   │       │ literals: │
                                              │  [Assign]│       │  LitTable │
                                              │ term:    │  ──►  │ lvt:      │
                                              │  Goto    │       │  LVTTable │
                                              └──────────┘       │ instrs:   │
                                                    │            │  [Instr]  │
                                                    ▼            └───────────┘
                                              SsaBlock                ▲
                                              ┌──────────┐           │
                                              │ phis: [] │           │
                                              │ stmts:   │     codegen_module()
                                              │  SsaStmt │  ────────┘
                                              │ defs:    │
                                              │  {x → 1} │
                                              └──────────┘
```

Each stage transforms the data into a richer representation:

1. **Tokens** — flat byte-level classification, spans only
   (`rust/tcl-lexer/src/tokens.rs`)
2. **SegmentedCommand** — word-level grouping with command boundaries
   (`rust/tcl-compiler/src/segmenter.rs`)
3. **`ir::Statement`** — typed, structured command semantics
   (`rust/tcl-compiler/src/ir.rs`)
4. **`cfg::Block`** — explicit control flow with terminators
   (`rust/tcl-compiler/src/cfg.rs`)
5. **SSA** — variable versioning with phi nodes at merge points
   (`rust/tcl-compiler/src/ssa.rs`)
6. **`FunctionAnalysis`** — constant values, types, liveness, dead stores
   (`rust/tcl-compiler/src/analyses.rs`)
7. **Bytecode** — executable instruction stream with literal and
   local-variable tables (`rust/tcl-bytecode/src/lib.rs`), emitted by
   `rust/tcl-compiler/src/codegen/`

To see any of this for a script of your own:

```
cargo run -p tcl-cli --bin tcl -- explore FILE.tcl \
    --show ir,cfg,ssa,opt,asm --text
```

`--json` emits the same views in the machine-readable explorer contract
shape, and `--serve` opens the interactive web GUI.
