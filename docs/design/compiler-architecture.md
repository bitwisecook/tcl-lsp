# Compiler Architecture

The tcl-lsp compiler is a multi-pass analysis pipeline that transforms Tcl
source text into progressively richer intermediate representations, culminating
in SSA-based dataflow analyses, optimisation suggestions, security diagnostics,
and bytecode assembly.  The primary outputs are fed back to the LSP server as
diagnostics, code actions, and editor hints.  The bytecode backend also
produces Tcl-compatible assembly for identity testing against reference
`tclsh` implementations.

## KCS quick map

For targeted maintenance tasks, prefer these focused KCS notes before editing this file:

- [compiler design index](compiler/README.md)

- [compiler-pipeline-overview.md](compiler/compiler-pipeline-overview.md)
- [lowering-contracts.md](compiler/lowering-contracts.md)
- [cfg-ssa-fact-model.md](compiler/cfg-ssa-fact-model.md)
- [diagnostics-integration.md](compiler/diagnostics-integration.md)
- [compilation-unit-contracts.md](compiler/compilation-unit-contracts.md)
- [downstream-pass-contracts.md](compiler/downstream-pass-contracts.md)
- [async-diagnostics-tiering.md](compiler/async-diagnostics-tiering.md)
- [bytecode-boundary.md](compiler/bytecode-boundary.md)
- [kcs-howto-add-compiler-pass.md](../kcs/kcs-howto-add-compiler-pass.md)

## Pipeline overview

```mermaid
flowchart TD
    SRC["Tcl source text"]

    SRC --> LEX["<b>Lexer</b><br/>tokenise_all()"]
    LEX -->|"flat token stream"| SEG["<b>Command Segmenter</b><br/>segment_commands()"]
    SEG -->|"SegmentedCommand list"| REC{"Unclosed<br/>delimiters?"}
    REC -- yes --> VREC["<b>Error Recovery</b><br/>inject virtual tokens"]
    VREC --> SEG
    REC -- no --> SPLIT{{"Pipeline splits"}}

    SPLIT --> ANA["<b>Semantic Analyser</b><br/>Analyser::analyse()"]
    SPLIT --> LOW["<b>IR Lowering</b><br/>lower_to_ir()"]

    LOW -->|ir::Module| CFG["<b>CFG Construction</b><br/>build_cfg_function()"]
    CFG -->|cfg::Function| SSA["<b>SSA Construction</b><br/>build_ssa()"]
    SSA -->|SsaFunction| CORE["<b>Core Analyses</b><br/>sccp / type_infer / taint"]
    LOW -->|ir::Module| IPA["<b>Interprocedural Analysis</b><br/>build_interprocedural_analysis()"]

    CORE -->|FunctionUnit| CU["<b>CompilationUnit</b>"]
    IPA -->|InterproceduralAnalysis| CU
    CFG --> CU

    CU --> OPT["<b>Optimiser</b><br/>optimise_unit()"]
    CU --> TAINT["<b>Taint Analysis</b><br/>find_taint_warnings_for_cu()"]
    CU --> SHIM["<b>Shimmer Detection</b><br/>find_shimmer_warnings_for_cu()"]
    CU --> GVN["<b>GVN / CSE</b><br/>gvn.rs"]
    CU --> IFLOW["<b>iRules Flow</b><br/>irules_checks.rs"]
    CU --> ANA

    CFG --> CGEN["<b>Bytecode Codegen</b><br/>codegen_module()"]
    CGEN -->|"ModuleAsm"| ASMTXT["Assembly text"]

    ANA -->|AnalysisResult| DIAG["<b>Diagnostics Provider</b><br/>compiler_checks.rs"]
    OPT -->|"Optimisation list"| DIAG
    TAINT -->|"TaintWarning list"| DIAG
    SHIM -->|"ShimmerWarning list"| DIAG
    GVN -->|"RedundantComputation list"| DIAG
    IFLOW -->|"IrulesFlowWarning list"| DIAG

    DIAG -->|"LSP Diagnostic list"| SCHED["<b>Async Scheduler</b><br/>tiered publishing"]
    SCHED -->|"Tier 1: immediate<br/>Tier 2: background"| LSP["<b>LSP Server</b><br/>publish to editor"]
```

## Stage details

### 1. Lexer

**File:** `rust/tcl-lexer/src/lexer.rs` — `Lexer::tokenise_all()`

Converts raw source text into a flat stream of `Token` values.  A token is
a `TokenType` plus a byte `Span`; the text and line/column position are
resolved on demand through a `SourceMap`, so the token stream itself stays
allocation-free.  `content_offset` records how many leading bytes of the span
are opening delimiters (`$`, `${`, `[`, `{`, `"`), which lets
`SourceMap::token_text` strip them without a second range: `0` for bare
`Esc` / `Sep` / `Eol` / `Comment` tokens whose span *is* the content, `1` for
most wrappers, `2` for `${…}`.  `in_quote` records whether the token was
emitted inside a quoted-string context.

```mermaid
flowchart LR
    SRC["source text"] --> LEX["Lexer::tokenise_all()"]
    LEX --> TOK["Vec&lt;Token&gt;"]

    subgraph Token
        direction TB
        TT["kind: TokenType"]
        SP["span: Span (byte range)"]
        DL["content_offset: u8 — leading delimiter bytes"]
        IQ["in_quote: bool"]
    end

    TOK --- Token
```

