# FP catalogue → Rust port: status & bug worklist

Tracks the port of the analyser false-positive precision catalogue
(`docs/design/compiler/FP.md` / `tests/test_fp_*.py`) into the Rust analyser,
per [`fp-rust-port-plan.md`](fp-rust-port-plan.md). Living document.

## Done

The harness and **all 11 families** are ported and committed under
`rust/tcl-compiler/src/analyser/diagnostics/fp/` (`mod fp`). **343 FP tests
pass.** Each test asserts against the live Rust pipeline via the `codes()/fires()`
helper (analyser + `run_all_checks`, and — for O-codes — `optimise_with_dialect`),
mirroring the user-facing `tcl diag` / `get_diagnostics` surface.

| Family | tests | divergences | notes |
|---|---:|---:|---|
| RBS (read-before-set) | 77 | 0 | already had the most Rust coverage |
| STY (style) | 60 | 0 | 1 bridge-only skip (Python lexer-API test) |
| OBJ (object dispatch) | 61 | 7 | snit/TclOO W307/W308 heuristics |
| BND (bounds/intervals) | 21 | 0 | |
| DS (dead-store) | 21 | 1 | FP-DS-04 cross-scope **fixed** |
| SH (shimmer) | 20 | 0 | |
| TNT (taint) | 18 | 0 | |
| NAB (confirm-correct) | 17 | 0 | FP-NAB-03/12 are internal-API, see below |
| INJ (injection) | 12 | 0 | |
| RCH (reachability) | 8 | 0 | drives the optimiser for O107 |
| OPT (optimiser) | 34 | 3 | 2 bridge-only IR-structure skips |

**Fixed bugs the port surfaced**

- **FP-DS-04 (cross-scope traced `::`-global).** A `::w` with a write trace in
  one proc but a `set ::w 1` in another emitted a false `W211`. Added
  `scan_module_traced_globals(cu)` and folded module-wide traced `::`-globals
  into every function's `cross_event_vars` suppression. (commit `b71f8859`)
- **FP-OBJ-09 (SCCP non-command overrides W307 multi-dispatch).** When SCCP
  proves every feasible value of a dispatched local is a concrete non-command
  literal, that evidence now overrides the heuristic object-dispatch
  suppressions (in-method, proc-param / multi-dispatch) so the real
  invalid-command-name hazard fires W307. (commit `6102bec8`)
- **FP-OBJ-05 (snit-instance W308).** A snit-typed receiver no longer fires
  W308 — snit method resolution routes through delegation / hull / options /
  built-ins, which the analyser does not model, so validation is unsound. Skip
  W308 when the receiver class's `metaclass` is a snit type. (commit `6001367f`)
- **FP-OBJ-07 (`[cmd]::method` ensemble).** A command word `[cmd]::method`
  composes a command-sub head with a literal `::method` tail (static method
  evidence), so it no longer fires W307; a bare `[cmd] arg` with no tail still
  fires. (commit `a7474fb2`)

- **FP-DS-02 (read inside `[expr {…}]`).** `incr i [expr {$w}]` reads `w`, but
  `$w` sits inside the expr's `{…}` braces which suppress `$`-substitution to
  the brace-aware scanner. `collect_rmw_hidden_reads` now collects every `$var`
  inside a `[…]` command substitution (`dollar_reads_in_cmd_subs`) and scans
  `Incr.amount`; over-approximating reads only ever silences a warning. (commit
  `6608a044`)

**5 of 12 fixed.** Remaining worklist below.

Astral-output C1 (lsp_e2e finding) — partly localised: `document_highlights`
and `references` already build ranges via the correct `span_to_range` /
`position_at_utf16`, so the scalar-column error is **upstream** — the analyser
stores `var_def.references` / proc `name_span` byte-spans that are themselves
miscounted by an astral char's extra UTF-16 unit. The fix is in the analyser's
span computation for references following an astral character, not in the LSP
range-lift; identifying that site is the remaining work.

