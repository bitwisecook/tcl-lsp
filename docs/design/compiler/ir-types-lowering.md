# IR types and lowering basics (Stage 3)

The IR node hierarchy, how a `SegmentedCommand` becomes a typed IR node, and
why a given command produces the node it does.

IR lowering transforms `SegmentedCommand` values into a `Module`
containing typed `Statement` variants.  The lowerer selects the most specific
variant possible — `AssignConst` for constants, `AssignExpr` for expressions,
`If` / `For` / `While` for control flow — falling back to `Call` for generic
commands.  Every statement carries a `Span` for diagnostic mapping.

Source: `rust/tcl-compiler/src/ir.rs`,
`rust/tcl-compiler/src/lowering/`

## Content

### IR node selection rules

| Tcl pattern | IR node | Why |
|-------------|---------|-----|
| `set x 42` (constant value) | `Statement::AssignConst` | Value known at compile time |
| `set x [expr {…}]` | `Statement::AssignExpr` | Expression can be statically analysed |
| `set x $y` / `set x "hi $n"` | `Statement::AssignValue` | Variable/interpolated — runtime resolution |
| `incr i` / `incr i 5` | `Statement::Incr` | Specialised increment |
| `expr {2 + 3}` | `Statement::ExprEval` | Standalone expression evaluation |
| `if {…} {…} else {…}` | `Statement::If` + `IfClause` | Structured control flow |
| `while {…} {…}` | `Statement::While` | Loop with structured condition |
| `for {…} {…} {…} {…}` | `Statement::For` | Loop with init/cond/step/body |
| `foreach var list body` | `Statement::Foreach` | Iteration |
| `catch {…} result` | `Statement::Catch` | Exception handling |
| `try {…} on … {…}` | `Statement::Try` + `TryHandler` | Structured exception |
| `switch $x {…}` | `Statement::Switch` + `SwitchArm` | Multi-way branch |
| `return $val` | `Statement::Return` | Procedure exit |
| `eval $script` | `Statement::Barrier` | Defeats static analysis |
| `puts $msg`, `regexp …` | `Statement::Call` | Generic command invocation |

### `Statement::AssignConst` vs `Statement::AssignValue`

The key distinction: does the lowerer know the value at compile time?

- `set x 42` → `argv[2].kind == ESC` and the text is a simple literal →
  `Statement::AssignConst { name: "x", value: "42", .. }`
- `set x $y` → `argv[2].kind == VAR` →
  `Statement::AssignValue { name: "x", value: "${y}", .. }`
- `set x {hello}` → `argv[2].kind == STR` →
  `Statement::AssignConst { name: "x", value: "hello", .. }`

### `Statement::Call`'s `defs` — variable definitions from commands

Commands like `regexp` define variables via match capture groups.  The lowerer
uses `ArgRole::VarWrite` to identify which arguments are variable names:

```rust
// regexp {(\d+)} $input match submatch
Statement::Call {
    command: "regexp".into(),
    args: vec![/* … */],
    defs: vec!["match".into(), "submatch".into()],
    ..
}
```

The `defs` vector tells the SSA builder that `regexp` creates new definitions
for `match` and `submatch`.

### `Statement::Barrier` — analysis boundary

Commands whose registry `arg_roles` mark an `ArgRole::Body` word that no
lowering hook specialises (`eval`, `uplevel`, `upvar`) produce
`Statement::Barrier`.  All downstream passes stop reasoning about variable
state at barrier points — the command can read/write any variable.

### `Module` structure

```rust
Module {
    source: /* the analysed text */,
    top_level: Script { statements: vec![/* … */] }, // code outside procs
    procedures: HashMap::from([("::add".into(), Procedure { .. })]),
    methods: HashMap::new(),                        // TclOO method bodies
    redefined_procedures: HashSet::new(),           // procs defined twice
    // …plus the namespace, trace, and TclOO evidence maps
}
```

Procedures are extracted from `top_level` into `procedures` during
lowering.  Top-level code emits `proc` registration as `invokeStk` calls
at codegen time.

### Expression bodies

Braced `expr` bodies are parsed into `ExprNode` AST trees at lowering time.
The AST lives inside `Statement::AssignExpr`'s and `Statement::ExprEval`'s
`expr` field, `Statement::If` clause conditions, and loop conditions.  Unbraced expressions fall back to
`ExprRaw` (diagnostic W100).

## Decision rule

- Use `Statement::AssignConst` only when the value is a compile-time constant
  (single-token `ESC` or `STR` with no interpolation).
- If a new command needs a special statement, set `lowering_hook` on its
  `CommandSpec` rather than dispatching on the command name in the lowerer.
- Commands with `ArgRole::Body` arguments that no hook handles produce
  `Statement::Barrier` — conservative but correct.
- Every statement must carry a `Span` — diagnostics need source positions.

## Related docs

- [Examples 1–4 in walkthroughs](../../../docs/design/example-script-walkthroughs.md#example-1-set-x-42)
- [Data structure reference — IR types](data-structure-reference.md#stage-3--ir-types-irrs)
- [kcs-lowering-dispatch.md](../../../docs/design/compiler/lowering-dispatch.md)
- [kcs-lowering-contracts.md](../../../docs/design/compiler/lowering-contracts.md)
