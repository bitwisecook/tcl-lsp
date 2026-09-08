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

Source: `rust/tcl-compiler/src/sccp.rs`, `rust/tcl-compiler/src/type_infer.rs`,
and `rust/tcl-compiler/src/types.rs` (`TypeLattice`). Optimiser consumers
live under `rust/tcl-compiler/src/optimiser/`.

### Constant folding via SCCP

**Example — `expr {2 + 3}`:**

1. IR: `Statement::ExprEval { expr: ExprNode::Binary { op: BinOp::Add, left: Literal("2"), right: Literal("3") }, .. }`
2. SCCP evaluates: `CONST(2) + CONST(3)` → `CONST(5)`
3. Bytecode: `push1 "5"; done` — no arithmetic opcodes emitted

**Example — `set a 10; set b 20; expr {$a + $b}`:**

1. `a₁ = CONST("10")`, `b₁ = CONST("20")`
2. SCCP propagates through the expression: `CONST(10) + CONST(20)` → `CONST(30)`
3. O101 fires: suggest replacing `expr {$a + $b}` with `30`

Note: tclsh emits `loadStk + add` (variables could be modified by traces),
so the O101 suggestion is a diagnostic hint, not a bytecode transformation.
The Rust `tcl-compiler` codegen keeps that separation structurally. The
`TclVM` bytecode emitter never reads `fu.sccp` or a `LatticeValue`; it only
ever sees whatever source text it is handed, literal or not. The WASM
pipeline reads a `LatticeValue` in exactly one place —
`selected_closed_native_coverage` in `rust/tcl-compiler/src/codegen/wasm/pipeline.rs`,
which takes a `Const(Int)` as typed evidence for the default-off
sealed-program native-integer plan of
[semantic-aot-optimisation.md](semantic-aot-optimisation.md) — and never to
shortcut a variable read. A traced variable is therefore not independently
at risk from codegen: the risk is confined to the optimiser's own
*suggested source rewrite* being wrong, and the propagation passes gate on
the trace and alias facts before proposing one. The membership test is
`sccp::is_externally_mutable` (a `::`-qualified name, a name in the
function's escaping set, or any dynamic variable trace); the escaping set
comes from `var_observability::analyse_var_observability` extended with
`var_observability::scan_module_global_names` for the top-level body and
with the whole-module `Module::traced_variables` carried by
`sccp::TraceInputs`. SCCP applies that test to every def it evaluates, so a
constant branch or a folded value never involves a traced or aliased name
in the first place; `propagation::run_load_forwarding` (O102) applies it
independently because its def-use walk never consults `fu.sccp`; and
`propagation::run_store_to_load_forwarding` (O127) adds the memory-SSA
half through `memory_ssa::compute_aliases` when the unit carries no
`MemorySsa`. There is no separate bytecode-level shortcut to guard.

### When folding fails

- **Loop-carried values**: `phi(CONST, ...)` from a loop → `OVERDEFINED`
- **Impure commands**: result cannot be known at compile time
- **Unbraced expressions**: `ExprNode::Raw` — no AST to fold
- **Variable traces**: tclsh does not fold through variables (observable side
  effects), so our bytecode matches the non-folded output

### Type inference lattice

```
UNKNOWN → KNOWN(INT) → SHIMMERED(INT, STRING) → OVERDEFINED
```

`TypeLattice` (`types.rs`) is a `TypeKind` — `Unknown` / `Known` /
`Shimmered` / `Overdefined` — over a bounded set of `TypeShape`s.

| Source | Inferred type |
|--------|--------------|
| `"42"` | `KNOWN(INT)` |
| `"3.14"` | `KNOWN(DOUBLE)` |
| `"hello"` | `KNOWN(STRING)` |
| `"true"` / `"1"` | `KNOWN(BOOLEAN)` |
| `[string length $s]` | `KNOWN(INT)` (from `SubCommand::return_type`) |
| `[HTTP::uri]` | `KNOWN(STRING)` |

### Return type propagation

Commands with `SubCommand::return_type` or `CommandSpec::return_type`
contribute known types; a command whose result shape moves with the call
names a `return_type_hook` instead.  For example, `string length` has
`return_type: Some(TclType::Int)`, so `[string length $s]` is typed as INT.

### Shimmer detection

When a value typed as INT is used in a string context (or vice versa),
the type lattice records `SHIMMERED(from_type, to_type)`.  This triggers:
- S100: single shimmer outside a loop
- S101: shimmer inside a loop body
- S102: a variable oscillating between two types across iterations

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

- [Examples 3–4 in walkthroughs](example-walkthroughs.md#example-3-expr-2--3)
- [Example 6 — constant condition](example-walkthroughs.md#example-6-if-1----else----constant-condition)
- [GLOSSARY.md — SCCP, Lattice, Shimmer](../../GLOSSARY.md#sccp)
- [sccp-core-analyses.md](sccp-core-analyses.md)
- [optimisation-passes.md](optimisation-passes.md)