Token types:

| Type | Meaning | Example |
|------|---------|---------|
| `Str` | Braced string | `{hello world}` |
| `Var` | Variable reference | `$name`, `${arr(idx)}` |
| `Cmd` | Command substitution | `[clock seconds]` |
| `Esc` | String / word fragment | `hello` |
| `Sep` | Whitespace separator | spaces, tabs |
| `Eol` | Command terminator | newline, `;` |
| `Eof` | End-of-input sentinel | — |
| `Comment` | Comment text | `# ...` |
| `Expand` | Argument-expansion prefix (8.5+) | `{*}` |

The lexer handles Tcl-specific constructs: nested command substitutions,
`$var`, `${name}`, `$arr(idx)`, namespace separators `::`, backslash
escapes, and brace nesting.  Base-offset parameters allow the same lexer to
be re-entered for nested bodies (brace/bracket contents).

### 2. Command Segmenter

**File:** `rust/tcl-compiler/src/segmenter.rs` — `segment_commands()`

Tcl has no traditional grammar — a "program" is a sequence of commands, each
being a list of whitespace-separated words terminated by a newline or
semicolon.  The segmenter groups the flat token stream into per-command
structures.

Internally `segment_commands()` no longer runs a bespoke token loop: it builds
the canonical lossless [red-green concrete syntax tree](compiler/syntax-tree.md)
for the region and *derives* the `SegmentedCommand` list from it,
byte-identically to the former loop (verified over the real-world corpus and
120k randomised differential cases).  The tree is the single representation the
formatter, minifier, AOT lowering, and per-command tooling are migrating onto.

```mermaid
flowchart LR
    TOKS["Token stream"] --> SEGR["segment_commands()"]
    SEGR --> CMDS["list of SegmentedCommand"]

    subgraph SegmentedCommand
        direction TB
        SP["span: Span"]
        AV["argv: Vec&lt;Token&gt;"]
        TX["texts: Vec&lt;String&gt;"]
        WF["word_fragments: Vec&lt;Vec&lt;WordFragment&gt;&gt;"]
        ST["single_token_word: Vec&lt;bool&gt;"]
        AT["all_tokens: Vec&lt;Token&gt;"]
        IP["is_partial: bool"]
        PD["partial_delimiter: Option&lt;UnclosedDelimiter&gt;"]
        EW["expand_word: Option&lt;Vec&lt;bool&gt;&gt;"]
        PC["preceding_comment: Option&lt;String&gt;"]
    end

    CMDS --- SegmentedCommand
```

Multi-token words (adjacent tokens with no separator, e.g. `$prefix.txt`) are
concatenated into a single `texts` entry.  The `single_token_word` flags
record which words are atomic — important for downstream constant tracking.
`word_fragments` is the lossless companion to the `argv` / `texts` parallel
arrays: it keeps every word's lexical fragments in substitution order, and
is what a new semantic IR consumer should read.  `partial_delimiter` names
which delimiter was left unclosed on an `is_partial` command — it is set by
the recovery segmenter and selects the precise E200 message.
`preceding_comment` carries the comment block immediately above the command,
which is what populates `ProcDef::doc` / `ClassDef::doc`.

### 3. Error Recovery

**File:** `rust/tcl-compiler/src/segmenter.rs` — `segment_with_recovery()`

When the segmenter encounters an unclosed delimiter (`{`, `[`, `"`), recovery
kicks in:

```mermaid
flowchart TD
    SEG["segment_commands()"] --> CHK{"Unclosed<br/>delimiter?"}
    CHK -- no --> OK["Commands ready"]
    CHK -- yes --> FIND["Find recovery offset<br/>scan for known command name"]
    FIND --> INJECT["Inject zero-width<br/>virtual closing token"]
    INJECT --> DIAG["Emit E200/E201/E202/E203<br/>diagnostic"]
    DIAG --> RESEG["Re-segment from<br/>recovery point"]
    RESEG --> SEG
```

Virtual tokens are zero-width characters inserted at the detected problem
site.  This preserves all position mapping — no source rewriting occurs.
Recovery diagnostics use E-series codes (E200 = generic unclosed, E201 =
unterminated `[`, E202 = unterminated `"`, E203 = unterminated `{`).

### 4. Semantic Analyser

**Module:** `rust/tcl-compiler/src/analyser/` — `Analyser` (state in
`state.rs`, dispatch in `dispatch.rs`, per-command handlers in
`handlers.rs` and `commands.rs`)

A single-pass walk over segmented commands that builds a semantic model:
scopes, procedure definitions, variable definitions, command invocations.
Dispatch is registry-driven — a command's `CommandSpec` selects the
handler — rather than a chain of name comparisons.

```mermaid
flowchart TD
    CMDS["Vec&lt;SegmentedCommand&gt;"] --> ANA["Analyser::analyse()"]
    ANA --> BODY["analyse_body()"]
    BODY --> |"per command"| PROC["dispatch.rs"]

    PROC --> MATCH{"CommandSpec<br/>lookup"}
    MATCH --> H_PROC["proc / namespace handlers"]
    MATCH --> H_SET["variable handlers (set, upvar, …)"]
    MATCH --> H_IF["control-flow handlers (if, for, …)"]
    MATCH --> H_OO["TclOO handlers (oo.rs)"]
    MATCH --> H_TK["Tk handlers (tk_checks.rs)"]
    MATCH --> H_ETC["...other handlers"]

    ANA --> VUSG["diagnostics/usage.rs"]
    ANA --> VCFG["diagnostics/dataflow.rs"]

    ANA --> RES["AnalysisResult"]

    subgraph AnalysisResult
        direction TB
        DX["diagnostics"]
        SC["global_scope"]
        PR["all_procs / all_classes / all_variables"]
        CI["command_invocations"]
        SL["suppressed_lines"]
    end

    RES --- AnalysisResult
```

