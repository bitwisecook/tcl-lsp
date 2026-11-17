# Test scrub: compiler analyser and binding

Ledger: [`ledger/analyser-binding.tsv`](ledger/analyser-binding.tsv). It has 418 rows, measured on `work` at `2195dfee` (17 November 2026).
- 416 are failing `tcl-compiler` library tests in this area's families: `command_binding`, `unit_scope`, `compilation_unit`, `inline_uplevel`, `dynamic_names`, `deferred_writes`, `word_subst`, `common_aot_plan`, `native_lowering` and `cfg_builder::upvar_info`.
- The other 2 are the `tcl-codegen` `it_3` inlining rows that live-inlining routed here.

Every row was run on the current binaries, and every behaviour claim was checked on tclsh 8.4.20, 8.5.19, 8.6.18, 9.0.4 and 9.1.0. Jim rows were checked on jimsh 0.84 and tcllib rows on tcllib 2.0.

The coordinator's reading is applied throughout: where the product still claims something tclsh contradicts, the row is a wrong answer and its cause starts `wrong-`. That holds even when no fold or edit is shown to consume the claim.

## Counts by verdict

| Verdict | Tests |
| --- | ---: |
| KEEP-CONTRACT | 140 |
| FIX-FIXTURE | 98 |
| KEEP-ORACLE | 94 |
| REWRITE | 50 |
| FIX-EXPECTATION | 22 |
| KEEP-F5 | 12 |
| RETIRE | 1 |
| MERGE | 1 |
| **Total** | **418** |

## Tests by family

| Family | Failing |
| --- | ---: |
| `command_binding` | 277 |
| `unit_scope` | 26 |
| `compilation_unit` | 24 |
| `inline_uplevel` | 16 |
| `common_aot_plan` | 15 |
| `native_lowering` | 13 |
| `dynamic_names` | 12 |
| `cfg_builder::upvar_info` | 11 |
| `deferred_writes` | 11 |
| `word_subst` | 11 |
| `tcl-codegen inlining (routed)` | 2 |

## Causes ranked by tests blocked

Wrong answers are listed separately below; this table is every cause with two or more tests.

