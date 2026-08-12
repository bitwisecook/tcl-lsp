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
    CFG -->|CFGFunction| SSA["<b>SSA Construction</b><br/>build_ssa()"]
    SSA -->|SsaFunction| CORE["<b>Core Analyses</b><br/>sccp / type_infer / taint"]
    LOW -->|ir::Module| IPA["<b>Interprocedural Analysis</b><br/>build_interprocedural_analysis()"]

    CORE -->|FunctionAnalysis| CU["<b>CompilationUnit</b>"]
    IPA -->|InterproceduralAnalysis| CU
    CFG --> CU

    CU --> OPT["<b>Optimiser</b><br/>optimise_unit()"]
    CU --> TAINT["<b>Taint Analysis</b><br/>find_taint_warnings_for_cu()"]
    CU --> SHIM["<b>Shimmer Detection</b><br/>find_shimmer_warnings()"]
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
allocation-free.  `delim_len` records how many leading bytes of the span
are opening delimiters (`$`, `${`, `[`, `{`, `"`), which lets
`SourceMap::token_text` strip them without a second range.

```mermaid
flowchart LR
    SRC["source text"] --> LEX["Lexer::tokenise_all()"]
    LEX --> TOK["Vec&lt;Token&gt;"]

    subgraph Token
        direction TB
        TT["kind: TokenType"]
        SP["span: Span (byte range)"]
        DL["delim_len: opening bytes"]
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
    end

    CMDS --- SegmentedCommand
```

Multi-token words (adjacent tokens with no separator, e.g. `$prefix.txt`) are
concatenated into a single `texts` entry.  The `single_token_word` flags
record which words are atomic — important for downstream constant tracking.
`word_fragments` is the lossless companion to the `argv` / `texts` parallel
arrays: it keeps every word's lexical fragments in substitution order, and
is what a new semantic IR consumer should read.

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
        PR["procedures"]
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
        TL["top_level: ir::Script"]
        PD["procedures: Map&lt;String, Procedure&gt;"]
    end

    MOD --- ir::Module
```

#### IR node hierarchy

```mermaid
classDiagram
    class IRStatement {
        <<union type>>
    }
    class IRAssignConst {
        +Range range
        +str name
        +str value
    }
    class IRAssignExpr {
        +Range range
        +str name
        +ExprNode expr
    }
    class IRAssignValue {
        +Range range
        +str name
        +str value
    }
    class IRIncr {
        +Range range
        +str name
        +str|None amount
    }
    class IRCall {
        +Range range
        +str command
        +tuple args
        +tuple defs
        +bool reads_own_defs
    }
    class IRReturn {
        +Range range
        +str|None value
    }
    class IRBarrier {
        +Range range
        +str reason
        +str command
    }
    class IRIf {
        +Range range
        +tuple~IRIfClause~ clauses
        +ir::Script|None else_body
    }
    class IRFor {
        +Range range
        +ir::Script init
        +ExprNode condition
        +ir::Script next
        +ir::Script body
    }
    class IRWhile {
        +Range range
        +ExprNode condition
        +ir::Script body
    }
    class IRForeach {
        +Range range
        +tuple iterators
        +ir::Script body
    }
    class IRCatch {
        +Range range
        +ir::Script body
        +str|None result_var
    }
    class IRTry {
        +Range range
        +ir::Script body
        +tuple~IRTryHandler~ handlers
        +ir::Script|None finally_body
    }
    class IRSwitch {
        +Range range
        +str subject
        +tuple~IRSwitchArm~ arms
    }
    class ir::Script {
        +tuple~IRStatement~ statements
    }
    class ir::Module {
        +ir::Script top_level
        +dict procedures
    }

    IRStatement <|-- IRAssignConst
    IRStatement <|-- IRAssignExpr
    IRStatement <|-- IRAssignValue
    IRStatement <|-- IRIncr
    IRStatement <|-- IRCall
    IRStatement <|-- IRReturn
    IRStatement <|-- IRBarrier
    IRStatement <|-- IRIf
    IRStatement <|-- IRFor
    IRStatement <|-- IRWhile
    IRStatement <|-- IRForeach
    IRStatement <|-- IRCatch
    IRStatement <|-- IRTry
    IRStatement <|-- IRSwitch
    ir::Script --* IRStatement
    ir::Module --* ir::Script
