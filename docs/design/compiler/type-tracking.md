# Type tracking — the value-type model

The contract for how the compiler models Tcl value types. Every claim in the
oracle corpus below is tclsh-verified (8.6.14, with 9.0 differences noted where
known) and locked in by a test.

## Goals

1. Model Tcl values the way **C Tcl** does — a string with at most one cached
   internal representation (intrep) — including *purity* (`typePtr == NULL`),
   the numeric tower (`int` wide / `bignum` / `double` / `booleanString`), and
   per-element container types.
2. Carry **multiple potential types** at a lattice node (bounded union), not
   just a single `Known` type or a `Shimmered` pair.
3. Compute **implied element types into containers** — `List<T1..Tn>` /
   `Dict<K,V>` / `Array<...>` — wherever provable, generalising today's
   object-only `element_class`.
4. Track **first-use commitment** flow-sensitively: a pure value's first read
   as type `T` commits intrep `T` at that program point; later reads as `U ≠ T`
   are genuine shimmers. Push the committed type back to the def site for
   visibility (hover) when all paths agree.
5. Give the **analyser and optimiser** leverage: exact from-types for S100/
   S101, must-vs-may warning policies over unions, list-op specialisation and
   folding over known element types, exact bignum-aware constant folding.

## Oracle corpus (tclsh 8.6.14, `tcl::unsupported::representation`)

### Purity and first conversion

| Program | Rep afterwards | Fact |
|---|---|---|
| `set x {10.0 12.0 16.0 24.0}` | `pure` | any literal is a pure string, whatever its shape |
| `set x 5` | `pure` | number-shaped literal is NOT a committed int |
| … then `foreach s $x` / `lindex $x 0` | `list` | first read installs the intrep once, losslessly (no shimmer) |
| `set l [list 1 2 3]` | `list` | command results are committed |
| … then `string length $l` | `string` | committed → different type = genuine shimmer |
| `set v 5; expr {$v+1}` | `int` | arithmetic commits the numeric intrep |
| … then `lindex $v 0` | `list` | second conversion = genuine shimmer (currently a known FN) |
| `set s [string trim "a b c"]` | `pure` | string-command results are pure |

### Numeric tower

| Expression | Result | Result rep | Fact |
|---|---|---|---|
| `expr {9007199254740992+1}` | `9007199254740993` | `int` | exact integer arithmetic at 2^53 (i64 wide) |
| `expr {9007199254740992+1.0}` | `9007199254740992.0` | `double` | one double operand contaminates; f64 precision loss is real and must be folded as C does |
| `expr {2**64}` | `18446744073709551616` | `bignum` | wide→bignum promotion is seamless, by result magnitude |
| `expr {$big + 1}` (bignum) | exact | `bignum` | bignum arithmetic exact |
| `expr {$big - $big}` | `0` | `int` | bignum can demote back to wide |
| `expr {9223372036854775807 * 2}` | exact | `bignum` | i64 overflow promotes, never wraps |
| `expr {7/2}` / `expr {-7/2}` | `3` / `-4` | `int` | floor division |
| `expr {-7%2}` | `1` | `int` | floor modulus |
| `incr b` on a bignum | exact | `bignum` | incr participates in the tower |
| `set t true; expr {$t && 1}` | — | `booleanString` | word-booleans have their own rep (keeps the spelling); numeric booleans are just `int` |

Current-compiler baseline (verified via `tcl explore` optimisedSource):
`-7/2 → -4` and `int+double → double` fold correctly; `i64max+1` and `2**64`
are (soundly) **not** folded — the enhancement is exact bignum folding, not a
wrapping fix.

### Containers hold typed objects by reference

| Program | Read-back rep | Fact |
|---|---|---|
| `set n [expr {2**20}]; set l2 [list $n other]; lindex $l2 0` | `int` | elements are shared `Tcl_Obj`s; a computed element **keeps its intrep** inside the list |
| `dict create k1 [expr {1+1}] …; dict get $d k1` | `int` | same for dict values |
| `foreach x {42 3.14 word}` | `pure` each | braced-literal elements are parsed fresh — pure |
| `lindex [list 1 2.5 {a b}] i` (word literals) | `string` | parser literal-table words carry the plain string rep |
| `array set arr {k 5}; expr {$arr(k)+1}` | `int` | array elements behave as independent scalars |

Consequence: `List<T1..Tn>` per-position element types are **faithful to the
runtime**, not an abstraction — `[list [expr {1+1}] "x y"]` genuinely is
`List<Int, String>`.

