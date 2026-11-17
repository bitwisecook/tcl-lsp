# Test scrub: dataflow-optimiser

Family: `rust/tcl-compiler` lowering, CFG, SSA, SCCP, `tcl_expr_eval`, the
optimiser, interprocedural, `compilation_unit`, GVN and the inliners, plus their
integration tests (`tcl-compiler` it_3 and it_5, `tcl-codegen` it_3
`optimiser_codegen_residual`).

The ledger is [`ledger/dataflow-optimiser.tsv`](ledger/dataflow-optimiser.tsv):
one row for each failing test in the 17 November baseline (423 tests). The
coordinator's 17 November rulings are applied. No test was edited or deleted
during triage.

## Counts by verdict

| Verdict | Tests |
| --- | ---: |
| KEEP-ORACLE | 82 |
| KEEP-F5 | 1 |
| KEEP-CONTRACT | 103 |
| FIX-EXPECTATION | 30 |
| FIX-FIXTURE | 142 |
| MERGE | 10 |
| REWRITE | 50 |
| RETIRE | 5 |
| UNDECIDED | 0 |
| **Total** | **423** |

## Wrong answers first

These causes make the product rewrite or prove something that a C Tcl release
contradicts. They come before every other fix.

| Cause | Tests | What goes wrong | tclsh evidence |
| --- | ---: | --- | --- |
| `pure-summary-misses-global-write` | 1 | `proc g {} {bump ::n; return 0}` is summarised as pure. A purity consumer (call removal, exit-value reuse) would drop the global write. | 8.6, 9.0, 9.1 set `::n` to 1 |
| `alias-prefix-call-unresolved` | 4 | A call through `interp alias {} via {} far x` is not seen as a call to `far`, so its `upvar` write does not invalidate the caller's constant and `return $x` can fold to 1. The other three rows only decline. | prints 2 |
| `call-specialised-body-rewrite` | 1 | A procedure body is rewritten from one call site's facts: `proc add` becomes `return 7`. | after sourcing the rewritten file, `add 1 1` returns 7 instead of 2 on 8.4–9.1 |
| `encoding-dependent-literal-fold` | 1 | `string length` of a UTF-8 source supplementary character folds to 2 under tcl8.6. The answer depends on the system encoding when the file is sourced. 8.4–8.6 must decline unless the encoding is proved. | 8.6 prints 4 under iso8859-1 and 2 under utf-8; 8.4/8.5 print 4; 9.0 prints 1 |
| `rename-not-in-mutation-scan` | 4 | `[list a b c]` / `[llength …]` still fold after the file renames `list` / `llength`. Routed to analyser-binding. | the renamed command raises |

The coordinator asked whether a statement-position `expr {2+2}` is still
rewritten to a bare value (the KCS O101 "Standalone expression limitation"
section). It is not. On 17 November the optimiser rewrote it to `expr 4` under
tcl8.4, 8.6, 9.0 and 9.1, which keeps the command wrapper. tclsh 8.4–9.1 run
`expr 4` and return 4. So there is no sixth wrong answer. The KCS section and
the open entry in `docs/design/compiler/precision-limitations.md` are stale.
Row 3 of `full_expression_folds_retain_the_native_result_protocol` is therefore
FIX-EXPECTATION, like rows 1 and 2, and not KEEP-ORACLE.

## Causes ranked by tests blocked

