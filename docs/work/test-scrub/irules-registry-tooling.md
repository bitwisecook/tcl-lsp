# Test scrub: irules-registry-tooling

Ledger: [`ledger/irules-registry-tooling.tsv`](ledger/irules-registry-tooling.tsv), one row per failing test of the families on the tree of 17 November 2026, evening: 149 tests (123 in my crates by a clean run of the nextest suites of `tcl-cli`, `tcl-mcp`, `tcl-diagram`, `tcl-explorer`, `tcl-spectcl`, `tcl-spec-studio`, `tcl-irules`, `tcl-registry`, `tcl-sslictcl`, `f5-xc`, `bigip-report-gen-rust`, `tcl-irule-test`, `f5-cli`, `tcl-bigip`, `tcl-bigip-io`, `tcl-bigip-query`, `tcl-f5mku`; 26 in the server groups `irules`, `spec_packs`, `sslictcl`, `jim`, read from the evening e2e run, a few of which taint and server-e2e-db have fixed since). The one 1200 s test `codegen_stamps::every_shape_of_inlined_body_answers_as_its_definition_does_at_every_site` is excluded and belongs to live-inlining. Fixed today and therefore not in the ledger: `tcl-sslictcl`, `tcl-registry`, `tcl-bigip`, `tcl-spec-studio` (the pack formatting test, the coverage tests and the port comparison), the `tcl-spectcl` golden/formatting/bare-CR group and the three `spec_packs` document tests.

## Counts by verdict

| Verdict | Tests |
| --- | ---: |
| `KEEP-ORACLE` | 40 |
| `KEEP-F5` | 48 |
| `KEEP-CONTRACT` | 47 |
| `FIX-EXPECTATION` | 0 |
| `FIX-FIXTURE` | 1 |
| `MERGE` | 15 |
| `REWRITE` | 0 |
| `RETIRE` | 0 |
| `UNDECIDED` | 0 |