The analyser also receives the `CompilationUnit` (when available) to emit
CFG/SSA-informed diagnostics such as unreachable-code and dead-store warnings.

### 5. IR Lowering

**File:** `rust/tcl-compiler/src/lowering/mod.rs` — `lower_to_ir()`

Converts segmented commands into a structured Intermediate Representation.
Each Tcl command maps to a typed IR node.

```mermaid
flowchart LR
    SRC["source text"] --> LOW["lower_to_ir()"]
    LOW --> MOD["ir::Module"]

    subgraph ir::Module
        direction TB
        SO["source: String"]
        TL["top_level: ir::Script"]
        PD["procedures: Map&lt;String, Procedure&gt;"]
        ME["methods: Map&lt;String, MethodDef&gt;"]
        BU["body_units: Map&lt;String, Procedure&gt;"]
    end

    MOD --- ir::Module
```

`ir::Module` keeps three separate body maps rather than one.  `procedures`
holds real named `proc`s — the only map codegen emits.  `methods` holds
`TclOO` method bodies keyed `class::method`.  `body_units` holds *synthetic*
frames that run a script argument but are not callable procedures (`apply`
lambdas, `namespace eval` bodies) under synthetic qualified names such as
`::apply#0`; they exist purely so CFG → SSA → SCCP → taint reaches inside the
body, and at runtime each still executes through its own
`Statement::Barrier`, so bytecode is unaffected by whether one was recorded.
`lambda_body_units` names the subset of `body_units` that are closed lambda
frames, which is the only subset read-before-set (`W210`) runs over.

#### IR node hierarchy

`ir::Statement` is a single Rust enum, not a class hierarchy — each construct
below is one variant with its own named fields.  Every variant carries a
`Span` (byte range into `Module::source`), and several also carry a
`*_base: Option<u32>` absolute offset so expression-AST leaf positions can be
mapped back to absolute operand spans.

```mermaid
classDiagram
    class Statement {
        <<enum>>
    }
    class AssignConst {
        +Span span
        +String name
        +bool name_braced
        +String value
        +Option~Span~ value_span
    }
    class AssignExpr {
        +Span span
        +String name
        +bool name_braced
        +ExprNode expr
        +Option~u32~ expr_base
    }
    class AssignValue {
        +Span span
        +String name
        +bool name_braced
        +String value
        +bool value_needs_backsubst
        +Option~CommandTokens~ tokens
    }
    class Incr {
        +Span span
        +String name
        +Option~String~ amount
        +bool safe_on_uninit
    }
    class ExprEval {
        +Span span
        +ExprNode expr
        +Option~u32~ expr_base
    }
    class Call {
        +Span span
        +String command
        +Option~String~ canonical_command
        +Vec~String~ args
        +Vec~String~ defs
        +Vec~String~ reads
        +bool reads_own_defs
    }
    class Return
    class Barrier
    class Block
    class UpFrame
    class If {
        +Vec~IfClause~ clauses
        +Option~Script~ else_body
    }
    class For {
        +Script init
        +ExprNode condition
        +Script next
        +Script body
    }
    class While
    class Foreach {
        +Vec~ForeachIterator~ iterators
        +Script body
    }
    class Catch
    class Try {
        +Script body
        +Vec~TryHandler~ handlers
        +Option~Script~ finally_body
    }
    class Switch {
        +Vec~SwitchArm~ arms
        +SwitchMode mode
    }
    class Script {
        +Vec~Statement~ statements
    }

    Statement <|-- AssignConst
    Statement <|-- AssignExpr
    Statement <|-- AssignValue
    Statement <|-- Incr
    Statement <|-- ExprEval
    Statement <|-- Call
    Statement <|-- Return
    Statement <|-- Barrier
    Statement <|-- Block
    Statement <|-- UpFrame
    Statement <|-- If
    Statement <|-- For
    Statement <|-- While
    Statement <|-- Foreach
    Statement <|-- Catch
    Statement <|-- Try
    Statement <|-- Switch
    Script --* Statement
```

Key design decisions:
- **Barriers** (`Statement::Barrier`) mark commands like `eval`, `uplevel`,
  and dynamically-named dispatch that defeat static analysis — no constant
  propagation or dead-store reasoning can cross them.  A command is lowered
  to a `Call` when the lowerer can name what it defines and reads, and to a
  `Barrier` when it cannot.
- **Two variants exist to *escape* the barrier where the body is static.**
  `Statement::Block` splices an inlined body into the enclosing statement
  stream with no new scope — produced by `inline_uplevel` for a passthrough
  callsite, and by const-propagation when an `eval` / `uplevel` body resolves
  to a brace literal.  `Statement::UpFrame` models `uplevel ?level? {body}`
  with a brace-literal body: the body is lowered inline and codegen brackets
  it with `frame_depth_stash` / `frame_depth_restore`, so the frame shift is
  expressed statically instead of through a runtime `Barrier` dispatch.