| Rank | Cause | Tests | Verdicts |
| ---: | --- | ---: | --- |
| 1 | `pass-context-unwired` | 48 | FIX-FIXTURE 47, MERGE 1 |
| 2 | `uncalled-body-generic-entry` | 32 | KEEP-CONTRACT 32 |
| 3 | `dialect-blind-fixture` | 19 | FIX-FIXTURE 19 |
| 4 | `no-proc-transfer-summary` | 19 | KEEP-CONTRACT 9, KEEP-ORACLE 6, REWRITE 4 |
| 5 | `unset-variable-fixture` | 16 | FIX-FIXTURE 14, MERGE 2 |
| 6 | `unknown-head-world-ends` | 14 | REWRITE 8, KEEP-CONTRACT 3, KEEP-ORACLE 2, FIX-EXPECTATION 1 |
| 7 | `direct-proc-plan-declines-authored-calls` | 13 | KEEP-CONTRACT 8, RETIRE 3, KEEP-ORACLE 1, MERGE 1 |
| 8 | `method-body-not-entered` | 13 | KEEP-ORACLE 10, FIX-FIXTURE 3 |
| 9 | `uncalled-body-withheld` | 13 | KEEP-CONTRACT 7, FIX-FIXTURE 5, RETIRE 1 |
| 10 | `lifecycle-fixture-proc-not-installed` | 12 | FIX-FIXTURE 12 |
| 11 | `unbounded-self-recursion-fixture` | 11 | FIX-FIXTURE 11 |
| 12 | `grouped-chain-edits-not-applied` | 10 | KEEP-ORACLE 9, MERGE 1 |
| 13 | `lenient-nested-substitution-inventory` | 8 | REWRITE 4, KEEP-CONTRACT 4 |
| 14 | `interpolation-fold-withheld` | 6 | KEEP-ORACLE 5, KEEP-CONTRACT 1 |
| 15 | `reads-unset-variable` | 6 | FIX-FIXTURE 6 |
| 16 | `fixture-reads-unset-variable` | 5 | FIX-FIXTURE 5 |
| 17 | `no-profile-registry-fixture` | 5 | KEEP-CONTRACT 4, REWRITE 1 |
| 18 | `o128-candidate-missing` | 5 | KEEP-ORACLE 4, KEEP-CONTRACT 1 |
| 19 | `alias-prefix-call-unresolved` | 4 | KEEP-CONTRACT 2, REWRITE 1, KEEP-ORACLE 1 |
| 20 | `braced-literal-name-roles` | 4 | KEEP-ORACLE 2, KEEP-CONTRACT 2 |
| 21 | `callback-edge-missing` | 4 | KEEP-CONTRACT 2, KEEP-ORACLE 1, KEEP-F5 1 |
| 22 | `class-definition-script-no-target` | 4 | KEEP-CONTRACT 4 |
| 23 | `fixture-without-trust-fact` | 4 | FIX-FIXTURE 3, MERGE 1 |
| 24 | `jim-entry-gained-name-policy` | 4 | FIX-FIXTURE 4 |
| 25 | `logical-chain-advice-missing` | 4 | KEEP-CONTRACT 4 |
| 26 | `raise-proof-calls-complete-withheld` | 4 | KEEP-ORACLE 3, KEEP-CONTRACT 1 |
| 27 | `rename-not-in-mutation-scan` | 4 | MERGE 2, KEEP-ORACLE 2 |
| 28 | `source-literal-operand-licence` | 4 | FIX-EXPECTATION 4 |
| 29 | `source-metadata-context-not-retained` | 4 | KEEP-CONTRACT 4 |
| 30 | `f5-handler-advisory-facts` | 3 | FIX-EXPECTATION 3 |
| 31 | `full-fold-reports-o101` | 3 | FIX-EXPECTATION 3 |
| 32 | `mathfunc-call-not-folded` | 3 | FIX-EXPECTATION 3 |
| 33 | `o112-owns-constant-compound` | 3 | FIX-EXPECTATION 3 |
| 34 | `o124-not-firing` | 3 | KEEP-CONTRACT 3 |
| 35 | `opaque-script-effect-markers` | 3 | REWRITE 3 |
| 36 | `trailing-statement-result-observed` | 3 | FIX-FIXTURE 3 |
| 37 | `upvar-procs-over-propagate` | 3 | REWRITE 3 |
| 38 | `boolean-ternary-o110-missing` | 2 | FIX-FIXTURE 1, KEEP-ORACLE 1 |
| 39 | `catch-flattened-in-unit` | 2 | FIX-FIXTURE 2 |
| 40 | `coupled-dead-store-not-removed` | 2 | KEEP-ORACLE 2 |
| 41 | `effect-call-operand-reads` | 2 | REWRITE 2 |
| 42 | `embedded-call-writes-missing` | 2 | REWRITE 2 |
| 43 | `foreach-constset-absent` | 2 | REWRITE 2 |
| 44 | `jim-expr-sugar-return` | 2 | REWRITE 1, FIX-EXPECTATION 1 |
| 45 | `native-direct-unit-omits-procedure` | 2 | KEEP-ORACLE 2 |
| 46 | `nested-namespace-eval-procs-missing` | 2 | KEEP-ORACLE 2 |
| 47 | `nested-write-value-not-forwarded` | 2 | KEEP-ORACLE 2 |
| 48 | `no-native-declaration-body-unit` | 2 | REWRITE 2 |
| 49 | `o101-fold-keeps-feeding-store` | 2 | KEEP-CONTRACT 1, FIX-EXPECTATION 1 |
| 50 | `rename-subject-resolution` | 2 | FIX-FIXTURE 1, MERGE 1 |
| 51 | `stale-metadata-input-accepted` | 2 | KEEP-CONTRACT 2 |
| 52 | `supplied-source-native-entry` | 2 | FIX-FIXTURE 2 |
| 53 | `tk-widget-object-command-rework` | 2 | REWRITE 2 |
| 54 | `walk-operands-withdrawn` | 2 | KEEP-ORACLE 2 |
| 55 | `whole-command-fold-keeps-expr` | 2 | FIX-EXPECTATION 2 |
| 56 | `assign-expr-lowering-shape` | 1 | REWRITE 1 |
| 57 | `bare-word-forward-is-o102` | 1 | FIX-EXPECTATION 1 |
| 58 | `binding-target-identity` | 1 | FIX-EXPECTATION 1 |
| 59 | `body-kind-classification` | 1 | REWRITE 1 |
| 60 | `braced-dollar-literal-blocks-o100` | 1 | KEEP-ORACLE 1 |
| 61 | `call-specialised-body-rewrite` | 1 | FIX-EXPECTATION 1 |
| 62 | `catch-now-lowered-structured` | 1 | FIX-FIXTURE 1 |
| 63 | `chain-name-rendering` | 1 | FIX-EXPECTATION 1 |
| 64 | `chain-setter-renders-qualified` | 1 | FIX-EXPECTATION 1 |
| 65 | `chain-var-piece-not-folded` | 1 | KEEP-ORACLE 1 |
| 66 | `code-absence-proxy` | 1 | REWRITE 1 |
| 67 | `completion-header-identity-unchecked` | 1 | KEEP-CONTRACT 1 |
| 68 | `completion-local-scalar-chain` | 1 | KEEP-CONTRACT 1 |
| 69 | `declaration-layout-advice-missing` | 1 | REWRITE 1 |
| 70 | `definite-writer-dse-missing` | 1 | KEEP-ORACLE 1 |
| 71 | `empty-name-var-forwarding` | 1 | KEEP-ORACLE 1 |
| 72 | `encoding-dependent-literal-fold` | 1 | KEEP-ORACLE 1 |
| 73 | `exit-finally-not-dead` | 1 | KEEP-CONTRACT 1 |
| 74 | `expr-arg-fold-ignores-availability` | 1 | KEEP-CONTRACT 1 |
| 75 | `f5-handler-store-overdefined` | 1 | FIX-EXPECTATION 1 |
| 76 | `failindex-writer-not-definite` | 1 | KEEP-ORACLE 1 |
| 77 | `for-runs-once-unproved` | 1 | REWRITE 1 |
| 78 | `foreign-body-id-accepted` | 1 | RETIRE 1 |
| 79 | `frozen-loop-cond-reads-written-place` | 1 | REWRITE 1 |
| 80 | `fused-expr-84-string-value` | 1 | KEEP-ORACLE 1 |
| 81 | `fused-expr-not-lowered` | 1 | KEEP-CONTRACT 1 |
| 82 | `grapheme-cursor-not-definite` | 1 | KEEP-ORACLE 1 |
| 83 | `hidden-read-scan-precision` | 1 | REWRITE 1 |
| 84 | `incr-constant-not-forwarded` | 1 | KEEP-ORACLE 1 |
| 85 | `incr-target-proof-missing` | 1 | REWRITE 1 |
| 86 | `info-exists-native-direct-barrier` | 1 | KEEP-ORACLE 1 |
| 87 | `interp-eval-untrusts` | 1 | KEEP-ORACLE 1 |
| 88 | `ir-shape-assertion` | 1 | REWRITE 1 |
| 89 | `jim-logical-body-absent` | 1 | KEEP-ORACLE 1 |
| 90 | `list-arity-is-exact` | 1 | FIX-EXPECTATION 1 |
| 91 | `literal-callback-sets-any-write` | 1 | KEEP-CONTRACT 1 |
| 92 | `logical-call-arity-unproved` | 1 | KEEP-ORACLE 1 |
| 93 | `mathfunc-fold-declined` | 1 | KEEP-ORACLE 1 |
| 94 | `missing-namespace` | 1 | FIX-FIXTURE 1 |
| 95 | `needs-run` | 1 | KEEP-ORACLE 1 |
| 96 | `nested-body-word-read-as-frame` | 1 | KEEP-ORACLE 1 |
| 97 | `nested-upframe-existence-resolved` | 1 | MERGE 1 |
| 98 | `o100-partial-path-dead` | 1 | REWRITE 1 |
| 99 | `o103-recursive-fold-missing` | 1 | KEEP-ORACLE 1 |
| 100 | `o107-report-count` | 1 | REWRITE 1 |
| 101 | `o108-top-level-kcs` | 1 | FIX-EXPECTATION 1 |
| 102 | `o108-transitive-chain-missing` | 1 | KEEP-CONTRACT 1 |
| 103 | `o110-double-report-set-shape` | 1 | KEEP-CONTRACT 1 |
| 104 | `o112-if-else-unwrap-missing` | 1 | KEEP-ORACLE 1 |
| 105 | `opaque-loop-body-writes-read` | 1 | REWRITE 1 |
| 106 | `opaque-switch-arm-writes-unknown` | 1 | KEEP-ORACLE 1 |
| 107 | `provider-unit-proc-summary-missing` | 1 | KEEP-ORACLE 1 |
| 108 | `pure-summary-misses-global-write` | 1 | KEEP-ORACLE 1 |
| 109 | `renamed-builtin-loses-identity` | 1 | KEEP-ORACLE 1 |
| 110 | `return-options-handler-falls-through` | 1 | REWRITE 1 |
| 111 | `self-compare-fold-declined` | 1 | KEEP-ORACLE 1 |
| 112 | `startup-scalar-definedness` | 1 | KEEP-ORACLE 1 |
| 113 | `static-loop-exit-decides-branch` | 1 | FIX-EXPECTATION 1 |
| 114 | `stub-redeclaration-ignored` | 1 | KEEP-CONTRACT 1 |
| 115 | `subst-known-body-marked-unseen` | 1 | REWRITE 1 |
| 116 | `trace-callback-fixture-raises` | 1 | FIX-FIXTURE 1 |
| 117 | `typed-incr-ignores-trust-gate` | 1 | REWRITE 1 |
| 118 | `untyped-operand-fixture` | 1 | FIX-FIXTURE 1 |
| 119 | `upvar-signed-level` | 1 | KEEP-ORACLE 1 |