The OBJ-10 fix is 80% done (a working `is_callback_array_slot` suppression gated
on `sccp_not_command`), but its paired SCCP-const TP variants
(`set state(doneCallback) notacommand; $state(...) a` must *fire*) need the
slot's concrete value to reach `sccp_not_command`. The direct `set arr(key) v`
form is *not* an `AssignConst` (the scalar `set_literal_body` lowering excludes
`(`-bearing names) nor a plain `Call "set"` — its IR shape needs identifying
before a harvester can capture it. Reverted pending that, to avoid
over-suppressing the TP variants.

## Remaining bug worklist (11)

Each is a **genuine Rust analyser/optimiser divergence** the port proved against
the Python verdict, currently a transient `#[ignore]` in the relevant `fp/*.rs`
(to keep the suite green) with the exact divergence in the ignore string. These
are **not** the acceptable-xfail category — each must be fixed so the test passes
and the `#[ignore]` removed. Ordered roughly by tractability.

### Dead-store

1. **FP-DS-02** — false `W220` on `set w 5` in `incr i [expr {$w}]`. The `$w`
   read sits inside the `expr {..}` braces; the substitution-hidden-reads scan
   (`collect_rmw_hidden_reads` / `substitution_hidden_reads_of`) does not treat
   an `expr` command-sub's braced argument as an expression, so the read is
   modelled as neither an SSA use nor a hidden read. Fix: give the cmd-sub read
   scan `expr`-brace awareness (parse the braced arg as an expression), or lower
   `[expr {…}]` arguments so the inner reads become SSA uses.

### Object dispatch (W307/W308) — analyser var-command heuristics

Over-fires (Rust emits where Python suppresses — add a suppression):

2. **FP-OBJ-05** — `W308` on a snit instance method (`$o delegated_or_builtin`).
   snit instances route through delegation/hull/options/built-ins, so method
   validation is unsound; suppress W308 for snit-instance receivers.
3. **FP-OBJ-07** — `W307` on `[cmd]::method` namespaced-ensemble dispatch. The
   literal `::method` tail is static method evidence; suppress.
4. **FP-OBJ-10** (×2) — `W307` on dash-prefixed (`$state(-command)`) and
   callback-suffix (`$state(doneCallback)`) array-element dispatch (callback
   registration slots). A `is_callback_array_slot` suppression (gated on the
   FP-OBJ-09 `sccp_not_command` check) fixes the no-evidence FP cases, but the
   paired TP variants (`set state(doneCallback) notacommand; $state(...) a` must
   *fire*) need a direct `set arr(key) literal` const harvester so the slot's
   concrete value reaches `sccp_not_command` — the current constset only
   captures `array set` element literals, not direct element assignment. Land
   both together.

Under-fires (Rust suppresses where Python emits — tighten the heuristic):

5. **FP-OBJ-09** — **FIXED** (commit `6102bec8`): SCCP-const non-command evidence
   now overrides the multi-dispatch suppression.
6. **FP-OBJ-D4-F5** — a local literal dispatched inside an `oo::class` method
   body (`set cmd nope; $cmd arg`) must fire `W307` (the `in_method` blanket
   suppression should not apply without positive object-evidence).
7. **FP-OBJ-VAR-as-cmd** — interproc param-constant seeding should propagate
   non-command evidence so a tainted-as-non-command param fires `W307`.

### Optimiser

8. **FP-OPT-03** — Rust LICM does not hoist an outer-pure/inner-pure nested
   `[format … [expr {…}]]` shape (`O106` not emitted).
9. **FP-OPT-06** — `O100` copy-propagation propagates a stale value past a
   command-sub write (the cmd-sub write is not wired into the SSA kill-sites).
10. **FP-OPT-08** — elimination deletes `set b 0` even though `$b` survives in
    the EXPR role (the EXPR/BODY descent in the overlap filter is not ported).

### Not-a-bug internal-API coverage (Rust-structure tests, not diagnostics)

