# Test scrub: irules-registry-tooling

Ledger: [`ledger/irules-registry-tooling.tsv`](ledger/irules-registry-tooling.tsv), one row per failing test of the families on the tree of 17 November 2026, evening: 149 tests (123 in my crates by a clean run of the nextest suites of `tcl-cli`, `tcl-mcp`, `tcl-diagram`, `tcl-explorer`, `tcl-spectcl`, `tcl-spec-studio`, `tcl-irules`, `tcl-registry`, `tcl-sslictcl`, `f5-xc`, `bigip-report-gen-rust`, `tcl-irule-test`, `f5-cli`, `tcl-bigip`, `tcl-bigip-io`, `tcl-bigip-query`, `tcl-f5mku`; 26 in the server groups `irules`, `spec_packs`, `sslictcl`, `jim`, read from the evening e2e run, a few of which taint and server-e2e-db have fixed since). The one 1200 s test `codegen_stamps::every_shape_of_inlined_body_answers_as_its_definition_does_at_every_site` is excluded and belongs to live-inlining. Fixed today and therefore not in the ledger: `tcl-sslictcl`, `tcl-registry`, `tcl-bigip`, `tcl-spec-studio` (the pack formatting test, the coverage tests and the port comparison), the `tcl-spectcl` golden/formatting/bare-CR group and the three `spec_packs` document tests.

## Counts by verdict

| Verdict | Tests |
| --- | ---: |
| `KEEP-ORACLE` | 40 |
| `KEEP-F5` | 48 |
| `KEEP-CONTRACT` | 43 |
| `FIX-EXPECTATION` | 0 |
| `FIX-FIXTURE` | 1 |
| `MERGE` | 17 |
| `REWRITE` | 0 |
| `RETIRE` | 0 |
| `UNDECIDED` | 2 |

The two `UNDECIDED` rows are `tcl-spec-hooks` pack-family tests the execution session routed here (they hold a pack-family runtime path; I have not triaged them yet), so the ledger has 151 rows. No test is retired or rewritten here. Triage evidence is the failing output read today plus tclsh 8.4/8.6/9.0 where the row says so; **Jim rows are not confirmed on jimsh (none installed here)**, and the oracle rows state what the test asserts, not that I re-ran each expected value on tclsh: the ones that are, name it. `MERGE` rows are the 17 `value_transfers_cli` tests that repeat a compiler value-transfer witness through diag, opt and explore (ruling 6 of the earlier session keeps them while they fail); their survivor is the compiler test of the same program, to be named per row when that session's ledger lists it.

## Causes ranked by tests blocked