## Distinct failing behaviours

These are the behaviours users or rewrites lose, backed by tclsh or a contract.
Fixture-only failures are not listed: unwired pass contexts, unbounded
recursion, unset variables, dialect-blind units, lifecycle procedures not
installed, and the changed Jim entry premise.

Propagation and folding:
- A constant store whose reads were all propagated stays. So does the feeding store after an expression fold (`set a 1`), and a coupled dead store.
- Constants do not reach quoted words (O129, `"x=$x"`), a `$x` after a braced literal `{$x}`, an `${}b` word, the value after `incr` (top level or nested), or a `$var` chain piece the lattice proves (#2052).
- Mathfunc calls (`abs(-5)`, aliased mathfuncs) and the recursive `fact 5` do not fold. `$x < $x` with an untyped `$x` does not fold either.
- `if {0} … else …` is not unwrapped. The boolean ternary `($a > $b) ? 1 : 0` is not simplified, and O114 is lost on a loop-body `set x [expr {$x+1}]`.
- O128 (`end`/`end-N`) is never offered.
- Chain folds: `append`/`lappend` chains and consecutive literal `set`s are not packed in the applied output, because grouped chain edits are lost (O104/O130/O119). Logical units get no chain advice.
- Nothing propagates inside a called TclOO method (`[self class]`, `namespace qualifiers` chains, method locals).
- A procedure defined in nested `namespace eval`s is not recorded, so calls to it do not fold.

Calls and summaries:
- No procedure transfer summaries. After `bump n` / `reset n` / a document procedure that writes a global, the caller gets no value or existence outcome (19 tests).
- A namespace-resolved or `unknown`-handled callee inside `[ ]`, and `switch` word substitutions, lose their caller-frame writes.
- `detect_upvar_procs` over-propagates, so a wrapper of a global or own-frame `uplevel` becomes a barrier. `upvar +0 $t g` (8.6+) is not seen as a possible global write.
- Native i64 add is never selected for a procedure called from authored source, including the sealed four-statement demonstration (tclsh prints 6). Its typed decline reasons collapse to a generic one.
- Call-graph edges are missing for `trace`, `lsort -command`, `time {catch {…}}` callbacks and iRules `call`. O124 never comments out an unused iRules procedure.
- Procedure completion: a straight-line local `set` chain does not complete, and a document stub that redeclares a builtin is ignored. A mutated or foreign header still completes, which is a soundness guard.

Untrust and barriers:
- After an unknown head the ruling ends the premise, but `set g 5; foo; puts $g` still forwards. A constant read before the unknown command is dropped. Flow-sensitive untrust is not yet implemented.
- A literal `after` callback makes every variable in the file externally mutable. A named child interpreter's `interp eval` blocks forwarding in the parent. An opaque `switch` whose arms write other names blocks an unrelated variable.
- `subst` of known code is marked unseen. The `info exists` fold under a captured native Direct entry is lost because the procedure is not published.

Dead stores:
- Definite writers do not kill an earlier store: the `regsub` / `catch` result variable, Tcl 9 `encoding -failindex`, the 9.1 grapheme cursor, and a top-level `$argv` copy.
- `try {exit 7} finally {…}` keeps the dead `finally` store, and an in-procedure transitive dead chain is not removed as O108.
- `while {[gets $fd line] >= 0}` is taken to read `line`, so `set line {}` before the loop stays. An effect call `[set x 10]` inside an expression reads the version it overwrites.
- An opaque `catch`/`try` does not state its body, handler or `finally` writes as may-definitions. A `try` handler ending in `return -code error` falls through to the join, which gives a spurious phi (the `auto_mkindex` regression).
- `for {set {a} 1} {$a == 1} …` is not proved to run at least once.

Names and roles:
- A braced literal name (`{$dict}`, `{$x}`) is read as the plain variable, which can mislead diagnostics and rename. tclsh keeps `{$x}` and `x` apart.
- An `lmap` body inside a command substitution reports its loop variable as a frame read.
- A `superclass` in a class definition script records no class relation.
- A builtin renamed away (`::orig_incr`) loses its binding. Tk `-textvariable` writes lose their widget receiver.
- `expr {"x"}` has no value under tcl8.4 (tclsh 8.4 prints `x`).
- O110 is reported twice for the `set v [expr {$r ** 2}]; return $v` shape (#1962). The O101 expression-argument fold ignores retained module availability.
- A direct lowering drops its Standalone / SuppliedSource / Logical source context. A stale analysis input still selects definition subjects.

## Notes for the second pass

- **The passing negative tests on the unwired PassContext helpers check nothing.** 48 library tests build a `PassContext` without a registry or module (`pass-context-unwired`, FIX-FIXTURE). The passing tests in the same files assert that a code is absent (`all(code != O112/O109/O114/O124 …)`) through those same helpers. They pass only because the pass never runs with its inputs, so they are vacuous until wired and go in the second pass. This is the largest risk in the family.
- **The multi-caller native-integer join code is unconsumed product code.** The range join over several callers in `native_integer_proof.rs` has no consumer: only the sealed four-statement add consumes NativeInteger, and no multi-caller source can select it. Its three tests are RETIRE. After the scrub the code is either removed or given a consumer.
- `return_and_set_bodies_agree_on_expression_rewrites_issue_1962` stays KEEP-CONTRACT for the double O110 report. Its later rows encode the withdrawn "native result protocol" and are re-read under the O101 ruling.
- `interprocedural::completion::tests::a_callee_counts_as_defined_only_where_its_definition_surely_ran` (`needs-run`) holds 11 sources still to be run on tclsh 8.4–9.1.
- Tail-call fixtures (`unbounded-self-recursion-fixture`, 11) each have a named, bounded, called recursion with interpreter output in their evidence.
- Uncalled bodies (`uncalled-body-generic-entry`, 32) are KEEP-CONTRACT: facts that hold for every entry must be proved without a call.

## Exact-copy and overlap groups

No test in part 3 (interprocedural, cfg_builder) is in any group. Passing tests
are included where they share a source.

### Optimiser library


| Group | Members (state) | Suggested survivor |
| --- | --- | --- |
| overlap `fa783cf34bc6` | `elimination::tests::unused_variable_fires_o126_in_proc_body` (failing), `elimination::tests::o126_still_fires_for_literal_rhs` (failing): same source `proc ::f {} { set y 42; return 1 }`, same `run_pass` and same O126 assertion | `unused_variable_fires_o126_in_proc_body` (FIX-FIXTURE); `o126_still_fires_for_literal_rhs` is MERGE |
| overlap `9520ebe95d81` | `tail_call::tests::o121_fires_on_8_6_plus_dialects` (failing), `tail_call::tests::o121_suppressed_on_pre_8_6_dialects` (passing): same unbounded `proc ::f {n} {f $n}` under `run_pass_with_dialect` | No merge, since they assert opposite polarities by release. Keep both and move both to one terminating, called recursion. The passing one only holds because nothing is ever rewritten on this source. |

No input test is in an exact-copy group.


### SCCP, SSA and native-integer proof

| Group | Members (all failing) | Suggested survivor |
| --- | --- | --- |
| exact `4b20c61e702c` / overlap `31a05d63fa1f` | `sccp::tests::fold_trust::list_arm_declines_once_the_unit_renames_list`, `sccp::original_value_facts::tests::fold_trust::list_arm_declines_once_the_unit_renames_list` | `sccp::tests::fold_trust::…` (KEEP-ORACLE); the `original_value_facts` copy is MERGE |
| exact `e1190b2c8f2f` / overlap `6e554a3d918e` | `sccp::tests::fold_trust::head_word_arms_decline_only_for_the_name_the_unit_touched`, `sccp::original_value_facts::tests::fold_trust::head_word_arms_…` | `sccp::tests::fold_trust::…` (KEEP-ORACLE) |
| exact `f8917f402364` / overlap `af9275c87727` | `sccp::tests::definition_values::evaluate_def_folds_irules_word_operator_only_under_an_irules_policy`, `sccp::original_value_facts::tests::definition_values::…` | `sccp::tests::definition_values::…` (FIX-FIXTURE onto a trusted fold entry; the iRules `contains` row also belongs in bigip-name-checks.md) |

These pairs are near-copies across `sccp/tests/` and `sccp/original_value_facts/tests/` that the cluster files do not list:
- `an_unnameable_rename_subject_…`: the two copies diverge. Survivor: the `original_value_facts` copy (it asserts the resolved case); the `sccp::tests` copy is MERGE.
- `sccp_folds_post_loop_branch_via_static_summary`: the two copies contradict each other on whether sccp may decide the branch. Keep the `original_value_facts` copy after its expectation fix, and move the `sccp::tests` copy onto a trusted re-run or merge it.
- `sccp_foreach_nested_braced_list_constset_not_corrupted`: both REWRITE; keep one after the rewrite.
- `existence_fold_*_nested_upframe_alias_unset_mutation`: survivor is the `original_value_facts` copy; the `sccp::tests` copy is MERGE.

Two `sccp::tests` refinement or statement tests could share one called-procedure table once their fixtures call the procedure (the `last_values` and `puts_reads` rows).


### Integration tests

`clusters_exact.tsv` and `clusters_overlap.tsv` contain **no** test of this input (checked by `crate target path`). The only nearby cluster is `variable_shape_guardrails` (it_3 and it_5, passing, not in this input). Groups found by hand: the same sources and checks, which the tool misses because the helpers or codes differ.

| Group | Members | Suggested survivor |
| --- | --- | --- |
| O115 unwrap | it_3 `idiom_rewrites::nested_expr_unwrap_o115`; it_5 `idiom_rewrites::o115_nested_expr_unwrap` | it_5 (adds the string-length TN). MERGE in the ledger |
| O120 constant operand | it_3 `idiom_rewrites::string_compare_eq_ne_o120`; it_5 `idiom_rewrites::o120_with_sccp_const_and_mixed` (verbatim subset) | it_3. MERGE in the ledger |
| build_banner multipass | it_3 `propagation::multipass_string_build_collapses_to_literal`; it_5 `robustness::profile_directive_and_multipass` | it_5. MERGE in the ledger |
| O114 | it_3 `incr_idiom_o114`; it_5 `o114_incr_idiom` | it_5, after adding it_3's `opt_fires` for add5/sub3. Not merged yet |
| O119 | it_3 `multi_set_packing_o119`; it_5 `o119_multi_set_packing` | it_5 if it takes it_3's `puts` few-row |
| O113 `%`→`&` | it_3 `strength_reduction_o113`; it_5 `o113_strength_reduction`; it_3 `optimiser_depth::branch_folding_condition_cascade` | it_5, once all are on `int_xy` |
| O104 string chain | it_3 `string_write_chains_o104`; it_5 `o104_string_build_chain`; unit `chain_fold::tests::string_chain_folds_to_single_set`, `string_chain_rewrite_applies_cleanly` (passing) | it_5 plus the unit pair |
| O104 interleaved | it_3 `optimiser_depth::chain_fold_o104_continues_past_interleaved_literal_set`; unit `chain_fold::tests::chain_continues_past_interleaved_literal_set` (passing, same source) | Keep both: one at the pass, one at the applied output |
| O130 list chain | it_3 `optimiser_depth::chain_fold_o130_list_build_chain`; unit `list_chain_folds_with_lappend`, `list_chain_quotes_spacey_elements` (passing) | it_3 depth |
| O125 basic sink | it_3 `code_motion::code_sinking_o125_positive`; it_5 `code_motion::o125_code_sinking` | Either; neither is a superset |
| O112 loops | it_3 `structure_elimination_if_while_for` (wf/ff/ffe); it_5 `o112_while_for_elimination` | it_3, if it takes the `while {1}` negative |
| O110 ternary | it_3 `instcombine::instcombine_ternary_and_boolean_context`; it_5 `instcombine::o110_ternary_and_double_not` | it_3 (output plus code) |
| O124 | it_3 `dead_code::unused_irule_procs_o124` and `…_eval_suppression`; it_5 `procedures::o124_unused_irule_procs` | it_3 `unused_irule_procs_o124`, with eval_suppression folded in |
| Tail calls | it_3 `tail_calls::tail_call_detection_o121_o122` (walk); `optimiser_depth::tail_call_o122_loop_conversion_from_switch_arm`; it_3 `tail_call_loop_conversion_o122`; it_5 `tail_call_o121_o122_o123` | depth for `walk`; it_3 loop-conversion for factorial |
| Same-event handler DSE | it_3 `optimiser::cross_event_dse::same_event_dead_store_still_eliminated`; it_3 `optimiser_depth::cross_event_same_event_dead_store_still_eliminated` (same source, asserts O109 too) | depth; both now FIX-EXPECTATION by ruling (no applied O109), so the it_3 copy can merge into it once both expect the store kept |
| Dead feeding store | it_3 `propagation::constant_propagation_into_commands_o100` (`set x 5; puts $x; set y …`); it_5 `robustness::pass_interactions` (same source, weaker `contains`) | it_3 |
| incr forwarding | it_3 `propagation::incr_of_a_constant_folds_through_to_its_reads`; unit `propagation::tests::load_forwarding::a_computed_definition_forwards_the_constant_sccp_proved` (also failing, same sources) | Keep both: pass level and applied output |
| Nested RMW | it_3 `nested_reads_and_writes::a_nested_rmw_read_keeps_its_feeding_store_alive`; unit `elimination::tests::store_read_by_a_nested_cell_update_is_not_dead` (also failing) | Keep both |
| Self-class chain | codegen `optimiser_codegen_residual::namespace_qualifiers_and_tail_fold…`, `sccp_closes_the_two_hop…`; it_4 `analyser::variables::const_cmd_subst_set_rhs::a_self_class_chain_folds_inside_an_instance_method` (also failing) | codegen `namespace_qualifiers…` for O129; it_4 for the analyser view |
| Nested namespace procs | it_5 `pipeline_coverage::compilation_unit_build::nested_namespace_procedures`; it_5 `robustness::edge_cases_loops_and_namespaces` (fold half) | Keep both: one at the unit, one at the fold |