- **Every variant carries a `Span`** for precise source mapping back to
  diagnostics, and `Call` carries both the source spelling (`command`) and
  the registry-resolved `canonical_command`, so a diagnostic can quote what
  the author wrote while dispatch uses what it resolves to.
- **Expression bodies** are parsed into `ExprNode` AST trees at lowering
  time (via `parse_expr()`), not left as opaque strings.
- **`Statement::Call` records `defs` / `reads` explicitly** — the variables a
  command writes and reads beyond the `$`-references visible in its argument
  text — so SSA does not have to model each command's behaviour itself.

### 6. Control Flow Graph

**File:** `rust/tcl-compiler/src/cfg_builder/mod.rs` — `build_cfg_function()`

Flattens structured IR (`Statement::If`, `Statement::For`,
`Statement::Switch`, etc.) into basic blocks with explicit control-flow
edges.  The per-function graph is `cfg::Function`; the whole-module pair of
top-level script plus procedures is `cfg::CfgModule`.

```mermaid
flowchart TD
    IR["ir::Script"] --> BUILD["build_cfg_function()"]
    BUILD --> FN["cfg::Function"]

    subgraph cfg::Function
        direction TB
        NA["name: String — qualified"]
        EN["entry: BlockId"]
        BL["blocks: Map&lt;BlockId, Block&gt;"]
        LN["loop_nodes: Map&lt;BlockId, LoopNode&gt;"]
        XE["exception_edges: Vec&lt;(BlockId, BlockId)&gt;"]
        IE["inline_body_error_sites: Vec&lt;InlineBodyErrorSite&gt;"]
    end

    FN --- cfg::Function

    subgraph cfg::Block
        direction TB
        NM["name: String — e.g. entry_1, if_then_2"]
        ST["statements: Vec&lt;ir::Statement&gt;"]
        TM["terminator: Option&lt;Terminator&gt;"]
    end

    subgraph Terminator["cfg::Terminator (enum)"]
        direction LR
        GT["Goto<br/>target: BlockId"]
        BR["Branch<br/>condition: ExprNode<br/>true_target / false_target: BlockId"]
        RT["Return<br/>value: Option&lt;String&gt;<br/>expr: Option&lt;ExprNode&gt;"]
    end
```

Blocks are keyed by an interned `BlockId`, not by name; `Function::block_name`
resolves an id back to its display name, and `SsaFunction` copies the interner
so an SSA-only consumer can still print block names.

Each basic block is a straight-line sequence of IR statements ending with at
most one terminator — `terminator` is `Option`, and `None` marks an
unreachable or incomplete block rather than an error.  Because a single
successor cannot express a `try` body reaching its handler, those edges are
carried out of band in `Function::exception_edges`; SSA consumes them as
extra phi predecessors so a handler sees the body's versions, and SCCP
consumes them as extra reachability edges so handler bodies are not reported
false-unreachable (O107).

Structured constructs decompose as follows:

```mermaid
flowchart TD
    subgraph "Statement::If decomposition"
        E1["entry block"] --> B1{"Terminator::Branch<br/>condition"}
        B1 -- true --> T1["then block"]
        B1 -- false --> F1["else block"]
        T1 --> M1["merge block"]
        F1 --> M1
    end
```

```mermaid
flowchart TD
    subgraph "Statement::For decomposition"
        INIT["init block"] --> COND{"Terminator::Branch<br/>loop condition"}
        COND -- true --> BODY["body block"]
        COND -- false --> EXIT["exit block"]
        BODY --> NEXT["next block"]
        NEXT --> COND
    end
```

A loop records a `LoopNode` keyed by its **exit** block, holding the header
block id, the `for` statement's span, and the original `Statement::For` — the
last of these is what lets SCCP statically summarise a bounded loop and fold
a branch that reads a loop-carried variable *after* the loop.

### 7. SSA Construction

**File:** `rust/tcl-compiler/src/ssa.rs` — `build_ssa()`

Converts the CFG to Static Single-Assignment form, where every variable is
defined exactly once.  Phi nodes are inserted at control-flow merge points.

```mermaid
flowchart TD
    CFG["cfg::Function"] --> DOM["Compute dominators<br/>& dominance frontier"]
    DOM --> PHI["Place phi nodes<br/>(iterated dominance frontier)"]
    PHI --> REN["Rename variables<br/>(Symbol, Version) pairs"]
    REN --> SSA["SsaFunction"]

    subgraph "SSA variable versioning"
        direction LR
        V0["x_0 = 10"]
        V1["x_1 = 20"]
        V2["x_2 = phi(x_0, x_1)"]
    end
```

Key types:
- `Version` (`u32`) — version number for each definition
- `Symbol` (`u32` newtype) — a variable name interned per `SsaFunction`, so
  the hot per-statement maps and the SCCP / type / taint lattices key on a
  copyable id instead of hashing and cloning name strings.  Its `Ord` is
  first-seen order during SSA construction — deterministic, but *not*
  lexicographic; resolve to a display name with `SsaFunction::var_name`.
- `ValueKey` — `(Symbol, Version)`, identifying a unique SSA value
- `SsaFunction` — per-block phi nodes (`SsaBlock::phis`), per-block entry and
  exit version maps, the immediate-dominator map, the dominance frontier, and
  the dominator tree, so downstream passes never recompute dominance
