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

- [compiler KCS index](kcs/compiler/README.md)

- [kcs-compiler-pipeline-overview.md](kcs/compiler/kcs-compiler-pipeline-overview.md)
- [kcs-lowering-contracts.md](kcs/compiler/kcs-lowering-contracts.md)
- [kcs-cfg-ssa-fact-model.md](kcs/compiler/kcs-cfg-ssa-fact-model.md)
- [kcs-diagnostics-integration.md](kcs/compiler/kcs-diagnostics-integration.md)
- [kcs-compilation-unit-contracts.md](kcs/compiler/kcs-compilation-unit-contracts.md)
- [kcs-downstream-pass-contracts.md](kcs/compiler/kcs-downstream-pass-contracts.md)
- [kcs-async-diagnostics-tiering.md](kcs/compiler/kcs-async-diagnostics-tiering.md)
- [kcs-bytecode-boundary.md](kcs/compiler/kcs-bytecode-boundary.md)
- [kcs-pass-authoring-checklist.md](kcs/compiler/kcs-pass-authoring-checklist.md)

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

    SPLIT --> ANA["<b>Semantic Analyser</b><br/>Analyser.analyse()"]
    SPLIT --> LOW["<b>IR Lowering</b><br/>lower_to_ir()"]

    LOW -->|IRModule| CFG["<b>CFG Construction</b><br/>build_cfg_function()"]
    CFG -->|CFGFunction| SSA["<b>SSA Construction</b><br/>build_ssa()"]
    SSA -->|SSAFunction| CORE["<b>Core Analyses</b><br/>analyse_function()"]
    LOW -->|IRModule| IPA["<b>Interprocedural Analysis</b><br/>analyse_interprocedural_ir()"]

    CORE -->|FunctionAnalysis| CU["<b>CompilationUnit</b>"]
    IPA -->|InterproceduralAnalysis| CU
    CFG --> CU

    CU --> OPT["<b>Optimiser</b><br/>find_optimisations()"]
    CU --> TAINT["<b>Taint Analysis</b><br/>find_taint_warnings()"]
    CU --> SHIM["<b>Shimmer Detection</b><br/>find_shimmer_warnings()"]
    CU --> GVN["<b>GVN / CSE</b><br/>find_redundant_computations()"]
    CU --> IFLOW["<b>iRules Flow</b><br/>find_irules_flow_warnings()"]
    CU --> ANA

    CFG --> CGEN["<b>Bytecode Codegen</b><br/>codegen_module()"]
    CGEN -->|"ModuleAsm"| ASMTXT["Assembly text"]

    ANA -->|AnalysisResult| DIAG["<b>Diagnostics Provider</b><br/>get_diagnostics()"]
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

**File:** `core/parsing/lexer.py` — class `TclLexer`

Converts raw source text into a flat stream of `Token` values.  Each token
carries its `TokenType`, text content, and precise `SourcePosition`
(line, column, byte offset).

```mermaid
flowchart LR
    SRC["source text"] --> LEX["TclLexer"]
    LEX --> TOK["Token stream"]

    subgraph Token
        direction TB
        TT["type: TokenType"]
        TX["text: str"]
        SP["start / end: SourcePosition"]
        IQ["in_quote: bool"]
    end

    TOK --- Token
```

Token types:

| Type | Meaning | Example |
|------|---------|---------|
| `STR` | Braced string | `{hello world}` |
| `VAR` | Variable reference | `$name`, `${arr(idx)}` |
| `CMD` | Command substitution | `[clock seconds]` |
| `ESC` | String / word fragment | `hello` |
| `SEP` | Whitespace separator | spaces, tabs |
| `EOL` | Command terminator | newline, `;` |
| `COMMENT` | Comment text | `# ...` |

The lexer handles Tcl-specific constructs: nested command substitutions,
`$var`, `${name}`, `$arr(idx)`, namespace separators `::`, backslash
escapes, and brace nesting.  Base-offset parameters allow the same lexer to
be re-entered for nested bodies (brace/bracket contents).

### 2. Command Segmenter

**File:** `core/parsing/command_segmenter.py` — function `segment_commands()`

Tcl has no traditional grammar — a "program" is a sequence of commands, each
being a list of whitespace-separated words terminated by a newline or
semicolon.  The segmenter groups the flat token stream into per-command
structures.

```mermaid
flowchart LR
    TOKS["Token stream"] --> SEGR["segment_commands()"]
    SEGR --> CMDS["list of SegmentedCommand"]

    subgraph SegmentedCommand
        direction TB
        AV["argv: list[Token]"]
        TX["texts: list[str]"]
        ST["single_token_word: list[bool]"]
        AT["all_tokens: list[Token]"]
        IP["is_partial: bool"]
    end

    CMDS --- SegmentedCommand
```

Multi-token words (adjacent tokens with no separator, e.g. `$prefix.txt`) are
concatenated into a single `texts` entry.  The `single_token_word` flags
record which words are atomic — important for downstream constant tracking.

### 3. Error Recovery

**File:** `core/parsing/recovery.py` — function `segment_with_recovery()`

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

**File:** `core/analysis/analyser.py` — class `Analyser`