11. **FP-NAB-03 / FP-NAB-12** — the Python tests assert on the interproc `pure`
    summary and the `is_pure_var_ref` value-shape parser directly (not via a
    diagnostic). Add equivalent Rust unit tests against the corresponding Rust
    APIs (interproc purity; the value-shape / pure-var-ref parser).

## Also deferred to the Rust-structure / LSP-layer track

- **FP-INJ-05 code-action**: the W101→`eval [list …]` quick-fix rewrite is an
  LSP code-action contract; add a test at the `tcl-lsp-core` code-action layer
  (the diagnostic side is covered in `fp/inj.rs`).
- **FP-STY-15 / FP-OPT-12 bridge-only skips**: Python tests that drive
  `TclLexer.tokenise_all()` / `lower_to_ir(...).procedures` IR structure with no
  public Rust surface; behaviourally covered by sibling diagnostic tests.

## Found via new lsp_e2e UTF-16 coverage

`tests/lsp_e2e/test_unicode_positions_e2e.py` adds exact UTF-16 column
correctness tests (the C1 byte-vs-UTF-16 class) — input mapping and output
columns across providers, with BMP-multibyte and astral (emoji) cases. 4 pass.

**Bug found — astral-character OUTPUT columns (cross-provider C1 residual).**
Provider *output* columns (`document_highlight`, go-to-definition target
ranges) are computed as a Unicode **scalar** (code-point) count rather than
UTF-16 code units, so an astral char (🚀 = 2 UTF-16 units, 1 code point) yields
a column **one short per astral char** (returns 13/15 where UTF-16 is 14/16;
byte would be 16/18). BMP multibyte output is correct (scalar==UTF-16 there),
and astral **input** mapping is correct (`offset_at_utf16` handles it). The fix
is to route the residual scalar column sites (`chars().count()` in
`lsp-core/{definition,completion,code_actions}.rs` and the highlight/symbol
output paths) through the UTF-16 conversion (`utf16_len` / `position_at_utf16`),
matching `span_to_range`. The focused astral-output assertion is withheld in
the test (with a NOTE) until that lands, to keep the suite green.

## Session progress — 7 of 12 fixed

Fixed (each full-suite regression-verified, committed):
1. FP-DS-04 — cross-scope traced `::`-globals (`scan_module_traced_globals`).
2. FP-DS-02 — reads inside `[expr {…}]` cmd-subs (`dollar_reads_in_cmd_subs`).
3. FP-OBJ-05 — snit-instance W308 suppression (metaclass check).
4. FP-OBJ-07 — `[cmd]::method` ensemble W307 suppression.
5. FP-OBJ-09 — SCCP non-command evidence overrides W307 multi-dispatch.
6. FP-OPT-06 — builtin var-writes inside cmd-subs modelled as SSA defs
   (`builtin_write_defs_from_text`), killing stale O100 copy-propagation.
7. FP-OBJ-VAR-as-cmd — cascade-fixed by (6): interproc param-constant
   non-command evidence now reaches the W307 site.

Remaining (5 `#[ignore]`s):
- **FP-OBJ-10** — callback-slot suppression is written, gated on the OBJ-09
  evidence check; blocked on the IR shape of a direct `set arr(key) literal`
  (excluded from `set_literal_body`, not a plain `Call "set"`) so the SCCP-const
  TP variants fire. Two prior attempts reverted to avoid breaking the TPs.
- **FP-OBJ-D4-F5** — const value not captured in `oo::class` method-body scope.
- **FP-OPT-03** — LICM does not hoist the outer-pure/inner-pure nested shape.
- **FP-OPT-08** — overlap arbitration deletes `set b 0` while keeping a `$b`
  reference (the EXPR-role consumption across the nested-if fold is not counted
  in `consumed_var_count` / `select_non_overlapping`).
- **astral-output C1** — analyser stores `var_def.references` / `name_span`
  byte-spans miscounted by an astral char's extra UTF-16 unit (the LSP
  range-lift is already correct).

## Update — 8 of 12 fixed