- `SsaStatement` — an `ir::Statement` plus its `uses` and `defs` version maps,
  with two refinement sets over them:
  - `may_defs` — the subset of `defs` that are *synthetic* array-element
    writes rather than writes the statement performs itself (the base refresh
    alongside `set arr(k) v`, and the element fan of a dynamic-key write).
    Type inference **joins** across a may-def; write-sensitive passes
    (shimmer oscillation, dead-store) must not count one as a real write.
  - `quoted_uses` — the subset of `uses` classified `UseClass::Quoted`,
    carried only by a brace-quoted word this statement does not substitute.
    The use is real for liveness (the text may be evaluated later) but is not
    a read *here*, so read-before-set (`W210`) must ignore it while
    liveness / dead-store (`W211`, `W220`) must not.  Filtering at either end
    breaks the other — see issues #1142 and #1237.

### 8. Core Analyses

**Modules:** `rust/tcl-compiler/src/sccp.rs`, `type_infer.rs`, `taint.rs`, `dead_stores.rs` — lattice and fact types in `analyses.rs`, carried per function by `FunctionUnit` (`compilation_unit.rs`)

Runs the main dataflow passes over the SSA graph:

```mermaid
flowchart TD
    SSA["SSAFunction"] --> SCCP["<b>SCCP</b><br/>Sparse Conditional<br/>Constant Propagation"]
    SSA --> LIVE["<b>Liveness</b><br/>backward dataflow"]
    SSA --> TYPE["<b>Type Inference</b><br/>infer_expr_type()"]

    SCCP --> SR["SccpResult:<br/>values, executable_blocks,<br/>executable_edges, constant_branches"]
    TYPE --> TL["types:<br/>ValueKey → TypeLattice"]

    SR --> FU["FunctionUnit"]
    TL --> FU

    LIVE --> LO["live_out_by_name()<br/>slot interference"]
    LIVE --> DS["liveness_dead_stores()<br/>DeadStore list"]
```

SCCP and type inference land on the per-function `FunctionUnit` that
`CompilationUnit::build_for()` builds; liveness has no stored home and is
recomputed by the two consumers that need it.  `FunctionAnalysis` in
`analyses.rs` names the same facts as one aggregate, but nothing builds,
returns, or reads one — its only construction is `::default()` inside that
module's own tests.  Issue #1406 tracks the gap.

#### SCCP lattice

SCCP uses a four-point lattice per SSA value (`LatticeValue` in
`analyses.rs`, joined by `sccp::join()`).  Values flow monotonically upward
and never narrow:

```mermaid
flowchart BT
    BOT["Unknown (bottom)<br/>not yet analysed"] --> MID["Const(v)<br/>provably one value"]
    MID --> SET["ConstSet([v…])<br/>a small closed set,<br/>up to MAX_CONSTSET_SIZE (32)"]
    SET --> TOP["Overdefined (top)<br/>too many values to track"]
```

`ConstSet` is the union of two or more distinct `Const`s — the shape a phi at
a merge of `if` / `switch` arms produces.  It stays a set only while the union
has at most `MAX_CONSTSET_SIZE` (32) members; a wider union widens to
`Overdefined`.  A union that collapses back to one member becomes `Const`
again, which is a narrowing of *representation*, not of the value set, so
monotonicity holds.

Branch conditions are evaluated against the lattice.  If a branch condition
is `Const`, only the taken edge is added to `SccpResult::executable_edges` —
the other side stays out of `executable_blocks`, which is what drives
unreachable-code detection.

### 9. Interprocedural Analysis

**File:** `rust/tcl-compiler/src/interprocedural.rs` — `build_interprocedural_analysis()`

Builds conservative procedure summaries across the entire module:

```mermaid
flowchart TD
    IR["ir::Module"] --> IPA["build_interprocedural_analysis()"]
    IPA --> SUM["ProcSummary per procedure"]
    IPA --> MSUM["MethodSummary per TclOO method"]

    subgraph ProcSummary
        direction TB
        QN["qualified_name"]
        PM["params, arity"]
        CG["calls / direct_calls"]
        BR["has_barrier, has_unknown_calls"]
        EF["pure, writes_global,<br/>effect_reads / effect_writes: EffectRegion"]
        CR["returns_constant,<br/>constant_return: Option&lt;ConstantReturn&gt;"]
        PS["return_depends_on_params,<br/>return_passthrough_param"]
        FD["can_fold_static_calls"]
        PT["param_traits: name → set&lt;ProcArgTrait&gt;"]
    end

    subgraph MethodSummary
        direction TB
        MB["base: ProcSummary"]
        MC["class_name, method_kind"]
        MV["reads_instance_vars / writes_instance_vars"]
        MN["calls_my, calls_next"]
    end

    SUM --- ProcSummary
    MSUM --- MethodSummary
    SUM --> IPA2["InterproceduralAnalysis"]
    MSUM --> IPA2
    IPA2 --> FOLD["optimiser::propagation::<br/>try_fold_static_proc_call()"]
    IPA2 --> OPT["used by Optimiser (O103)"]
```

Summaries describe:
- **Side-effect / purity** — `pure` and `writes_global` as booleans, plus the
  finer-grained `effect_reads` / `effect_writes` regions
