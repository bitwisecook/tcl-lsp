# Test scrub: values

Session `values`, 17 November 2026. Ledger: [`ledger/values.tsv`](ledger/values.tsv).

Families: `tcl-compiler` `it_2/value_transfer_witnesses`, `cfg_builder::global_write_info`,
`shimmer`, `type_infer`, `value_transfer`, `it_2/var_escape_cfg`, `it_5/var_escape_cfg_catch`, and
the 28 compiler-only tests moved from `tcl-codegen` (`it_2/compiler_analysis_residual`).

Measured on `work` at `afa3529f`, one run of each family (`cargo nextest`, `--no-fail-fast`): 218
failing of the 253 the morning's lists marked. The 35 fixed since are not in the ledger. No test was
edited.

| Family | Failing |
| --- | ---: |
| `it_2/value_transfer_witnesses` | 108 |
| `shimmer` | 31 |
| `tcl-codegen` `compiler_analysis_residual` | 19 |
| `cfg_builder::global_write_info` | 18 |
| `value_transfer` | 14 |
| `it_2/var_escape_cfg` | 13 |
| `type_infer` | 8 |
| `it_5/var_escape_cfg_catch` | 7 |

## Verdicts

| Verdict | Tests |
| --- | ---: |
| KEEP-ORACLE | 127 |
| KEEP-CONTRACT | 31 |
| KEEP-F5 | 30 |
| FIX-FIXTURE | 16 |
| FIX-EXPECTATION | 8 |
| REWRITE | 6 |
| RETIRE | 1 |
| UNDECIDED | 0 |
| MERGE | 0 |

Behaviour rows checked on tclsh here say which releases (8.4.20 to 9.1.0 where checked). A class C
row that is kept still needs its citation written into the test; the evidence column has it.

## Causes, ranked by tests blocked