| Cause | Tests | Verdicts |
| --- | ---: | --- |
| `profileless-registry-fixture` | 45 | FIX-FIXTURE 45 |
| `uncalled-body-generic-entry` | 26 | KEEP-CONTRACT 24, REWRITE 2 |
| `fixture-lowering-route-incomplete` | 16 | FIX-FIXTURE 16 |
| `unseen-call-marker-withdraws-every-procedure` | 12 | KEEP-CONTRACT 6, KEEP-ORACLE 6 |
| `lenient-nested-substitution-inventory` | 10 | KEEP-CONTRACT 8, REWRITE 2 |
| `callback-shape-not-read` | 9 | KEEP-ORACLE 8, KEEP-CONTRACT 1 |
| `wasm-native-lowering` | 9 | KEEP-CONTRACT 9 |
| `conditional-declaration-after-unset-rename` | 8 | FIX-FIXTURE 8 |
| `fixture-raises-before-subject` | 8 | FIX-FIXTURE 8 |
| `unentered-site-has-reached-lookup` | 7 | KEEP-CONTRACT 7 |
| `bounded-unknown-not-a-barrier` | 7 | FIX-EXPECTATION 7 |
| `call-specialised-body-rewrite` | 7 | KEEP-CONTRACT 4, KEEP-ORACLE 2, FIX-EXPECTATION 1 |
| `aot-direct-call-trace-unproved` | 6 | KEEP-CONTRACT 3, KEEP-ORACLE 2, REWRITE 1 |
| `stub-declared-words-lenient-tcl` | 5 | KEEP-CONTRACT 5 |
| `isolated-body-source-purpose-missing` | 5 | KEEP-ORACLE 5 |
| `entry-independent-binding-missing` | 5 | KEEP-CONTRACT 5 |
| `jim-source-class-declaration-missing` | 5 | KEEP-ORACLE 3, KEEP-CONTRACT 2 |
| `lenient-tcl-logical-source-records` | 5 | REWRITE 4, KEEP-CONTRACT 1 |
| `fabricated-availability-registry` | 4 | FIX-FIXTURE 4 |
| `evaluated-body-binding-change-not-replayed` | 4 | KEEP-ORACLE 4 |
| `native-body-template-refused` | 4 | KEEP-CONTRACT 2, REWRITE 2 |
| `slot-presence-unknown` | 4 | KEEP-ORACLE 4 |
| `fixture-reads-unset-variable` | 4 | FIX-FIXTURE 4 |
| `rule-loader-declaration-inventory` | 3 | KEEP-F5 3 |
| `braced-word-extra-chars-raises` | 3 | FIX-EXPECTATION 3 |
| `no-native-declaration-body-unit` | 3 | REWRITE 3 |
| `logical-procedure-argv-roles-missing` | 3 | KEEP-CONTRACT 3 |
| `logical-lambda-decoded-field-missing` | 3 | REWRITE 3 |
| `oo-destroy-deletion-unknown` | 3 | KEEP-ORACLE 3 |
| `wrong-evaluated-body-rename-not-replayed` | 3 | KEEP-ORACLE 3 |
| `logical-unentered-site-advice-frame` | 3 | KEEP-CONTRACT 3 |
| `walk-operands-withdrawn` | 3 | REWRITE 2, KEEP-CONTRACT 1 |
| `conditional-original-child-refused` | 3 | KEEP-F5 3 |
| `supplied-fixture-no-source-input` | 3 | FIX-FIXTURE 3 |
| `build-for-profile-not-standalone` | 2 | FIX-FIXTURE 2 |
| `native-entry-slot-target-missing` | 2 | KEEP-CONTRACT 2 |
| `report-compare-field-correspondence-unavailable` | 2 | KEEP-CONTRACT 2 |
| `jim-static-reference-capture` | 2 | KEEP-ORACLE 2 |
| `original-namespace-world-8-4` | 2 | KEEP-CONTRACT 2 |
| `proc-after-interp-alias-not-recorded` | 2 | KEEP-ORACLE 2 |
| `aot-native-admission-premise-missing` | 2 | KEEP-CONTRACT 1, REWRITE 1 |
| `logical-navigation-candidates-missing` | 2 | REWRITE 2 |
| `abstract-export-new-not-visible` | 2 | KEEP-ORACLE 2 |
| `logical-publication-proposal-missing` | 2 | REWRITE 2 |
| `discarded-procedure-history` | 2 | KEEP-CONTRACT 2 |
| `command-trace-callback-not-replayed` | 2 | KEEP-ORACLE 2 |
| `f5-unknown-compiler-protocol` | 2 | KEEP-F5 2 |
| `logical-deferred-body-not-retained` | 2 | KEEP-CONTRACT 2 |
| `bare-literal-expr-result-proved` | 2 | FIX-EXPECTATION 2 |
| `conditional-original-body-parent-missing` | 2 | KEEP-CONTRACT 2 |
| `export-boundary-drops-procedure-unit` | 2 | KEEP-ORACLE 1, KEEP-CONTRACT 1 |
| `callee-global-write-widens-prior-read` | 2 | KEEP-ORACLE 2 |
| `f5-call-invoker-not-recorded` | 2 | KEEP-F5 2 |
| `lenient-tcl-logical-caller-records` | 2 | KEEP-CONTRACT 1, REWRITE 1 |
| `uplevel-passthrough-undetected` | 2 | MERGE 1, REWRITE 1 |
| `entered-call-erases-declaration-formal-advice` | 2 | REWRITE 1, KEEP-CONTRACT 1 |
| `jim-wrapper-future-body-missing` | 2 | KEEP-CONTRACT 1, KEEP-ORACLE 1 |
| `stock-literal-class-record-displaced` | 2 | REWRITE 2 |
| `logical-site-records-native-head-input` | 2 | KEEP-CONTRACT 1, REWRITE 1 |
| `math-occurrence-receipt-missing` | 2 | REWRITE 2 |
| `stock-object-types-unhosted-fixture` | 2 | FIX-FIXTURE 2 |
| `original-world-import-move-not-completed` | 2 | KEEP-CONTRACT 2 |
| `namespace-path-not-recorded` | 2 | KEEP-CONTRACT 2 |
| `native-admission-obligation-missing` | 2 | KEEP-CONTRACT 2 |