| Cause | Tests | Owner | Distinct failing behaviour |
| --- | ---: | --- | --- |
| `lassign-host-refusal` | 29 | execution | The iRules simulator (LiveSession) runs its TMM surface on a C9 host engine; `lassign` is refused because the VM takes the invocation dialect from the f5-irules profile. |
| `sccp-overdefined-values` | 18 | dataflow-optimiser | SCCP leaves values overdefined where tclsh decides them (catch results, name-argument writes, summaries, loop conditions), seen through diag, opt and explore. |
| `irule-quick-fix-actions` | 14 | taint (actions), server-e2e-db | iRules quick fixes (html/uri encode procs, CRLF strip, regex quote, static:: replacement) are not offered for the diagnostic the server publishes. |
| `xc-switch-subject-lookup` | 10 | taint (hosted lookup) | The XC translator finds no registry words for an unbraced `switch` subject in a handler, so no route is produced. |
| `jim-document-diagnostics` | 10 | diagnostics, class-facts | A Jim program (class factory objects, proc statics, two-element proc names, Expect commands) draws diagnostics tclsh/jimsh would not. |
| `diagram-return-completions` | 8 | dataflow-optimiser, values | Diagram projections of return/exit/try completion codes and trap patterns. |
| `sccp-selection-output` | 7 | dataflow-optimiser | Explorer and CLI text for the SCCP/AOT selection views. |
| `wasm-native-lowering` | 5 | execution | The native tier emits no i64.add / storeStk / lowering record. |
| `live-reference-inlining-off` | 5 | live-inlining | Pack reference bodies are not spliced (switched off by ruling 97e7f8d5). |
| `server-pack-latency-and-index` | 5 | server-e2e-db | Closed pack-extension file index, generation memo, notices while editing, bundled UPF resolver latency. |
| `optimiser-release-spellings` | 4 | dataflow-optimiser | Literal spellings per release (int64 min, doubles on 8.4 and Jim, no integer pow under Jim, operators the release lacks). |
| `pack-implementation-value-facts` | 4 | values, analyser-binding | A pack reference body donates an executable stored value for a declaration the analysis never entered. |
| `irule-s110-roundtrips` | 3 | taint | S110 payload round-trip warnings per protocol are not published. |
| `rule-object-walker-reachability` | 2 | taint (hosted lookup), me (walker) | Rule-to-object reachability proves nothing under the hosted source (everything "possible", nothing "confirmed"), and a handler constant is not propagated. |
| `jim-class-factory-walk` | 2 | see ledger | Jim 0.84 `class c {}; set s [c new]; $s destroy`: W307 on `puts [$s model]` (diagnostics, class-facts); jimsh not installed here to confirm |
| `package-requires-collection` | 2 | see ledger | `package require $dep $minimum` with proc-local constants not listed; literal Tk in `if` has no name (analyser-binding); docs/kcs pkg discov |
| `diagram-call-resolution` | 2 | see ledger | name resolution of calls in the caller namespace and alias heads: tclsh namespace resolution (name-handling contract) |
| `explorer-original-profile` | 2 | see ledger | taintReplays/profile retention for the original source (analyser-binding) |
| `irule-completion-hover` | 2 | see ledger | editor behaviour on iRules commands (completion ranking, wrapping braced variables) |
| `rename-planner-foreach-list` | 1 | see ledger | `foreach {first second}` is a variable-definition list; renaming first to a must keep the program meaning on tclsh 8.6/9.0 (lsp-features) |
| `rename-planner-alias-capture` | 1 | see ledger | tclsh 9.0: `list $a` after the alias `set fixed` raises can't read "a"; both uses and the alias-captured definition must rename together (ls |
| `optimiser-hosted-word-operator-fold` | 1 | see ledger | F5 word operators (eq/and) fold only under TMM rules; CLI dialect detection adds the no-flag path [check TS-04] |
| `optimiser-jim-wide-int` | 1 | see ledger | Jim `puts -9223372036854775808` literal; tclsh 8.6/9.0 compute wide edges (dataflow) |
| `optimiser-double-range` | 1 | see ledger | tclsh 8.4: `expr {4.9e-324}` is a range error literal, 8.5+ folds (dataflow) |
| `samples-golden-stale` | 1 | see ledger | samples/optimiser outputs follow the optimiser; regenerate per samples/optimiser/README.md after dataflow's rewrites land (dataflow said che |
| `pack-body-diagnostics` | 1 | see ledger | find-legacy W100 inside a pack command's Body-role argument (docs/design/registry/spec-packs.md); analyser-binding |
| `hosted-body-card` | 1 | see ledger | hosted iRule proc has no body unit so bodyId is null (taint/values: hosted declaration body units) [check TS-07] |
| `registry-words-select-throw-under-84` | 1 | see ledger | tclsh 8.4 has no `throw`; registry_words_for_original_call selects it under an 8.4 availability (analyser-binding) |
| `diagram-loop-exit-reason` | 1 | see ledger | third exit reason `exhausted` missing; docs/design diagram contract (compiler exit reason) |
| `explorer-function-rows` | 1 | see ledger | two procs of the same name in different namespaces are two rows and the top frame is `::top` (tclsh namespace rules) |
| `walker-handler-constant-propagation` | 1 | see ledger | golden row: `set p /Common/api_pool; pool $p` expects one pool reference; dataflow constant through a handler variable (message to dataflow- |
| `irule4002-not-published` | 1 | see ledger | review of `when RULE_INIT { set static::debug 0 }` returns [] where IRULE4002 expected; taint family [check TS-09] |
| `spectcl-unknown-property-diagnostic` | 1 | see ledger | W123 for a `summary` row written at command level: docs/design/registry/spec-packs.md (hover keys only inside the hover block); the loader a |
| `walk-drops-e003-after-pack-e002` | 1 | see ledger | a built-in E003 after a proved pack E002 (analyser-binding) |

## Distinct failing behaviours worth fixing

Weighted by tests blocked. The first is one cause for 29 tests; fixing it also unblocks the F5 simulator for `f5-cli`.

1. **lassign-host-refusal** (29): The iRules simulator (LiveSession) runs its TMM surface on a C9 host engine; `lassign` is refused because the VM takes the invocation dialect from the f5-irules profile.
2. **sccp-overdefined-values** (18): SCCP leaves values overdefined where tclsh decides them (catch results, name-argument writes, summaries, loop conditions), seen through diag, opt and explore.
3. **irule-quick-fix-actions** (14): iRules quick fixes (html/uri encode procs, CRLF strip, regex quote, static:: replacement) are not offered for the diagnostic the server publishes.
4. **xc-switch-subject-lookup** (10): The XC translator finds no registry words for an unbraced `switch` subject in a handler, so no route is produced.
5. **jim-document-diagnostics** (10): A Jim program (class factory objects, proc statics, two-element proc names, Expect commands) draws diagnostics tclsh/jimsh would not.
6. **diagram-return-completions** (8): Diagram projections of return/exit/try completion codes and trap patterns.
7. **sccp-selection-output** (7): Explorer and CLI text for the SCCP/AOT selection views.
8. **wasm-native-lowering** (5): The native tier emits no i64.add / storeStk / lowering record.
9. **live-reference-inlining-off** (5): Pack reference bodies are not spliced (switched off by ruling 97e7f8d5).
10. **server-pack-latency-and-index** (5): Closed pack-extension file index, generation memo, notices while editing, bundled UPF resolver latency.
11. **optimiser-release-spellings** (4): Literal spellings per release (int64 min, doubles on 8.4 and Jim, no integer pow under Jim, operators the release lacks).
12. **pack-implementation-value-facts** (4): A pack reference body donates an executable stored value for a declaration the analysis never entered.

The remaining one-test causes are in the ledger (rename planner, optimiser spellings, package requires, hosted body cards, registry words under 8.4, the `samples` golden, the W123 unknown-property diagnostic).

## BIG-IP checks

48 rows are `KEEP-F5`; every `KEEP-F5` row names its check `TS-01` to `TS-10`, written as items 19 to 28 of [`bigip-name-checks.md`](../name-handling/bigip-name-checks.md) (what to run on a BIG-IP, what answer confirms the test). `TS-06` also covers the loader contract rows.

## Exact and overlap groups touching these families

`ledger/irules-registry-tooling-clusters.tsv` lists every member (passing tests included, members in other crates too) of the groups in `data/clusters_exact.tsv` and `data/clusters_overlap.tsv` that contain a test of these crates: 1 exact group and 58 overlap groups. Within them: `tcl-explorer` `serialise::tests::irules_flow_empty_for_plain_tcl` and `event_order_empty_for_plain_tcl` are the same check on two views; `tcl-cli` `inspection_verbs::explore_summary_lists_views` and `smoke_explore_reports_summary_for_tiny_snippet` repeat each other (the smoke name is kept on purpose: the Makefile's smoke tier selects `smoke_*`); `tcl-irules` `walker::tests::extracts_refs_nested_in_body_and_command_substitution` repeats `tests/object_refs.rs`; `tcl-spectcl` export round-trip pairs (`a_canonical_environment_block_round_trips_byte_identically` and `an_environment_block_with_kind_and_short_name_round_trips_byte_identically`) differ by one input and are a table. These are second-pass work; none of them fails.