| Tests | Cause | Owner | What fails |
| ---: | --- | --- | --- |
| 37 | `binding-substitution-word-no-layout` | analyser-binding | A procedure-body statement whose words hold a substitution gets no declared layout: `set d [expr {sqrt($x)}]` stays `Call set` with no definition of `d`, `set a($k) 9` raises every dynamic-name flag |
| 21 | `f5-rule-procedure-frame` | taint (queue item 1) | Under f5-irules a rule procedure called at the top level gets no SSA definitions, or names its variables by a frame identity (`@frame:47:::p@0/…:r`) |
| 12 | `unknown-head-untrust` | dataflow, then values | Ruling of 17 November: untrust after an unknown head is flow-sensitive and fixtures say bounded or unbounded. Eleven `handler_widening` fixtures must be restated (FIX-FIXTURE) |
| 11 | `binding-caller-frame-opaque-builtin` | analyser-binding | A builtin whose binding is not proved at its site (a top-level `switch`, `puts` in an f5 procedure) is followed by a `<caller-frame-opaque>` barrier, "switch upvar-aliases a dynamic caller variable"; no selection, branch or merge facts |
| 9 | `loop-enumeration` | analyser-binding | A `foreach` over a literal list is not lowered structurally, so nothing is enumerated after the loop |
| 9 | `frame-script-body-unread` | analyser-binding | A script run at `#0` or `1` (`uplevel`, an alias to it, `eval` inside it) has no invocation metadata, so the global summary goes opaque |
| 9 | `f5-handler-command-unbound` | analyser-binding | Inside a `when` body `set`, `TCP::payload` resolve as `UnprovedBinding` |
| 8 | `summary-link-local-def` | dataflow issue session | A write through `upvar 1 $name v` defines nothing at the callee's exit (summary SSA step 4 in progress) |
| 9 | `binding-catch-result-variable-no-place` | analyser-binding | `catch {…} m ?opts?`: the result variable gets no place, and the commands from the catch end on are unproved, so the computed-name hazard widens the whole procedure (root found by analyser-binding: the walk is widened after a raising catch body). Four rows moved here from `catch-body-transfer` and two from `try-body-transfer` on 18 November |
| 5 | `ssa-var-symbol-lookup-refused` | dataflow | The procedure lists the variable (`vars [x, m, o]`, or two symbols named `g`) yet `var_symbol` returns None |
| 5 | `f5-top-level-not-executable` | values | f5-irules rows of top-level fixtures expect Tcl folding; iRules refuses executable top-level code (IRULE5007) |
| 4 | `escape-top-level-fixture` | values | Escape fixtures run at `#0`, where `global` links nothing and a variable is Local by ruling (37cf6fd2); they need a procedure |
| 4 | `binding-alias-invocation-metadata` | analyser-binding | An aliased invocation keeps no original Logical invocation |
| 4 | `ssa-link-statement-version` | dataflow issue session | Which SSA version a link statement defines (REWRITE three to the observable, RETIRE one) |
| 3 each | `edge-refinement`, `escape-catch-body-walk`, `rebase-memoised-receipts` | dataflow, values, analysis-performance | |
| 3 | `binding-procedure-absent-after-statement` | analyser-binding | A procedure defined in the file is absent after a top-level statement |
| 2 | `lifted-call-body-not-original` | dataflow-optimiser | No original body for the script of a `catch` lifted from a substitution word (`set c [catch {return 5} m]`), so the catch declines unsupported; the code table and the nested catch body |
| 2 each | `binding-catch-body-children-unavailable`, `binding-conditional-alias-join`, `binding-renamed-builtin-head`, `class-facts-object-element`, `escape-param-upvar-source-precision`, `escape-entry-point-ignores-registry`, `existence-chain-fold`, `expr-decline-reason`, `fixture-body-absent`, `ordered-nested-write`, `orig-diagnostic-refusal`, `wrong-rewrite-callback-write` | | |
| 1 each | 35 more, listed in the ledger, among them the five former `declared-route-evaluation` rows (three analyser-binding, two dataflow-optimiser; the route answers in each), `optimiser-handler-forwarding` and `dead-store-unreachable-use` (dataflow-optimiser, from `try-body-transfer`), `o101-non-finite-render` (dataflow-optimiser: O101 renders no non-finite double), `wrong-rewrite-try-error-path`, `wrong-rewrite-shift-beyond-width`, `release-tower-fold` and `panics-before-assertion` | | |

By owner of the cause: analyser-binding 100, values 51, dataflow-optimiser 27 and its issue session 12, taint
21, analysis-performance 4, class-facts 3 (the 12 unknown-head rows count under dataflow; their
fixtures are restated here afterwards). What values can move now: the escape walker, 5 f5 expectations and
the fixtures.

## Distinct failing behaviours

What is worth fixing, one line each; the wrong rewrites first.

1. **Wrong rewrite:** `try {set x [expr {1 / $d}]; error boom} on error {} {}` then
   `info exists x`: the rewrite drops the `yes` path (original prints no, yes; rewrite no, no).
2. **Wrong rewrite, fixed in d8402cd92f:** a write made by an `after` callback (`tick`, or `{*}[list set done 1]`) is
   folded away; the original prints 1, the rewrite 0 (8.4.20, 8.6.18, 9.0.4).
3. **Wrong rewrite, fixed in 88e66f2577:** `expr {1 << 70}` is folded under 8.4, where a shift count beyond the word
   width is undefined in C and differs by build (ruling of 17 November: O101 declines there).
4. **Wrong diagnostic (new witness, 18 November):** in a procedure body S100 reports an
   int-to-list conversion for `set n [string length abc]; lindex $n 0`; the bytecode compiler pushes
   a literal there from 8.4 to 9.0, a pure string (`shimmer-compiled-literal-representation`, values).
5. A declined declared route is still folded by O129 (outputs agree on the releases present; the
   route did not prove it).