Additional fixes since the 7-of-12 mark:
8. **FP-OBJ-10** — callback-array-slot W307 suppression (`is_callback_array_slot`)
   gated on the OBJ-09 evidence check, plus `harvest_array_element_set_constants`
   capturing the slot's literal value from the `AssignValue { name: "arr(key)" }`
   shape that a direct `set arr(key) literal` actually lowers to (confirmed by IR
   dump). All FP-OBJ-10 + FP-OBJ-17 cases pass. (commit `6ea321c6`)

Remaining 3 `#[ignore]`s + astral, each with a now-confirmed structural root:
- **FP-OBJ-D4-F5** — `oo::class create C { method m {…} }` lowers the *entire
  class body as an opaque `Barrier`* ("unsupported body command"); the method
  body is never lowered to IR/CFG/SSA, so `set cmd nope` is invisible and the
  in-method W307 gate cannot see the non-command evidence. Needs oo::class
  method-body lowering (structural), not a heuristic.
- **FP-OPT-03** — LICM hoist of an outer-pure/inner-pure nested shape (a pass
  capability the Rust LICM does not yet have).
- **FP-OPT-08** — overlap arbitration (`select_non_overlapping` /
  `consumed_var_count`) deletes `set b 0` while keeping a `$b` reference; it
  must count EXPR-role consumption across the nested-if fold.
- **astral-output C1** — analyser var-reference / `name_span` byte-spans
  miscounted by an astral char's extra UTF-16 unit (LSP range-lift is correct).

## Final — all 12 resolved (0 non-eglot `#[ignore]` remaining)

The last four items are now closed:

9. **FP-OPT-08** — `drop_def_elims_resurrected_by_replacements` (optimiser
   `manager.rs`): after `couple_propagated_const_dead_stores`, drop any
   def-elimination (empty replacement) whose target var still appears as
   `$var`/`${var}` in another surviving optimisation's replacement text. So
   `set b 0` is no longer deleted when an O112 `if {$a}` unwrap keeps `$b` live.

10. **FP-OPT-03** — the `oo::class` "opaque Barrier" diagnosis was wrong for
    LICM; the real defect was in `gvn::find_loop_invariants`, which skipped the
    loop **header** block entirely. In a bottom-test `for`/`while` loop the
    header carries the *body* statements while the guard lives in the latch's
    `Branch` terminator (never a statement), so scanning header statements is
    safe and necessary. Removed the header skip; `set s [format %04d [expr
    {$k + 1}]]` now surfaces O106. (O106/O107 are diagnostics from
    `run_all_checks`, not optimiser-manager rewrites — the paired tests were
    rewired to probe that surface via a new `check_fires` helper.)

11. **FP-OBJ-D4-F5** — `oo::class` method bodies *are* lowered to full
    `FunctionUnit`s in `cu.methods` (the earlier "opaque Barrier" note was stale
    for the analysis path). The W307 `in_method` gate yields to SCCP
    non-command evidence, but `all_constsets` / `all_object_types` were gathered
    only from `cu.top_level` + `cu.procedures`. Extending both collection loops
    to include `cu.methods.values()` makes `set cmd nope; $cmd arg` inside a
    method body fire W307, while undetermined instance-var / snit-typevariable
    dispatch stays silent (no const → in-method suppression still applies).

12. **astral-output C1** — **not a bug.** Re-verified end-to-end against the
    native `tcl-lsp-server`: `document_highlight` returns the exact UTF-16
    column (16) for `$z` after a 🚀 prefix, not the scalar 15. The providers
    already lift every range through `span_to_range` → `position_at_utf16`
    (`encode_utf16().count()`), which is correct for supplementary-plane chars.
    The withheld assertion is now enabled as
    `test_highlight_columns_are_utf16_through_astral_prefix` and passes; the
    earlier NOTE was stale.

**Result: 11 genuine defects fixed + 1 non-reproducing, 0 non-eglot `#[ignore]`
remaining in the FP suite.** Full `tcl-compiler` lib suite: 3270 passed, 0
failed, 9 ignored (the 9 remaining are the pre-existing static-uplevel
"pending VM frame-shift opcodes" lowering tests, unrelated to the FP port).
