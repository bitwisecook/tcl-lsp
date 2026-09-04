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

That exclusion only bites where the expectation really is `Int`, so the
`expr` operators are split accordingly. `+`, `-`, `*`, `/` and `**` expect
`Numeric`; `%`, `<<`, `>>`, `&`, `|` and `^` are **integer-only** and expect
`Int` — `Tcl_GetNumberFromObj` reads the operand either way, but
`tclExecute.c` then rejects a `TCL_NUMBER_DOUBLE` outright for the second
group (`can't use floating-point value as operand of "%"`, verified for all
six on tclsh 8.6.16 and 9.0.4). Classifying all eleven as `Numeric` would
have let a committed double pass silently through the integer-only half.

### A compatible read preserves the representation

`CommitState::commit` records what a typed read leaves behind, not what it
asked for. A read the value's current intrep already satisfies installs
nothing, so the committed representation survives it: after
`set d [expr {sqrt($x)}]`, `expr {$d && 1}` leaves `d` a double (tclsh
8.6.16, `::tcl::unsupported::representation`), and a later `incr d` still
raises `expected integer but got "2.5"`. Overwriting the state with the
*expectation* would record `Boolean` there, which `must_pay(Int)` finds
integer-compatible, and the genuine `Double` → `Int` mismatch would go
unreported. An *incompatible* read re-represents as it always did.

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

## Nested command substitutions