One test each (93 causes): `advice-tape-rename-lineage-missing`, `after-callback-captured-method-prefix-missing`, `alias-captured-prefix-operand-value`, `alias-chain-procedure-call-missing`, `aot-sealed-program-coverage-declined`, `apply-lambda-global-callee`, `binding-summary-shared-arc`, `body-fact-from-call-site-state`, `bytecode-object-if-not-structured`, `caller-scan-records-no-top-level-call`, `catalogue-unloaded-slot-closed-absent`, `cfg-binding-without-site-proof`, `cfg-lattice-seed-ignored`, `cfg-seed-unknown-entry-opaque-mutation`, `child-interp-body-widens-parent`, `classmethod-inheritance-not-recorded`, `classmethod-subclass-factory-missing`, `closed-world-source-boundary-still-declines`, `compiled-list-head-none-bytecode-entry`, `completed-world-now-proved`, `constructor-custom-code-destructor`, `cross-analysis-join-refused`, `custom-unknown-fallback-target-missing`, `declared-frame-own-callback-any`, `empty-variable-name-footprint`, `eval-concat-empty-word-vanishes`, `evaluated-index-place-missing`, `f5-eval-script-admission`, `f5-realm-materialized-footprint`, `fixture-formal-is-name-default-pair`, `fixture-rename-onto-existing-command`, `fixture-uncalled-caller-value`, `foreach-body-lookup-state`, `foreach-loop-variable-store-error-reached`, `foreign-declaration-arc-refusal`, `frozen-value-fixture-not-dynamic`, `interp-create-in-namespace-eval-slot-unknown`, `jim-alias-caller-namespace`, `jim-colon-namespace-proc-not-inventoried`, `jim-instance-key-normalisation`, `lambda-local-variable-symbol-missing`, `lassign-array-target-store-order`, `layout-projection-config-mismatch`, `literal-execution-trace-read-as-dynamic`, `logical-abrupt-prefix-footprint-missing`, `logical-computed-declaration-recipe`, `logical-definition-through-alias-of-renamed-proc`, `logical-entry-admits-f5-native`, `logical-eval-executed-script-missing`, `logical-head-name-input-issued`, `logical-name-advice-input-missing`, `logical-source-structure-none`, `lone-surrogate-procedure-name-not-recorded`, `lseq-provider-unselected-9-1`, `materialised-body-not-recorded`, `memo-semantic-bundle-source-metadata-missing`, `method-entry-instance-variable-cell-missing`, `module-binding-history-slot`, `multi-walk-body-operands-withdrawn`, `native-entry-cfg-statement-shape`, `native-entry-deleted-namespace-revival`, `native-entry-for-releaseless-profile`, `native-entry-observer-return-target`, `native-restoration-refuses-unchanged-failure`, `nested-compiler-name-effects`, `nested-declaration-layout-original-missing`, `nested-layout-visit-worlds-shared`, `oo-abstract-class-unbound`, `possible-trace-registration-withdraws-world`, `registered-instance-cache-strong-cycle`, `renamed-class-second-definition-inventory`, `retained-script-restoration-refused`, `rule-loader-when-admission`, `same-source-foreign-owner-indistinguishable`, `scalar-body-alpha-proposed-name-refusal`, `self-result-frozen-head-missing`, `single-word-puts-role-selected`, `snit-type-command-unbound`, `source-point-entry-namespace-path`, `standalone-template-layout-missing`, `stub-script-body-unbraced-geometry`, `stub-script-body-words-missing`, `tcllib-callback-prefix-missing`, `tcllib-named-factory-instance-missing`, `tk-fixture-without-tk`, `tk-widget-instance-class-missing`, `top-level-site-now-reached`, `unavailable-context-abstain`, `unentered-site-unknown-flag`, `unknown-autoload-transition-modelled`, `unknown-head-dispatches-through-unknown`, `upframe-inventory-record`, `upvar-other-var-cell-role`.

