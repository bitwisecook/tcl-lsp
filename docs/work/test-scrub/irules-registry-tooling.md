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

## `tcl-diagram`: the first wrong value, traced (18 November)

Probes in `tcl-diagram`'s `command_completion`, `walk_call` and `registry_command` (removed) on each failing test show where the diagram's value first differs.

- **Eight of the 13 are fixtures that never reach the code under test** (`docs/work/name-handling/fixing-guide.md`, "Fixture is not what it claims": a source that reads an unset variable). The unset-variable group is `dynamic_return_forms_keep_their_distinct_possible_completions`, `return_completion_preserves_source_word_shape`, `trap_patterns_are_projected_as_tcl_list_elements`, `loop_nodes_carry_their_compiler_exit_reason` and `exit_statuses_project_release_specific_errors_and_typed_dynamic_outcomes`: their procedures read `$code`, `$options`, `$status`, `$pattern`, `$ready` or `$items`, which no frame holds. tclsh 8.6.18 and 9.0.4: `proc p {} {try {return -code $code payload} finally {set cleaned yes}; puts second}`, `catch {p}` returns 1, message `can't read "code": no such variable`, `second` is never printed. So everything after the first dynamic word is dead code; the compiler gives those calls no `invocation_site()` (value at the point of loss: `tokens.source_binding.invocation_site()` is `None` for the second and later calls, `Some(offset)` for the first) and the diagram correctly draws nothing for them. Replacement: declare the variables as formals (`proc paths {code options status pattern} {...}`, `proc loops {ready items} {...}`), assertions unchanged.
- **Dead code after a certain exit** (`numeric_return_codes_project_as_their_canonical_completions`, `literal_return_options_and_exit_keep_distinct_try_outcomes`, `exact_custom_return_codes_keep_their_integer_payload`): statements follow a `try ... finally {}` whose body certainly leaves the procedure (a `return -code` break, error, custom code, or `exit`). Replacement: one `try` per procedure.
- **One compiler row, `literal_invalid_return_forms_project_as_catchable_errors`:** `on error` catches both statements, so both run in tclsh. The second `try { return -level -1 payload } on error {m o} {}` reaches the diagram as a plain `Call` of `try`, not a structured `Try`, so its body's `return` is never walked; registry knowledge for `return -level -1` is `Exact(Error)` on every release (`32ee6901`). First wrong value: the IR statement kind chosen by lowering for a `try` whose body holds an invalid `return`. Compiler lowering, not `tcl-diagram`.
- **`registry_actions_resolve_relative_names_in_the_callers_namespace` (f5-irules):** the diagram's data is `{"procedures":[{"bodyId":null,"bodyKey":null,"flow":null,"name":"::HTTP::caller","source_policy":"hosted"}]}`. Under a hosted entry bodies bind as advice and no body card is issued, so no flow is drawn; the test's claim (a relative `respond` in namespace `::HTTP` resolves to the registry action) is a name-resolution question for the device, BIGIP-NAME-002 and 003, and its expectation is to be re-pointed at the hosted policy when they are answered. `KEEP-F5`.
- **`procedure_aliases_and_rebound_registry_heads_stay_source_sensitive` (tcl8.6), compiler:** the data holds one procedure, `::library::helper` (`bodyId` and `flow` null); `proc error {args} {}` and `proc caller {...}`, declared after `interp alias {} invoke {} ::library::helper` and `rename error saved_error`, have no row at all. First wrong value: the declaration inventory the diagram reads (`unit.ir_module.procedures`) lacks them, i.e. the compiler issues no declaration or body card for procedures declared after the rebinding of a builtin head. analyser-binding's opaque-world-after-`rename` family.
- **Not yet traced:** `hosted_diagram_procedures_require_the_current_body_card` (`bodyId` null where the test says a current body card must be required: probably the same hosted policy, expectation direction to be read) and `original_diagram_metadata_keeps_actual_availability_and_source_currency` (a row reports `throw` where none is expected).

## Prepared for analyser-binding (18 November)

Reproducers, the first wrong value and the function that returns it. Nothing of analyser-binding's was changed.

