# Type tracking — comprehensive value-type model (design)

Status: **in progress** — this document is the contract for the type-tracking
redesign started against issue #940's follow-up work. Each claim in the oracle
corpus below is tclsh-verified (8.6.14; 9.0 differences noted where known) and
must be locked in by a test when the corresponding phase lands.

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

## Phasing

1. **P1 — commit dataflow (shimmer-local)** ✅ **landed**:
   `shimmer::commit` (`CommitState` must/may pass over SCCP-executable
   blocks); the use-site / expr / `incr` detectors consult it (second
   conversions fire with the committed from-type, merges fire only when every
   path pays, loop re-thunks are S101 with the steady-state rep);
   `first_use_commitments_for_cu` feeds hover's
   "string (first used as: list)". Locked in by `shimmer/commit.rs` unit
   tests, FP-SH-22, and the `commit_dataflow_*` lsp_e2e suite.
2. **P2 — TypeShape + unions**: introduce alongside `TypeLattice`, migrate
   consumers via accessors; `element_class` generalises to `Elements`.
   Unify the three ad-hoc union carriers (SCCP `ConstSet` ≤ 32, the
   `Shimmered` pair, `class_lattice::ClassValue::Set`) behind one bounded
   type-set primitive, un-masking the 3+-way merge collapse at
   `types.rs` `type_join` (documented lossy point).
3. **P3 — container element inference**: `list`/`dict create`/`lappend`
   builders + literal parses populate `Elements`; retrieval sites read them.
   The VM already stores per-element typed values (`IntRep::List(Rc<Vec<Value>>)`),
   so the static facts mirror the runtime truth.
4. **P4 — bignum-exact folding**: `ConstValue` gains exact beyond-wide
   integers, folding `2**64` / `i64max+1` as C does. Reuse the faithful
   runtime's tower (`runtime/rust/src/bignum.rs` — `NumVal::Wide/Big/Float`
   with `store`'s demote-when-fits canonicalisation), which already matches
   `docs/design/contracts/numeric-tower-and-expr-semantics.md`; today's
   compiler folder *declines* on overflow (sound, never wraps), so this is an
   enhancement, not a soundness fix.

Each phase lands with its slice of the oracle corpus as tests (unit + FP-SH
catalogue + lsp_e2e where user-visible).

## Cross-links

- `docs/design/contracts/shimmer-reference-behaviour.md` — shimmer contract.
- `docs/design/compiler/FP.md` §FP-SH-21 — pure-first-conversion catalogue.
- `docs/design/contracts/numeric-tower-and-expr-semantics.md` — expr numeric
  semantics.
- `rust/tcl-compiler/src/shimmer/hints.rs` — `is_pure_intrep` /
  `is_valid_instance_of` building blocks.