## Internal records: what depends on them

`command_binding` holds receipts, cards, footprints, transition advice and previews. Every row of that kind was judged on its consumer, under the README rule.
- **KEEP-CONTRACT (140):** each row cites the design document, proof page or ruling that states the record. The evidence column carries the path.
- **REWRITE (50):** each row names the observable the record reaches: a diagnostic code, a fold, a navigation answer or a native op. The test is re-pointed there.
- **RETIRE (1):** `native_static_read_receipt_keeps_the_current_unit_allocation_owner`. Its "foreign" unit is a second build of the identical source, so no answer can differ between the two, and it cites no contract.
- **No test is retired on a group argument.**

## Fixture faults (not product behaviour)

These are fixed in the tests, one commit per cause:
- `profileless-registry-fixture` (45): the registry is built with no profile, so it names no builtin (ruling 02f66c9c).
- `fixture-lowering-route-incomplete` (16): `lower_to_ir` installs no procedures; the same sources through `CompilationUnit::build_for` inline the `uplevel` passthrough. This makes live-inlining's `uplevel-passthrough-undetected` a fixture cause.
- `conditional-declaration-after-unset-rename` (8)
- `fixture-raises-before-subject` (8)
- `fabricated-availability-registry` (4)
- `fixture-reads-unset-variable` (4)
- `stock-object-types-unhosted-fixture` (2): moves to `analyse_hosted` under the stock-object-types ruling.
- `build-for-profile-not-standalone`
- `tk-fixture-without-tk`
- `fixture-rename-onto-existing-command`: tclsh raises `command already exists`.
- `fixture-formal-is-name-default-pair`: `{{with space}}` is the formal `with` with default `space`.

Two memo rows and two `mutation_summary` rows are marked to be read again once their fixture is fixed: what their failure shows may be a product fault.

## Exact-copy and overlap groups

No exact-copy group touches these families. One overlap group does: `728365a0436c`, with `dynamic_names::tests::unbraced_substituted_target_still_raises_the_write_flag` and `literal_head_with_a_computed_name_still_raises_the_write_flag`. Both pass. It is a second-pass MERGE candidate.

## Wrong answers in this ledger (27 tests)

| Cause | Tests | tclsh witness |
| --- | ---: | --- |
| `call-specialised-body-rewrite` | 7 | `helper`'s body is folded from its in-file caller (`helper 007`; `$mode eq "prod"`). main.tcl's `source lib.tcl; helper dev` prints 2 on 8.4 to 9.1. Same cause as dataflow; ruled 22:20 (the seed is dropped). |
| `wrong-evaluated-body-rename-not-replayed` | 3 | `namespace eval n {rename set saved_set}` makes `::n::saved_set`. `apply` (8.5+) and a `proc` built from a variable do the same. |
| `wrong-rename-deletion-keeps-builtin` | 1 | After `rename string {}`, `string toupper b` raises on 8.4 to 9.1. |
| `wrong-rename-target-opaque` | 1 | After `rename string mystr`, `mystr toupper b` prints B. |
| `wrong-rename-not-in-mutation-scan` | 1 | `proc p {} {rename expr saved_expr}; p`, then `expr {1+1}` raises. |
| `wrong-unknown-handler-alias-prefix-not-replayed` | 1 | An alias `unknown` → `rename expr` deletes `expr` when `maybe` falls to it. |
| `wrong-proc-trust-after-dynamic-rename` | 1 | `rename $command saved_expr` with command=add, then `add 2 4` raises. |
| `wrong-native-definition-after-proc-replaced` | 1 | After `proc` is replaced, `proc :audit …` defines nothing; `:audit` raises. |
| `wrong-declared-words-after-known-replacement` | 1 | After `rename check {}`, `check` raises; the product still gives it stub words. |
| `wrong-completed-world-after-unknown-head` | 1 | `unknown_before; …` raises at once, so no completed load world exists. |
| `wrong-handler-proved-in-unknown-frame` | 1 | Under an unknown frame, `puts` can be the caller namespace's own; `uplevel 1 {puts …}` from `::x` returns LOCAL. |
| `wrong-body-advice-under-missing-namespace` | 1 | `proc A::p …` with `::A` missing raises `unknown namespace`; the body never exists. |
| `wrong-w210-after-raising-unset` | 1 | The second `unset x` raises, so `puts $x` is never reached. |
| `wrong-callback-unset-not-counted` | 1 | `after 1 {unset u}; update; info exists u` gives 0. |
| `wrong-jim-entry-grammar-not-threaded` | 1 | jimsh reads `$café` whole; the Jim entry is lexed with the 8.6 config. |
| `wrong-unknown-command-braced-word-lifted` | 1 | `unknown {[dict size DATA]}` raises; `dict` never runs. |
| `wrong-package-callback-without-require` | 1 | `struct::graph` with no `package require` raises `invalid command name`. |
| `wrong-constructor-shape-after-unexport-new` | 1 | After `oo::objdefine C unexport new`, `C new` raises `unknown method "new"`. |
| `wrong-namespace-export-linkage-missing` | 1 | `namespace eval ::a {namespace export *}`: the export is imported and called from elsewhere. |