**1. UPF witness (position 4): `tcl-spectcl` `it_5` `upf_hooks::the_analyser_enters_the_trailing_body_the_resolver_names`.**
- Smallest source, dialect `xilinx-eda-tcl`: `define_power_model PM {\n    upf_body_plain_missing\n}\n` (and `define_power_model PM2 -for {MODEL} {...}`).
- tclsh: not applicable, UPF is not Tcl; the pack's own tests pin the Body role.
- Registry is right: `arg_indices_for_role("define_power_model", ["PM","{x}"], Body)` is `[1]` and `["PM","-for","M","{x}"]` is `[3]` once the hook plan is published.
- Explorer: `tcl explore --source 'define_power_model PM {...}' --dialect xilinx-eda-tcl --show ir --text --no-colour` prints `barrier unsupported body command (define_power_model)` (kind `IRBarrier`). That is the first stage where the value is wrong: the statement is lowered to a `Statement::Barrier` and its body is never a script. `tcl diag` then prints `no diagnostics`.
- Function: `body_barrier` returned from `lowering/commands/default_call.rs:235`: `has_same_invocation_executable && tokens.evaluated_body().is_none()`. The registry names the Body (`authored_source_script_timing_at(index)` is `SameInvocation`), but `tokens.evaluated_body()` is `None`: the source walk never entered the resolver-derived Body word. Expected `Some(body)` for the trailing word.

**2. Arity item (position 5): `spec_packs::pack_commands::a_workspace_pack_subcommand_arity_error_is_published`.**
- Smallest source, pack `cad` (`subcommand box { arity 3 ... }`, `required_package cad`): `package require cad 0.4\n\ncad box 10 20\ncad box 1 2 3\nstring length one two\n`, tcl8.6 or tcl9.0.
- First wrong value: with the pack overlay the analysis holds only `E002` (pack arity, line 2); `E003` for `string length one two` (line 4) is absent. Without the pack the same text gives the `E003`.
- IR stage is right: `tcl explore --show ir` lists `call cad box 10 20`, `call cad box 1 2 3`, `call string length one two` as `IRCall`s, so the loss is after lowering, in the walk.
- Controls, measured in-tree on `work`: `proc f {a} {}\nf 1 2\nstring length one two` gives `E003` at 18 and 38; `string length a b\nstring length one two` gives `E003` at 16 and 36. A proc's or builtin's arity error does not suppress later `E003`; the pack's does.
- tclsh: an arity error aborts the script at run time in every case (`wrong # args`), which is why the proc and builtin controls are the reference for static arity diagnostics: they stay on later lines.

**3. `native_normal_store::native_parent_store_uses_the_actual_pack_implementation_normal_outcome` (position 8).**
- `rust/tcl-spectcl/tests/it_5/native_normal_store.rs:130`: `assert!(procedure.body.executed_source.is_none())` fails; the native parent's procedure body holds an `executed_source`. Run: `cargo nextest run -p tcl-spectcl --test it_5 -E 'test(native_parent_store_uses_the_actual_pack_implementation)'`. The first wrong value is that field of `procedure.body` (expected `None`); I have not traced which walk sets it.

**4. `project_spec_packs`: `tcl-cli` `cli::project_spec_packs::analyser_only_verbs_install_the_current_projects_spec_pack_overlay`.**
- Source: `package require tcl_lsp_fixture\n::tcl_lsp_fixture::collect output { proc nested {} { expr $input + 1 } }`. `tcl symbols` finds `nested`; `tcl find-legacy` returns no `W100` (`issues[0].code` null). Same family as item 1: the Body word of a pack command is entered by the symbol walk and not by the diagnostic walk; probably the same `body_barrier` at `default_call.rs:235` (not run on the project pack).

## Prepared for taint: `f5-xc` 10 (18 November)

The seven `switch_*` model tests, `event_shaped_procedure_names_do_not_select_response_event_semantics`, `diagnostics::tests::emitted_codes_are_catalogued` (XC101 pinned but never emitted) and `differential::rust_translator_matches_golden_fixture` share one first wrong value.