- **Constant return** — whether a proc always returns the same value, and
  what that value is
- **Parameter sensitivity** — `return_depends_on_params` names the parameters
  that affect the return value, and `return_passthrough_param` the single
  parameter returned verbatim, if any
- **Call graph edges** — `calls` (transitive-ready) and `direct_calls`
- **Opacity** — `has_barrier` and `has_unknown_calls`, the two reasons a
  summary must be read as conservative rather than complete
- **Argument traits** — `param_traits`, the per-parameter `ProcArgTrait` sets
  that [proc-arg-traits.md](contracts/proc-arg-traits.md) specifies

`InterproceduralAnalysis` holds `procedures` and `methods` side by side; a
`MethodSummary` embeds a whole `ProcSummary` as `base` and adds the `TclOO`
facts (owning class, method kind, instance-variable reads and writes, and
whether the body dispatches through `my` or `next`).

The optimiser uses these summaries for O103 (fold static procedure calls)
and the taint analysis uses them for cross-procedure taint propagation.

### 10. Compilation Unit

**File:** `rust/tcl-compiler/src/compilation_unit.rs` — `CompilationUnit::build_for()`

`CompilationUnit` remains the shared artefact boundary for IR/CFG/SSA/interprocedural
facts consumed across diagnostics and downstream passes. For operational contracts,
cache semantics, and regression anchors, use:

- [compilation-unit-contracts.md](compiler/compilation-unit-contracts.md)

```mermaid
flowchart TD
    SRC["source text"] --> CS["CompilationUnit::build_for()"]

    CS --> LOW["lower_to_ir() → ir::Module"]
    LOW --> CFG_T["build_cfg_function(::top)"]
    LOW --> CFG_P["build_cfg_function() per proc"]
    CFG_T --> SSA_T["build_ssa()"]
    CFG_P --> SSA_P["build_ssa()"]
    SSA_T --> ANA_T["core dataflow analyses"]
    SSA_P --> ANA_P["core dataflow analyses"]

    ANA_T --> TU["FunctionUnit (top-level)"]
    ANA_P --> PU["FunctionUnit per proc"]

    LOW --> IPA["build_interprocedural_analysis()"]
    PU --> IPA

    TU --> CU["CompilationUnit"]
    PU --> CU
    IPA --> CU
    LOW --> CU

    subgraph CompilationUnit
        direction TB
        S["source: String"]
        IM["ir_module: ir::Module"]
        CM["cfg_module: cfg::CfgModule"]
        TL["top_level: FunctionUnit"]
        PR["procedures: Map&lt;String, FunctionUnit&gt;"]
        ME["methods: Map&lt;String, FunctionUnit&gt;"]
        BU["body_units: Map&lt;String, FunctionUnit&gt;"]
        IP["interproc: Option&lt;InterproceduralAnalysis&gt;"]
        CO["connection_scope: Option&lt;ConnectionScope&gt;"]
        CS2["caller_scope: UnitCallerScope"]
    end

    CU --- CompilationUnit
```

The three body maps of `ir::Module` are mirrored one-for-one as
`FunctionUnit` maps, so a `TclOO` method body and an `apply` lambda body each
get the same CFG → SSA → SCCP → type → taint treatment a `proc` does.
`interproc` is optional because `build_for` can be asked to defer it, so a
consumer must handle `None` rather than assume summaries are present.
`caller_scope` carries what the *workspace* knows about this unit's callers —
the linkage traits, whether cross-file evidence exists, the call-site
evidence, and per-proc constant arguments — which is how a single-file
compilation unit participates in cross-file reasoning.

### 11. Downstream Analysis Passes

All downstream passes consume the `CompilationUnit` and produce typed warnings
converted by the diagnostics provider. Contract details, ownership guidance,
and pass/test anchors are tracked in:

- [downstream-pass-contracts.md](compiler/downstream-pass-contracts.md)
- [kcs-howto-add-compiler-pass.md](../kcs/kcs-howto-add-compiler-pass.md)

```mermaid
flowchart LR
    CU["CompilationUnit"]

    CU --> OPT["<b>Optimiser</b><br/>O100–O130"]
    CU --> TAINT["<b>Taint Analysis</b><br/>T100–T106, IRULE3xxx"]
    CU --> SHIM["<b>Shimmer Detection</b><br/>S100–S103, S110"]
    CU --> GVN["<b>GVN / CSE</b><br/>O105, O106"]
    CU --> IFLOW["<b>iRules Flow</b><br/>IRULE1xxx–6xxx"]

    OPT --> D["Diagnostics"]
    TAINT --> D
    SHIM --> D
    GVN --> D
    IFLOW --> D
```

### 12. Diagnostics Provider

**Modules:** `rust/tcl-compiler/src/analyser/diagnostics/` (per-family emitters) and `compiler_checks.rs` (the downstream-pass aggregation)

The aggregation layer is the policy boundary that merges analyser and pass
findings, applies suppression / disable rules, and converts to LSP
diagnostics. Integration contracts live in:

- [diagnostics-integration.md](compiler/diagnostics-integration.md)
- [async-diagnostics-tiering.md](compiler/async-diagnostics-tiering.md)

