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
| **AST** | Abstract Syntax Tree — a tree representation of parsed source code structure. In this compiler, expression bodies (`expr {…}`) are parsed into `ExprNode` AST trees ([`expr_ast.py:174`](../core/compiler/expr_ast.py)). |
| **Basic block** | A straight-line sequence of IR statements with no branches except at the end.  Represented by [`CFGBlock`](../core/compiler/cfg.py) (`cfg.py:374`). |
| **CFG** | Control Flow Graph — a directed graph of basic blocks connected by jumps and branches.  Built by [`build_cfg()`](../core/compiler/cfg.py) (`cfg.py:1058`). |
| **Dominator / idom** | Block A *dominates* block B if every path from the entry to B passes through A.  The *immediate dominator* (`idom`) is the closest dominator.  Stored in [`SSAFunction.idom`](../core/compiler/ssa.py) (`ssa.py:210`). |
| **Dominance frontier** | The set of blocks where a variable's dominance "ends" — these are where phi nodes must be inserted.  Stored in [`SSAFunction.dominance_frontier`](../core/compiler/ssa.py) (`ssa.py:210`). |
| **GVN** | Global Value Numbering — an optimisation that detects redundant computations by assigning a canonical identity to each expression.  See [`gvn.py:76`](../core/compiler/gvn.py). |
| **IR** | Intermediate Representation — a structured, typed representation of Tcl commands between parsing and code generation.  Defined in [`ir.py`](../core/compiler/ir.py); the union type `IRStatement` (`ir.py:265`) covers all statement kinds. |
| **Lattice** | A mathematical structure used in dataflow analysis where values flow from *bottom* (unknown) toward *top* (overdefined).  The SCCP value lattice is [`LatticeValue`](../core/compiler/core_analyses.py) (`core_analyses.py:111`); the type lattice is [`TypeLattice`](../core/compiler/types.py) (`types.py:53`). |
| **Liveness** | A dataflow analysis that determines which SSA values are "live" (may still be read) at each program point.  Results are in [`FunctionAnalysis.live_in / live_out`](../core/compiler/core_analyses.py) (`core_analyses.py:176`). |
| **LVT** | Local Variable Table — maps variable names to integer slot indices for fast access inside procedures.  See [`LocalVarTable`](../core/compiler/codegen/_types.py) (`codegen/_types.py:63`). |
| **Phi node (φ)** | An SSA construct placed at control flow merge points.  `φ(x₁, x₃)` means "use `x₁` if control arrived from predecessor 1, or `x₃` if from predecessor 2."  Represented by [`SSAPhi`](../core/compiler/ssa.py) (`ssa.py:168`). |
| **SCCP** | Sparse Conditional Constant Propagation — a combined constant propagation and unreachable-code analysis that runs over the SSA graph.  Implemented in [`analyse_function()`](../core/compiler/core_analyses.py) (`core_analyses.py:1210`). |
| **Shimmer** | Tcl's internal type coercion: when a value's string representation is reinterpreted as a different type (e.g. `"42"` read as an integer).  Tracked by `TypeLattice.SHIMMERED` (`types.py:53`). |
| **SSA** | Static Single Assignment — a form where every variable is defined exactly once.  Multiple definitions of the same source variable get unique *version numbers* (e.g. `x₁`, `x₂`).  Built by [`build_ssa()`](../core/compiler/ssa.py) (`ssa.py:359`). |
| **SSA value key** | A `(variable_name, version)` tuple that uniquely identifies one definition of a variable.  Type alias [`SSAValueKey`](../core/compiler/ssa.py) (`ssa.py:50`). |
| **Taint analysis** | Tracks whether values originate from untrusted sources (user input).  Uses [`TaintLattice`](../core/compiler/taint/_lattice.py) (`taint/_lattice.py:44`). |
| **Taint colour** | A `Flag` enum describing safety properties of tainted data (e.g. `CRLF_FREE`, `URL_ENCODED`, `HTML_ESCAPED`).  Colours compose with `\|` and join by intersection (`&`) — only properties shared by all incoming paths survive.  Defined in [`TaintColour`](../core/commands/registry/taint_hints.py) (`taint_hints.py:17`). |
| **Taint source** | A command whose return value introduces tainted data (e.g. `HTTP::host`, `HTTP::uri`).  Declared via `TaintHint.source` on the command's registry spec (`taint_hints.py:60`). |
| **Taint sink** | A dangerous argument position where tainted data can cause harm (XSS, header injection, SSRF).  Classified by [`_classify_sink()`](../core/compiler/taint/_sinks.py) (`taint/_sinks.py:99`). |
| **CSE** | Common Subexpression Elimination — detects when the same pure computation is evaluated more than once and suggests extracting it to a variable.  Part of the GVN pass, reported as `O105`.  See [`gvn.py`](../core/compiler/gvn.py). |
| **ICIP** | Interprocedural Constant/Inline Propagation — evaluates procedure calls with known constant arguments at compile time and replaces the call with the result.  Reported as `O103`.  See [`optimise_static_proc_calls()`](../core/compiler/optimiser/_propagation.py) (`_propagation.py:271`). |
| **LCP** | Loop Constant Propagation / Code Sinking — moves invariant assignments out of the hot path into the specific branch that uses them.  Reported as `O125`.  See [`_code_sinking.py`](../core/compiler/optimiser/_code_sinking.py). |
| **DCE** | Dead Code Elimination — removes code whose result is never used.  `O107` (basic DCE), `O108` (aggressive DCE tracking statement liveness), `O109` (dead store elimination).  See [`_elimination.py`](../core/compiler/optimiser/_elimination.py). |
| **InstCombine** | Instruction Combine — canonicalises and simplifies expressions by applying algebraic identities (e.g. `$x * 1` → `$x`, DeMorgan's law).  Reported as `O110`.  See [`_expr_simplify.py`](../core/compiler/optimiser/_expr_simplify.py). |
| **CommandSpec** | The central metadata type for a Tcl command — describes its argument layout, purity, side effects, taint properties, event validity, and dialect membership.  See [`models.py:462`](../core/commands/registry/models.py). |
| **SubCommand** | An ensemble operation selected by the first argument (e.g. `string length`, `HTTP::header value`).  Each has its own arity, purity, return type, and taint transform hooks.  See [`models.py:319`](../core/commands/registry/models.py). |
| **FormSpec** | An invocation form of a command — getter (reads state) or setter (writes state), each with its own arity and side-effect classification.  See [`models.py:249`](../core/commands/registry/models.py). |

---

## Pipeline stage summary

Every Tcl source string passes through these stages (the orchestrating
entry point is [`compile_source()`](../core/compiler/compilation_unit.py)
at `compilation_unit.py:89`):

```
Source text
  │
  ▼
┌───────────────────────────────────────────────────────────────────────┐
│ 1. Lexer         TclLexer.tokenise_all()  → list[Token]              │  lexer.py:494
│ 2. Segmenter     segment_commands()       → list[SegmentedCommand]   │  command_segmenter.py:390
│ 3. IR Lowering   lower_to_ir()            → IRModule                 │  lowering.py:1020
│ 4. CFG           build_cfg() / build_cfg_function()  → CFGModule     │  cfg.py:1058
│ 5. SSA           build_ssa()              → SSAFunction              │  ssa.py:359
│ 6. Core analyses analyse_function()       → FunctionAnalysis         │  core_analyses.py:1210
│ 7. Codegen       codegen_module()         → ModuleAsm                │  codegen/_emitter.py:904
└───────────────────────────────────────────────────────────────────────┘
```

---

## Data structure reference

Before diving into examples, here are the key types that appear at each stage.
All types live under `core/` and are frozen dataclasses unless noted.

### Stage 1 — Lexer types ([`core/parsing/tokens.py`](../core/parsing/tokens.py))

```python
class TokenType(Enum):              # tokens.py:9
    ESC     # plain string / word fragment (possibly with escape sequences)
    STR     # braced string {…}
    CMD     # command substitution [… ]
    VAR     # variable substitution $name
    SEP     # whitespace separator
    EOL     # end-of-line / semicolon
    EOF     # end-of-input
    COMMENT # comment (# to end of line)
    EXPAND  # {*} expansion prefix

@dataclass(frozen=True, slots=True)
class SourcePosition:               # tokens.py:24
    line: int       # 0-based line number
    character: int  # 0-based column (UTF-16 code units per LSP spec)
    offset: int     # byte offset into the source string

@dataclass(frozen=True, slots=True)
class Token:                        # tokens.py:33
    type: TokenType
    text: str
    start: SourcePosition
    end: SourcePosition
    in_quote: bool = False
```

- `Token.type` distinguishes variables (`$x` → `VAR`), braced strings
  (`{hello}` → `STR`), command substitutions (`[foo]` → `CMD`), and plain
  word fragments (`set` → `ESC`).
- `SourcePosition` tracks the exact location in source text.  `offset` is
  the byte offset for fast slicing; `line`/`character` are 0-based for LSP.

### Stage 2 — Segmenter types ([`core/parsing/command_segmenter.py`](../core/parsing/command_segmenter.py))

```python
@dataclass(slots=True)
class SegmentedCommand:              # command_segmenter.py:65
    range: Range                         # source span of the entire command
    argv: list[Token]                    # first token of each word
    texts: list[str]                     # concatenated text per word
    single_token_word: list[bool]        # True when a word is a single token
    all_tokens: list[Token]              # every token in the command
    preceding_comment: str | None = None
    is_partial: bool = False             # True when unclosed delimiter detected
    partial_delimiter: UnclosedDelimiter | None = None
    expand_word: list[bool] | None = None  # {*} expansion per word
```

- `texts[0]` is the command name, `texts[1:]` are the arguments.
- `single_token_word[i]` is `True` when word `i` is a single atomic token
  (no interpolation) — important for constant tracking downstream.
- `argv[i]` is the *first* token of word `i`; multi-token words (e.g.
  `$prefix.txt`) are concatenated into `texts[i]`.

### Stage 3 — IR types ([`core/compiler/ir.py`](../core/compiler/ir.py))

```python
@dataclass(frozen=True, slots=True)
class IRAssignConst:                     # ir.py:35 — set a 1
    range: Range
    name: str                            # variable name
    value: str                           # constant string value

@dataclass(frozen=True, slots=True)
class IRAssignExpr:                      # ir.py:42 — set x [expr {$a + 1}]
    range: Range
    name: str
    expr: ExprNode                       # parsed expression AST

@dataclass(frozen=True, slots=True)
class IRAssignValue:                     # ir.py:49 — set x $y, set x "hello $name"
    range: Range
    name: str
    value: str                           # interpolated value text
    value_needs_backsubst: bool = False

@dataclass(frozen=True, slots=True)
class IRIncr:                            # ir.py:57 — incr i, incr i 5
    range: Range
    name: str
    amount: str | None = None            # None means +1

@dataclass(frozen=True, slots=True)
class IRCall:                            # ir.py:76 — generic command invocation (puts, append, etc.)
    range: Range
    command: str
    args: tuple[str, ...] = ()
    defs: tuple[str, ...] = ()           # variables this command defines
    reads: tuple[str, ...] = ()          # variables this command reads by name
    reads_own_defs: bool = False         # True for read-modify-write (append, lappend)
    tokens: CommandTokens | None = None

@dataclass(frozen=True, slots=True)
class IRReturn:                          # ir.py:99
    range: Range
    value: str | None = None

@dataclass(frozen=True, slots=True)
class IRBarrier:                         # ir.py:105 — eval, uplevel, upvar — defeats static analysis
    range: Range
    reason: str
    command: str = ""
    args: tuple[str, ...] = ()
    tokens: CommandTokens | None = None

@dataclass(frozen=True, slots=True)
class IRIf:                              # ir.py:137
    range: Range
    clauses: tuple[IRIfClause, ...]      # one per if/elseif branch
    else_body: IRScript | None = None

@dataclass(frozen=True, slots=True)
class IRIfClause:                        # ir.py:129
    condition: ExprNode                  # parsed condition expression
    condition_range: Range
    body: IRScript
    body_range: Range

@dataclass(frozen=True, slots=True)
class IRFor:                             # ir.py:145
    range: Range
    init: IRScript                       # {set i 0}
    init_range: Range
    condition: ExprNode                  # {$i < 10}
    condition_range: Range
    next: IRScript                       # {incr i}
    next_range: Range
    body: IRScript
    body_range: Range

@dataclass(frozen=True, slots=True)
class IRWhile:                           # ir.py:159
    range: Range
    condition: ExprNode
    condition_range: Range
    body: IRScript
    body_range: Range

@dataclass(frozen=True, slots=True)
class IRForeach:                         # ir.py:171
    range: Range
    iterators: tuple[tuple[tuple[str, ...], str], ...]  # ((var_names, list_arg), ...)
    body: IRScript
    body_range: Range
    is_lmap: bool = False

@dataclass(frozen=True, slots=True)
class IRScript:                          # ir.py:124
    statements: tuple[IRStatement, ...] = ()

@dataclass
class IRModule:                          # ir.py:259
    top_level: IRScript                  # statements outside any proc
    procedures: dict[str, IRProcedure]   # qualified_name → proc definition
    redefined_procedures: set[str]

# IRStatement is a union type (ir.py:265):
IRStatement = (
    IRAssignConst | IRAssignExpr | IRAssignValue | IRExprEval
    | IRIncr | IRCall | IRReturn | IRBarrier
    | IRIf | IRFor | IRWhile | IRForeach | IRCatch | IRTry | IRSwitch
)
```

- Every IR node carries a `Range` (start/end `SourcePosition`) for precise
  diagnostic mapping.
- `IRBarrier` marks commands (`eval`, `uplevel`, `upvar`) whose side effects
  defeat static analysis — no constant propagation or dead-store reasoning
  can cross them.
- Expression conditions are parsed into `ExprNode` AST trees at lowering time.

### Expression AST ([`core/compiler/expr_ast.py`](../core/compiler/expr_ast.py))

```python
@dataclass(frozen=True)
class ExprLiteral:     # expr_ast.py:90 — 42, 3.14, true
    text: str; start: int; end: int

@dataclass(frozen=True)
class ExprVar:         # expr_ast.py:108 — $x, ${arr(idx)}
    text: str; name: str; start: int; end: int

@dataclass(frozen=True)
class ExprBinary:      # expr_ast.py:127 — $a + $b, $x < 10
    op: BinOp; left: ExprNode; right: ExprNode

@dataclass(frozen=True)
class ExprUnary:       # expr_ast.py:136 — -$x, !$flag
    op: UnaryOp; operand: ExprNode

@dataclass(frozen=True)
class ExprCall:        # expr_ast.py:153 — sin($x), int($y)
    function: str; args: tuple[ExprNode, ...]; start: int; end: int

@dataclass(frozen=True)
class ExprCommand:     # expr_ast.py:118 — [clock seconds]
    text: str; start: int; end: int

@dataclass(frozen=True)
class ExprRaw:         # expr_ast.py:163 — fallback for unparseable expressions
    text: str

# BinOp (expr_ast.py:27): ADD, SUB, MUL, DIV, MOD, POW, LT, LE, GT, GE, EQ, NE,
#                          STR_EQ, STR_NE, AND, OR, LSHIFT, RSHIFT, BIT_AND, ...
# UnaryOp (expr_ast.py:76): NEG, POS, BIT_NOT, NOT
```

### Stage 4 — CFG types ([`core/compiler/cfg.py`](../core/compiler/cfg.py))

```python
@dataclass(frozen=True, slots=True)
class CFGGoto:         # cfg.py:351 — unconditional jump
    target: str                          # target block name

@dataclass(frozen=True, slots=True)
class CFGBranch:       # cfg.py:357 — conditional jump
    condition: ExprNode                  # condition expression
    true_target: str
    false_target: str

@dataclass(frozen=True, slots=True)
class CFGReturn:       # cfg.py:365 — procedure exit
    value: str | None = None

CFGTerminator = CFGGoto | CFGBranch | CFGReturn  # cfg.py:370

@dataclass(frozen=True, slots=True)
class CFGBlock:        # cfg.py:374
    name: str
    statements: tuple[IRStatement, ...]  # straight-line IR statements
    terminator: CFGTerminator | None     # exactly one per block

@dataclass(frozen=True, slots=True)
class CFGFunction:     # cfg.py:381
    name: str
    entry: str                           # entry block name
    blocks: dict[str, CFGBlock]          # name → block
    loop_nodes: dict[str, tuple[str, IRFor]]  # for-loop metadata

@dataclass(frozen=True, slots=True)
class CFGModule:       # cfg.py:389
    top_level: CFGFunction
    procedures: dict[str, CFGFunction]
```

### Stage 5 — SSA types ([`core/compiler/ssa.py`](../core/compiler/ssa.py))

```python
SSAVersion = int               # ssa.py:44 — each definition gets a unique version
SSAValueKey = tuple[str, int]  # ssa.py:50 — (variable_name, version) — unique SSA value

@dataclass(frozen=True, slots=True)
class SSAPhi:                  # ssa.py:168 — phi node (see Glossary)
    name: str                            # variable name
    version: SSAVersion                  # version produced by this phi
    incoming: dict[str, SSAVersion]      # predecessor_block → version

@dataclass(frozen=True, slots=True)
class SSAStatement:            # ssa.py:181
    statement: IRStatement               # the original IR statement
    uses: dict[str, SSAVersion]          # variables read → their versions
    defs: dict[str, SSAVersion]          # variables written → new versions

@dataclass(frozen=True, slots=True)
class SSABlock:                # ssa.py:195
    name: str
    phis: tuple[SSAPhi, ...]             # phi nodes at merge points
    statements: tuple[SSAStatement, ...]
    entry_versions: dict[str, SSAVersion]
    exit_versions: dict[str, SSAVersion]

@dataclass(frozen=True, slots=True)
class SSAFunction:             # ssa.py:210
    name: str
    entry: str
    blocks: dict[str, SSABlock]
    idom: dict[str, str | None]          # immediate dominator tree (see Glossary)
    dominance_frontier: dict[str, tuple[str, ...]]  # (see Glossary)
    dominator_tree: dict[str, tuple[str, ...]]
```

### Stage 6 — Analysis types ([`core/compiler/core_analyses.py`](../core/compiler/core_analyses.py))

```python
class LatticeKind(Enum):       # core_analyses.py:104 (see Glossary → Lattice)
    UNKNOWN      # not yet analysed (bottom)
    CONST        # provably constant
    OVERDEFINED  # multiple possible values (top)

@dataclass(frozen=True, slots=True)
class LatticeValue:            # core_analyses.py:111
    kind: LatticeKind
    value: int | float | bool | str | None = None

@dataclass(frozen=True, slots=True)
class FunctionAnalysis:        # core_analyses.py:176 — produced by analyse_function() (line 1210)
    live_in: dict[str, set[SSAValueKey]]     # (see Glossary → Liveness)
    live_out: dict[str, set[SSAValueKey]]
    dead_stores: tuple[DeadStore, ...]       # DeadStore at line 154
    unreachable_blocks: set[str]
    constant_branches: tuple[ConstantBranch, ...]  # ConstantBranch at line 145
    values: dict[SSAValueKey, LatticeValue]     # SCCP results (see Glossary → SCCP)
    types: dict[SSAValueKey, TypeLattice]        # type inference results
    read_before_set: tuple[ReadBeforeSet, ...]   # ReadBeforeSet at line 162
    unused_variables: tuple[UnusedVariable, ...]  # UnusedVariable at line 169
```

#### Type lattice ([`core/compiler/types.py`](../core/compiler/types.py))

```python
class TclType(Enum):           # types.py:30
    STRING = auto()
    INT = auto()
    DOUBLE = auto()
    BOOLEAN = auto()
    LIST = auto()
    DICT = auto()
    BYTEARRAY = auto()
    NUMERIC = auto()   # abstract join of INT and DOUBLE

class TypeKind(Enum):          # types.py:43
    UNKNOWN      # not yet analysed (bottom)
    KNOWN        # concrete type determined
    SHIMMERED    # forced type change detected (see Glossary → Shimmer)
    OVERDEFINED  # multiple incompatible types (top)

@dataclass(frozen=True, slots=True)
class TypeLattice:             # types.py:53
    kind: TypeKind
    tcl_type: TclType | None = None
    from_type: TclType | None = None  # only for SHIMMERED

# Lattice order:  UNKNOWN < KNOWN(t) < SHIMMERED(a,b) < OVERDEFINED
```

### Stage 7 — Codegen types ([`core/compiler/codegen/`](../core/compiler/codegen/))

```python
class Op(Enum):        # codegen/opcodes.py:61 — ~100 Tcl 9.0.2 bytecode opcodes
    PUSH1, PUSH4, POP, DUP,
    LOAD_SCALAR1, STORE_SCALAR1,
    INVOKE_STK1, INVOKE_STK4,
    JUMP1, JUMP4, JUMP_TRUE1, JUMP_FALSE1,
    ADD, SUB, MULT, DIV, LT, GT, EQ, NEQ,
    ...

@dataclass(slots=True)
class Instruction:     # codegen/_types.py:11
    op: Op
    operands: tuple[int | str, ...]  # int = literal/imm, str = label ref
    comment: str = ""
    offset: int = -1                 # filled by layout pass

class LiteralTable:    # codegen/_types.py:32 — intern pool: string → object-array index
class LocalVarTable:   # codegen/_types.py:63 — LVT: variable name → slot index (see Glossary)

@dataclass(slots=True)
class FunctionAsm:     # codegen/_types.py:91
    name: str
    literals: LiteralTable
    lvt: LocalVarTable
    instructions: list[Instruction]
    labels: dict[str, int]           # label → byte offset

@dataclass(slots=True)
class ModuleAsm:       # codegen/_types.py:105
    top_level: FunctionAsm
    procedures: dict[str, FunctionAsm]
```

### Orchestration ([`core/compiler/compilation_unit.py`](../core/compiler/compilation_unit.py))

```python
@dataclass(frozen=True, slots=True)
class FunctionUnit:                # compilation_unit.py:27
    cfg: CFGFunction
    ssa: SSAFunction
    analysis: FunctionAnalysis
    execution_intent: FunctionExecutionIntent

@dataclass(frozen=True, slots=True)
class CompilationUnit:             # compilation_unit.py:37 — produced by compile_source() (line 89)
    source: str
    ir_module: IRModule
    cfg_module: CFGModule
    top_level: FunctionUnit
    procedures: dict[str, FunctionUnit]
    interproc: InterproceduralAnalysis            # interprocedural.py:83
    connection_scope: ConnectionScope | None = None  # connection_scope.py:38
```

---

## Command infrastructure

The compiler's view of every Tcl command — its argument layout, purity,
side effects, taint properties, event validity, and dialect membership —
comes from the **command registry**.  This section explains each layer
of that infrastructure and how the pieces connect.

### Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                     CommandRegistry                             │
│                                                                 │
│   ┌───────────┐  ┌───────────┐  ┌──────────┐  ┌─────────────┐  │
│   │ Tcl defs  │  │ iRules    │  │ iApps    │  │ Tk / tcllib │  │
│   │ (tcl/*.py)│  │ (irules/) │  │ (iapps/) │  │ (tk/ stdlib)│  │
│   └─────┬─────┘  └─────┬─────┘  └────┬─────┘  └──────┬──────┘  │
│         │              │              │               │         │
│         ▼              ▼              ▼               ▼         │
│                  CommandSpec                                     │
│     ┌──────────────────────────────────────────┐                │
│     │ name, dialects, forms, subcommands,      │                │
│     │ validation, event_requires, pure,        │                │
│     │ side_effect_hints, taint_sink, ...       │                │
│     └──────────────────────────────────────────┘                │
│         │          │          │          │                       │
│         ▼          ▼          ▼          ▼                       │
│     FormSpec   SubCommand   TaintHint   SideEffect              │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

Every command is defined as a `CommandDef` subclass (`_base.py:48`) whose
`spec()` classmethod returns a `CommandSpec` (`models.py:462`).  At import
time, the `@register` decorator adds each definition to its dialect's
registry list, and the singleton `CommandRegistry` (`command_registry.py:63`)
merges all dialect lists into a unified lookup table.

### CommandDef — defining a command

```python
class CommandDef:                      # _base.py:48
    name: str                          # command name (e.g. "string", "HTTP::host")

    @classmethod
    def spec(cls) -> CommandSpec: ...  # returns the full metadata

    @classmethod
    def taint_hints(cls) -> TaintHint | None: ...  # optional taint metadata
```

Each dialect sub-package (`tcl/`, `irules/`, `iapps/`, `tk/`) has its own
`_REGISTRY` list and `@register` decorator, both created by `make_registry()`
(`_base.py:75`).

**Concrete example** — `string` (`tcl/string.py`):

```python
@register
class StringCommand(CommandDef):
    name = "string"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="string",
            forms=(FormSpec(kind=FormKind.DEFAULT, synopsis="string option arg ?arg ...?", ...),),
            subcommands={"length": SubCommand(name="length", arity=Arity(1,1), pure=True, ...), ...},
            validation=ValidationSpec(arity=Arity(1)),
            cse_candidate=True,
            ...
        )
```

### FormSpec — invocation forms

A single command can have multiple invocation forms — e.g. a getter
(no args, reads state) and a setter (one arg, writes state).  Each
form is a `FormSpec` (`models.py:249`):

```python
@dataclass(frozen=True, slots=True)
class FormSpec:                        # models.py:249
    kind: FormKind                     # DEFAULT, GETTER, or SETTER
    synopsis: str                      # human-readable signature
    arity: Arity | None = None         # per-form arg count (None → inherit)
    options: tuple[OptionSpec, ...] = ()  # valid switch options
    pure: bool = False                 # no side effects
    mutator: bool = False              # modifies external state
    side_effect_hints: tuple[SideEffect, ...] = ()  # structured effects
    arg_values: dict[int, tuple[ArgumentValueSpec, ...]] = {}  # completable values
```

**Form resolution** — `CommandSpec.resolve_form(args)` (`models.py:600`)
matches actual arguments against per-form arities.  Example for
`HTTP::host`:

```python
forms=(
    FormSpec(kind=FormKind.GETTER, synopsis="HTTP::host", arity=Arity(0, 0),
             pure=True,  # reading is side-effect-free
             side_effect_hints=(SideEffect(target=HTTP_HEADER, reads=True, ...),)),
)
```

When a command has both getter and setter forms, the resolved form determines
whether the invocation is pure (a read) or a mutator (a write).

### Arity — argument count constraints

```python
@dataclass(frozen=True, slots=True)
class Arity:                           # signatures.py:34
    min: int = 0                       # minimum args (after command name)
    max: int = sys.maxsize             # max args (sys.maxsize = unlimited)

    def accepts(self, n: int) -> bool: # True when min ≤ n ≤ max
```

The `ValidationSpec` on each `CommandSpec` holds the overall arity.  The
arity checker emits diagnostic `W101` (wrong number of arguments) when
an invocation falls outside the bounds.

### SubCommand — ensemble commands

Commands like `string`, `dict`, `info`, and `HTTP::header` use
subcommands (the first argument selects the operation).  Each
subcommand is a `SubCommand` (`models.py:319`):

```python
@dataclass(slots=True)
class SubCommand:                      # models.py:319
    name: str                          # "length", "match", "replace", ...
    arity: Arity                       # arg count for this subcommand
    pure: bool = False                 # no side effects
    mutator: bool = False              # modifies state
    return_type: TclType | None        # return value type
    options: tuple[OptionSpec, ...] = ()      # per-subcommand options
    option_terminator: OptionTerminatorSpec | None = None
    arg_values: dict[int, tuple[ArgumentValueSpec, ...]] = {}  # completions
    deprecated_replacement: str | None = None  # replacement command name
    dialects: frozenset[str] | None = None     # None = inherit from parent
    forms: tuple[FormSpec, ...] = ()   # per-subcommand getter/setter forms
    handler: SubcommandHandler | None  # VM execution hook
    codegen: CodegenHook | None        # bytecode specialisation
    lowering: LoweringHook | None      # IR lowering specialisation
    taint_transform: TaintTransformHook | None   # taint colour transform
    side_effect_hints: tuple[SideEffect, ...] | None = None
    validation_hook: ValidationHook | None = None
```

**Example** — `string length` has `arity=Arity(1, 1)` (exactly one arg),
`pure=True`, and `return_type=TclType.INT`.  The arity checker validates
each subcommand invocation independently.

Subcommands can be dialect-filtered: `SubCommand.supports_dialect()` checks
the subcommand's own `dialects` set, falling back to the parent command's
dialects.

### OptionSpec and option terminators

Commands that accept `-flag` switches declare them via `OptionSpec`
(`models.py:230`):

```python
@dataclass(frozen=True, slots=True)
class OptionSpec:                      # models.py:230
    name: str                          # e.g. "-nocase", "-length"
    takes_value: bool = False          # True if the option consumes the next arg
    detail: str = ""                   # completion description
```

**Option terminators** (`--`) prevent a dynamic argument from being
mistaken for a flag.  The `OptionTerminatorSpec` (`models.py:298`)
configures the `W304` diagnostic:

```python
@dataclass(frozen=True, slots=True)
class OptionTerminatorSpec:            # models.py:298
    scan_start: int = 0                # arg index where option scanning begins
    options_with_values: frozenset[str] = frozenset()  # options that eat next arg
    warn_without_terminator: bool = False  # warn even for static values
    subcommand: str | None = None      # restrict to a specific subcommand
```

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

1. **Arity** — `ValidationSpec.arity` on the `CommandSpec` sets the
   overall arg count.  Each `SubCommand` has its own `arity`.  Violations
   produce `W101`.

2. **Option terminator** — `OptionTerminatorSpec` triggers `W304` when
   `--` is missing before dynamic arguments.

3. **Validation hooks** — per-command or per-subcommand
   `ValidationHook` callables run additional checks.

4. **Event validity** — `event_requires` and `excluded_events` are
   checked against the active event context (see [Events](#events-irules-only)
   below).

### Argument processing — roles, values, and types

Beyond arity and options, the registry describes the *semantic role* of
each argument position, what values are valid there, what type the
command expects, and what hover/completion information to present.

#### ArgRole — what each argument means

`ArgRole` (`signatures.py:15`) classifies how the compiler should
treat each argument position:

```python
class ArgRole(Enum):                   # signatures.py:15
    BODY            # Tcl script body — recursively lowered into IR
    EXPR            # Expression — parsed into ExprNode AST
    VAR_NAME        # Variable name written by the command (set, incr)
    VAR_READ        # Variable name read without modification (info exists)
    PARAM_LIST      # Procedure parameter list (proc)
    NAME            # Symbolic name (proc name, namespace name)
    PATTERN         # Pattern or regex argument
    OPTION          # A switch/flag argument
    VALUE           # Generic value (default for unlisted positions)
    SUBCOMMAND      # The subcommand word ("length" in "string length")
    OPTION_TERMINATOR  # The "--" terminator
    CHANNEL         # Channel identifier (stdout, channelId)
    INDEX           # List/string index expression
```

Roles are declared via `CommandSpec.arg_roles` or
`SubCommand.arg_roles` — a `dict[int, ArgRole]` mapping argument
index (0-based after the command name) to its role.

For variable-layout commands like `if`, `try`, and `switch` (where
argument structure depends on the actual arguments), an
`ArgRoleResolver` (`models.py:73`) callback dynamically maps argument
values to roles:

```python
ArgRoleResolver = Callable[[list[str]], dict[int, ArgRole]]
```

The IR lowering pass uses `ArgRole.BODY` and `ArgRole.EXPR` to decide
which arguments should be recursively lowered or parsed as expressions,
and `ArgRole.VAR_NAME` to extract variable definitions for dataflow
analysis.

#### ArgumentValueSpec — completable values

`ArgumentValueSpec` (`models.py:221`) describes a valid value for a
specific argument position, providing completion text and hover
documentation:

```python
@dataclass(frozen=True, slots=True)
class ArgumentValueSpec:               # models.py:221
    value: str                         # completion text (e.g. "length", "alnum")
    detail: str = ""                   # short description in completion list
    hover: HoverSnippet | None = None  # full hover documentation
```

Argument values are declared in three places:

1. **FormSpec.arg_values** (`dict[int, tuple[ArgumentValueSpec, ...]]`)
   — maps arg index to valid values.  Index 0 typically holds
   subcommand names:

   ```python
   FormSpec(
       kind=FormKind.DEFAULT,
       synopsis="string option arg ?arg ...?",
       arg_values={0: (
           ArgumentValueSpec("length", "Return number of characters."),
           ArgumentValueSpec("match", "Test glob-style pattern match."),
           ...
       )},
   )
   ```

2. **SubCommand.arg_values** — per-subcommand completable values.
   For example, `string is` has character-class values at arg index 0:

   ```python
   SubCommand(name="is", ..., arg_values={
       0: (ArgumentValueSpec("alnum", "Any Unicode alphabet or digit character."),
           ArgumentValueSpec("integer", "Any valid integer of arbitrary size."),
           ...),
   })
   ```

3. **FormSpec.subcommand_arg_values** (`dict[tuple[str, int], ...]`)
   — keyed by `(subcommand_name, sub_arg_index)` for legacy definitions.

`CommandSpec` provides lookup helpers:
- `argument_values(arg_index)` — collects values across all forms.
- `subcommand_argument_values(subcmd, arg_index)` — values for a
  subcommand argument, preferring `SubCommand.arg_values` over the
  legacy `FormSpec.subcommand_arg_values`.

#### HoverSnippet — documentation content

`HoverSnippet` (`models.py:170`) carries hover and signature-help
content derived from man pages or vendor documentation:

```python
@dataclass(frozen=True, slots=True)
class HoverSnippet:                    # models.py:170
    summary: str                       # one-line description
    synopsis: tuple[str, ...] = ()     # invocation signatures
    snippet: str = ""                  # extended description (for signature help)
    source: str = ""                   # attribution (e.g. "Tcl man page string.n")
    examples: str = ""                 # code example
    return_value: str = ""             # return value description
```

`HoverSnippet` appears on `CommandSpec.hover`, `SubCommand.hover`,
`ArgumentValueSpec.hover`, and `OptionSpec.hover`.  The LSP server
uses `render_hover_lean()` for compact hover tooltips and
`render_markdown()` for signature help documentation.

#### ArgTypeHint — expected types

`ArgTypeHint` (`type_hints.py:16`) declares what Tcl internal
representation (intrep) a command expects for a given argument:

```python
@dataclass(frozen=True, slots=True)
class ArgTypeHint:                     # type_hints.py:16
    expected: TclType | None = None    # expected type (None = any)
    shimmers: bool = False             # True if the command forces conversion
```

Type hints are declared via `SubCommand.arg_types` or
`CommandSpec.arg_types` — a `dict[int, ArgTypeHint]`.  The type
inference pass uses these to detect shimmer risks (diagnostic `O130`)
and propagate types through the SSA graph.

Return types are declared via `SubCommand.return_type` or
`CommandSpec.return_type` — a `TclType | None`.  For example,
`string length` has `return_type=TclType.INT`.

#### KeywordCompletion — variable-layout scaffolding

Commands like `if`, `try`, and `switch` have keyword-delimited
structure rather than fixed argument positions.
`KeywordCompletion` (`models.py:90`) provides completion items
for these structural keywords:

```python
@dataclass(frozen=True, slots=True)
class KeywordCompletion:               # models.py:90
    keyword: str                       # e.g. "elseif", "else", "on", "finally"
    detail: str = ""                   # completion list description
    snippet: str | None = None         # LSP snippet with placeholders
```

A `KeywordCompletionProvider` callback on `CommandSpec` generates
context-aware keyword suggestions based on the arguments entered so far.

#### Deprecation

Commands and subcommands can be marked as deprecated with a replacement:

- `CommandSpec.deprecated_replacement` — the replacement command name
  (either a string or a `CommandDef` class reference).
- `SubCommand.deprecated_replacement` — per-subcommand replacement.
- `CommandSpec.deprecation_fixer` / `SubCommand.deprecation_fixer` —
  a `DeprecationFixer` callback that generates LSP code actions to
  automatically rewrite deprecated usage.

### Side effects and purity

The side-effect model lives in `core/compiler/side_effects.py` and
classifies what each command invocation reads and writes.

**Enums** describe the vocabulary:

```python
class SideEffectTarget(Enum):          # side_effects.py:156
    VARIABLE, SESSION_TABLE, HTTP_HEADER, HTTP_BODY, HTTP_URI,
    RESPONSE_COMMIT, POOL_SELECTION, FILE_IO, LOG_IO, ...

class StorageScope(Enum):              # side_effects.py:69
    PROC_LOCAL, NAMESPACE, GLOBAL, UPVAR,       # Tcl-universal
    EVENT, CONNECTION, STATIC, SESSION_TABLE,    # F5 iRules-specific
    DATA_GROUP, FILE_SYSTEM, NETWORK_SOCKET, ...

class ConnectionSide(Enum):            # side_effects.py:137
    CLIENT, SERVER, BOTH, GLOBAL, NONE

class StorageType(Enum):               # side_effects.py:50
    SCALAR, LIST, DICT, ARRAY, UNKNOWN
```

**Per-invocation facts** compose into `SideEffect` and
`CommandSideEffects`:

```python
@dataclass(frozen=True, slots=True)
class SideEffect:                      # side_effects.py:365
    target: SideEffectTarget           # what resource
    reads: bool = False                # does it read?
    writes: bool = False               # does it write?
    storage_type: StorageType          # data shape
    scope: StorageScope                # where it lives
    connection_side: ConnectionSide    # F5 proxy context
    key: str | None = None             # literal variable/header name

@dataclass(frozen=True, slots=True)
class CommandSideEffects:              # side_effects.py:419
    effects: tuple[SideEffect, ...]    # individual effects
    pure: bool = False                 # no observable side effects
    deterministic: bool = False        # same inputs → same outputs
    dynamic_barrier: bool = False      # eval/uplevel — unknowable
```

**Classification** — `classify_side_effects(command, args, ...)`
(`side_effects.py:556`) combines registry hints with runtime arguments:

1. Check interprocedural summary (for user-defined procs).
2. Check for dynamic barriers (`eval`, `uplevel`).
3. Resolve the subcommand and matching `FormSpec`.
4. Read `side_effect_hints` from the resolved form, subcommand, or
   command spec.
5. Check command-level or subcommand-level `pure`/`mutator` flags.

**How purity propagates:**

- A command is pure if `CommandSpec.pure = True` (e.g. `string`, `list`).
- A subcommand can override: `SubCommand.pure = True` makes
  `string length` pure even if the parent weren't.
- A subcommand can also override downward: `SubCommand.mutator = True`
  makes `HTTP::header replace` impure even though `HTTP::header` itself
  is pure (the getter form reads without side effects).
- A `FormSpec` can further refine: `FormSpec.pure` / `FormSpec.mutator`
  overrides all higher levels for that specific arity match.

The GVN optimiser uses purity to decide whether a command's result can be
cached (`cse_candidate = True`), and the SCCP analysis uses it to infer
through pure calls without bailing out.

### Taint analysis

Taint tracking determines whether values originate from untrusted input
(user-controlled HTTP headers, URI, query parameters, etc.).

**TaintColour** (`taint_hints.py:17`) is a `Flag` enum — colours compose
with `|` and the lattice join is their intersection (`&`):

```python
class TaintColour(Flag):               # taint_hints.py:17
    TAINTED         # base: value comes from untrusted input
    PATH_PREFIXED   # starts with "/" (HTTP::uri, HTTP::path)
    CRLF_FREE       # no CR/LF characters (header-injection safe)
    IP_ADDRESS      # IPv4/IPv6 digits-dots-colons
    PORT            # integer 0-65535
    FQDN            # fully qualified domain name
    LIST_CANONICAL  # canonical Tcl list (safe for list operations)
    HTML_ESCAPED    # HTML-escaped text context
    URL_ENCODED     # URL-encoded text context
    ...
```

Colours represent *safety properties* of tainted data.  A value with
`TAINTED | IP_ADDRESS` is tainted but known to be a safe IP address
format, which may satisfy certain sinks (e.g. connecting to a backend).

**TaintHint** (`taint_hints.py:60`) declares a command's taint sources
and sinks:

```python
@dataclass(frozen=True, slots=True)
class TaintHint:                       # taint_hints.py:60
    source: dict[Arity | None, TaintColour] | None  # return value is tainted
    source_subcommands: frozenset[str] | None        # restrict to specific subcmds
    sinks: tuple[TaintSinkSpec, ...]                  # dangerous arg positions
    setter_constraints: tuple[SetterConstraint, ...]  # e.g. must start with "/"
```

**Example** — `HTTP::host` is a taint source (returns user-controlled
data):

```python
@classmethod
def taint_hints(cls) -> TaintHint:
    return TaintHint(source={None: TaintColour.TAINTED})
```

**TaintSinkSpec** marks argument positions as dangerous:

```python
@dataclass(frozen=True, slots=True)
class TaintSinkSpec:                   # taint_hints.py:42
    code: str                          # diagnostic code (e.g. "IRULE3001")
    subcommands: frozenset[str] | None # None = all invocations
```

The taint engine (`core/compiler/taint/`) propagates colours through the
SSA graph, and emits diagnostics (e.g. `IRULE3001` for XSS,
`IRULE3002` for header injection) when tainted data reaches a sink
without sufficient safety colours.

### Dialects

Dialects partition command availability across Tcl versions and tool
contexts.  Known dialects (`dialects.py:5`):

```python
KNOWN_DIALECTS = frozenset({
    "tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0",   # Tcl version dialects
    "f5-irules",                                 # F5 iRules
    "f5-iapps",                                  # F5 iApps
    "synopsys-eda-tcl",                          # Synopsys EDA
    "cadence-eda-tcl",                           # Cadence EDA
    "xilinx-eda-tcl",                            # Xilinx/AMD EDA
    "intel-quartus-eda-tcl",                     # Intel Quartus
    "mentor-eda-tcl",                            # Mentor/Siemens EDA
})
```

Every `CommandSpec` has an optional `dialects` field:

- `dialects = None` → available in **all** dialects.
- `dialects = frozenset({"f5-irules"})` → iRules-only command
  (e.g. `HTTP::host`, `pool`, `table`).

Subcommands can have their own `dialects` set that overrides the parent
command's.  `SubCommand.supports_dialect()` checks the subcommand's own
set first, falling back to the parent.

**DialectStatus** (`models.py:24`) is the result of a dialect lookup:

```python
class DialectStatus(Enum):             # models.py:24
    EXISTS       # available in this dialect
    DEPRECATED   # available but has a replacement
    DISALLOWED   # exists in some dialect, but not this one
    NOT_EXISTS   # not known anywhere
```

### Events (iRules only)

In F5 iRules, commands are only valid in certain events (e.g. `HTTP::uri`
requires an HTTP profile and only works in HTTP events).  This is
modelled by `EventRequires` (`namespace_models.py:142`):

```python
@dataclass(frozen=True, slots=True)
class EventRequires:                   # namespace_models.py:142
    client_side: bool = False          # needs client-side connection
    server_side: bool = False          # needs server-side connection
    transport: str | None = None       # "tcp" or "udp"
    profiles: frozenset[str] = frozenset()  # needs one of these profiles
    also_in: frozenset[str] = frozenset()   # always valid in these events
    init_only: bool = False            # only valid in RULE_INIT
    flow: bool = False                 # needs an active traffic flow
    capability: str | None = None      # profile capability (e.g. "sni")
```

**Example** — `HTTP::host` requires TCP transport and an HTTP or FASTHTTP
profile:

```python
event_requires=EventRequires(transport="tcp", profiles=frozenset({"HTTP", "FASTHTTP"}))
```

The validator matches `event_requires` against the event's `EventProps`
(`namespace_models.py:117`), which describes what each event provides
(client/server side, transport, implied profiles).  Mismatches produce
diagnostic `IRULE1001`.

`CommandSpec.excluded_events` lists events where a command is explicitly
forbidden (e.g. a command that crashes in `RULE_INIT`).

### How the infrastructure feeds the compiler

The registry metadata flows into every stage of the compilation pipeline:

1. **IR Lowering** — `lower_to_ir()` uses `arg_roles` to identify
   which arguments are bodies (`BODY`), expressions (`EXPR`), or
   variable names (`VAR_NAME`).  This drives recursive lowering of
   script bodies and expression parsing.

2. **CFG** — `creates_dynamic_barrier` marks commands that defeat
   static analysis (e.g. `eval`, `uplevel`).  The CFG builder emits
   `IRBarrier` for these.

3. **SSA/SCCP** — `pure` commands can be inferred through without
   invalidating the lattice state.  Impure commands force variables
   to `OVERDEFINED`.

4. **GVN** — `cse_candidate` and `pure` determine whether a command's
   result can be cached and reused (common subexpression elimination).

5. **Codegen** — `codegen` hooks on `SubCommand` or `CommandSpec`
   emit specialised bytecode (e.g. `string length` → `strLen` opcode
   instead of generic `invokeStk`).

6. **Taint engine** — `taint_hints()` marks sources and sinks; the
   taint lattice propagates `TaintColour` through the SSA graph.

7. **Diagnostics** — arity, option terminators, event requirements,
   deprecation, and validation hooks all produce diagnostics
   (`W101`, `W304`, `IRULE1001`, etc.).

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
Token(type=ESC, text="set",  start=Pos(0,0,0),  end=Pos(0,3,3))
Token(type=SEP, text=" ",    start=Pos(0,3,3),  end=Pos(0,4,4))
Token(type=ESC, text="x",    start=Pos(0,4,4),  end=Pos(0,5,5))
Token(type=SEP, text=" ",    start=Pos(0,5,5),  end=Pos(0,6,6))
Token(type=ESC, text="42",   start=Pos(0,6,6),  end=Pos(0,8,8))
Token(type=EOF, text="",     start=Pos(0,8,8),  end=Pos(0,8,8))
```

Key observations:
- `set`, `x`, and `42` are all `ESC` (plain word fragments) — no variable
  substitution or braces involved.
- Whitespace becomes `SEP` tokens — they delimit words but carry no semantic
  value.

### Stage 2 — Segmenter → SegmentedCommand

The segmenter groups tokens into commands at `EOL`/`EOF` boundaries:

```python
SegmentedCommand(
    range=Range(Pos(0,0,0), Pos(0,8,8)),
    argv=[Token(ESC,"set",...), Token(ESC,"x",...), Token(ESC,"42",...)],
    texts=["set", "x", "42"],
    single_token_word=[True, True, True],
    all_tokens=[...all 6 tokens...],
)
```

- `texts[0] == "set"` → command name
- `texts[1] == "x"` → variable name argument
- `texts[2] == "42"` → value argument
- All words are single-token (no interpolation), so `single_token_word` is
  all `True` — this tells the lowerer the value is a compile-time constant.

### Stage 3 — IR Lowering → IRAssignConst

The lowerer pattern-matches `set` with two arguments where the second argument
is a single-token constant:

```python
IRModule(
    top_level=IRScript(statements=(
        IRAssignConst(
            range=Range(Pos(0,0,0), Pos(0,8,8)),
            name="x",
            value="42",
        ),
    )),
    procedures={},
)
```

Why `IRAssignConst` and not `IRAssignValue`?  Because `"42"` is a single
atomic token with no variable substitution — it's known at compile time.

### Stage 4 — CFG → single basic block

With no control flow, the CFG is trivial:

```
CFGFunction(
    name="::top",
    entry="entry_1",
    blocks={
        "entry_1": CFGBlock(
            name="entry_1",
            statements=(IRAssignConst(name="x", value="42"),),
            terminator=None,
        ),
        "exit_2": CFGBlock(
            name="exit_2",
            statements=(),
            terminator=None,
        ),
    },
)
```

The builder creates an entry block containing the statement, linked to an
exit block via `CFGGoto`.

### Stage 5 — SSA → x₀

With a single block and a single definition, SSA is trivial:

```
SSAFunction blocks:
  entry_1:
    phis: ()
    statements: (
        SSAStatement(
            statement=IRAssignConst(name="x", value="42"),
            uses={},
            defs={"x": 1},
        ),
    )
    entry_versions: {}
    exit_versions: {"x": 1}
```

- `x` gets version 1 (its first definition): SSA value key `("x", 1)` (see [Glossary → SSA](#glossary)).
- No phi nodes — there is only one path through the program (see [Glossary → Phi node](#glossary)).
- `uses` is empty — `set x 42` doesn't read any variables.

### Stage 6 — Core analyses

**SCCP** (see [Glossary](#glossary))**:** `("x", 1)` → `LatticeValue(CONST, "42")` — provably constant.

**Type inference:** `("x", 1)` → `TypeLattice.of(TclType.INT)` — `"42"` is
a valid integer literal, so the intrep is INT.

**Liveness:** `("x", 1)` is dead if nothing reads it.

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
Token(ESC, "set")  Token(SEP, " ")  Token(ESC, "x")  Token(SEP, " ")  Token(ESC, "42")  Token(EOL, "\n")
Token(ESC, "set")  Token(SEP, " ")  Token(ESC, "y")  Token(SEP, " ")  Token(VAR, "x")   Token(EOF, "")
```

Note the critical difference: `42` is `ESC` (plain text), but `$x` produces
`Token(type=VAR, text="x")` — the `$` prefix is consumed by the lexer, and
the token text is the bare variable name.

### Stage 2 — Segmenter

Two `SegmentedCommand` objects:

```python
# Command 1: set x 42
SegmentedCommand(
    texts=["set", "x", "42"],
    single_token_word=[True, True, True],   # all constant
)

# Command 2: set y $x
SegmentedCommand(
    texts=["set", "y", "${x}"],             # VAR token → "${x}" text
    single_token_word=[True, True, True],   # single token, but it's a VAR
)
```

In command 2, `texts[2]` is `"${x}"` — the segmenter wraps `VAR` tokens in
`${...}` form so the text reflects what the user wrote.

### Stage 3 — IR Lowering

```python
IRScript(statements=(
    IRAssignConst(name="x", value="42"),
    IRAssignValue(name="y", value="${x}"),     # has variable reference
))
```

The second `set` produces `IRAssignValue` (not `IRAssignConst`) because the
value `${x}` contains a variable substitution that must be resolved at
runtime.

### Stage 5 — SSA

```
  entry_1:
    SSAStatement(IRAssignConst(name="x", value="42"), uses={}, defs={"x": 1})
    SSAStatement(IRAssignValue(name="y", value="${x}"), uses={"x": 1}, defs={"y": 1})
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

```python
IRScript(statements=(
    IRExprEval(
        range=Range(...),
        expr=ExprBinary(
            op=BinOp.ADD,
            left=ExprLiteral(text="2", start=0, end=1),
            right=ExprLiteral(text="3", start=4, end=5),
        ),
    ),
))
```

The expression parser produces a structured `ExprBinary` node, not a raw
string.

### Stage 6 — SCCP

SCCP evaluates the expression: `ExprLiteral("2") + ExprLiteral("3")` →
`CONST(5)`.  The compiler knows the result at compile time.

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

```python
IRScript(statements=(
    IRAssignConst(name="a", value="10"),
    IRAssignConst(name="b", value="20"),
    IRExprEval(
        expr=ExprBinary(
            op=BinOp.ADD,
            left=ExprVar(text="$a", name="a", start=0, end=2),
            right=ExprVar(text="$b", name="b", start=5, end=7),
        ),
    ),
))
```

### Stage 5 — SSA

```
  a₁ = "10"   (CONST)
  b₁ = "20"   (CONST)
  ExprEval uses: {a: 1, b: 1}
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

The simplest conditional — introduces `CFGBranch` and forked control flow.

### Source

```tcl
set x 1
if {$x} {
    set y 10
}
```

### Stage 3 — IR Lowering

```python
IRScript(statements=(
    IRAssignConst(name="x", value="1"),
    IRIf(
        range=Range(...),
        clauses=(
            IRIfClause(
                condition=ExprVar(text="$x", name="x", start=0, end=2),
                condition_range=Range(...),
                body=IRScript(statements=(
                    IRAssignConst(name="y", value="10"),
                )),
                body_range=Range(...),
            ),
        ),
        else_body=None,
    ),
))
```

- `IRIf` holds a tuple of `IRIfClause` objects (one per `if`/`elseif`).
- The condition `{$x}` is parsed as `ExprVar(name="x")`.
- No `else_body` for this example.

### Stage 4 — CFG decomposition

The `IRIf` is decomposed into basic blocks:

```
  entry_1:
    statements: [IRAssignConst(name="x", value="1")]
    terminator: CFGBranch(
        condition=ExprVar("$x"),
        true_target="if_then_3",
        false_target="if_next_4"
    )

  if_then_3:
    statements: [IRAssignConst(name="y", value="10")]
    terminator: CFGGoto(target="if_end_2")

  if_next_4:
    statements: []
    terminator: CFGGoto(target="if_end_2")

  if_end_2:
    statements: []
    terminator: CFGGoto(target="exit_5")
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
```python
ConstantBranch(
    block="entry_1",
    condition="$x",
    value=True,
    taken_target="if_then_3",
    not_taken_target="if_next_4",
)
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

```python
IRIf(
    clauses=(
        IRIfClause(
            condition=ExprLiteral(text="1", start=0, end=1),
            body=IRScript((IRAssignConst(name="x", value="1"),)),
        ),
    ),
    else_body=IRScript((IRAssignConst(name="x", value="2"),)),
)
```

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

```python
IRIf(
    clauses=(
        IRIfClause(
            condition=ExprBinary(op=BinOp.LT,
                left=ExprVar(text="$x", name="x"),
                right=ExprLiteral(text="0")),
            body=IRScript((IRAssignConst(name="sign", value="-1"),)),
        ),
        IRIfClause(
            condition=ExprBinary(op=BinOp.GT,
                left=ExprVar(text="$x", name="x"),
                right=ExprLiteral(text="0")),
            body=IRScript((IRAssignConst(name="sign", value="1"),)),
        ),
    ),
    else_body=IRScript((IRAssignConst(name="sign", value="0"),)),
)
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

```python
IRScript(statements=(
    IRAssignConst(name="i", value="0"),
    IRWhile(
        condition=ExprBinary(op=BinOp.LT,
            left=ExprVar(text="$i", name="i"),
            right=ExprLiteral(text="5")),
        body=IRScript((IRIncr(name="i"),)),
    ),
))
```

- `IRWhile` has a structured `ExprBinary` condition and an `IRScript` body.
- `IRIncr(name="i")` with `amount=None` means increment by 1.

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
- A header block with the condition `CFGBranch`
- A body block that jumps back to the header (back edge)
- An exit block for when the condition is false

### Stage 5 — SSA with loop phi

```
  while_header_3:
    phi: i₂ = phi(i₁ from entry_1, i₃ from while_body_4)
    branch uses: {i: 2}

  while_body_4:
    IRIncr(i) → i₃ = i₂ + 1
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

```python
IRFor(
    init=IRScript((IRAssignConst(name="i", value="0"),)),
    condition=ExprBinary(op=BinOp.LT,
        left=ExprVar(text="$i", name="i"),
        right=ExprLiteral(text="10")),
    next=IRScript((IRIncr(name="i"),)),
    body=IRScript((IRAssignValue(name="x", value="${i}"),)),
)
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

```python
IRForeach(
    iterators=((("item",), "{a b c}"),),
    body=IRScript((IRAssignValue(name="x", value="${item}"),)),
    is_lmap=False,
)
```

### Stage 4 — CFG (top-level deferral)

At top level, `foreach` is **not** inlined into a loop CFG.  Instead, it is
emitted as an opaque `IRCall`:

```python
CFGBlock(
    statements=[
        IRCall(command="foreach", args=("item", "{a b c}", "\n    set x $item\n"),
               defs=("item",)),
    ],
)
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

```python
IRModule(
    top_level=IRScript(statements=()),   # proc def is extracted
    procedures={
        "::add": IRProcedure(
            name="add",
            qualified_name="::add",
            params=("a", "b"),
            body=IRScript(statements=(
                IRExprEval(
                    expr=ExprBinary(op=BinOp.ADD,
                        left=ExprVar(text="$a", name="a"),
                        right=ExprVar(text="$b", name="b")),
                ),
            )),
        ),
    },
)
```

The procedure definition is extracted from `top_level` into
`IRModule.procedures`.  The top-level code emits the `proc` registration
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

**`HTTP::header`** — the `value` subcommand is a taint source:

```python
# From irules/http_header.py (simplified)
CommandSpec(
    name="HTTP::header",
    subcommands={
        "value": SubCommand(
            name="value",
            arity=Arity(1, 1),
            pure=True,                # reading a header has no side effects
            return_type=TclType.STRING,
            taint_transform=None,     # no colour transform
        ),
        "replace": SubCommand(
            name="replace",
            arity=Arity(2, 2),
            mutator=True,             # writing a header IS a side effect
        ),
        ...
    },
)

# TaintHint for HTTP::header:
TaintHint(
    source={None: TaintColour.TAINTED},   # return value is tainted
    source_subcommands=frozenset({"value", "values", "names"}),  # only these subcmds
    sinks=(),
)
```

**`HTTP::respond`** — the `content` argument is a taint sink:

```python
# TaintSinkSpec for HTTP::respond body content:
TaintSinkSpec(
    code="IRULE3001",               # XSS diagnostic code
    subcommands=None,               # all invocations
)
```

**`string tolower`** — a pure command, but *not* a sanitiser:

```python
SubCommand(
    name="tolower",
    arity=Arity(1, 3),
    pure=True,
    return_type=TclType.STRING,
    taint_transform=None,           # does NOT strip taint
)
```

### Stage 3 — IR Lowering

```python
IRModule(
    top_level=IRScript(statements=()),
    procedures={
        "::when::HTTP_REQUEST": IRProcedure(
            body=IRScript(statements=(
                IRAssignValue(name="host", value="[HTTP::header value Host]"),
                IRAssignValue(name="lower", value="[string tolower ${host}]"),
                IRCall(command="HTTP::respond",
                       args=("200", "content",
                             "<h1>Welcome to ${lower}</h1>"),
                       defs=(), reads=("lower",)),
            )),
        ),
    },
)
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
    SSAStatement(IRAssignValue(name="host", ...), uses={}, defs={"host": 1})
    SSAStatement(IRAssignValue(name="lower", ...), uses={"host": 1}, defs={"lower": 1})
    SSAStatement(IRCall("HTTP::respond", ...), uses={"lower": 1}, defs={})
```

### Taint propagation

The taint engine ([`_propagation.py`](../core/compiler/taint/_propagation.py))
walks the SSA graph and computes a `TaintLattice` for each SSA value key:

1. **`("host", 1)`** — the `[HTTP::header value Host]` command substitution
   is evaluated:
   - `_taint_source_colour("HTTP::header", ("value", "Host"))` looks up the
     `TaintHint` from the registry → returns `TaintColour.TAINTED`.
   - Result: `TaintLattice(tainted=True, colour=TAINTED)`

2. **`("lower", 1)`** — `[string tolower $host]`:
   - The argument `$host` has taint `("host", 1)` → `TAINTED`.
   - `_is_sanitiser("string", ("tolower", ...))` → `False` (case
     conversion does not sanitise).
   - `_derive_transform_colours("string", ("tolower", ...))` → no extra
     colours.
   - The command is pure, so taint flows through: result inherits from
     arguments.
   - Result: `TaintLattice(tainted=True, colour=TAINTED)`

3. **`HTTP::respond`** — the sink check:
   - `_classify_sink(IRCall("HTTP::respond", ...))` queries the registry:
     `REGISTRY.classify_taint_sinks("HTTP::respond", None, dialect)`.
   - Returns `[("IRULE3001", "HTTP::respond")]` — the content body is an
     XSS-sensitive output sink.
   - `_stmt_var_arg_indexes(stmt, "lower")` → `(2,)` — `$lower` appears at
     arg index 2 (the content argument).
   - The taint of `("lower", 1)` is `TAINTED` with no mitigating colours
     (e.g. `HTML_ESCAPED` would suppress the warning).

### Taint warning emitted

```python
TaintWarning(
    range=Range(Pos(3,4,...), Pos(3,55,...)),   # HTTP::respond line
    variable="lower",
    sink_command="HTTP::respond",
    code="IRULE3001",
    message="Tainted variable $lower in HTTP response body (HTTP::respond); "
            "risk of XSS or content injection",
)
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

```python
IRModule(
    top_level=IRScript(statements=(
        IRAssignValue(name="result", value="[double 21]"),
        IRCall(command="puts", args=("${result}",)),
    )),
    procedures={
        "::double": IRProcedure(
            name="double",
            qualified_name="::double",
            params=("n",),
            body=IRScript(statements=(
                IRExprEval(
                    expr=ExprBinary(op=BinOp.MUL,
                        left=ExprVar(text="$n", name="n"),
                        right=ExprLiteral(text="2")),
                ),
            )),
        ),
    },
)
```

### Interprocedural analysis

The `InterproceduralAnalysis` pass (`interprocedural.py`) builds summaries
for each procedure.  For `::double`:

1. The body is a single `expr {$n * 2}` in tail position.
2. SCCP within the proc body: parameter `n` is initially `UNKNOWN`.
3. The interprocedural solver calls `fold_static_proc_call("::double", ("21",))`:
   - Binds `n₁ = "21"` (constant).
   - Evaluates `$n * 2` → `21 * 2` → `CONST(42)`.
4. Result: the call `[double 21]` folds to the string `"42"`.

### Optimisation pass — O103

`optimise_static_proc_calls()` in
[`_propagation.py:271`](../core/compiler/optimiser/_propagation.py):

1. Encounters the `[double 21]` command substitution token.
2. Resolves `double` → qualified name `::double`.
3. Checks: `::double` is not in `ir_module.redefined_procedures`.
4. All arguments are static: `"21"` is a literal.
5. Calls `fold_static_proc_call(interproc, "::double", ("21",))`.
6. Gets back `"42"`.
7. Emits:

```python
Optimisation(
    code="O103",
    message="Fold static procedure call",
    range=Range(Pos(4,13,...), Pos(4,23,...)),  # [double 21]
    replacement="42",
)
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
[`_code_sinking.py:514`](../core/compiler/optimiser/_code_sinking.py):

1. **Sinkability check** (`_is_sinkable()`): `IRAssignConst(name="msg", value="Request denied")`
   is sinkable — it is a simple constant assignment with no command
   substitutions.

2. **Variable reference scan** (`_stmt_uses_var()`): walks the `IRIf` body
   recursively.  `$msg` appears only in the `else` body (`HTTP::respond ...
   $msg`), not in the `if` body (which defines a new `msg`).

3. **Deepest target** (`_find_deepest_sink_targets()`): the only use of the
   original `msg` is in the `else` branch → sink target is the else body.

4. **Emission** (`_emit_sinking_opts()`): emits a grouped pair of O125
   optimisations:

```python
# Part 1: Comment out the original statement
Optimisation(
    code="O125",
    message="Sink set msg \"Request denied\" into else body",
    range=Range(Pos(0,0,...), Pos(0,24,...)),   # set msg "Request denied"
    replacement="",
    group=0,
)

# Part 2: Insert at the start of the else body
Optimisation(
    code="O125",
    message="Insert sunk set msg \"Request denied\"",
    range=Range(Pos(6,0,...), Pos(6,0,...)),     # start of else body
    replacement="    set msg \"Request denied\"\n",
    group=0,
)
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
[`gvn.py`](../core/compiler/gvn.py):

1. **Purity check** (`_is_pure_command()`): looks up `HTTP::uri` in the
   command registry → `CommandSpec.pure = True`, `cse_candidate = True`.

2. **Value numbering**: assigns a canonical `ExprKey` to each computation.
   Both `[HTTP::uri]` calls get the same key `("HTTP::uri",)`.

3. **Dominance check**: the first occurrence (in the `if` condition)
   dominates the second (in the `log` command) — every path to the `log`
   statement passes through the `if` condition first.

4. **Kill check**: no barriers or mutating commands between the two
   occurrences invalidate the value.

5. **Emission**:

```python
RedundantComputation(
    range=Range(Pos(4,28,...), Pos(4,40,...)),    # second [HTTP::uri]
    first_range=Range(Pos(1,8,...), Pos(1,20,...)),  # first [HTTP::uri]
    expression_text="HTTP::uri",
    code="O105",
    message="Redundant computation of [HTTP::uri]; result already "
            "available from line 2",
)
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
[`_elimination.py`](../core/compiler/optimiser/_elimination.py):

**O109 — Dead Store Elimination:**
`unused₁` is assigned but never read.  The assignment `set unused 99` is
a dead store:

```python
Optimisation(
    code="O109",
    message="Dead store: unused is set but never read",
    range=Range(...),   # set unused 99
    replacement="",
)
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
[`_structure_elimination.py`](../core/compiler/optimiser/_structure_elimination.py)
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

[`recovery.py`](../core/parsing/recovery.py) runs a first-pass parse via
`segment_commands()`.  The segmenter detects that the `CMD` token starting
at `[` is unterminated (the character after the CMD text is not `]`).

**Heuristic — command-break detection** (`_detect_missing_bracket_at_command`):
The next non-blank line starts with `set` — a known command name.  This
signals that `]` should be inserted at the end of line 1.

A `VirtualToken` is created:

```python
VirtualToken(
    offset=29,        # end of "hello" on line 1
    char="]",         # the missing delimiter
    diagnostic=Diagnostic(
        code="E201",
        message='missing close-bracket',
        range=Range(Pos(0,8,...), Pos(0,8,...)),
        severity=Severity.ERROR,
        fix=CodeFix(
            description='Insert "]"',
            ...
        ),
    ),
)
```

**Re-parse with virtual token:** The lexer sees the injected `]` and
produces a clean `CMD` token.  The second parse yields two well-formed
`SegmentedCommand` objects:

```python
# Command 1 (recovered):
SegmentedCommand(texts=["set", "x", '[string length "hello"]'])

# Command 2 (clean):
SegmentedCommand(texts=["set", "y", "42"])
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
ExprToken(VAR, "$a")
ExprToken(OP, "+")
ExprToken(VAR, "$b")
ExprToken(OP, "*")
ExprToken(NUM, "2")
```

**Pratt parsing** (`_PrattParser` in
[`expr_parser.py`](../core/parsing/expr_parser.py)):

The parser uses binding powers to handle precedence:
- `*` has binding power (22, 23) — higher than `+` at (20, 21).
- So `$b * 2` binds tighter than `$a + ...`.

Result:

```python
ExprBinary(
    op=BinOp.ADD,
    left=ExprVar(text="$a", name="a"),
    right=ExprBinary(
        op=BinOp.MUL,
        left=ExprVar(text="$b", name="b"),
        right=ExprLiteral(text="2"),
    ),
)
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

```python
ExprRaw(text="${a} + ${b} * 2")
```

`ExprRaw` is the fallback — the compiler cannot statically analyse the
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
[`lowering.py`](../core/compiler/lowering.py) dispatches each command to
the appropriate IR node using registry metadata.

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

### Example: `lower_set()` — the `set` lowering hook

`set` has a registered lowering hook
([`_var.py:36`](../core/compiler/lowering_hooks/_var.py)).
It pattern-matches on the second argument's token type:

| Token type of `args[1]` | IR node produced | Example |
|-------------------------|-----------------|---------|
| `STR` (braced string) | `IRAssignConst` | `set x {hello}` |
| `ESC` (decimal integer) | `IRAssignConst` | `set x 42` |
| `CMD` wrapping `expr` | `IRAssignExpr` | `set x [expr {$a + 1}]` |
| `VAR` or interpolated | `IRAssignValue` | `set x $y`, `set x "hi $name"` |
| 0 args (getter) | `IRCall` | `set x` (read variable) |

### Example: fallthrough with `arg_roles`

For a command like `regexp`:

```tcl
regexp {(\d+)} $input match submatch
```

The registry declares `ArgRole.VAR_NAME` at arg indices 2 and 3 (the
match variables).  The fallthrough path calls:

```python
var_indices = arg_indices_for_role("regexp", args, ArgRole.VAR_NAME)
# → {2, 3}  (match, submatch)
```

This produces:

```python
IRCall(
    command="regexp",
    args=(r"(\d+)", "${input}", "match", "submatch"),
    defs=("match", "submatch"),   # SSA tracks these as definitions
)
```

The `defs` tuple tells the SSA builder that `regexp` defines `match` and
`submatch`, so they get new SSA versions.

### Example: barrier commands

Commands in `_DYNAMIC_BARRIER_COMMANDS` (e.g. `eval`, `uplevel`, `upvar`)
always produce `IRBarrier`:

```tcl
eval $script
```

```python
IRBarrier(
    range=Range(...),
    reason="dynamic command",
    command="eval",
    args=("${script}",),
)
```

`IRBarrier` tells all downstream passes: *stop reasoning about variable
state here* — the command can read/write any variable, define new
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
[`execution_intent.py`](../core/compiler/execution_intent.py) walks each
`IRAssignValue` in the CFG and parses the command substitution:

**`[llength $items]`:**

```python
CommandSubstitutionIntent(
    command="llength",
    args=("$items",),
    arg_categories=(SubstitutionCategory.SCALAR_VAR,),
    side_effect=SideEffectClass.PURE,      # llength is pure
    escape=EscapeClass.NO_ESCAPE,          # no dynamic barriers
    shimmer_pressure=1,                     # one var arg
)
```

**`[format "Total: %d" $count]`:**

```python
CommandSubstitutionIntent(
    command="format",
    args=('"Total: %d"', "$count"),
    arg_categories=(SubstitutionCategory.LITERAL, SubstitutionCategory.SCALAR_VAR),
    side_effect=SideEffectClass.PURE,
    escape=EscapeClass.NO_ESCAPE,
    shimmer_pressure=1,
)
```

**`[http::geturl $url]`:**

```python
CommandSubstitutionIntent(
    command="http::geturl",
    args=("$url",),
    arg_categories=(SubstitutionCategory.SCALAR_VAR,),
    side_effect=SideEffectClass.MAY_SIDE_EFFECT,  # network I/O
    escape=EscapeClass.MAY_ESCAPE,                 # may throw
    shimmer_pressure=1,
)
```

### How ADCE uses execution intent

`_is_adce_removable_statement()` checks the intent before removing a
"dead" assignment:

- `[llength $items]` → PURE + NO_ESCAPE → **safe to remove** if `count`
  is never read.
- `[http::geturl $url]` → MAY_SIDE_EFFECT → **cannot remove** even if
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

```python
ProcLocalSummary(
    qualified_name="::helper",
    params=("x",),
    arity=Arity(1, 1),
    calls=(),                        # no internal proc calls
    has_barrier=False,               # no eval/uplevel
    has_unknown_calls=False,
    writes_global=False,
    local_effect_reads=EffectRegion(0),   # reads only local params
    local_effect_writes=EffectRegion(0),  # writes only local var
    returns_constant=False,               # depends on param
    constant_return=None,
    return_depends_on_params=("x",),      # return value depends on x
    return_passthrough_param=None,
)
```

**`::main`:**

```python
ProcLocalSummary(
    qualified_name="::main",
    params=("a", "b"),
    arity=Arity(2, 2),
    calls=("::helper",),            # calls helper
    has_barrier=False,
    has_unknown_calls=False,
    writes_global=False,
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

```python
ProcSummary(
    qualified_name="::helper",
    params=("x",),
    arity=Arity(1, 1),
    calls=(),
    has_barrier=False,
    has_unknown_calls=False,
    writes_global=False,
    pure=True,
    effect_reads=EffectRegion(0),
    effect_writes=EffectRegion(0),
    returns_constant=False,
    constant_return=None,
    return_depends_on_params=("x",),
    return_passthrough_param=None,
    can_fold_static_calls=True,
)
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

`_extract_event_summary()` in
[`connection_scope.py`](../core/compiler/connection_scope.py) walks
each event's SSA blocks:

**CLIENT_ACCEPTED:**

```python
EventVarSummary(
    event="CLIENT_ACCEPTED",
    defs=frozenset({"conn_start", "request_count"}),
    uses_before_def=frozenset(),    # no version-0 reads
    unsets=frozenset(),
)
```

**HTTP_REQUEST:**

```python
EventVarSummary(
    event="HTTP_REQUEST",
    defs=frozenset({"request_count"}),      # incr defines it
    uses_before_def=frozenset({"request_count", "conn_start"}),  # version 0
    unsets=frozenset(),
)
```

### Cross-event set computation

`build_connection_scope()` compares every pair of events:

- `CLIENT_ACCEPTED` defines `{conn_start, request_count}`.
- `HTTP_REQUEST` uses-before-def `{request_count, conn_start}`.
- Intersection: `{conn_start, request_count}` — these flow across events.

```python
ConnectionScope(
    summaries={...},
    cross_event_defs=frozenset({"conn_start", "request_count"}),
    cross_event_imports=frozenset({"conn_start", "request_count"}),
)
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

[`naming.py`](../core/common/naming.py) provides the canonical form:

```python
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

```python
IRModule(
    procedures={
        "::mylib::helper": IRProcedure(name="helper", ...),
        "::mylib::compute": IRProcedure(name="compute", ...),
    },
)
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

```python
LocalVarTable(params=("n",))
# LVT slots: %v0 = "n"
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

[`layout.py`](../core/compiler/codegen/layout.py) iterates up to 10
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

```python
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

```python
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

[`side_effects.py:556`](../core/compiler/side_effects.py):

**`HTTP::uri` (getter form):**

```python
CommandSideEffects(
    effects=(
        SideEffect(
            target=SideEffectTarget.HTTP_URI,
            reads=True,
            writes=False,
            storage_type=StorageType.SCALAR,
            scope=StorageScope.EVENT,
            connection_side=ConnectionSide.CLIENT,
        ),
    ),
    pure=True,             # reading is side-effect-free
    deterministic=True,    # same result within one event
    dynamic_barrier=False,
)
```

**`HTTP::header replace Host "example.com"` (setter form):**

```python
CommandSideEffects(
    effects=(
        SideEffect(
            target=SideEffectTarget.HTTP_HEADER,
            reads=False,
            writes=True,                     # modifying a header
            storage_type=StorageType.SCALAR,
            scope=StorageScope.EVENT,
            connection_side=ConnectionSide.CLIENT,
            key="Host",                      # literal header name
        ),
    ),
    pure=False,            # writing is a side effect
    deterministic=False,
    dynamic_barrier=False,
)
```

**`pool my_pool`:**

```python
CommandSideEffects(
    effects=(
        SideEffect(
            target=SideEffectTarget.POOL_SELECTION,
            reads=False,
            writes=True,
            storage_type=StorageType.SCALAR,
            scope=StorageScope.CONNECTION,
            connection_side=ConnectionSide.SERVER,
        ),
    ),
    pure=False,
    deterministic=False,
)
```

**`log local0. "Routing $uri"`:**

```python
CommandSideEffects(
    effects=(
        SideEffect(
            target=SideEffectTarget.LOG_IO,
            reads=False,
            writes=True,
            storage_type=StorageType.SCALAR,
            scope=StorageScope.GLOBAL,
            connection_side=ConnectionSide.NONE,
        ),
    ),
    pure=False,
    deterministic=False,
)
```

### How classification resolves form and subcommand

The classification function follows this resolution order:

1. **Interprocedural summary** — if `callee_summary` is provided (for
   user-defined procs), use its `effect_reads`/`effect_writes` directly.

2. **Dynamic barriers** — `eval`, `uplevel` → `dynamic_barrier=True`,
   all effects unknown.

3. **Subcommand resolution** — for `HTTP::header replace`:
   - Look up `CommandSpec` for `HTTP::header`.
   - Find `SubCommand` for `replace` → `mutator=True`.
   - Read `side_effect_hints` from the subcommand.

4. **Form resolution** — for `HTTP::uri` (no args):
   - `CommandSpec.resolve_form(args)` matches the getter form
     (`arity=Arity(0, 0)`).
   - Getter form has `pure=True` and `reads=True` hints.

### How consumers use side effects

| Consumer | Uses |
|----------|------|
| **GVN/CSE** | `pure=True` → result can be cached (O105) |
| **ADCE** | `pure=True` + `NO_ESCAPE` → statement is removable |
| **Optimiser** | `pure=False` → cannot propagate across this command |
| **iRules flow** | `RESPONSE_LIFECYCLE` write → response-commit tracking |
| **Taint engine** | `pure=True` → taint flows through unchanged |

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
[`diagnostics.py:487`](../lsp/features/diagnostics.py) runs on every
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
[`diagnostics.py:568`](../lsp/features/diagnostics.py) runs in a
background thread via `asyncio.to_thread` to avoid blocking the editor.
It reuses the `CompilationUnit` from Phase 1 (shared IR, CFG, SSA,
and analysis results).

```
CompilationUnit (shared)
    │
    ├───► Optimiser (find_optimisations)
    │     → O100–O126: All optimisation suggestions
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
    │     → T200: Collect without release
    │     → T201: Release without collect
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
[`async_diagnostics.py`](../lsp/async_diagnostics.py) manages the
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

The suppression map `suppressed_lines: dict[int, frozenset[str]]` is built
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
  Token stream         SegmentedCommand        IRAssignConst         Instruction
  ┌──────────┐        ┌──────────────┐        ┌───────────┐        ┌───────────┐
  │ type:ESC │   ──►  │ texts:       │  ──►   │ name:"x"  │  ──►   │ op:PUSH1  │
  │ text:"set"│       │  ["set",     │        │ value:"42"│        │ operands: │
  │ start:0,0│        │   "x","42"]  │        │ range:... │        │  (0,)     │
  │ end:0,3  │        │ single:      │        └───────────┘        └───────────┘
  └──────────┘        │  [T, T, T]   │              │                    ▲
                      └──────────────┘              │                    │
                                                    ▼                    │
                                              CFGBlock            FunctionAsm
                                              ┌──────────┐       ┌───────────┐
                                              │ stmts:   │       │ literals: │
                                              │  [Assign]│       │  LitTable │
                                              │ term:    │  ──►  │ lvt:      │
                                              │  Goto    │       │  LVTTable │
                                              └──────────┘       │ instrs:   │
                                                    │            │  [Instr]  │
                                                    ▼            └───────────┘
                                              SSABlock                ▲
                                              ┌──────────┐           │
                                              │ phis: () │           │
                                              │ stmts:   │     codegen_module()
                                              │  SSAStmt │  ────────┘
                                              │ defs:    │
                                              │  {x: 1}  │
                                              └──────────┘
```

Each stage transforms the data into a richer representation:
1. **Tokens** — flat character-level classification (`tokens.py:33`)
2. **SegmentedCommand** — word-level grouping with command boundaries (`command_segmenter.py:65`)
3. **IR nodes** — typed, structured command semantics (`ir.py:265`)
4. **CFG blocks** — explicit control flow with terminators (`cfg.py:374`)
5. **SSA** — variable versioning with phi nodes at merge points (`ssa.py:195`)
6. **FunctionAnalysis** — constant values, types, liveness, dead stores (`core_analyses.py:176`)
7. **Bytecode** — executable instruction stream with literal/variable tables (`codegen/_types.py:91`)