A single-pass walk over segmented commands that builds a semantic model:
scopes, procedure definitions, variable definitions, command invocations.
It pattern-matches command names and dispatches to specialised handlers.

```mermaid
flowchart TD
    CMDS["SegmentedCommand list"] --> ANA["Analyser.analyse()"]
    ANA --> BODY["_analyse_body()"]
    BODY --> |"per command"| PROC["_process_command()"]

    PROC --> MATCH{"Match<br/>command name"}
    MATCH --> H_PROC["_handle_proc()"]
    MATCH --> H_SET["_handle_set()"]
    MATCH --> H_IF["_handle_if()"]
    MATCH --> H_FOR["_handle_for()"]
    MATCH --> H_CALL["_handle_call()"]
    MATCH --> H_ETC["...other handlers"]

    ANA --> VUSG["_emit_variable_usage_diagnostics()"]
    ANA --> VCFG["_emit_cfg_ssa_diagnostics()"]

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

**File:** `core/compiler/lowering.py` — function `lower_to_ir()`

Converts segmented commands into a structured Intermediate Representation.
Each Tcl command maps to a typed IR node.

```mermaid
flowchart LR
    SRC["source text"] --> LOW["lower_to_ir()"]
    LOW --> MOD["IRModule"]

    subgraph IRModule
        direction TB
        TL["top_level: IRScript"]
        PD["procedures: dict[str, IRProcedure]"]
    end

    MOD --- IRModule
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
        +IRScript|None else_body
    }
    class IRFor {
        +Range range
        +IRScript init
        +ExprNode condition
        +IRScript next
        +IRScript body
    }
    class IRWhile {
        +Range range
        +ExprNode condition
        +IRScript body
    }
    class IRForeach {
        +Range range
        +tuple iterators
        +IRScript body
    }
    class IRCatch {
        +Range range
        +IRScript body
        +str|None result_var
    }
    class IRTry {
        +Range range
        +IRScript body
        +tuple~IRTryHandler~ handlers
        +IRScript|None finally_body
    }
    class IRSwitch {
        +Range range
        +str subject
        +tuple~IRSwitchArm~ arms
    }
    class IRScript {
        +tuple~IRStatement~ statements
    }
    class IRModule {
        +IRScript top_level
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
    IRScript --* IRStatement
    IRModule --* IRScript
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

**File:** `core/compiler/cfg.py` — function `build_cfg_function()`

Flattens structured IR (`IRIf`, `IRFor`, `IRSwitch`, etc.) into basic blocks
with explicit control-flow edges.

```mermaid
flowchart TD
    IR["IRScript"] --> BUILD["build_cfg_function()"]
    BUILD --> FN["CFGFunction"]

    subgraph CFGFunction
        direction TB
        EN["entry: str"]
        BL["blocks: dict[str, CFGBlock]"]
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

**File:** `core/compiler/ssa.py` — function `build_ssa()`

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

**File:** `core/compiler/core_analyses.py` — function `analyse_function()`

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

**File:** `core/compiler/interprocedural.py` — function `analyse_interprocedural_ir()`

Builds conservative procedure summaries across the entire module:

```mermaid
flowchart TD
    IR["IRModule"] --> IPA["analyse_interprocedural_ir()"]
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

**File:** `core/compiler/compilation_unit.py` — function `compile_source()`

`CompilationUnit` remains the shared artefact boundary for IR/CFG/SSA/interprocedural
facts consumed across diagnostics and downstream passes. For operational contracts,
cache semantics, and regression anchors, use:

- [kcs-compilation-unit-contracts.md](kcs/compiler/kcs-compilation-unit-contracts.md)

```mermaid
flowchart TD
    SRC["source text"] --> CS["compile_source()"]

    CS --> LOW["lower_to_ir() → IRModule"]
    LOW --> CFG_T["build_cfg_function(::top)"]
    LOW --> CFG_P["build_cfg_function() per proc"]
    CFG_T --> SSA_T["build_ssa()"]
    CFG_P --> SSA_P["build_ssa()"]
    SSA_T --> ANA_T["analyse_function()"]
    SSA_P --> ANA_P["analyse_function()"]

    ANA_T --> TU["FunctionUnit (top-level)"]
    ANA_P --> PU["FunctionUnit per proc"]

    LOW --> IPA["analyse_interprocedural_ir()"]
    PU --> IPA

    TU --> CU["CompilationUnit"]
    PU --> CU
    IPA --> CU
    LOW --> CU

    subgraph CompilationUnit
        direction TB
        S["source"]
        IM["ir_module: IRModule"]
        CM["cfg_module: CFGModule"]
        TL["top_level: FunctionUnit"]
        PR["procedures: dict[str, FunctionUnit]"]
        IP["interproc: InterproceduralAnalysis"]
        CO["connection_scope: ConnectionScope|None"]
    end

    CU --- CompilationUnit
```

### 11. Downstream Analysis Passes

All downstream passes consume the `CompilationUnit` and produce typed warnings
converted by the diagnostics provider. Contract details, ownership guidance,
and pass/test anchors are tracked in:

- [kcs-downstream-pass-contracts.md](kcs/compiler/kcs-downstream-pass-contracts.md)
- [kcs-pass-authoring-checklist.md](kcs/compiler/kcs-pass-authoring-checklist.md)

```mermaid
flowchart LR
    CU["CompilationUnit"]

    CU --> OPT["<b>Optimiser</b><br/>O100–O125"]
    CU --> TAINT["<b>Taint Analysis</b><br/>T100–T201, IRULE3xxx"]
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

**File:** `lsp/features/diagnostics.py` — function `get_diagnostics()`

`get_diagnostics()` is the policy boundary that merges analyser + pass findings,
applies suppression/disable rules, and converts to LSP diagnostics. Integration
contracts now live in:

- [kcs-diagnostics-integration.md](kcs/compiler/kcs-diagnostics-integration.md)
- [kcs-async-diagnostics-tiering.md](kcs/compiler/kcs-async-diagnostics-tiering.md)

```mermaid
flowchart TD
    SRC["source text"] --> GD["get_diagnostics()"]

    GD --> CS["compile_source() → CU"]
    GD --> AN["analyse() → AnalysisResult"]

    GD --> STYLE["Style checks<br/>W111 line length<br/>W112 trailing whitespace<br/>W115 comment continuation"]

    CS --> OPT["find_optimisations()"]
    CS --> SHIM["find_shimmer_warnings()"]
    CS --> TAINT["find_taint_warnings()"]
    CS --> GVN["find_redundant_computations()"]
    CS --> IFLOW["find_irules_flow_warnings()"]

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

**File:** `core/compiler/codegen.py` — functions `codegen_function()`, `codegen_module()`

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

**File:** `lsp/async_diagnostics.py` — class `DiagnosticScheduler`

Tiered publishing and cancellation rules are maintained in:

- [kcs-async-diagnostics-tiering.md](kcs/compiler/kcs-async-diagnostics-tiering.md)

```mermaid
flowchart TD
    EDIT["Document edit"] --> BASIC["<b>Tier 1: Basic</b><br/>Lexer/parser errors (E-codes)<br/>Analysis warnings (W-codes)<br/>Style checks"]
    BASIC -->|"Publish immediately"| ED["Editor"]

    EDIT --> DEEP["<b>Tier 2: Deep</b><br/>asyncio.to_thread()"]

    subgraph "Background thread"
        direction TB
        OPT["Optimiser (O100–O125)"]
        SHIM["Shimmer (S100–S102)"]
        TAINT["Taint (T100–T201)"]
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
| O100–O125 | Optimisation suggestions | Optimiser + GVN |
| S100–S102 | Shimmer / type thunking | Shimmer detection |
| T100–T106 | Taint / security | Taint analysis |
| T200–T201 | Collect/release pairing | Taint analysis |
| IRULE1xxx–5xxx | iRules-specific | iRules flow + taint |

## Key source files

| File | Responsibility |
|------|---------------|
| `core/parsing/lexer.py` | Tokenisation with position tracking |
| `core/parsing/tokens.py` | Token, SourcePosition, TokenType definitions |
| `core/parsing/command_segmenter.py` | Command segmentation and chunking |
| `core/parsing/recovery.py` | Virtual token injection for unclosed delimiters |
| `core/parsing/expr_lexer.py` | Expression tokenisation |
| `core/parsing/expr_parser.py` | Expression parsing to ExprNode AST |
| `core/parsing/substitution.py` | Tcl backslash substitution helpers |
| `core/analysis/analyser.py` | Semantic analysis, scope tracking |
| `core/analysis/checks/` | Best-practice and security checks (W-series) |
| `core/analysis/irules_checks.py` | iRules-specific checks (IRULE-series) |
| `core/analysis/semantic_model.py` | AnalysisResult, Diagnostic, Scope, ProcDef |
| `core/compiler/ir.py` | IR node definitions |
| `core/compiler/lowering.py` | IR construction from token stream |
| `core/compiler/cfg.py` | Control flow graph construction |
| `core/compiler/ssa.py` | SSA form construction |
| `core/compiler/core_analyses.py` | SCCP, liveness, type inference |
| `core/compiler/compilation_unit.py` | Pipeline orchestration and caching |
| `core/compiler/interprocedural.py` | Call graph and procedure summaries |
| `core/compiler/optimiser/` | Optimisation passes (O100–O125) |
| `core/compiler/gvn.py` | Global value numbering / CSE / PRE / LICM (O105–O106) |
| `core/compiler/taint/` | Taint analysis for untrusted I/O (T100–T201) |
| `core/compiler/shimmer.py` | Type representation issue detection (S100–S102) |
| `core/compiler/irules_flow.py` | iRules control-flow checks |
| `core/compiler/codegen.py` | Tcl VM bytecode assembly backend |
| `core/compiler/effects.py` | Command side-effect classification |
| `core/compiler/types.py` | Type lattice definitions |
| `lsp/async_diagnostics.py` | Background diagnostic scheduler (tiered publishing) |
| `lsp/features/diagnostics.py` | LSP diagnostic aggregation |
| `core/analysis/semantic_graph.py` | Call/symbol/data-flow graph queries |