### Shimmer interactions (already locked in by FP-SH-20/21)

- Comparison operators probe per-operand without generating a string rep and
  fall back to string comparison — never flagged statically.
- `string` subcommand transparency (`transparent_from`) and dual-ported reads
  keep intreps — registry-declared, per subcommand.
- A pure value's first conversion is free (issue #940, FP-SH-21).

## Design

### 1. `TypeShape` — the value-type term

```text
TypeShape ::= PureString                       ;# typePtr == NULL (any string)
            | WideInt | Bignum | Double
            | BooleanWord                      ;# "true"/"off"… word reps
            | StringRep                        ;# committed UTF cache
            | ByteArray
            | List(Elements) | Dict(Elements)  ;# element shapes
            | Object(class) | Channel
Elements  ::= Exact([TypeShape; n])            ;# small, per-position (n ≤ K)
            | Uniform(TypeShape)               ;# homogeneous, any length
            | Unknown
```

`TclType` (registry) stays the coarse public vocabulary; `TypeShape` refines it
inside the compiler. `Numeric` remains the abstract join of the numeric tower.

### 2. Union nodes

A lattice node carries a **bounded set** of possible `TypeShape`s (mirroring
SCCP's `ConstSet`, widening to `Overdefined` beyond the bound). `Shimmered(a,b)`
becomes the 2-union carrying its provenance; existing consumers read through
compatibility accessors.

Warning policy over unions: *must*-style diagnostics (S100 use-site, W126)
fire only when **every** member mismatches; *may*-style notes can be introduced
later where a single risky member justifies it.

### 3. Purity and first-use commitment (flow-sensitive)

Per program point, per SSA `(sym, ver)`:

```text
CommitState ::= Pure | Committed(TypeShape, first_span) | Conflict(a, b)
```

- Forward must-dataflow over SCCP-executable blocks; registry-declared shimmer
  positions are the transfer function (a use at an `expected`-typed position
  moves `Pure → Committed(expected)` when the value is a valid instance).
- Join: equal commitments survive; differing ones → `Conflict` — a later use
  matching *neither* side fires (at least one path pays), matching the
  "multiple different dominator types" analysis.
- Defs reset state (a new SSA version starts from its producer's shape).
- Def-site pushback: when every use of a version commits the **same** shape,
  expose `(def_span → shape)` for hover/inlay ("first used as: list").

### 4. Numeric tower in constant folding

`ConstValue` gains exact integer semantics beyond i64 (bignum), folding
`2**64` and `i64max+1` exactly as C does, keeping floor div/mod, and modelling
`int⊕double → double` with genuine f64 rounding. Never wrap; never fold what C
would not.

### 5. Consumers and leverage

| Consumer | Today | With this design |
|---|---|---|
| S100/S101 use-site | single Known type; pure-first-conversion free | exact committed from-type + Conflict-aware second-conversion detection |
| Hover/inlay | dominant single type | purity + committed shape + "first used as" pushback |
| Optimiser const-fold | i64-bounded | exact bignum folds |
| List-op passes | no element facts (object `element_class` only) | `lindex` fold on `Exact` elements, `llength` on `Exact(n)`, guard specialisation on `Uniform` |
| W126/W307/W308 | single type | must-policy over unions (fewer FPs on merged paths) |

## The model, part by part

### Commit dataflow (shimmer-local)

`shimmer::commit` (`CommitState` must/may pass over SCCP-executable
blocks); the use-site / expr / `incr` detectors consult it (second
conversions fire with the committed from-type, merges fire only when every
path pays, loop re-thunks are S101 with the steady-state rep);
`first_use_commitments_for_cu` feeds hover's
"string (first used as: list)". Locked in by `shimmer/commit.rs` unit
tests, FP-SH-22, and the `commit_dataflow_*` lsp_e2e suite.

### `TypeShape` and unions

`TypeLattice` is a bounded
canonical union of `TypeShape`s (`bounded_set::BoundedSet`, cap
`MAX_TYPE_UNION`); 3+-way merges stay tracked (phi reports every member,
hover/inlay render full unions), `Shimmered` survives as the ≥2-member
classification, numeric members collapse per the tower, and W126 runs a
must-policy over members. SCCP's `ConstSet` and the class lattice keep
their own storage deliberately — their dedup is not plain equality
(`cv_eq` numeric cross-equality; sorted persistent sets) — documented in
`bounded_set.rs`.

### Container element inference

Registry facts
(`ReturnElements` on `list`/`dict create`/`lindex`/`dict get`/`lrange`,
`VarElementsEffect` on `lappend`/`dict set/append/lappend`,
`VarWriteTyping::ElementsOf` on `lassign`/`foreach`/`lmap`) drive
`type_infer`'s element machinery — the old object-only
`collection_element_class` / `container_retrieval_object_type`
command-name matches are deleted. Pure/value-unknown element positions
stay agnostic (`None`) so FP-SH-17's pins hold; committed sources carry
real shapes (`[list [expr {2**20}] x]` is `List<Numeric, ?>`, and
`lassign` of an object list types its targets).

### Bignum-exact folding

`TclValue::Big` over
`num-bigint` folds `2**64` / `i64::MAX + 1` / `1 << 63` to C's exact
values (demote-when-fits; SCCP carries a bignum as its canonical decimal
string so chained folds re-parse exactly). The operator semantics live
once in `tcl_syntax::number_tower` (`BigIntOps` backend trait), and every
backend is pinned by the shared, generic
`number_tower::conformance::assert_backend` corpus: the compiler and the
**VM** (`tcl_vm::expr`, adopted — beyond-i128 operands, `dict incr`
promotion, and the exact `int()`/`wide()`/`entier()`/`abs()`/`double()`
windows included) run it over `num-bigint`, and the faithful runtime
runs it over the **real libtommath `mp_int`** (`runtime/rust`'s
`TowerMp` adapter) — backend swappable with zero semantic drift, now
proven by one test per backend rather than by review.
Float edges are oracle-pinned (`tcl_expr_eval::float_edge_oracle_table`):
NaN comparisons are IEEE-unordered values while NaN operands/results in
arithmetic decline (C errors), `Inf` propagates, signed zero and
denormals round-trip, `isqrt` is exact at the f64-rounding edge, and an
inexact bignum→double contamination declines rather than bet on rounding
parity.

### Arrays as per-constant-key scalars

A Tcl array is a collection of *variables*, not a value; the oracle corpus
("array elements behave as independent scalars") fixes the design. Each
**constant-keyed** element (`arr(k)`, `set {a($x)} v`'s literal `$x`) is
its own SSA variable (`naming::element_var_name`; the def/use scanners,
expr AST, lowering `Call` defs, and `def_use` terminator reads all report
element-qualified names), so it types, folds, and shimmer-checks as the
independent scalar it is — per-element hover, per-element SCCP constants,
and the *same*-element loop oscillation a true S102 (conflating elements
onto the base would force a false negative there).

A dynamic key (`arr($i)`) stays on the conflated base, and its write
**fans** as `SsaStatement::may_defs` over the array's known elements:
each fanned def records a use of the element's prior version, and both
SCCP and type inference **join** old with written (`set a($k) 9` over an
INT `a(x)` stays INT; over a STRING one widens; constants become sets,
never a wrong single fold). An element write also refreshes the base (a
may-def carrying no value of its own — `$arr` never folds to an
element's value) so whole-array readers (`array get`) stay ordered.
Base-name policy sets (special vars like `env`, traced, scope-alias,
instance/cross-event) are consulted through the element's base, keeping
`set env(FOO) x` and traced/instance elements exempt as before. The
FP-SH-13 exclusions remain only for the base symbols they were built
for; element symbols get full precision.

### Boolean acceptors (companion centralisation)

C Tcl's two boolean acceptors live once in `tcl_syntax::boolean`, both
oracle-table-pinned: `parse_boolean_strict` (`ParseBoolean` — word prefixes
plus exactly `0`/`1`; `string is boolean`) and `truthiness`
(`Tcl_GetBooleanFromObj` — word else any number vs zero; `NaN` is a domain
error, `±Inf` truthy). Every former hand-rolled word list (`string is`
const-fold, VM `as_bool`, intervals, the folder's boolean contexts, the
type classifier, expr-simplify's vocabulary) now routes through them; the
migration fixed two real divergences (VM treated `NaN` as truthy; the
folder folded `!NaN`).

Each phase lands with its slice of the oracle corpus as tests (unit + FP-SH
catalogue + lsp_e2e where user-visible).

## Cross-links

- `docs/design/contracts/shimmer-reference-behaviour.md` — shimmer contract.
- `rust/tcl-compiler/src/analyser/diagnostics/fp/sh.rs` — the paired
  pure-first-conversion regression tests.
- `docs/design/contracts/numeric-tower-and-expr-semantics.md` — expr numeric
  semantics.
- `rust/tcl-compiler/src/shimmer/hints.rs` — `is_pure_intrep` /
  `is_valid_instance_of` building blocks.