```mermaid
flowchart TD
    SRC["source text"] --> GD["compiler_checks.rs"]

    GD --> CS["CompilationUnit::build_for() → CU"]
    GD --> AN["Analyser::analyse() → AnalysisResult"]

    GD --> STYLE["Style checks<br/>W111 line length<br/>W112 trailing whitespace<br/>W115 comment continuation"]

    CS --> OPT["optimiser::manager::optimise_unit()"]
    CS --> SHIM["shimmer::find_shimmer_warnings_for_cu()"]
    CS --> TAINT["taint::find_taint_warnings_for_cu()"]
    CS --> GVN["gvn.rs"]
    CS --> IFLOW["irules_checks.rs"]

    AN --> FILT["Filter & suppress"]
    OPT --> FILT
    SHIM --> FILT
    TAINT --> FILT
    GVN --> FILT
    IFLOW --> FILT
    STYLE --> FILT

    FILT -->|"# noqa, disabled codes"| CONV["Convert to LSP Diagnostic"]
    CONV --> LSP["Publish to editor"]
```

### 13. Bytecode Assembly Backend

**File:** `rust/tcl-compiler/src/codegen/emitter/mod.rs` — `codegen_function()`, `codegen_module()`

Takes a pre-SSA `cfg::CfgModule` and emits assembly text matching the format
produced by `tcl::unsupported::disassemble` in Tcl 9.0.2.

```mermaid
flowchart LR
    CFG["cfg::CfgModule"] --> CG["codegen_module()"]
    CG --> ASM["ModuleAsm"]

    subgraph "Two-phase approach"
        direction TB
        P1["1. Walk CFG blocks<br/>emit Instruction nodes<br/>with symbolic labels"]
        P2["2. Layout pass<br/>resolve labels → byte offsets<br/>format to text"]
        P1 --> P2
    end

    ASM --> FMT["format_module_asm()"]
    FMT --> TXT["Assembly text<br/>(tclsh-compatible)"]
```

The codegen module produces bytecode assembly that can be compared against
reference output from `tclsh` 8.5, 8.6, and 9.0, enabling bytecode identity
testing to verify that the compiler's output matches the canonical C Tcl
implementation.

### 14. Async Diagnostic Scheduler

**File:** `rust/tcl-lsp-server/src/lib.rs` — the tiered diagnostic scheduler

Tiered publishing and cancellation rules are maintained in:

- [async-diagnostics-tiering.md](compiler/async-diagnostics-tiering.md)

```mermaid
flowchart TD
    EDIT["Document edit"] --> BASIC["<b>Tier 1: Basic</b><br/>Lexer/parser errors (E-codes)<br/>Analysis warnings (W-codes)<br/>Style checks"]
    BASIC -->|"Publish immediately"| ED["Editor"]

    EDIT --> DEEP["<b>Tier 2: Deep</b><br/>background worker"]

    subgraph "Background worker"
        direction TB
        OPT["Optimiser (O100–O130)"]
        SHIM["Shimmer (S100–S103, S110)"]
        TAINT["Taint (T100–T106)"]
        GVN["GVN/CSE (O105–O106)"]
        IFLOW["iRules flow"]
    end

    DEEP --> OPT
    DEEP --> SHIM
    DEEP --> TAINT
    DEEP --> GVN
    DEEP --> IFLOW

    OPT --> PUB["Publish incrementally"]
    SHIM --> PUB
    TAINT --> PUB
    GVN --> PUB
    IFLOW --> PUB
    PUB --> ED

    EDIT -.->|"Cancel stale"| DEEP
```

## Expression sub-pipeline

Tcl `expr` bodies are parsed into a separate AST, used by SCCP, type
inference, the optimiser, and shimmer detection.

Both stages are free functions, not types: `tcl_lexer::expr_lexer::
tokenise_expr()` and `tcl_syntax::expr::parser::parse_expr()`.  Each takes an
optional dialect name, because a dialect can widen the operator and function
surface an expression accepts.

```mermaid
flowchart LR
    EXPR["expr source string"] --> ELEX["tokenise_expr(src, dialect)<br/>→ Vec&lt;ExprToken&gt;"]
    ELEX --> EPAR["parse_expr(src, dialect)"]
    EPAR --> EAST["ExprNode AST"]

    subgraph "ExprNode variants"
        direction TB
        LIT["Literal"]
        STR["String"]
        VAR["Var"]
        CMD["Command"]
        BIN["Binary"]
        UNA["Unary"]
        TER["Ternary"]
        CAL["Call"]
        RAW["Raw (fallback)"]
    end
```

`ExprNode::Raw` is a fallback for expressions the parser cannot handle —
every consumer must treat it as opaque and conservative.

## Diagnostic code taxonomy

Every code is declared exactly once, in the `diagnostic_codes!` table in
`rust/tcl-core-types/src/diag_code.rs`.  Two orthogonal groupings hang off
that table and are easy to confuse:

- **`DiagFamily`** is derived mechanically from the letter prefix
  (`DiagCode::family()`), with two special cases: `IRULE####` is `IRule`, and
  `TK###` is `Warning` rather than `Taint` despite the shared `T`.
- **`DiagSection`** is the finer documentation grouping, declared explicitly
  as the first argument of each row.  `W###` codes deliberately spread across
  several sections — `W101`–`W103` and `W300`–`W313` sit in `Security`,
  `W130`–`W134` in `Tclpkg`, `W123` and `W242` in `Hint` — so a code's
  section cannot be inferred from its number.