```

Key design decisions:
- **Barriers** (`IRBarrier`) mark commands like `eval`, `uplevel`, `upvar`
  that defeat static analysis — no constant propagation or dead-store
  reasoning can cross them.
- **Every node carries a `Range`** for precise source mapping back to
  diagnostics.
- **Expression bodies** are parsed into `ExprNode` AST trees at lowering
  time (via `parse_expr()`), not left as opaque strings.

### 6. Control Flow Graph

**File:** `rust/tcl-compiler/src/cfg_builder/mod.rs` — `build_cfg_function()`

Flattens structured IR (`IRIf`, `IRFor`, `IRSwitch`, etc.) into basic blocks
with explicit control-flow edges.

```mermaid
flowchart TD
    IR["ir::Script"] --> BUILD["build_cfg_function()"]
    BUILD --> FN["CFGFunction"]

    subgraph CFGFunction
        direction TB
        EN["entry: str"]
        BL["blocks: Map&lt;BlockId, Block&gt;"]
        LN["loop_nodes: dict"]
    end

    FN --- CFGFunction

    subgraph CFGBlock
        direction TB
        NM["name: str"]
        ST["statements: tuple[IRStatement]"]
        TM["terminator: CFGTerminator"]
    end

    subgraph CFGTerminator["CFGTerminator (union)"]
        direction LR
        GT["CFGGoto<br/>target: str"]
        BR["CFGBranch<br/>condition: ExprNode<br/>true_target: str<br/>false_target: str"]
        RT["CFGReturn<br/>value: str|None"]
    end
```

Each basic block is a straight-line sequence of IR statements ending with
exactly one terminator.  Structured constructs decompose as follows:

```mermaid
flowchart TD
    subgraph "IRIf decomposition"
        E1["entry block"] --> B1{"CFGBranch<br/>condition"}
        B1 -- true --> T1["then block"]
        B1 -- false --> F1["else block"]
        T1 --> M1["merge block"]
        F1 --> M1
    end
```

```mermaid
flowchart TD
    subgraph "IRFor decomposition"
        INIT["init block"] --> COND{"CFGBranch<br/>loop condition"}
        COND -- true --> BODY["body block"]
        COND -- false --> EXIT["exit block"]
        BODY --> NEXT["next block"]
        NEXT --> COND
    end
```

### 7. SSA Construction

**File:** `rust/tcl-compiler/src/ssa.rs` — `build_ssa()`

Converts the CFG to Static Single-Assignment form, where every variable is
defined exactly once.  Phi nodes are inserted at control-flow merge points.

```mermaid
flowchart TD
    CFG["CFGFunction"] --> DOM["Compute dominators<br/>& dominance frontier"]
    DOM --> PHI["Place phi nodes<br/>(iterated dominance frontier)"]
    PHI --> REN["Rename variables<br/>(name, version) pairs"]
    REN --> SSA["SSAFunction"]

    subgraph "SSA variable versioning"
        direction LR
        V0["x_0 = 10"]
        V1["x_1 = 20"]
        V2["x_2 = phi(x_0, x_1)"]
    end
```

Key types:
- `SSAVersion` (int) — version number for each definition
- `SSAValueKey` — `tuple[str, SSAVersion]` identifying a unique SSA value
- `SSAFunction` — contains per-block phi nodes, variable version maps,
  and dominance information

### 8. Core Analyses

**Modules:** `rust/tcl-compiler/src/sccp.rs`, `type_infer.rs`, `taint.rs`, `dead_stores.rs` — result types in `analyses.rs` (`FunctionAnalysis`)

Runs the main dataflow passes over the SSA graph:

```mermaid
flowchart TD
    SSA["SSAFunction"] --> SCCP["<b>SCCP</b><br/>Sparse Conditional<br/>Constant Propagation"]
    SSA --> LIVE["<b>Liveness</b><br/>backward dataflow"]
    SSA --> TYPE["<b>Type Inference</b><br/>infer_expr_type()"]

    SCCP --> VL["var_lattice:<br/>SSAValueKey → LatticeValue"]
    SCCP --> CB["constant_branches:<br/>block → ConstantBranch"]
    SCCP --> UB["unreachable_blocks"]
    LIVE --> LI["live_in / live_out<br/>per block"]
    LIVE --> DS["dead_stores"]
    TYPE --> TL["type_lattice:<br/>SSAValueKey → TclType"]

    VL --> FA["FunctionAnalysis"]
    CB --> FA
    UB --> FA
    LI --> FA
    DS --> FA
    TL --> FA
```

#### SCCP lattice

SCCP uses a three-point lattice per SSA value.  Values flow monotonically
upward and never narrow:

```mermaid
flowchart BT
    BOT["UNKNOWN (bottom)<br/>not yet analysed"] --> MID["CONST(v)<br/>provably constant"]
    MID --> TOP["OVERDEFINED (top)<br/>multiple possible values"]
```

Branch conditions are evaluated against the lattice.  If a branch condition
is `CONST`, only the taken edge is explored — the other side is marked
unreachable, enabling dead-code detection.

### 9. Interprocedural Analysis

**File:** `rust/tcl-compiler/src/interprocedural.rs` — `build_interprocedural_analysis()`

Builds conservative procedure summaries across the entire module:

```mermaid
flowchart TD
    IR["ir::Module"] --> IPA["build_interprocedural_analysis()"]
    IPA --> SUM["ProcSummary per procedure"]

    subgraph ProcSummary
        direction TB
        QN["qualified_name"]
        PM["params, arity"]
        EF["side_effects: EffectRegion"]
        CR["constant_return"]
        CG["call_graph edges"]
        PS["parameter_sensitivity"]
    end

    SUM --- ProcSummary
    SUM --> IPA2["InterproceduralAnalysis"]
    IPA2 --> FOLD["fold_static_proc_call()"]
    IPA2 --> OPT["used by Optimiser (O103)"]