The `native_lowering` row on a dynamic trace target is not a wrong answer: probed, `incr a` lowers to a generic invoke, so the trace still fires.

## Ranked list: this ledger merged with what other areas routed here

Wrong answers come first, then missing facts by tests blocked (this ledger plus the routed rows). Fixes resume from the top. The owner is analyser-binding unless the row names another.

### Wrong answers

1. **A procedure body is analysed and rewritten from its callers' values.** `call-specialised-body-rewrite`: 7 here, 2 in dataflow. Dataflow removes the strict seed in `procedure_units.rs` (ruling 22:20). I then reduce `unit_scope`'s caller evidence to advice; the cross-file merge by implementation identity matters only if a strict reader is left.
2. **An unproved command in a procedure leaves its written variable undefined.** `binding-substitution-word-no-layout`, values 37. Its wrong member is an O101 fold of `[info exists x]` to 0 after `set x [expr {1/$d}]`, where tclsh prints no then yes. The fix: a statement whose words hold a substitution gets its declared layout, and an unproved command leaves its writes MayBound.
3. **Renames the binding misses or misreads**, 14 tests. `rename-not-in-mutation-scan` (dataflow 4), plus 10 here: the evaluated-body rename, deletion keeping the builtin, rename-target-opaque, mutation scan, unknown-handler alias, dynamic rename, replaced `proc`, and declared words after a replacement. One cause: the mutation scan and the CFG lattice must read the walk's move, delete and opaque-rename facts.
4. **`proc` into a namespace that does not exist is not an error** (class-facts walk_blockers, plus `wrong-body-advice-under-missing-namespace` here). Its mirror, a TclOO creation under a missing namespace withdrawing the world (class-facts, 7, a missing fact), is one fact: the namespace geometry of a creation. Class-facts writes the TclOO half; I write the `proc` half.
5. **The world is treated as complete or proved where tclsh stops or rebinds.**
   - `wrong-completed-world-after-unknown-head`
   - `wrong-handler-proved-in-unknown-frame`
   - `computed-head-qualified-literally` (analysis-performance): `::${ns}::setdef` is recorded as a candidate.
6. **Diagnostics and inventories after a raise.**
   - `wrong-w210-after-raising-unset`
   - `wrong-unknown-command-braced-word-lifted`
   - `wrong-package-callback-without-require`
   - `wrong-constructor-shape-after-unexport-new`
7. **Smaller single rows.**
   - `wrong-callback-unset-not-counted`
   - `wrong-jim-entry-grammar-not-threaded`
   - `wrong-namespace-export-linkage-missing`

### Missing facts, by tests blocked