Two per-row flags qualify a code's status.  `diag_internal(…)` marks a code
as *internal*: always active and never offered as a user-configurable toggle
(parse and structure errors, host-config validators).  `diag_reserved(…)`
marks a code as *reserved*: a real, documented identity that no analyser or
compiler-checks path emits yet.  Both are excluded from the generated
editor-settings catalogue and both stay in `DiagCode::ALL` and the published
tables.

| Range | Section | Source |
|-------|---------|--------|
| E001–E006 | Arity / subcommand / definition errors | Analyser |
| E100–E103 | Lexical errors | Lexer |
| E200–E207 | Unterminated-construct errors | Segmenter / Recovery |
| H300, I230–I231 | Paste-error and constant-branch hints | Analyser / SCCP |
| W001–W004 | Command warnings | Analyser |
| W100–W147 | Semantic & style warnings | Analyser / Diagnostics |
| W130–W134 | `tclpkg` manifest warnings — **reserved**, not yet emitted | — |
| W200–W250 | Variable & versioning warnings | Analyser |
| W300–W315 | Security warnings | Analyser |
| O100–O130 | Optimisation suggestions | Optimiser + GVN |
| S100–S103, S110 | Shimmer / type thunking | Shimmer detection |
| T100–T106 | Taint / security | Taint analysis |
| TK1001–TK1003 | Tk widget and geometry checks | Analyser (`tk_checks.rs`) |
| IRULE1xxx | iRules flow and lifecycle | `irules_checks.rs` |
| IRULE2xxx | Deprecated / unsafe iRules commands | Analyser |
| IRULE3xxx | iRules security | Taint analysis |
| IRULE4xxx | iRules variable scope | Analyser (`irules_event_checks.rs`) |
| IRULE5xxx–6xxx | iRules event and command-model checks | Analyser (`irules_event_checks.rs`) |

## Key source files

| File | Responsibility |
|------|---------------|
| `rust/tcl-lexer/src/lexer.rs` | Tokenisation |
| `rust/tcl-lexer/src/tokens.rs` | `Token` / `TokenType` definitions |
| `rust/tcl-lexer/src/span.rs`, `source_map.rs`, `line_index.rs` | Byte spans and offset → line/column resolution |
| `rust/tcl-lexer/src/substitution.rs` | Tcl backslash substitution helpers |
| `rust/tcl-lexer/src/expr_lexer.rs` | Expression tokenisation |
| `rust/tcl-compiler/src/segmenter.rs` | Command segmentation, chunking, and recovery |
| `rust/tcl-compiler/src/parsing/syntax/` | The lossless red-green concrete syntax tree |
| `rust/tcl-syntax/src/expr/` | Expression parsing (`parser.rs`), AST (`ast.rs`), operators, and folding |
| `rust/tcl-compiler/src/analyser/` | Semantic analysis, scope tracking, per-command handlers |
| `rust/tcl-compiler/src/analyser/diagnostics/` | Diagnostic emitters by family — `usage.rs`, `security.rs`, `validity.rs`, `dataflow.rs`, `version_gate.rs`, … |
| `rust/tcl-compiler/src/analyser/diagnostics/fp/` | False-positive suppression rules |
| `rust/tcl-compiler/src/irules_checks.rs` | iRules-specific checks (IRULE series) |
| `rust/tcl-compiler/src/ir.rs` | IR node definitions (`Module`, `Script`) |
| `rust/tcl-compiler/src/lowering/` | IR construction from the token stream |
| `rust/tcl-compiler/src/cfg.rs`, `cfg_builder/` | Control-flow graph construction |
| `rust/tcl-compiler/src/ssa.rs`, `memory_ssa.rs`, `state_ssa.rs` | SSA form construction |
| `rust/tcl-compiler/src/sccp.rs`, `type_infer.rs`, `dead_stores.rs` | SCCP, type inference, dead-store detection |
| `rust/tcl-compiler/src/analyses.rs` | Core analysis fact and lattice types (`LatticeValue`, `DeadStore`, …; the `FunctionAnalysis` aggregate here is declared but not on the live path — issue #1406) |
| `rust/tcl-compiler/src/compilation_unit.rs` | Pipeline orchestration and caching |
| `rust/tcl-compiler/src/interprocedural.rs` | Call graph and procedure summaries |
| `rust/tcl-compiler/src/optimiser/` | Optimisation passes (O100–O130) and the pass manager |
| `rust/tcl-compiler/src/gvn.rs` | Global value numbering / CSE / PRE / LICM (O105–O106) |
| `rust/tcl-compiler/src/taint.rs`, `taint_interproc.rs` | Taint analysis for untrusted I/O (T100–T106) |
| `rust/tcl-compiler/src/shimmer/` | Type-representation issue detection (S100–S103, S110) |
| `rust/tcl-compiler/src/codegen/` | Tcl VM bytecode assembly backend |
| `rust/tcl-compiler/src/side_effects.rs` | Command side-effect classification |
| `rust/tcl-compiler/src/types.rs` | Type lattice definitions |
| `rust/tcl-compiler/src/compiler_checks.rs` | Downstream-pass aggregation into diagnostics |
| `rust/tcl-lsp-server/src/lib.rs` | Tiered diagnostic scheduling and LSP publishing |
| `rust/tcl-lsp-core/src/graphs.rs` | Call / symbol / data-flow graph queries |
