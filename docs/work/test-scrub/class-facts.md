# Test scrub: class and object facts

Ledger: [`ledger/class-facts.tsv`](ledger/class-facts.tsv), 246 failing `tcl-compiler` tests, measured on `work` at `33cfe167` (17 November 2026): the library families `registry_invocation`, `lowering`, `analyser::oo`, `analyser::original_members`, `object_types`, and `it_4` `analyser::class_factories`, `analyser::tcloo`, `analyser::method_bodies`. Every TclOO fixture behind a KEEP-ORACLE row was run on tclsh 8.6.18, 9.0.4 and 9.1.0, the others also on 8.4.20 and 8.5.19; no jimsh was available, so the Jim rows rest on contracts.

## Counts by verdict

| Verdict | Tests |
| --- | ---: |
| KEEP-ORACLE | 74 |
| FIX-FIXTURE | 67 |
| KEEP-CONTRACT | 43 |
| REWRITE | 40 |
| FIX-EXPECTATION | 14 |
| UNDECIDED | 4 |
| KEEP-F5 | 2 |
| RETIRE | 1 |
| MERGE | 1 |

## Causes ranked by tests blocked

| Cause | Tests | Verdicts |
| --- | ---: | --- |
| `user-metaclass-made-class-publishes-no-factory` | 15 | KEEP-ORACLE 14, FIX-EXPECTATION 1 |
| `fixture-reads-unset-variable` | 13 | FIX-FIXTURE 13 |
| `f5-normal-handler-proofs` | 10 | REWRITE 8, KEEP-ORACLE 2 |
| `fixture-lowering-route-incomplete` | 9 | FIX-FIXTURE 8, REWRITE 1 |
| `creation-under-missing-namespace-withdraws-world` | 7 | KEEP-ORACLE 7 |
| `uncalled-body-nested-proc-not-registered` | 7 | KEEP-ORACLE 7 |
| `logical-source-structure-none` | 6 | KEEP-CONTRACT 3, FIX-EXPECTATION 2, REWRITE 1 |
| `class-layer-oo-define-has-no-closed-route` | 5 | KEEP-ORACLE 3, KEEP-CONTRACT 2 |
| `trace-target-command-undefined` | 5 | FIX-FIXTURE 5 |
| `jim-original-metadata` | 4 | KEEP-CONTRACT 4 |
| `dialect-blind-fixture` | 4 | FIX-FIXTURE 4 |
| `context-registry-lowering-loses-metadata` | 4 | KEEP-CONTRACT 4 |
| `package-not-required` | 4 | FIX-FIXTURE 4 |
| `uncalled-body-eval-not-inlined` | 4 | REWRITE 4 |
| `standalone-unit-empty-class-roster` | 4 | FIX-FIXTURE 4 |
| `lenient-tcl-project-receiver-refused` | 4 | KEEP-CONTRACT 2, KEEP-ORACLE 1, FIX-EXPECTATION 1 |
| `namespace-path-refuses-source-point` | 4 | KEEP-ORACLE 2, UNDECIDED 1, REWRITE 1 |
| `f5-hosted-source-metadata` | 3 | KEEP-CONTRACT 3 |
| `fabricated-availability-registry` | 3 | FIX-FIXTURE 3 |
| `possible-candidates-dropped` | 3 | KEEP-ORACLE 1, FIX-FIXTURE 1, REWRITE 1 |
| `lone-surrogate-class-name-not-recorded` | 3 | KEEP-ORACLE 2, KEEP-CONTRACT 1 |
| `loop-installed-members-leave-ledger-unreadable` | 3 | KEEP-CONTRACT 2, KEEP-ORACLE 1 |
| `cross-file-metaclass-hand-off` | 3 | KEEP-ORACLE 1, REWRITE 1, FIX-FIXTURE 1 |
| `fixture-names-missing-namespace` | 3 | FIX-FIXTURE 3 |
| `collection-transport-capture-missing` | 3 | KEEP-ORACLE 2, FIX-FIXTURE 1 |
| `construction-makes-cards-incomplete` | 3 | KEEP-ORACLE 2, FIX-EXPECTATION 1 |
| `source-point-child-analysis-refused` | 3 | REWRITE 2, KEEP-ORACLE 1 |
| `authored-simulation-structure-selection` | 2 | REWRITE 2 |
| `conditional-declaration-after-unset-rename` | 2 | FIX-FIXTURE 2 |
| `build-for-profile-not-standalone` | 2 | FIX-FIXTURE 2 |
| `jim-body-parameter-source` | 2 | REWRITE 2 |
| `uncalled-caller-dependencies-closed` | 2 | KEEP-ORACLE 2 |
| `logical-oo-definition-layout` | 2 | REWRITE 2 |
| `unresolved-point-now-resolved` | 2 | FIX-FIXTURE 2 |
| `class-definition-child-source-words` | 2 | KEEP-CONTRACT 2 |
| `class-definition-member-region` | 2 | FIX-FIXTURE 2 |
| `list-head-taint-selection` | 2 | REWRITE 2 |
| `store-advice-withdrawn` | 2 | KEEP-CONTRACT 2 |
| `f5-return-normal-completion` | 2 | KEEP-ORACLE 2 |
| `alias-effective-words-none` | 2 | KEEP-ORACLE 2 |
| `fixture-calls-undefined-command` | 2 | FIX-FIXTURE 2 |
| `structured-switch-not-lowered-under-supplied-input` | 2 | KEEP-CONTRACT 2 |
| `foreach-literal-installer-not-simulated` | 2 | KEEP-ORACLE 2 |
| `fixture-definition-raises` | 2 | FIX-FIXTURE 2 |
| `unknown-dispatch-constructor-not-proved` | 2 | REWRITE 2 |
| `helpers-candidate-list-now-exact` | 2 | FIX-EXPECTATION 2 |
| `unavailable-metadata-context-not-withheld` | 2 | KEEP-CONTRACT 2 |
| `irules-top-level-executable-command` | 2 | FIX-FIXTURE 2 |
| `brace-continuation-derived-body-origin` | 2 | KEEP-CONTRACT 2 |
| `irules-proc-declaration-dropped` | 2 | KEEP-F5 2 |
| `class-instance-var-not-class-owned` | 2 | KEEP-ORACLE 2 |
| `member-body-route-changed` | 2 | UNDECIDED 2 |
| `uncalled-body-source-association-unknown` | 2 | KEEP-CONTRACT 2 |
| `foreachline-receiver-not-lowered` | 1 | KEEP-ORACLE 1 |
| `tk-factory-publication` | 1 | KEEP-CONTRACT 1 |
| `body-view-projection-mismatch` | 1 | RETIRE 1 |
| `expression-advice-uncalled-body` | 1 | REWRITE 1 |
| `jim-ensemble-worker-replacement` | 1 | KEEP-ORACLE 1 |
| `objdefine-abort-on-variable-held-instance-not-reported` | 1 | KEEP-ORACLE 1 |
| `uncalled-method-body-read-before-set-not-reported` | 1 | KEEP-ORACLE 1 |
| `fixture-requires-missing-package` | 1 | FIX-FIXTURE 1 |
| `class-not-recorded-under-lenient-tcl-environment` | 1 | KEEP-ORACLE 1 |
| `child-interp-receiver-declaration-not-retained` | 1 | KEEP-CONTRACT 1 |
| `hosted-snit-class-declarations-not-retained` | 1 | KEEP-CONTRACT 1 |
| `alias-head-records-original-head-name-input` | 1 | REWRITE 1 |
| `oo-define-special-members-double-counted` | 1 | REWRITE 1 |
| `variable-head-now-resolved-by-the-walk` | 1 | FIX-EXPECTATION 1 |
| `helper-branch-return-not-propagated` | 1 | KEEP-ORACLE 1 |
| `unprovable-package-require-hides-later-classes` | 1 | KEEP-ORACLE 1 |
| `second-creation-of-existing-class-not-proved-failing` | 1 | FIX-EXPECTATION 1 |
| `next-in-entered-method-body-draws-w123` | 1 | KEEP-ORACLE 1 |
| `fixture-proc-under-missing-namespace` | 1 | FIX-FIXTURE 1 |
| `relative-head-in-uncalled-method-body` | 1 | KEEP-ORACLE 1 |
| `fixture-release-lacks-command` | 1 | FIX-FIXTURE 1 |
| `oo-define-on-undeclared-class` | 1 | FIX-EXPECTATION 1 |
| `no-profile-registry-fixture` | 1 | FIX-FIXTURE 1 |
| `isolated-body-source-purpose-missing` | 1 | REWRITE 1 |
| `return-two-words-is-option-pair` | 1 | FIX-EXPECTATION 1 |
| `colon-named-proc-body-not-lowered` | 1 | KEEP-ORACLE 1 |
| `held-jim-body-lookup-state-missing` | 1 | KEEP-CONTRACT 1 |
| `apply-body-proc-not-installed` | 1 | KEEP-ORACLE 1 |
| `advisory-names-duplicated-per-candidate` | 1 | REWRITE 1 |
| `objdefine-variable-not-in-method-scope` | 1 | KEEP-ORACLE 1 |
| `advisory-fallback-widens-instance-vars` | 1 | KEEP-ORACLE 1 |
| `wrapper-member-marked-unretained` | 1 | REWRITE 1 |
| `same-invocation-prefix-shape` | 1 | KEEP-CONTRACT 1 |
| `opaque-call-vs-barrier-shape` | 1 | REWRITE 1 |
| `irules-standalone-priority-not-reapplied` | 1 | KEEP-CONTRACT 1 |
| `irules-dynamic-priority-setter` | 1 | FIX-EXPECTATION 1 |
| `ns-export-in-namespace-eval-not-recorded` | 1 | KEEP-ORACLE 1 |
| `lenient-tcl-second-class-method-missing` | 1 | REWRITE 1 |
| `logical-candidates-need-lenient-profile` | 1 | FIX-FIXTURE 1 |
| `analysis-nondeterminism-aliases` | 1 | UNDECIDED 1 |
| `tk-command-without-tk` | 1 | FIX-FIXTURE 1 |
| `factory-return-candidates-without-input` | 1 | KEEP-CONTRACT 1 |
| `unprovided-package-still-types-handle` | 1 | KEEP-ORACLE 1 |
| `snit-nominal-name-missing` | 1 | KEEP-ORACLE 1 |
| `nested-expr-vector-missing` | 1 | MERGE 1 |
| `logical-source-words-missing-plain-tcl` | 1 | KEEP-CONTRACT 1 |
| `substituted-body-control-lookup-missing` | 1 | REWRITE 1 |
| `lenient-tcl-root-fallback-missing` | 1 | KEEP-ORACLE 1 |
| `bounded-unknown-not-a-barrier` | 1 | FIX-EXPECTATION 1 |
| `sourced-file-namespace-home-lost` | 1 | KEEP-ORACLE 1 |
| `operand-view-includes-own-load` | 1 | FIX-EXPECTATION 1 |
| `insert-offset-after-terminator-whitespace` | 1 | REWRITE 1 |
| `conditional-child-gets-invocation-realm` | 1 | KEEP-CONTRACT 1 |
| `deferred-source-selected-as-current-lookup` | 1 | KEEP-CONTRACT 1 |
| `literal-expansion-origin-flattened` | 1 | REWRITE 1 |

