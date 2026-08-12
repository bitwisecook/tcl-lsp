# KCS: IR types and lowering basics (Stage 3)

## Symptom

A contributor needs to understand the IR node hierarchy, how commands are
lowered from `SegmentedCommand` into typed IR nodes, or why a particular
command produces a specific IR node type.

## Context

IR lowering transforms `SegmentedCommand` values into a `Module`
containing typed `Statement` nodes.  The lowerer selects the most specific
variant possible — `Statement::AssignConst` for constants,
`Statement::AssignExpr` for expressions, `Statement::If` / `For` / `While` for
control flow — falling back to `Statement::Call` for generic commands.  Every
statement carries a `Span` for diagnostic mapping.

Source: `rust/tcl-compiler/src/ir.rs`,
`rust/tcl-compiler/src/lowering/mod.rs`,
`rust/tcl-compiler/src/lowering_hooks.rs`

## Content

### IR node selection rules

| Tcl pattern | `Statement` variant | Why |
|-------------|---------------------|-----|
| `set x 42` (constant value) | `AssignConst` | Value known at compile time |
| `set x [expr {…}]` | `AssignExpr` | Expression can be statically analysed |
| `set x $y` / `set x "hi $n"` | `AssignValue` | Variable/interpolated — runtime resolution |
| `incr i` / `incr i 5` | `Incr` | Specialised increment |
| `expr {2 + 3}` | `ExprEval` | Standalone expression evaluation |
| `if {…} {…} else {…}` | `If` | Structured control flow |
| `while {…} {…}` | `While` | Loop with structured condition |
| `for {…} {…} {…} {…}` | `For` | Loop with init/cond/step/body |
| `foreach var list body` | `Foreach` | Iteration |
| `catch {…} result` | `Catch` | Exception handling |
| `try {…} on … {…}` | `Try` | Structured exception |
| `switch $x {…}` | `Switch` | Multi-way branch |
| `return $val` | `Return` | Procedure exit |
| `eval $script` | `Barrier` | Defeats static analysis |
| `puts $msg`, `regexp …` | `Call` | Generic command invocation |

Two further variants have no direct source-pattern counterpart:
`Statement::Block` splices an inline group of statements into the enclosing
script without introducing a scope (produced by `inline_uplevel` and by
const-propagation when an `eval` / `uplevel` body resolves to a brace
literal), and `Statement::UpFrame` carries an `uplevel` body together with the
`frame_shift` magnitude and the `absolute` flag that distinguishes `uplevel
#0` from `uplevel 0`.

### `AssignConst` vs `AssignValue`

The key distinction: does the lowerer know the value at compile time?

- `set x 42` → the value token's `kind` is `Esc` and the text is a simple
  literal → `AssignConst { name: "x", value: "42", .. }`
- `set x $y` → the value token's `kind` is `Var` →
  `AssignValue { name: "x", value: "${y}", .. }`
- `set x {hello}` → the value token's `kind` is `Str` →
  `AssignConst { name: "x", value: "hello", braced: true, .. }`

### `Statement::Call::defs` — variable definitions from commands

Commands like `regexp` define variables via match capture groups.  The lowerer
uses the registry's `ArgRole::VarWrite` role to identify which arguments name
variables. For `regexp {(\d+)} $input match submatch`, the resulting `Call`
carries `command: "regexp"` and `defs: vec!["match", "submatch"]`.

`defs` tells the SSA builder that `regexp` creates new definitions for
`match` and `submatch`. The companion fields refine that: `reads` lists
variables read beyond the `$`-references in `args`, `reads_own_defs` marks a
read-before-write of the defined names, and `safe_on_uninit` records whether
it is safe for the defined variables to have been uninitialised.

### `Statement::Barrier` — analysis boundary

A command whose registry spec carries `Traits::CREATES_BARRIER` or
`Traits::EVALUATES_CODE` (`eval`, `uplevel`, `upvar`, …) lowers to
`Statement::Barrier`, which records a human-readable `reason` alongside the
original command and arguments.  All downstream passes stop reasoning about
variable state at barrier points — the command can read or write any
variable.

### `Module` structure

The top-level container (`rust/tcl-compiler/src/ir.rs`) holds, among other
fields:

```rust
pub struct Module {
    /// The original source text this module was lowered from.
    pub source: String,
    /// Top-level script (code outside any procedure).
    pub top_level: Script,
    /// Named procedures.
    pub procedures: HashMap<String, Procedure>,
    /// Named methods (keyed by `class::method`).
    pub methods: HashMap<String, MethodDef>,
    /// Synthetic *body units* — `apply` lambdas and `namespace eval` bodies.
    pub body_units: HashMap<String, Procedure>,
    /// Procedure names that were defined more than once.
    pub redefined_procedures: HashSet<String>,
    // … OO evidence, namespace imports/exports, trace facts, …
}
```

Procedures are extracted from `top_level` into `procedures` during lowering.
`body_units` are lowered into `Procedure`s purely so the static-analysis
pipeline (CFG → SSA → SCCP → taint) reaches inside them; they are kept in a
separate map so codegen — which emits `procedures` — never materialises them
as callable procs. Top-level code emits `proc` registration as `invokeStk`
calls at codegen time.

### Expression bodies

Braced `expr` bodies are parsed into `ExprNode` trees at lowering time.
The tree lives inside `AssignExpr::expr`, `ExprEval::expr`, `If` clause
conditions, and loop conditions.  Unbraced expressions fall back to the raw
form (diagnostic W100).

## Decision rule

- Use `AssignConst` only when the value is a compile-time constant
  (single-token `Esc` or `Str` with no interpolation).
- If a new command needs special IR, register a lowering hook
  (`CommandSpec::lowering_hook`, dispatched through
  `rust/tcl-compiler/src/lowering_hooks.rs`) rather than adding a command-name
  branch to the lowerer.
- Commands with `ArgRole::Body` arguments that are not explicitly handled
  produce `Statement::Barrier` — conservative but correct.
- Every statement must carry a `Span` — diagnostics need source positions.

## Related docs

- [Examples 1–4 in walkthroughs](../../../docs/design/example-script-walkthroughs.md#example-1-set-x-42)
- [Data structure reference — IR types](../../../docs/design/example-script-walkthroughs.md#stage-3--ir-types-corecompilerirpy)
- [kcs-lowering-dispatch.md](../../../docs/design/compiler/lowering-dispatch.md)
- [kcs-lowering-contracts.md](../../../docs/design/compiler/lowering-contracts.md)
