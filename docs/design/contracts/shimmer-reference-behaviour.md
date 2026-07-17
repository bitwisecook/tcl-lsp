# KCS: Shimmer reference behaviour and validation

## What we mean by shimmer

A Tcl object can be used as different semantic types over time (string, list, int, double, etc.). Shimmering is the conversion churn between internal representations when usage changes. In C Tcl, this happens when `Tcl_ConvertToType()` calls `FreeInternalRep` on the old type and `setFromAnyProc` on the new type.

## Practical expectations for this project

- A one-off mismatch at a use site is informative (S100).
- Repeated mismatch in loops is more expensive and should be elevated (S101).
- Oscillation patterns across loop iterations are the strongest signal (S102).

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

### Numeric subtype hierarchy

BOOLEAN → INT promotion is **not** flagged because it matches Tcl 9.0's O(1) conversion path. The `_is_numeric_compatible` function implements: BOOLEAN ⊆ INT ⊆ NUMERIC, DOUBLE ⊆ NUMERIC.

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
`hello`) still fires. This is what fixed issue #940 (`foreach $bracedList`); see
`docs/design/compiler/FP.md` §FP-SH-21.

**Known limitation (tracked follow-up).** Because each use is evaluated against
the *pure origin* independently — the sound, branch-safe choice — a value that a
*dominating* earlier use has already committed to a different intrep is not
re-flagged: straight-line `set a 1; lindex $a 0; expr {$a + 1}` genuinely
shimmers list→int at the `expr`, but is currently silent (a false negative, not
a false positive). Recovering it needs a shared committed-intrep forward
dataflow across the use-site / expr / incr detectors keyed on the dominator
tree — fire when a use's intrep differs from that committed by a *dominating*
use of the same value. The branch case where two paths commit different types
(`if {…} {expr {$a+1}} else {foreach x $a}`) correctly stays silent: neither use
dominates the other, and at runtime each path is single-type.

## Reference validation status

C source analysis completed against Tcl 9.0.3. The `arg_types` shimmer hints
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

## Fixture scenarios

The Python-era `tests/fixtures/shimmer/` corpus was retired along with the
Python implementation. Coverage today lives in:

- Unit tests co-located with each shimmer module (`rust/tcl-compiler/src/shimmer/*.rs`) and in `rust/tcl-compiler/tests/checks.rs`.
- TP/FP/TN/FN regression fixtures in `rust/tcl-compiler/src/analyser/diagnostics/fp/sh.rs` (the `FP-SH-NN` series).
- Native `lsp_e2e` coverage in `rust/tcl-lsp-server/tests/e2e/diagnostics.rs` and `rust/tcl-lsp-server/tests/e2e/code_actions.rs` (the noqa suppress quick fix).
- VS Code integration coverage in `editors/vscode/src/test/shimmerPrecision.test.ts` against `editors/vscode/testFixture/shimmerPrecision.tcl`.

## Cross-links

- Implementation: `rust/tcl-compiler/src/shimmer/` (`hints.rs`, `use_site.rs`, `thunking.rs`, `byte_array.rs`, `phi.rs`, `graph.rs`, `span.rs`).
- Registry data: `rust/tcl-registry/src/commands/**` (`arg_types` on each `CommandSpec`/`SubCommand`).
- Suppression: `rust/tcl-compiler/src/analyser/utils.rs` (`parse_noqa_line_suppressions`, `apply_preceding_noqa`), consumed by `lift_compiler_diagnostics` in `rust/tcl-lsp-server/src/lib.rs`.