## Distinct failing behaviours worth fixing

Test-only faults (FIX-FIXTURE: unset variables, a lowering route that installs no procedure, undefined trace targets, packages never required, dialect-blind or standalone units) are not listed; they are fixed in the tests. What remains is product behaviour, wrong answers first.

Wrong answers (the product says something tclsh contradicts):

1. A creation after `proc ::T::D::define` in a namespace that does not exist: tclsh raises `unknown namespace` and stops, the product still records `::T::D::class` (`walk_blockers::a_declaration_body_the_walk_cannot_read_is_not_a_blocker`; the fixture also needs the namespace).
2. A call inside an uncalled procedure is treated as closed to the in-file callee, but a later `proc f` or `rename` decides it on tclsh (`SECOND:1`, `THIRD:1`); feeds interprocedural taint.
3. `HTTP::respond` hosted metadata also issues a native invocation, which the hosted-source taint contract forbids.
4. A nested `proc inner` is selected as structured, feeding rename's declaration coverage.
5. A Jim parameter source is still given after an unknown command.
6. `set target VALUE` yields two advisory assignments where one is right.
7. `struct::graph g` is typed as an object handle under tcl8.6 with no tcllib provider (tclsh: `can't find package`), adding call-graph candidates.
8. Factory return candidates are produced with missing or foreign input, against `original-analysis-metadata-context.md`.
9. Under an 8.4 or foreign context the tcltest body provider is still selected and `namespace export` still recorded; the contract withholds both.
10. Two analyses of the same `aliases.tcl` under tcl8.6 compare unequal (`object_types::report_comparison::tests::current_native_alias_report_capture_control`).