```

Summaries describe:
- **Side-effect / purity** — whether a proc is pure (no I/O, no globals)
- **Constant return** — whether a proc always returns the same value
- **Parameter sensitivity** — which parameters affect the return value
- **Call graph edges** — for transitive analysis

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
        S["source"]
        IM["ir_module: ir::Module"]
        CM["cfg_module: CFGModule"]
        TL["top_level: FunctionUnit"]
        PR["procedures: Map&lt;String, FunctionUnit&gt;"]
        IP["interproc: InterproceduralAnalysis"]
        CO["connection_scope: ConnectionScope|None"]
    end

    CU --- CompilationUnit
```

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
    CU --> SHIM["<b>Shimmer Detection</b><br/>S100–S102"]
    CU --> GVN["<b>GVN / CSE</b><br/>O105, O106"]
    CU --> IFLOW["<b>iRules Flow</b><br/>IRULE1xxx–5xxx"]

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
    CS --> SHIM["shimmer::find_shimmer_warnings()"]
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

Takes a pre-SSA `CFGModule` and emits assembly text matching the format
produced by `tcl::unsupported::disassemble` in Tcl 9.0.2.

```mermaid
flowchart LR
    CFG["CFGModule"] --> CG["codegen_module()"]
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
        SHIM["Shimmer (S100–S102)"]
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

```mermaid
flowchart LR
    EXPR["expr source string"] --> ELEX["ExprLexer<br/>tokenise_expr()"]
    ELEX --> EPAR["ExprParser<br/>parse_expr()"]
    EPAR --> EAST["ExprNode AST"]

    subgraph "ExprNode variants"
        direction TB
        LIT["ExprLiteral"]
        VAR["ExprVar"]
        CMD["ExprCommand"]
        STR["ExprString"]
        BIN["ExprBinary"]
        UNA["ExprUnary"]
        TER["ExprTernary"]
        CAL["ExprCall"]
        RAW["ExprRaw (fallback)"]
    end
```

`ExprRaw` is a fallback for expressions the parser cannot handle — every
consumer must treat it as opaque and conservative.

## Diagnostic code taxonomy

| Range | Category | Source |
|-------|----------|--------|
| E001–E003 | Arity/subcommand errors | Analyser |
| E1xx–E2xx | Syntax errors | Lexer / Recovery |
| H300 | Paste-error hints | Analyser |
| W001–W002 | Command warnings | Analyser |
| W100–W120 | Semantic & style warnings | Analyser / Diagnostics |
| W200–W214 | Variable & versioning warnings | Analyser |
| W300–W313 | Security warnings | Analyser |
| O100–O130 | Optimisation suggestions | Optimiser + GVN |
| S100–S102 | Shimmer / type thunking | Shimmer detection |
| T100–T106 | Taint / security | Taint analysis |
| IRULE1007–IRULE1008 | Collect/release pairing (side-aware) | iRules flow analysis |
| IRULE1xxx–5xxx | iRules-specific | iRules flow + taint |

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
| `rust/tcl-compiler/src/analyses.rs` | Core analysis result types (`FunctionAnalysis`, the lattices) |
| `rust/tcl-compiler/src/compilation_unit.rs` | Pipeline orchestration and caching |
| `rust/tcl-compiler/src/interprocedural.rs` | Call graph and procedure summaries |
| `rust/tcl-compiler/src/optimiser/` | Optimisation passes (O100–O130) and the pass manager |
| `rust/tcl-compiler/src/gvn.rs` | Global value numbering / CSE / PRE / LICM (O105–O106) |
| `rust/tcl-compiler/src/taint.rs`, `taint_interproc.rs` | Taint analysis for untrusted I/O (T100–T106) |
| `rust/tcl-compiler/src/shimmer/` | Type-representation issue detection (S100–S102) |
| `rust/tcl-compiler/src/codegen/` | Tcl VM bytecode assembly backend |
| `rust/tcl-compiler/src/side_effects.rs` | Command side-effect classification |
| `rust/tcl-compiler/src/types.rs` | Type lattice definitions |
| `rust/tcl-compiler/src/compiler_checks.rs` | Downstream-pass aggregation into diagnostics |
| `rust/tcl-lsp-server/src/lib.rs` | Tiered diagnostic scheduling and LSP publishing |
| `rust/tcl-lsp-core/src/graphs.rs` | Call / symbol / data-flow graph queries |
