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

### Command values: the registry's routes

A command's value reaches the lattice through the route the registry
declares for the resolved invocation, never through a compiler arm keyed by
the command's name ([value-transfers.md](value-transfers.md)). The route runs
the runtime's own core over the compile-time value model under the target's
release, so `set x 010; incr x` is 9 under Tcl 8.6 and 11 under 9.0;
`append`, `lappend`, `set`, and the `dict` keyed updates give their
variable its exact new value; `string range`, `list`, `llength`, and
`string length` give their result. A profile that names no release gets
only the answer every release gives.

`expr` is the one *expression* route: the registry assembles its argument
words as the command specifies — one braced word is the expression text,
any other word is substituted first, several words join with one space
(`ExpressionSource`, `ExpressionRoute::assemble`) — and
`tcl_expr_eval::evaluate_expression` runs the shared engine over the same
lattice inputs, so `expr {"x"}` folds to the string `x`, not only a
number. A nested `[…]` inside the expression resolves through the
registry too, under an effect-free policy, and a `::tcl::mathfunc` call
checks its binding before folding — `expr {abs(-2)}` folds under
`tcl8.4` but declines once the module renames `::tcl::mathfunc::abs`
(`abs_rebinding_declines`). A branch condition folds the same way,
per member when exactly one of its reads is a finite SSA value.

**Example — `set s hello; set p again; append s $p; puts $s`:**

1. `s₁ = CONST("hello")`, `p₁ = CONST("again")`
2. The `append` route reads both from the lattice: `s₂ = CONST("helloagain")`
3. O104 folds the chain to `set s helloagain`, and O100 forwards the value
   into `puts`

### Destructuring writers and folded types (slice 5)

`regexp`, `regsub`, `scan`, `binary scan`, `lassign` and `array set` fold
through the same registry-owned routes: each declares its targets'
outcomes — `Write` (the target holds exactly this value), `Preserve`
(untouched — a no-match, or a `scan` field past the exhausted input),
`Unbind`, `MayWrite` (the value is bounded but not exact) or
`WriteElement` (one array element by key) — and the driver applies them
per place in execution order, so a repeated target composes (the last
write wins) and a no-match keeps the prior value rather than manufacturing
one. `binary format` packs its `H*`/`a`/`i`/… fields into the bytes every
release agrees on and constructs a byte array (`RepresentationEvidence::Constructed(ByteArray)`);
a rewrite never spells a computed byte array's bytes into the source
(`SccpResult::materialises`), so the optimised program still carries the
command that built it.

Beside the value, `SccpResult::folded_types` carries each definition's
`FoldedType` — the intrep, shape and representation evidence its
evaluation states, joined across a φ's members and forgotten at a
barrier. This is additive to type inference below: a folded type refines
only what the static typing in [Type inference lattice](#type-inference-lattice)
leaves unknown, and the shimmer purity read
(`is_pure_value`, `is_free_first_conversion`) checks representation
before the literal rule, so a computed constant such as `binary format`'s
byte array never hides a conversion. `tcl explore --show sccp` prints it
beside the value (`h#1 = const('ABCDEF')` · `type: bytearray
(constructed)`).

### When folding fails

- **A declined route**: the operand is not exact, the answer differs
  between the releases a profile can denote, or the module rebinds the
  command's name; the definition stays `OVERDEFINED`, and
  `tcl explore --show sccp` prints the reason (`declined:
  release-ambiguous: numeral-grammar`, `declined: rebinding-suspected`)
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