- Smallest source: `when HTTP_REQUEST {\n    switch [HTTP::path] {\n        "/a*" { pool star_pool }\n    }\n}`, dialect `f5-irules` (`tests/model.rs::switch_exact_path_with_star_is_literal`).
- Explorer, `tcl explore --source '<that rule>' --dialect f5-irules --show ir --text --no-colour`: the event body `::when::HTTP_REQUEST` holds `call switch [HTTP::path] ...` of kind `IRCall`. It is a plain call, not a `Statement::Switch`. The translator's `walk_statement` (`f5-xc/src/translator/ir_walk.rs:67`) only translates a `Statement::Switch`, and `SourceExpressionAdvice::command_for_operand` (`source_advice.rs:96`) is never reached for this rule (probe, removed): the route is lost before the subject lookup.
- First wrong value: the IR statement kind of the `switch` inside a hosted event body (`IRCall`; expected `Switch` with its arms), from the lowering of `switch`; with `switch` arms entered as scripts the translator then asks `registry_words_for_original_substitution` for `[HTTP::path]`. This is the same family as the UPF barrier (`lowering/commands/default_call.rs:235`: `tokens.evaluated_body()` is `None` when the walk does not enter the body words).
- Oracle for the test claims: tclsh 8.6.18 and 9.0.4 (ledger rows `KEEP-ORACLE`, cause `xc-switch-subject-lookup`): `switch -glob`, `-regexp`, `-nocase`, fall-through `-`, exact arms with `*` as a literal, `default`. Transcripts in the "Oracle runs" section above. The BIG-IP check for event-shaped procedure names is TS-07 (item 25).

## `tcl-irule-test`: 27 of 61 red, by refusing reader (18 November)

Every one is a refusal of the Rust VM under the `f5-irules` profile, and every refusal is one registry or dialect reader that returns nothing for an F5 point, deliberately (no C recipe is selected for F5 without a BIG-IP measurement, `docs/work/name-handling/bigip-name-checks.md` §1).

| Count | Refusal | Reader that returns nothing | What decides it |
| ---: | --- | --- | --- |
| 22 | `ValueAccessRefusal(StringAccess(Unavailable(ProtocolUnavailable)))` | `NativeStringProtocol::for_point` (`tcl-syntax/src/native_string.rs`) via `CommandRegistry`'s `native_string_protocol` (`tcl-registry/src/native_string_materialization.rs:89`): `None` for an F5 point | BIG-IP string model: open question 11 and TS-05 (`measured_counted_lengths…`, `authored_functions_keep_independent…`) in `bigip-name-checks.md` |
| 2 | `CommandProtocolUnavailable("native string cache origin")` (`irule_source_rewrite_only_touches_command_positions`, `scf_loader_stores_parsed_snatpool_members`) | the same protocol, read at `tcl-vm/src/value/native_string.rs:851` and `:970` | same |
| 1 | `CommandProtocolUnavailable("coroutine publication name purpose")` (`measured_event_names_enter_the_actual_connection_frame_and_restore_policy`) | the name protocol, `NativeNameProtocol::for_point` (`tcl-syntax/src/naming/native.rs`), read at `tcl-vm/src/interp/native_coroutine_names.rs:51`: `None` for F5 | the F5 naming checks BIGIP-NAME-001 to 073 (`tmsh`, TMM) |
| 1 | `HostCommandRefusal("list-assignment invocation grammar is not selected")` (`harness_runs_the_tmm_84_surface`) | `list_assignment_invocation` (`tcl-registry/src/native_list_assignment.rs:69`): Tcl 8.5, 8.6, 9.x and Jim only | new check 32 in `bigip-name-checks.md` (`lassign` in TMM) |
| 1 | value mismatch, `77` against `63` (`authored_functions_keep_independent_worker_random_and_precision_contexts`) | not a refusal: the numeric reading of `077` inside an authored function context | TS-05 (`expr {abs(077)}`): tclsh 8.4 reads `077` as octal, 63 |

So 25 of the 27 wait on one decision, whether an F5 point may select the Tcl 8.4 string and name protocols (the rule "F5 Tcl is Tcl 8.4 unless a measurement says otherwise" points that way), and that decision is the owner's with the answers to open question 11 and TS-05. Until then these tests keep failing by design: they hold iRules behaviour that only the device settles (ledger causes `KEEP-F5`, checks `TS-01` to `TS-05`).