A `[cmd …]` written as a statement is an IR statement the detectors walk. The
same `[cmd …]` written inside another command's word is not — `Statement::Call`
keeps its arguments as flat text — so before this was closed the detectors saw
`lindex $x 0` but not `puts [lindex $x 0]`, and the same expression reported
differently depending only on its position (issue #1814).

That cost twice over. The conversion at the nested site went unreported, and,
worse, it never reached the commit state, so *every later read of the same
variable* was judged against a stale representation: `set x [llength $l]` then
`puts [lindex $x 0]` then `incr x` reported nothing at all, while the identical
code with `lindex $x 0` on its own line reported both halves.

`word_subst` lifts those substitutions, and `commit`, `use_site` and `expr`
each consume them. Four properties matter:

- **It reads `word_exprs`, never the argument text.** Tcl substitutes `[…]` in
  bare and `"…"`-quoted words but not in braced ones, and
  `Statement::Call::args` renders all three identically as `[lindex $x 0]` —
  lifting from that text would report a command Tcl never runs. The segmenter's
  `WordExpr` already models the braced word as `BracedLiteral` and the
  substituted one as `CommandSubstitution`, so the distinction is free and
  exact.
- **That holds at every depth.** A substitution's own words are recovered by
  the same segmenter (`word_subst::nested_command_words`, the one owner the
  native and WASM lowerings also plan from), so `[list "[lindex $x 0]"]` and
  `[list a[lindex $x 0]b]` run their `lindex` and `[list {[lindex $x 0]}]` does
  not. A `[`-prefix test over the argument text missed the first two and the
  structure decides all three.
- **Order is innermost-first**, Tcl's own evaluation order, so the commit state
  moves exactly as the runtime converts.
- **`return [expr …]` needs no lifting at all**: the lowerer already parses it
  onto `Terminator::Return::expr`, and leaves it `None` for the braced
  `return {[expr …]}`. The walker just reads it, as it already did for
  `Terminator::Branch`.

### A braced argument word substitutes nothing

The same distinction applies to a statement's **own** words, and for the same
reason: `Statement::Call::args` holds the *de-braced* text, so `lindex {$x} 0`
and `lindex $x 0` both arrive as the argument `$x`. Only the second is a read
— tclsh 9.0.4 answers `$x` (two characters) for the braced form and `5` for
the bare and `"…"`-quoted ones — so `commit` and `use_site` gate the argument
loop of their `Statement::Call` arm on `hints::inert_braced_args`, which pairs
the segmenter's `CommandTokens::arg_is_braced_literal` with the registry's
`CommandRegistry::arg_indices_evaluated_in_frame` (issue #1845).

That second half makes the rule role-aware rather than "skip every braced
argument": `expr` and `if` re-evaluate their braced word where the caller's
variables are in scope, so `expr {$x} + 1` really is `6` for `x` = 5 and its
operand stays a read, while `apply {{} {puts $x}}` runs in a fresh frame and
does not. The authority is `ArgRole::braced_word_evaluated_in_frame`, the same
one `ssa::braced_word_class` consults for the identical question — the
detectors ask it, they do not restate it.

Both passes need the gate because they have separate jobs: `commit` only moves
the committed-intrep state and emits nothing, so gating solely the emitting
pass still recorded a conversion at the braced word and made the *next*,
genuine read report a shimmer against an intrep the runtime never installed.

The substitution paths need no gate: a `[cmd …]` lifted out of a word and the
command substitution inside a `Statement::AssignValue` both carry their
argument words as raw source text with the braces still on, which
`is_pure_var_ref` already declines.

### Known limitation — the underlying representational gap

The lift above is a *shimmer-side* repair of a defect that is not
shimmer-specific. A nested substitution is not a command in the IR, so its
words never receive the registry-role classification that
`ssa::braced_word_class` applies to a statement's words. That classification is
what makes an `ArgRole::Expr` word's variables `UseClass::Substituted` — "a
genuine read, here and now" — and without it the operands of a nested
`[expr {…}]` are absent from `SsaStatement::uses` entirely.

`uses` feeds SCCP, taint, type inference, def-use, GVN and interval bounds, so
the hole is not confined to shimmer. It is observable in taint, where the only
difference between the two lines is a pair of braces:

```tcl
eval [expr $tainted]      ;# T100
eval [expr {$tainted}]    ;# nothing — the read is never recorded
expr {$tainted}           ;# T100 again, once it is a statement
```

Nothing is mis-compiled — dead-store and unused-variable elimination use a
separate textual use analysis — but the analysis stack is reading a def-use
graph with holes. Because the operands never reach `uses`, `find_expr_shimmers`
resolves a nested expression's variables against the versions live at the
statement rather than through that map; that reconstruction is a workaround for
the gap, not a design.

The correct fix is to lower a nested substitution as a command, so its words
receive the same role-driven classification every statement's words already
get, and every consumer of `uses` becomes correct with no per-consumer code.
That change reaches lowering, SSA, codegen and the optimiser, so it is tracked
separately.

Two narrower residual gaps remain in the interp-alias handling (see the
doc comment on `shimmer::use_site::check_invocation`): an alias that
prepends fixed arguments (`interp alias {} foo {} ::bar prefix`) does not
index-shift the checked argument, and a read-modify-write shimmering
argument that is a bare variable name rather than a `$`-prefixed read
(e.g. `interp alias {} myincr {} ::incr; myincr x`) is not seen through an
alias, since `incr`'s own canonical name bypasses this path via the
dedicated `Statement::Incr` node.

## Coverage

- Nested-substitution coverage: `word_subst` (lifting, the braced-word
  distinction at every depth, and `arg_words` alignment), `shimmer::expr` (`expr_shimmer_fires_in_a_return_expression`,
  `expr_shimmer_fires_in_a_nested_call_argument` and their braced twins),
  `shimmer::use_site` (`use_site_shimmer_fires_in_a_nested_call_argument`,
  `a_nested_conversion_is_visible_to_later_reads`).
- Braced-argument coverage: `shimmer::use_site`
  (`a_braced_argument_reads_only_where_the_callee_evaluates_it_in_frame` pins
  the braced / quoted / bare triple for both a non-evaluating and an
  evaluating command, `a_braced_argument_commits_no_intrep_for_later_reads`
  the commit half) and the `FP-SH-24` fixtures.
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