The two `UNDECIDED` rows are `tcl-spec-hooks` pack-family tests the execution session routed here (triaged: both `KEEP-CONTRACT`; the `::tclvm` variants pass, the `::runtime` variants fail because `RuntimeEngine` does not implement `Engine::host_metadata_list_reader`, so `word_list_in` in `tcl-spec-hooks/src/emit.rs` refuses the list-valued argument of `prefix` and `invalid` and nothing is emitted; the fix is in the runtime engine, execution's area), so the ledger has 151 rows. No test is retired or rewritten here. Triage evidence is the failing output read today plus tclsh 8.4/8.6/9.0 where the row says so; the oracle rows below were run on tclsh 8.4.20, 8.5.19, 8.6.18 and 9.0.4 and on jimsh 0.84 (`scripts/dev/ensure-jim-oracle.sh`); every run agrees with what its tests expect, so no row is `FIX-EXPECTATION`. `MERGE` rows are the 17 `value_transfers_cli` tests that repeat a compiler value-transfer witness through diag, opt and explore (ruling 6 of the earlier session keeps them while they fail); 15 of them name their survivor, a `tcl-compiler` `it_2` `value_transfer_witnesses` test of the same program (see `survivor`), and two stay (below).

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

## Oracle runs

One script per cause group, run on the releases named. Every result agrees with the expectation of the tests in the group.

**Numeric spellings and wide edges** (`optimiser-release-spellings`, `optimiser-jim-wide-int`, `optimiser-double-range`, `optimiser-hosted…` oracle side): `expr {-(-9223372036854775808)}`, `{(-9223372036854775807 - 1) / -1}`, `{(-9223372036854775807 - 1) % -1}`, `{4.9e-324}`, and `string is double` on `1.7976931348623157e308 0e999 0.0e-400 Inf`.
- tclsh 8.4.20: `-8`, `-8`, then the interpreter dies with SIGFPE on the `%` line; `4.9e-324` raises `floating-point value too large to represent`; the four `string is double` answers are 1.
- tclsh 8.5.19 and 9.0.4: `9223372036854775808`, `9223372036854775808`, `0`, `5e-324`, and 1 for each `string is double`.
- jimsh 0.84: `-9223372036854775808` for both wide expressions (it wraps), `4.94065645841e-324`, `1e999` is `Inf`, and the interpreter dies with SIGFPE on the `%` line.
So the folds are right only from 8.5 (the test's `folds` set), the 8.4 range-error literal must be left to run, and Jim must decline.

**Return completions** (`diagram-return-completions`), tclsh 8.6.18 and 9.0.4 (`catch` options need 8.6): `return -level 0 -code 42 x` completes with code 42; `-code 4294967295` completes with code **-1**; `return -code bogus` raises `bad completion code "bogus": must be ok, error, return, break, continue, or an integer`; `try {…} on 42 {…}` catches the custom code; `try {error E {} {X Y}} trap {X Y}` matches the list as elements. tclsh 8.4.20 has no `catch` options dictionary.

**`switch` arms** (`xc-switch-subject-lookup`), tclsh 8.6.18 and 9.0.4 (8.4 rejects the `{*}` in my script, not the semantics): `-glob` arms `/api/*`, `/static/*` falling through `-` to `*.css`, an exact `a.example.com`, `default`; `-regexp` `^/api/`; `-nocase -glob` on `WWW.EXAMPLE.COM` with `*.example.com`; and `/a*` as an exact (non-option) string without `-glob`. Each answers as the XC tests expect (A, CSS, CSS, EXACT, D, R, NC, LIT).

**Namespace call resolution** (`diagram-call-resolution`, `explorer-function-rows`), tclsh 8.4.20, 8.6.18, 9.0.4: in `::beta`, `helper` is `::beta::helper`, `alpha::helper` falls back to `::alpha::helper` (relative lookup finds it from the global namespace), `::alpha::helper` is direct; in `::unrelated`, `helper` is `invalid command name "helper"`.

**`throw`** (`registry-words-select-throw-under-84`): tclsh 8.4.20 `invalid command name "throw"`; 8.6.18 and 9.0.4 raise with the message.

**Rename planner** (`rename-planner-alias-capture`, `rename-planner-foreach-list`), tclsh 8.4.20, 8.6.18, 9.0.4: after `interp alias {} assign {} set fixed`, renaming only the use `$fixed` to `$a` raises `can't read "a": no such variable`; `foreach {first second}` and `foreach {a second}` run identically when both the definition and the uses rename.

**Jim documents** (`jim-document-diagnostics`, `jim-class-factory-walk`), jimsh 0.84: every program in the `jim_document.rs` tests runs without error. `proc counter {} {{n 0}} {return [incr n]}` counts 1, 2; `proc mix {a} {seed {n 0} &shared} {…}` returns 7; `proc pair {a b} {{n 0}} {…}` gives `120` and `wrong # args: should be "pair a b"` for 1 and 3 arguments; `proc {a b} {x} {…}` defines the two-word command (`{a b} 1` is 1, `{a c} 1` is `invalid command name "a c"`, `{a b}` is `wrong # args: should be "a b x"`); `class Point {x 0}` then `Point method clone {} {…}` and `[Point new]` work; the `JIM_PROGRAM` of the tests (class, method `proc {system model}`, `loop`, `sleep`, `set s [system new]`, `$s model`) prints `0 1 2` and an empty model; `system` alone is `invalid command name "system"` (the Expect command is unknown to Jim). tclsh 8.6 answers `invalid command name "class"`. So the findings those tests expect from the analyser (none for the Jim program, W123 for `system`) are what Jim itself shows.

## `value_transfers_cli`

15 rows are `MERGE` into a `tcl-compiler` `it_2` `value_transfer_witnesses` test of the same program (named in `survivor`; at merge time the survivor must be shown to assert the same value fact, which I have not compared line by line). Two stay as `KEEP-CONTRACT` because they hold what only the front end holds:
- `decided_branches::program_four_reaches_diag_and_opt_in_every_form`: runs the program under tclsh and checks that `diag` (the printed diagnostics and exit code) and `opt` (the optimised text) agree; the witness holds the analysis facts.
- `transfer_routes::explore_sccp_prints_the_route_tally`: the one test of the `explore sccp` text format (the tally lines and their order); `expressions::route_entries_are_counted_per_family` holds the counts.
The CLI adds the printed format, the exit code and the flags (`--profile`, `--dialect`, `--json`); the other 15 assert only the value fact the witness holds. `release_spellings` (4) and `ordered_state::opt_prints_what_tclsh_prints_for_the_seven_ordered_state_programs` are `KEEP-ORACLE`: they run the release themselves.
