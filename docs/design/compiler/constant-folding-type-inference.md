# Constant folding and type inference

How the compiler evaluates expressions at compile time and how type
information propagates through the SSA graph. Read this when adding a folding
rule, or when an expression that looks foldable is left alone.

Constant folding is performed by SCCP during core analysis.  When all
operands of an expression are `CONST`, SCCP evaluates the result at compile
time.  Type inference runs alongside, tracking `TypeLattice` values per SSA
key.  Together they enable optimisations O101 (fold constant expression),
O102 (forward a variable's single reaching literal load to its use
sites — see [O102's KCS
note](../../kcs/codes/kcs-optimisation-o102-load-forwarding.md) for the
current, corrected description; it is not itself an `[expr {...}]}`-result
fold, though it frequently feeds one), and O112 (constant condition).

Source: `rust/tcl-compiler/src/sccp.rs` and
`rust/tcl-compiler/src/type_infer.rs`. Optimiser consumers live under
`rust/tcl-compiler/src/optimiser/`.

### Constant folding via SCCP

**Example — `expr {2 + 3}`:**

1. IR: `IRExprEval(expr=ExprBinary(ADD, ExprLiteral("2"), ExprLiteral("3")))`
2. SCCP evaluates: `CONST(2) + CONST(3)` → `CONST(5)`
3. Bytecode: `push1 "5"; done` — no arithmetic opcodes emitted

**Example — `set a 10; set b 20; expr {$a + $b}`:**

1. `a₁ = CONST("10")`, `b₁ = CONST("20")`
2. SCCP propagates through the expression: `CONST(10) + CONST(20)` → `CONST(30)`
3. O101 fires: suggest replacing `expr {$a + $b}` with `30`

Note: tclsh emits `loadStk + add` (variables could be modified by traces),
so the O101 suggestion is a diagnostic hint, not a bytecode transformation.
The Rust `tcl-compiler` codegen (bytecode VM and WASM emitters) upholds
this separation structurally — it never reads `fu.sccp`/`LatticeValue`
directly; it only ever sees whatever source text it is handed, literal or
not. A traced variable is therefore not independently at risk from
codegen: the risk is confined to the optimiser's own *suggested source
rewrite* being wrong (which O100/O101/O102/O103/O109/O112 now gate on
`Module::traced_variables` / `has_dynamic_variable_trace` /
memory-SSA-aliasing before ever proposing a forward — see
`propagation::trace_and_alias_unsafe_names`), not from a separate
bytecode-level shortcut.

### When folding fails

- **Loop-carried values**: `phi(CONST, ...)` from a loop → `OVERDEFINED`
- **Impure commands**: result cannot be known at compile time
- **Unbraced expressions**: `ExprRaw` — cannot parse the AST
- **Variable traces**: tclsh does not fold through variables (observable side
  effects), so our bytecode matches the non-folded output

### Type inference lattice

```
UNKNOWN → KNOWN(INT) → SHIMMERED(INT, STRING) → OVERDEFINED
```

| Source | Inferred type |
|--------|--------------|
| `"42"` | `KNOWN(INT)` |
| `"3.14"` | `KNOWN(DOUBLE)` |
| `"hello"` | `KNOWN(STRING)` |
| `"true"` / `"1"` | `KNOWN(BOOLEAN)` |
| `[string length $s]` | `KNOWN(INT)` (from `SubCommand.return_type`) |
| `[HTTP::uri]` | `KNOWN(STRING)` |

### Return type propagation

Commands with `SubCommand.return_type` or `CommandSpec.return_type` contribute
known types.  For example, `string length` has `return_type=TclType.INT`, so
the result of `[string length $s]` is typed as INT.

### Shimmer detection

When a value typed as INT is used in a string context (or vice versa),
the type lattice records `SHIMMERED(from_type, to_type)`.  This triggers:
- S100: Value accessed as incompatible type
- S101: Implicit shimmer
- S102: Cross-command type conflict

### Interaction with optimisation passes

| Pass | Uses constant folding / types |
|------|------------------------------|
| O101 | All expr operands are CONST → fold |
| O102 | `[expr {…}]` result is CONST → replace |
| O110 | Algebraic identity with known types (e.g. `$x * 1` → `$x`) |
| O112 | Branch condition is CONST → eliminate dead branch |
| O113 | Strength reduction with known small constants |
| O117 | `[string length $s] == 0` with known INT return |
| O120 | `==`/`!=` on STRING-typed operands → `eq`/`ne` |

## Decision rule

- If an expression should be folded but isn't, check that all operands
  resolve to `CONST` in the SCCP results — any `OVERDEFINED` operand
  prevents folding.
- Type inference depends on `return_type` annotations on `SubCommand` /
  `CommandSpec` — add these when defining new commands.
- Shimmer warnings require both the "from" type and the "to" type to be
  known — `OVERDEFINED` values do not trigger shimmer warnings.
- Our bytecode matches tclsh's output (which does not fold through
  variables), but our optimiser *suggests* folding as diagnostics.

## Related docs

- [Examples 3–4 in walkthroughs](../../../docs/design/example-script-walkthroughs.md#example-3-expr-2--3)
- [Example 6 — constant condition](../../../docs/design/example-script-walkthroughs.md#example-6-if-1----else----constant-condition)
- [GLOSSARY.md — SCCP, Lattice, Shimmer](../../GLOSSARY.md#sccp)
- [kcs-sccp-core-analyses.md](../../../docs/design/compiler/sccp-core-analyses.md)
- [kcs-optimisation-passes.md](../../../docs/design/compiler/optimisation-passes.md)