Missing facts (tclsh shows the behaviour; the product abstains or withdraws):

1. A class made by a user metaclass publishes no factory, and a creation through a metaclass whose constructor is inherited from `oo::class` leaves the command world unknown (15 tests; tcl-lsp-db `class_factory_fixpoint` and the e2e `issue1296_metaclass_chain` share it).
2. TclOO creation under a namespace that does not exist withdraws the command world (7; analyser-binding asked).
3. Members added by `oo::define` that the closed route does not cover: an inline constructor, `renamemethod`, a replaced `method`, `superclass`, a target held in a variable (5; e2e `tcloo_navigation` shares it).
4. A procedure that a never-called body defines on every call is not registered (7; reading of the uncalled-body ruling asked).
5. iRules handler routes at rule-loader top level (10; 8 are REWRITE to a `when` body, 2 KEEP-ORACLE).
6. `if`/`switch` shapes and expression advice under the lenient `tcl` input (6).
7. A file sourced inside `namespace eval app {namespace path ::lib; source child.tcl}` gets no source point (4), and three sourced-child analyses are refused outright (3).
8. Under the lenient `tcl` profile a project class card answers no receiver for `[C new] ping` (4); a command substituted into a root head has no navigation recipe there (a fix is held: it measured one regression, `project_class_cards_require_selected_constructor_shape_and_compatible_inputs`).
9. A construction in the declaring file makes a project card incomplete although a class with no constructor runs no script (3).
10. Lowering with `with_context_registry` drops the supplied context from the module (4; contract `original-analysis-metadata-context.md`).
11. Jim native-entry metadata (4) and F5 hosted-source metadata (3), both contract-stated.
12. `C\uD800` is a real class distinct from `C\uD801`; no declaration is recorded (3).
13. `foreach m {…} {method $m …}` leaves the member ledger unreadable (3).
14. Cross-file metaclass hand-off (3); a braced body with a backslash-newline gets no derived source origin (2); a later standalone iRules `priority 400` is ignored (1); `proc :f` in `::N` is recorded but not lowered (1); `namespace export` inside `namespace eval` is lost (1); an `oo::objdefine` per-object `variable z` is missing from the object's method scope (1); a fallback adds `info exists z` arguments to a class method's instance variables (1); a `proc` inside an executed `apply` body is not installed (1).