| Rank | Cause | Tests | Source |
| ---: | --- | ---: | --- |
| 1 | Generic entry and entry-independent binding for bodies not entered. Here: `uncalled-body-generic-entry` 26, `entry-independent-binding-missing` 5, `unentered-site-has-reached-lookup` 7. Routed: dataflow's uncalled-procedure rows 29, class-facts' `uncalled-body-nested-proc-not-registered` 7 | 74 | here, dataflow, class-facts; design accepted 21:57 |
| 2 | Cross-file source evidence: W123 after a resolved `source` of an open sibling | ~28 | server-e2e-db (trace first: world or wiring) |
| 3 | Run-time compile admission under the VM's own entry | 20 | execution |
| 4 | Walk presence family (`walk-w123-presence` 8, `walk-unreached-site-presence` 4, `walk-loop-body-lookup` 4, `walk-computed-callee` 4, `walk-zero-trip-loop` 3), here `slot-presence-unknown` 4 and the parked `namespace unknown` fix | 27 | diagnostics, here |
| 5 | Lenient `tcl` profile records. Here: `lenient-nested-substitution-inventory` 10, `stub-declared-words-lenient-tcl` 5, `lenient-tcl-logical-*` and `logical-*` 32. Routed: dataflow 8, diagnostics `registry-lenient-profile-words` 6, values `binding-lenient-tcl-top-level` 5, lsp-features `param_traits` 2 | 68 | here, dataflow, diagnostics, values, lsp-features |
| 6 | Builtin not proved at its site, so a caller-frame-opaque barrier follows (top-level `switch`, `puts` in f5) | 20 | values (`binding-caller-frame-opaque-builtin` 10, `switch-not-lowered-structurally` 10) |
| 7 | `unseen-call-marker-withdraws-every-procedure` | 12 | here |
| 8 | Native-direct unit omits a procedure, or procedures vanish after a statement | 16 | diagnostics 14 (with dataflow), values 2 |
| 9 | F5 handler bodies: `f5-handler-command-unbound` 9, rule-loader inventory and admission 4, `rule_declaration` 4 (irules-tooling writes it under my review), the KEEP-F5 rows here 12 | 29 | values, irules-tooling, here |
| 10 | `oo-instance-w307` (Jim `class`, bodiless `oo::class create`) | 12 | diagnostics, class-facts |
| 11 | `loop-enumeration` 9 and `frame-script-body-unread` 9 | 18 | values |
| 12 | `callback-shape-not-read` | 9 | here |
| 13 | `wasm-native-lowering` | 9 | here |
| 14 | `aot-direct-call-trace-unproved` 6, `aot-*` admission 3 | 9 | here |
| 15 | Evaluated-body binding changes not replayed (missing half) 4, `walk-operands-withdrawn` 3, `multi-walk-body-operands-withdrawn` 1, held operand join (values' witnesses) | 8+ | here |
| 16 | `jim-source-class-declaration-missing` 5, `jim-*` 4 | 9 | here |
| 17 | `isolated-body-source-purpose-missing` | 5 | here; values moves those fixtures to whole modules, so this may close |
| 18 | Pack bodies: `pack-implementation-value-facts` 4, `pack-body-diagnostics` 1, `walk-drops-e003-after-pack-e002` 1 | 6 | irules-tooling |
| 19 | `binding-alias-invocation-metadata` 4, `binding-catch-result-variable-no-place` 3 | 7 | values |
| 20 | Smaller rows. Here: `native-body-template-refused` 4, `oo-destroy-deletion-unknown` 3, `conditional-original-child-refused` 3, `namespace-path-not-recorded` 2, `original-world-import-move-not-completed` 2 and the single-test causes above. Routed: `package-requires-collection` 2, `explorer-original-profile` 2, `registry-words-select-throw-under-84` 1 | — | here, irules-tooling |

The user-metaclass creation (class-facts, 15) is written by class-facts and reviewed here. The live-inlining design items (the `[incr n]` word proof, structured control in reference bodies) and the `source $dir` display-tier row follow the ranked fixes.
