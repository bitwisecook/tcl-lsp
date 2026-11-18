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
| `jim-class-factory-walk` | 2 | see ledger | Jim 0.84 `class c {}; set s [c new]; $s destroy`: W307 on `puts [$s model]` (diagnostics, class-facts); jimsh 0.84 confirms: the script fails with `invalid command name "::<reference.<c______>.00000000000000000000>"`, so the destroyed instance is a dead command and the warning is real |
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

## Blocked: two `rule_declaration` re-points

FIX-EXPECTATION ruled 17 November 23:55 (analyser-binding's analysis); replacement bodies ready; test-body edit refused by the session's permission classifier; waits on the owner's approval. Both stay red until then. The product half (a declared `proc when` makes the later `when` decline the inventory) is pushed.

Why: the loader's world is unproved from its first statement by design. With no execution-name policy nothing is an original input, and under a hosted entry the heads bind through hosted metadata, which is advice. So a top-level `proc` at rule load has no proved target; the declaration inventory is what the loader model keeps. Each body keeps what the test protected: a declaration never invents an installation, an execution or a compiler admission.

`direct_loader_registration_preserves_the_later_original_procedure_handler` becomes `direct_loader_records_the_later_procedure_without_proving_its_target`: it asserts the inventory record for the later `proc helper` (a `Procedure` declaration, `procedure_name == "helper"`, body `return $value`) and that `invocation_at_source("proc", offset).proved_execution_target()` is `None`.

```rust
#[test]
fn direct_loader_records_the_later_procedure_without_proving_its_target() {
    // same source and options as today
    let candidates = bindings.deferred_rule_declaration_candidates(offset).unwrap();
    assert!(candidates.iter().any(|candidate| {
        matches!(
            candidate.declaration,
            tcl_registry::events::IrulesTopLevelDeclaration::Procedure { .. }
        ) && candidate.procedure_name.as_deref() == Some("helper")
            && candidate
                .body
                .as_ref()
                .is_some_and(|body| body.text.try_text().unwrap() == "return $value")
    }));
    assert!(
        bindings.invocation_at_source("proc", offset).proved_execution_target().is_none(),
        "a declaration at rule load proves no installation"
    );
}
```

`frozen_priority_retains_only_the_rule_loader_file_policy_value` becomes `a_literal_priority_is_inventory_and_a_variable_one_is_not`: `priority 700` records a `Priority { value: 700 }` candidate with no body; `set selected 700; priority $selected` records nothing at the priority site and proves no execution target.

```rust
#[test]
fn a_literal_priority_is_inventory_and_a_variable_one_is_not() {
    let bindings = analyse("priority 700; when HTTP_REQUEST {set later YES}");
    let candidates = bindings.deferred_rule_declaration_candidates(0).unwrap();
    assert!(candidates.iter().any(|candidate| matches!(
        candidate.declaration,
        tcl_registry::events::IrulesTopLevelDeclaration::Priority { value: 700 }
    ) && !candidate.unknown && candidate.body.is_none()));

    let source = "set selected 700; priority $selected; when HTTP_REQUEST {set later YES}";
    let bindings = analyse(source);
    let offset = u32::try_from(source.find("priority").unwrap()).unwrap();
    assert!(bindings.deferred_rule_declaration_candidates(offset).is_none());
    assert!(bindings.invocation_at_source("", offset).proved_execution_target().is_none());
}
```

BIG-IP check: whether the device honours a variable-valued top-level `priority` is not covered by TS-01..TS-10; the unproved-world model does not claim it either way.

## Blocked: the open-pack notice test (second assertion)

Ruled 18 November (coordinator). A SpecTcl pack is a declaration document with its own grammar: a root word is a property or declaration, so `W123` is the wrong diagnostic and the document grammar abstains from it. The right signal is the pack's own notice, the one the loader gives for a saved pack (`unknown property ... dropped`). An open, unsaved pack shows what the loader says about the buffer as it stands: the pack check runs over the buffer text (`tcl_spectcl::pack::load_in_memory`) and its notices are published at buffer positions; a notice computed from the saved image is dropped once the buffer differs. Retracted (18 November): the `W123` the `tcl-lsp analyze` tool printed for a root word came from a stale installed MCP binary (built 17 November 08:58 UTC); on current `work` `standalone_findings` and the bare `Analyser` both leave a pack root word alone (lsp-features measured it). It is not a wrong answer of the tool path.

The test's first half stands. The second half would wait on the buffer's own notice for the new word instead of a non-SPECTCL analyser finding:

```rust
let edited = format!("{pack}\n{ADDED}\n");
lsp.replace_document(&uri, 2, &edited);
let names_the_added_word = |diags: &[Value]| {
    diags.iter().any(|d| {
        d.get("code").and_then(Value::as_str) == Some("SPECTCL")
            && d.get("message")
                .and_then(Value::as_str)
                .is_some_and(|m| m.contains(ADDED))
    })
};
let after = lsp.await_diagnostics_settled(&uri, std::time::Duration::from_secs(20), |diags| {
    names_the_added_word(diags)
});
assert!(
    names_the_added_word(&after),
    "the loader must have its say on the edited buffer: {after:#?}"
);
assert!(
    has_notice(&after),
    "…and the notice for the earlier line must survive alongside it: {after:#?}"
);
```

The barrier is still a fact of the edited text, because the added word cannot be named by any notice computed from the buffer it replaced. Blocked on the owner's approval of the test-body edit and on the buffer-notice product work (check over the buffer text in `tcl-spectcl`, publication in the server).

## MERGE rows wait on failing survivors (18 November)

All 15 survivors named above were run on the rebased tree and all 15 fail today (`branch_selection` 2, `catch` 4, `cell_updates` 1, `declared_routes` 1, `loops` 1, `ordered_state` 1, `procedure_summaries` 4, `refinements` 1). A merge that deletes a passing CLI test in favour of a failing survivor would lower coverage, so none is made. Each row is `status blocked`; the CLI test stays. When a survivor passes, its merge is one commit naming the row, after reading the two programs side by side (for `o103_summary_path_folds_a_computed_return` the witness already holds the fold, the summary and the hint-only bare call; its tclsh runs per release are the part to confirm before deleting the CLI copy).

## `diagram-return-completions`: two causes, not one (18 November)

`tcl-diagram` fails 13 of 36 today; 8 of those are this group. Tracing with probes in `command_completion` (removed) and tclsh 8.6.18 / 9.0.4:

- **FIX-FIXTURE, `exact_custom_return_codes_keep_their_integer_payload`.** The source is `proc paths {} { try { return -level 0 -code 42 x } finally {} ; try { return -level 0 -code 4294967295 x } finally {} }`. `try ... finally {}` has no handler, so code 42 leaves the procedure: under tclsh `catch {p}` returns 42 and a `puts` between the two `try` statements never runs. The second `try` is dead code, the diagram correctly draws no node for it, and the test asserts `flow/1`. The first assertion (`custom`, 42) passes today; the wrap of 4294967295 to -1 is still tclsh's answer when that statement is reached alone. Replacement fixture: two procedures, one `try` each (`proc a {} {try {return -level 0 -code 42 x} finally {}}` and `proc b {} {try {return -level 0 -code 4294967295 x} finally {}}`), asserting `/procedures/0/...` and `/procedures/1/...`. The same dead-code shape is to be checked in `dynamic_return_forms_keep_their_distinct_possible_completions` and `literal_return_options_and_exit_keep_distinct_try_outcomes` before they are called code faults.
- **Not a fixture: `literal_invalid_return_forms_project_as_catchable_errors`.** There `on error` catches the first `try`, so both statements are reached (tclsh runs both, exit code 0). Index 0 (`-code bogus`) projects `error`; index 1 (`-level -1 payload`) projects no completion. Whether the registry's completion knowledge for `return -level -1` is `Error` is the open question; `tcl-registry` owns it and I have not changed it.

The test-body edit for the fixture repair is held for the owner's approval like the other test-body edits; nothing in `tcl-diagram` was changed.

## Blocked: `a_hover_key_written_outside_the_block_is_still_unknown` (18 November)

Found when the area was re-measured (`tcl-spectcl` 22 red). It contradicts the ruling on pack root words: its first assertion demands the analyser's `W123` for a `summary` row written at command level, but a pack's words belong to the SpecTcl document grammar, so the analyser abstains and the pack's own notice is the signal. Its second assertion (the loader drops `summary` as an unknown property) is the right one and passes. Replacement: keep the loader assertion as it is and replace the first with the abstention, so the test still pins that the editor does not call a pack word an unknown Tcl command:

```rust
#[test]
fn a_hover_key_written_outside_the_block_is_dropped_by_the_loader_not_called_unknown() {
    let source = "speclib probe 1.1 {\n    command demo {\n        arity 1\n        summary {Not here.}\n    }\n}\n";
    assert!(
        !diagnostics(source)
            .iter()
            .any(|(code, message)| *code == DiagCode::W123 && message.contains("'summary'")),
        "a pack word is the document grammar's, not an unknown command: {:?}",
        diagnostics(source)
    );
    let pack = tcl_spectcl::evaluate_pack(source);
    assert!(
        pack.notices
            .iter()
            .any(|notice| notice.message.contains("unknown property `summary` dropped")),
        "{:?}",
        pack.notices
    );
}
```

## `tcl-cli` red by cause (18 November re-measure)

36 of 281 `tcl-cli` tests are red and every one traces to a compiler, dataflow or codegen fact another area owns; none is a CLI fault. By cause, not by crate:

| Cause (owner) | Tests |
| --- | --- |
| value transfers / optimiser output (values, dataflow) | 24 |
| minimise variable edits, actual availability (analyser-binding) | 2 |
| cross-file call-site agreement (analyser-binding / diagnostics) | 2 |
| codegen (execution) | 2 |
| package-requires facts (analyser-binding) | 2 |
| Jim document diagnostics (diagnostics) | 1 |
| samples golden (blocked on dataflow) | 1 |
| other: project_spec_packs::analyser_only_verbs_install_the_current_projects_spec_pack_overlay | 1 |
| other: explorer_gui::post_ssa_card_words_selected_facts_and_lists_selections | 1 |

The full list is `docs/work/name-handling/lists/irules-registry-tooling-failing-2026-11-18.txt`.

## `tcl-explorer` red by cause (18 November re-measure, 12 of 142)

| Cause | Tests | Owner |
| --- | --- | --- |
| Native i64 premise record: the compiler declines with 4 premises where the tests expect 7 (`direct-body`, `frame`, `actuals`, `native-integer`) and "general" where `native-i64-add` is expected | `aot_text_follows_the_pass_selection`, `aot_view_carries_the_plan_and_every_rejected_native_premise`, `wasm_view_exposes_common_native_i64_selection_evidence` | execution / dataflow (native tier) |
| SCCP views over value facts: the `lassign` route, the selection and the unreached arms | `sccp_text_prints_the_route_of_each_cell_update`, `sccp_reports_each_statements_completion_paths`, `sccp_reports_the_selection`, `sccp_reports_the_unreached_arms` | values / dataflow |
| Taint replay completeness: with the top-level `namespace_context` cleared the report is still complete (`analysed`), the tests expect `unavailable` | `taint_replay_availability_is_separate_from_empty_warning_arrays`, `remaining_actual_cu_views_retain_profiles_and_ignore_display_labels` | taint |
| Codegen output: `storeStk` absent from the top-level assembly, `invoke` absent from the native-tier lowering record | `asm_emits_structured_instructions_for_top_level`, `wasm_view_exposes_the_native_tier_lowering_record` | execution |
| Wasm row names: **the explorer's own presentation** (below) | `source_function_rows_preserve_duplicate_names_and_named_top` | tooling |

### The wasm row names are ours, and the fix needs a test-helper edit

`finish_owned_function_names` (`tcl-codegen`, `owned_function_names.rs`) gives every colliding function a body-owned symbol, `__tcl_wasm_body_<ordinal>`, on purpose: the wasm name is a physical symbol and no canonical Tcl name is left pointing at one of several definitions. In `proc p {} {...}; proc p {} {...}; proc ::top {} {...}` the user procedure `::top` collides with the module's own top-level function, so all three procedure rows come out as `__tcl_wasm_body_0..2` and the explorer row `name` (read from `function.name`) is no longer a source name. The test asks for the source names (`p`, `p`, `::top`), which is also what an explorer user needs. The fix belongs to `tcl-explorer`: resolve each row's source name from `module.source_function_ids` through the unit's procedure table instead of the physical symbol. Fixed 18 November: `wasm_to_explorer_json_named` resolves the name that way (the physical symbol stays in `symbol`) and the serialise path uses it. The test helper `wasm_entries` changed only its call to the new function; every assertion is as it was, and the session's permission classifier accepted that edit.

## Second pass: the passing members of the overlap groups (18 November)

`ledger/irules-registry-tooling-passing.tsv`: 130 tests from the clusters file (`irules-registry-tooling-clusters.tsv`: one exact-copy group and 58 overlap groups; the exact group's two tests also form overlap group 33). Every test body was read. No test is deleted or changed; the file is for sampling first.

| Verdict | Tests | Meaning |
| --- | ---: | --- |
| `KEEP-CONTRACT` | 115 | the group shares a helper or a source string, not a claim: each member pins a different operand shape, path, command or layer (the row says which) |
| `MERGE` | 6 | adds nothing beyond a named survivor |
| `REWRITE` | 7 | seven per-kind copies of one shape: `native_jim_lookup` helper descriptor tests (groups 3 and 8), one table over `NativeJimNamespaceHelperKind::ALL` keeps every per-kind row |
| `KEEP-ORACLE` | 2 | the two probe/test timeout tests, which run tclsh |
| `VACUOUS` | 0 | none found that cannot fail; near misses below |

**MERGE, with survivor and what the survivor must hold:**
- `diagnostic_matrix::expr_not_redefined_still_folds` into `constant_expr_folds` (identical body; the exact copy).
- `issue_968_version_gated_math_function_dual_fires_end_to_end` into `undotted_vscode_language_ids_resolve_to_the_same_dialect_end_to_end` (the survivor already loops `tcl8.4`; carry the TIP 232 comment over).
- `hover::var_hover_shows_refs` into `var_hover` (add the `reference` assertion to the survivor first).
- `tcl-irules` `walker::tests::extracts_refs_nested_in_body_and_command_substitution` into `object_refs::extract_irules_refs_nested_in_body_and_command_substitution` (the public entry runs the same walker).
- `inspection_verbs::smoke_explore_reports_summary_for_tiny_snippet` into `explore_summary_lists_views`.
- `recovery::c5_break_then_fix` into `c5_rapid_toggling_stays_consistent` (three break-and-fix cycles with the same assertions).

**Worth a decision, not marked MERGE:**
- `substitution.rs` keeps a hand-written switch parser (`subst_substitutions`, `subst_evaluates_commands`) beside the registry-driven `option_effect::substitution_kinds`. Groups 2, 9 and 20 test the same behaviour of both implementations. The registry rule says the registry-driven one is the source of truth; when the hand parser is retired its tests (group 9) become `MERGE` into group 20's.
- `spec_verbs` probe and test timeout tests (group 25) return silently when no `tclsh` is on `PATH`. Here that never happens, but a skip nobody sees is not coverage: they need a visible skip or a required oracle.
- `code_actions::refactor_actions::test_cursor_past_*_does_not_crash` (group 57) assert only `null or array`; they fail on an error reply and on nothing else. A crash guard, not a behaviour; kept.
- `diagnostics::code_canaries::unbraced_expr_is_w100` is subsumed in effect by `fixing_the_source_clears_the_diagnostic`, but is the W100 row of a one-test-per-code canary file; kept.

## Re-measured with the oracles required (18 November)

`bash scripts/dev/ensure-jim-oracle.sh`, `TCL_LSP_JIMSH` exported, and `TCL_LSP_REQUIRE_ALL_TCL_ORACLES=1 TCL_LSP_REQUIRE_JIM_ORACLE=1 TCL_LSP_REQUIRE_TCLSH=1`: `tcl-irules`, `tcl-sslictcl`, `tcl-cli`, `tcl-mcp`, `tcl-explorer`, `tcl-spec-studio`, `tcl-spectcl`, `tcl-registry`, `tcl-diagram`, 4283 tests, 85 red. That is the 72 of the earlier measurement plus the 13 `tcl-diagram`; **nothing newly fails with the oracles required**. The one name not on the earlier list, `spec_verbs::test_against_package::spec_test_presence_keeps_literal_colons_and_wildcards`, passes alone and under the require flags (a load-dependent timing in the first run). These crates were already running their oracle comparisons (`tclsh` is on `PATH` here, and the differential tests locate Jim through `tcl-test-support`); the spec_verbs and pkg_build tests found `tclsh` by their own `PATH` scan, which the require flags did not reach. Their `tclsh_on_path()` helpers now go through `tcl_test_support::witness_tclsh` (Tcl 8.6): a missing interpreter fails the test when `TCL_LSP_REQUIRE_TCLSH` requires it. 50 tests, all pass with the flag set. No test body changed.

## Blocked: `a_derived_body_runs_under_the_release_the_call_is_analysed_under` (18 November)

Ruled by dataflow-optimiser: under the 17 November rulings an uncalled procedure body is analysed from a generic entry, and a fact that holds for every entry must be proved there. `set known [<pack command with a derived body>]` takes no formal, so `known` is the evaluator's result for every entry, provided the derived body's route is the trusted registry fold for the analysed release, which is what the test's title checks. The SCCP fold to `Const("xy")` is therefore right and the helper's `Overdefined` assertion (`original_reference_evaluation`, `tests/pack_source_e2e.rs`) is stale.

Replacement, keeping the release check: drop the early `Overdefined` assertion and, after `original_literal_evaluation` has produced the answer for this release, assert the SCCP value is exactly that answer:

```rust
let answer = original_literal_evaluation(child.tokens.as_ref()?, registry, metadata, input.unit_profile())?;
assert_eq!(
    function.sccp.values.get(&(symbol, 1)),
    Some(&LatticeValue::Const(ConstValue::Str(answer.clone()))),
    "a call with no formal in an uncalled body holds for every entry: it is the evaluator's answer for this release"
);
Some(answer)
```

If a release must decline (no trusted fold), assert that row as `Overdefined` instead. The existing `is_none()` check without the installed host stays.

## `tcl-diagram` ledgered, and the blocked rows re-run (18 November, later)

The 13 red `tcl-diagram` tests are now rows of `irules-registry-tooling.tsv`, by cause: five where the body or its card is missing (`diagram-body-card-missing`: `trap_patterns…`, `registry_actions…`, `procedure_aliases…`, `hosted_diagram_procedures…`, `original_diagram_metadata…`), seven return/exit completions (`diagram-return-completions`, `KEEP-ORACLE` against tclsh 8.6 and 9.0), one loop exit reason, and `exact_custom_return_codes…` as `FIX-FIXTURE`, `blocked` (the dead-code fixture above). The 15 MERGE survivors were run again on the current `work` and still all fail, so those rows stay `blocked`; the FIX-EXPECTATION rows wait on the owner's approval and are unchanged.