## Expectations corrected by tclsh or a ruling (FIX-EXPECTATION, 14)

Among them: `switch -exact $cmd {…}` is not ambiguous on 8.5 to 9.1 (tclsh prints `OTHER` for `--`, `-regexp`, `-nocase`, `-bogus`, `-glob`; only 8.4 reads the value as an option), so the test and its passage in `original-analysis-metadata-context.md` change; `return first second` is an option pair on 8.5 to 9.1; on a pinned release a bare `unknown_effect` mutates nothing under the #2427 rule (three rows apply to `tcl` only); only `::oo::Helpers::callback` is reachable on 9.0 and 9.1 even with `proc ::callback`; a second creation of an existing class raises `command already exists`.

## Rulings applied (coordinator, 17 November)

- The generic entry of a never-called body covers a procedure that body defines on every call: that procedure's body facts only; whether it exists for an outside caller still needs the outer body to have surely run, and a conditional or computed definition gets nothing.
- Same input, same result to the byte is a contract of the analysis: the two unequal analyses of `aliases.tcl` are a wrong answer (the tenth).
- A catalogue an edit-producing consumer reads stays on the proof side: a relative head in an uncalled method body has no proof until the entry-independent binding lands.
- Abstaining at `source -encoding` with a recorded read obligation is sound design.

## Overlap with other tests

No exact cluster touches these families. Overlap groups (all members pass unless said): `f32f43be1de9` (two `cfg_builder` tests, exact); `363ac8b46027` (four `unit_return_*` tests, one failing, FIX-EXPECTATION); `6dd526ea73a6` (switch braced bodies); `781abf795d42` (`incr_single_arg_lowers_to_incr` and `incr_command`, a second-pass MERGE candidate); `c1f3bb261e85` (two `trace_add_variable_*`); `dd54e22b481c` (`return_value_lowers_to_return` and `return_statement`, a second-pass MERGE candidate); six passing groups in the class-factory families, and the failing server `tcloo_navigation` pair that shares the `oo::define` cause. Two passing negative guards in the class-factory family read an unset `$runtime` and cannot fail; they go to the second pass. One MERGE: `nested_factorial_…` folds into `tcl-lsp-core` `it_3` `lsp_edit_workspace::minify_strips_comments_and_collapses_whitespace_factorial`.
