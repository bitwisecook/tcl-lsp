# Shimmer reference behaviour

## What we mean by shimmer

A Tcl object can be used as different semantic types over time (string, list, int, double, etc.). Shimmering is the conversion churn between internal representations when usage changes. In C Tcl, this happens when `Tcl_ConvertToType()` calls `FreeInternalRep` on the old type and `setFromAnyProc` on the new type.

## Practical expectations for this project

- A one-off mismatch at a use site is informative (S100).
- Repeated mismatch in loops is more expensive and should be elevated (S101).
- Oscillation patterns across loop iterations are the strongest signal (S102).

Two further codes sit in the same module but answer different questions:

- **S103** — mutation of a **potentially shared** value. Not a
  representation change at all: C Tcl duplicates a shared value before
  writing it, so `lappend` / `lset` / `dict set` on a value with refcount ≥ 2
  is an O(n) whole-value copy every call. Detected by `shimmer::sharing`,
  severity Hint. It is a deliberate under-approximation: it fires only where
  the pass can see *both* holders, starting from a same-block pure-copy
  assignment (`set b $a`).
- **S110** — a **correctness** shimmer, distinct from the S100/S101/S102
  performance family: a byte array forced through a character-string
  operation and written back to a byte sink silently re-encodes every byte
  `>= 0x80`. Detected by `shimmer::byte_array`; see
  [byte-array-corruption.md](../compiler/byte-array-corruption.md).

## Mapping to C Tcl 9.0.3 functions

Each detector diagnostic maps to specific C functions that trigger `FreeInternalRep`:

| Detector diagnostic | C function / macro | Source file |
|---|---|---|
| STRING → LIST (S100/S101) | `Tcl_GetListFromObj` → `SetListFromAny` | `tclListObj.c` |
| STRING → INT (S100/S101) | `TclGetIntFromObj` / `Tcl_GetWideIntFromObj` | `tclObj.c` |
| STRING → DOUBLE (S100/S101) | `Tcl_GetDoubleFromObj` | `tclObj.c` |
| STRING → BOOLEAN (S100/S101) | `Tcl_GetBooleanFromObj` | `tclObj.c` |
| STRING → DICT (S100/S101) | `Tcl_DictObjGet` → `SetDictFromAny` | `tclDictObj.c` |
| STRING → INT via `incr` | `TclIncrObj` → `TclGetNumberFromObj` | `tclObj.c` |
| STRING → NUMERIC in `expr` | `TclGetNumberFromObj` in `INST_ADD` etc. | `tclExecute.c` |
| INT/DOUBLE → STRING in `expr` | `Tcl_GetStringFromObj` in `INST_STR_EQ` etc. | `tclExecute.c` |
| LIST ↔ DICT oscillation | Bidirectional `SetListFromAny` / `SetDictFromAny` | `tclListObj.c`, `tclDictObj.c` |
| BOOLEAN → INT promotion | `TclGetIntFromObj` (cheap path) | `tclObj.c` |

### Numeric interchangeability

BOOLEAN → INT promotion is **not** flagged because it matches Tcl 9.0's O(1)
conversion path. `shimmer::hints::is_numeric_compatible(current, expected)`
implements this as an **equivalence class**, not a subtype hierarchy:
`Boolean`, `Int`, `Double`, and `Numeric` are mutually interchangeable in
arithmetic and boolean contexts, in either direction, and no intrep
conversion is needed between any pair of them.

`Double` belongs in that class because `Tcl_GetNumberFromObj` and
`Tcl_GetBooleanFromObj` read a cached `tclDoubleType` intrep in place, and
`Tcl_GetDoubleFromObj` widens a cached int / boolean intrep without replacing
it. Verified on tclsh 8.6 and 9.0 with
`::tcl::unsupported::representation`: after `set u [expr {1.0 + 1.5}]`, both
`expr {$u * 2}` and `expr {$u && 1}` leave `u` holding the same double
intrep, and after `set n [expr {1 + 2}]`, `expr {$n * 1.5}` leaves `n`
holding the same int intrep. Excluding it was the S100 false positive in
issue #1814 — a double accumulator (`set u0 0.0` … `expr {$u0 * $dx}`)
reported as "has double intrep used in arithmetic expression".

`Double` → `Int` is the one direction left out. Tcl never reads a double
where an integer is required: the read either errors with the double intrep
intact (`incr`, `string index`) or re-represents on the way to the error
(`lindex {a b c d} $d` with `$d` = 2.0 replaces the double intrep), so it is
not a free numeric read either way.

### When shimmering does NOT occur

- Same-type access (fast path in all `Tcl_Get*FromObj` functions)
- String rep generation from intrep (intrep is preserved alongside string rep)
- Pure string objects (`typePtr == NULL`) — first type assignment is not a shimmer
- Shared object duplication (`Tcl_DuplicateObj`) — original intrep is not affected

#### How the analyser implements "first type assignment is not a shimmer"

The compiler classifies committed-vs-pure generically (never by command name)
through `shimmer::hints::is_uncommitted_first_conversion`, using the type
lattice plus the SCCP constant lattice:

- A `String`-typed value (`TclType::String` is documented "pure string, no
  cached intrep") is always uncommitted.
- A numeric-typed value is uncommitted only when it is a compile-time constant
  (a constant-folded literal push, still `typePtr == NULL`); a runtime
  `expr` / `incr` result is a committed numeric intrep.
- `List` / `Dict` / `Object` / `ByteArray` / `Channel` are always committed
  (only `[list]` / `[dict create]` / a constructor / `binary format` produce
  them).

A use is suppressed only when the pure value is a **valid instance** of the
required type — a well-formed list (`Tcl_SplitList` succeeds), an even-length
list for a dict, or a parseable number — so a genuine runtime error (`incr` on
`hello`) still fires. `foreach $bracedList` (issue #940) is the anchor case,
pinned as `FP-SH-21` in
`rust/tcl-compiler/src/analyser/diagnostics/fp/sh.rs`. The
`cargo xtask fp-sweep` harness ([fp-sweep.md](../compiler/fp-sweep.md)) is
what a shimmer-emitter change is measured against before it lands.

#### The committed-intrep dataflow (first-use commit)

The follow-up above is implemented by `shimmer::commit` — a forward must/may
dataflow over the SCCP-executable blocks, shared by the use-site, expr, and
`incr` detectors. Per SSA value it tracks the bounded set of intreps the value
*may* have committed on some path and whether *every* path has committed one:

- **Second conversions fire with the true from-type**: straight-line
  `set v 5; expr {$v + 1}; lindex $v 0` reports "numeric intrep but `lindex`
  expects list" (oracle: rep `int` → `list`), with an "intrep first committed
  here" related span. Same through `llength` → `incr` (List → Int).
- **Branch arms stay silent, merges fire only when every path pays**: with
  `if {$c} { llength $a } else { dict size $a }`, each arm's own first
  conversion is free; a post-merge use matching *one* arm stays silent (only
  the other path pays — not an every-execution claim), while a use matching
  *neither* fires with path-dependent wording ("has path-dependent (list or
  dict) intrep …").
- **Loop re-thunk**: a pure value read as two distinct intreps inside a loop
  (`llength $l` + `dict size $l` per pass) re-converts every iteration
  (oracle: list ↔ dict) — both reads are S101, the steady-state rep naming the
  from side.
- **Def-site pushback**: a pure def whose every executable typed read commits
  the same intrep exposes it via
  `shimmer::first_use_commitments_for_cu` — hover renders
  "string (first used as: list)" at the creation site.

## Where the command-level knowledge lives

The mapping above is validated against the Tcl 9.0.3 C sources. The `arg_types`
shimmer hints
carried on each `CommandRegistry` `CommandSpec`/`SubCommand` (see
`rust/tcl-registry/src/commands/**`) correctly map Tcl commands to their
underlying `Tcl_Get*FromObj` calls — command-level shimmer knowledge lives
there as data (`ArgTypeHint { expected, shimmers }`), never hard-coded in the
compiler.

## Detection scope

Shimmer analysis (`rust/tcl-compiler/src/shimmer/`) runs over every
analysable function unit in a compilation unit — top-level code, `proc`
bodies, TclOO method bodies (`cu.methods`), and synthetic body units such as
`namespace eval` bodies and `apply` lambdas (`cu.body_units`) — not just
top-level procs. Command resolution follows `interp alias` through
`canonical_command`, so an alias to a shimmering builtin (e.g. `interp alias
{} myindex {} ::lindex`) is detected the same as calling the builtin
directly. A use is treated as unstably-typed (no shimmer flagged) when the
variable carries a live write-trace, either in the same function
(`var_observability::analyse_var_observability`) or anywhere else in the
module (`ModuleVariableTraces`) — a traced variable's type cannot be
statically trusted, since the trace callback may rewrite it.

Diagnostic spans are tightened to the offending argument (or substitution)
rather than the whole statement — see `shimmer::use_site::InvocationSite`
and `value_shapes::parse_command_substitution_with_spans`. The one
documented exception is `expr {...}` bodies: `ExprNode` offsets have no
absolute-position anchor without a larger IR change, so shimmer inside an
expression string still spans the whole statement.

Two narrower residual gaps remain in the interp-alias handling (see the
doc comment on `shimmer::use_site::check_invocation`): an alias that
prepends fixed arguments (`interp alias {} foo {} ::bar prefix`) does not
index-shift the checked argument, and a read-modify-write shimmering
argument that is a bare variable name rather than a `$`-prefixed read
(e.g. `interp alias {} myincr {} ::incr; myincr x`) is not seen through an
alias, since `incr`'s own canonical name bypasses this path via the
dedicated `Statement::Incr` node.

## Coverage

- Unit tests co-located with each shimmer module (`rust/tcl-compiler/src/shimmer/*.rs`) and in `rust/tcl-compiler/tests/checks.rs`.
- TP/FP/TN/FN regression fixtures in `rust/tcl-compiler/src/analyser/diagnostics/fp/sh.rs` (the `FP-SH-NN` series).
- Native `lsp_e2e` coverage in `rust/tcl-lsp-server/tests/e2e/diagnostics.rs` and `rust/tcl-lsp-server/tests/e2e/code_actions.rs` (the noqa suppress quick fix).
- VS Code integration coverage in `editors/vscode/src/test/shimmerPrecision.test.ts` against `editors/vscode/testFixture/shimmerPrecision.tcl`.

## Cross-links

- Implementation: `rust/tcl-compiler/src/shimmer/` — `mod.rs` (the
  per-unit entry points), `hints.rs` (registry hints, numeric
  compatibility, the uncommitted-first-conversion rule), `use_site.rs`,
  `expr.rs`, `commit.rs` (the committed-intrep dataflow), `thunking.rs`,
  `sharing.rs` (S103), `byte_array.rs` (S110), `phi.rs`, `graph.rs`,
  `span.rs`.
- Registry data: `rust/tcl-registry/src/commands/**` (`arg_types` on each `CommandSpec`/`SubCommand`).
- Suppression: `rust/tcl-compiler/src/analyser/utils.rs` (`parse_noqa_line_suppressions`, `apply_preceding_noqa`), consumed by `lift_compiler_diagnostics` in `rust/tcl-lsp-server/src/lib.rs`.