6. Procedure-body statements with substituted words define nothing (types, values, shimmer, escape).
7. A rule procedure called at the iRules top level has no definitions.
8. Commands in an iRules `when` body are unbound (S110 payload rows, handler types).
9. `foreach` over a literal list is not enumerated.
10. `switch` and `case` select nothing.
11. Scripts run at another frame (`uplevel`, alias to `uplevel`/`eval`) are unread.
12. Writes through `upvar 1 $name` do not reach the callee's summary.
13. The `catch` result and options variables have no place; catch and try bodies transfer no state.
14. Flow-sensitive untrust after an unknown head (dataflow), then the restated fixtures.
15. Edge refinements in `if` arms and merges.
16. Escape walker: the `eval` join, a `try` handler, a `switch` arm over-tagging, literal tracking of
    `set $target`.
17. Global summaries: conditional alias joins, a renamed `set` behind an alias, an alias that
    creates its namespace, a dynamic local linked to the global named "".
18. Smaller single rows: increment amount words, release-dependent `try` handler numerals, stub
    frame effects, chained dead-list writes (O130), the `split ""` fold.

## Rulings of 17 November on the ten undecided rows

- `o101_rewrites_only_what_the_route_proves`: KEEP-ORACLE, the 8.4 shift fold is a wrong answer
  (`wrong-rewrite-shift-beyond-width`).
- `escape-param-upvar-source-precision`: with the interprocedural step on the precise caller answer is
  sound (FIX-EXPECTATION); with it off the unbounded answer stands (KEEP-CONTRACT).
- `escape-entry-point-ignores-registry` (2): KEEP-CONTRACT. The premise holds on this tree: f5-irules
  `puts` is not frameless, tcl8.6 `puts` is.
- The two `global_write_info` internal-carrier rows and `memo-trust-key`: REWRITE to the observable the
  ruling names.
- `actual_traced_alias_store_preserves_only_a_closed_unchanged_value`: the count is the two writes of
  `dictionary` and the test already asserts the type answer (tclsh 8.6.18, 9.0.4: a dict unless the
  callback writes "altered"); KEEP-ORACLE under `binding-substitution-word-no-layout`.
- `native_jim_alias_metadata_cannot_use_raw_logical_name_eligibility`: KEEP-CONTRACT,
  `panics-before-assertion`.

## Exact and overlap groups touching these families

Exact copies (4 groups, 8 tests, all passing):

| Group | Members | Proposal |
| --- | --- | --- |
| `9ef7578fb736` | `compiler_analysis_residual::auto_path::auto_path_nested_and_double_dirname`, `auto_path_eval::tests::nested_dirname` | MERGE into the library test |
| `a9820cf15b32` | `…::auto_path_unsupported_command_declines`, `auto_path_eval::tests::unsupported_command_is_none` | MERGE into the library test |
| `b969f09f5902` | `…::auto_path_variable_subst_declines`, `auto_path_eval::tests::variable_substitution_is_unsupported` | MERGE into the library test |
| `b43aa9d0279c` | `sccp::original_value_facts::tests::assignment_folds::split_head_basic`, `value_transfer::tests::split_head_basic` | MERGE; the owner of `value_transfer` keeps one |

Overlap (9 groups, 18 tests): the three `auto_path` pairs above again, plus
`auto_path_normpath_collapses_dotdot_and_dot` with `parent_dir_join_idiom_collapses_dotdot`, and
`places::place_whole_array_read_overlaps_every_element` with
`place_bridge::tests::var_read_role_arg_resolves_whole_array` (cross-crate copies from the codegen
move: candidates to MERGE into the library tests once each survivor asserts as much). Four groups
inside `shimmer` (`expr_shimmer_string_in_arithmetic` and `…span_narrows_to_operand_assign_expr`;
the integer-only and true-arithmetic double pair; two `hints` subcommand rows; two `thunking` rows)
assert different things on the same source and stay. The double pair is the one group with a
failing member.
