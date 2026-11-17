# Hand-over: analyser diagnostics

Outgoing session `session_017u5g3GuH17itfsbWe5t3Md`, 17 November 2026. Last code commit on `work`: `2afecd39`. Nothing is held back as code; the tree was clean when this was written. The area's running notes are in the "Analyser diagnostics area" section of [current-state.md](../current-state.md); this file is what a successor needs first.

## Scope

Owns, in `rust/tcl-compiler`:

- `src/analyser/diagnostics/**` (emitters and their tests), `src/analyser/handlers/**`, `src/analyser/param_traits/**`, `src/analyser/types/**`.
- Integration tests that assert diagnostics: `tests/it_4` `analyser::*`, `fix_safety`, `span_char_boundaries`; `tests/it_5` `checks::*`; `tests/it_3` `fp_depth::*`.

Does not own: the binding walk, realm, `command_binding/**`, `registry_invocation/**` (`analyser-binding`); CFG, SSA, SCCP, lowering, optimiser (`dataflow-optimiser`); taint and iRules checks (`taint`); the existence and value domains (`values`); class and object facts (`class-facts`). `it_4 analyser::class_factories` (about 50 failing) was left to `analyser-binding` throughout, although it sits under this area's test prefix.

Two small changes were made outside the area by agreement: `SourceCommandBindings::frozen_written_word_value_at_span_in_source_with` (`src/command_binding/original_name_value/span_queries.rs`) and the `OptionPlacement` count in `tcl-registry` (`InvocationOptions::placement`, `count_invocation_argument_layout`).

## How to measure

```bash
source scripts/dev/agent-build-env.sh
export CARGO_BUILD_JOBS=2 CARGO_PROFILE_TEST_OPT_LEVEL=1 RUST_BACKTRACE=0
# edit loop: the compiler's library tests only
cargo nextest run -p tcl-compiler --lib --no-fail-fast --failure-output final -E 'test(/<name>/)'
# before a push: both suites, whole
cargo nextest run -p tcl-compiler -p tcl-irules --no-fail-fast --status-level fail --final-status-level none --failure-output never > full.log 2>&1
grep -E "^\s+(FAIL|TIMEOUT|SIGABRT|SIGSEGV)" full.log | sed -E 's/^\s+\S+ \[[^]]*\] \([^)]*\) //' | sort -u > failing.txt
comm -13 baseline.txt failing.txt    # new failures
comm -23 baseline.txt failing.txt    # fixed
```

Measured on the 4-core, 15 GB container: the wide run is about 12 minutes of build after a library edit plus 8 to 12 minutes of tests (about 13,230 tests). A library edit in `tcl-compiler` rebuilds in 7 to 10 minutes even with `--lib`; an edit to a test file alone rebuilds in under a minute. No test was excluded; nothing in these two packages reached the 1,200-second limit.

The baseline list below predates the file split, so its test paths lack the module segment the split added (`tests::x` is now `tests::<module>::x`). Compare by the last path segment: `sed -E 's/.*:://'` on both lists before `comm`.

Gates before a push: `cargo fmt --all -- --check`; `CARGO_PROFILE_TEST_INCREMENTAL=false cargo clippy -p tcl-compiler --all-targets -- -D warnings -A clippy::assert_is_empty` (add `-p tcl-registry` after a registry edit); `cargo xtask registry-axes`, `value-transfers --check`, `owner-resolution`, `callback-inventory --check`; `make xtask-gates` after touching `tcl-registry`.

## Current numbers

Last full run: 16 November 2026, on `836282e8` plus the W100 fix-safety change (the content of `2afecd39` before its final rebase): 13,233 run, 2,248 failing, 426 of them under this area's prefixes (479 at the 15 November baseline of 2,487). One of the 2,248, `original_expression_diagnostics_keep_effective_roles_and_whole_written_words`, was corrected after that run and passes. Not re-measured on the restructured tree.

This session fixed 42 tests that were failing at its baseline and broke none; the rest of the drop from 2,487 is the other areas' work.

## Baseline list

[`../lists/diagnostics-failing-2026-11-16.txt`](../lists/diagnostics-failing-2026-11-16.txt): the 2,248 failing tests of that run, both packages, one per line as `<binary> <test path>`, pre-split paths.

## Queue

In order. Paths are on the restructured tree.

1. **Recovered tail, prefix-declared procedures** (ruled, not started). A procedure the reached prefix declares and does not rebind is checked for arity in the tail. `src/analyser/diagnostics/recovered_tail.rs`: `recovered_tail_head_is_rebound` currently refuses any head in `all_procs`; take the arity from the `ProcDef` instead when the name has one declaration before the cut and no rename, alias, deletion or import touches it. Fixture: `proc my_helper {x} {puts $x}\n\nset q {\n  aaa\nmy_helper\n` expects E002 on `my_helper`. Turns green the server tests `recovery::user_defined_proc_recovers_the_tail_like_a_builtin` and its namespace-qualified twin (the pass skips qualified heads today; decide that with the fixture). Add rows to `a_recovered_tail_draws_registry_count_errors_and_nothing_else`.
2. **Recovered tail, unknown subcommand and unknown option.** Only arity (E002, E003) and missing subcommand (E001) are reported. Same file; `resolve_call` gives the subcommand resolution.
3. **`unit_scope::tests::agreeing_callers_still_seed_through_a_substitution`**: accepted regression from the `return`/`set` selection patch (`92c87dfc`), assigned to this area; a precision loss, not a wrong answer. Not looked at.
4. **W210 withheld after a user-procedure call** (server e2e report, cause 1). Control that works: `proc h {a b} {}\nproc build {} {\n    h itemLegend1 tree1\n    return $neverSet\n}`. Failing: the same with `proc estruct {name value} {upvar 1 $name obj; set obj $value}` as the callee (expected one W210 on `neverSet`); `proc setInCaller {var} {set $var 99}` then `return $answer` (expected W210, only W212 today). Start in `src/analyser/diagnostics/dataflow/read_before_set.rs` (`definite_missing_read_spans`, `record_chain_w210_uses`) with `TCL_LSP_TRACE_RBS_SOURCE_OWNER=1`, which prints the stage that keeps or drops each read.
5. **First-iteration read in a loop body**: `proc f {items} {foreach i $items {puts $r; lappend r $i}}` reports nothing (`w210_first_iteration_read_in_a_loop_body_fires`, in `tests/dataflow/`). Probably the same loop-body state as item 1 of Hand-offs.
6. **`for {set i 5} {$i<3} {incr i} {set y $i}; puts $y`** draws no W210 although the loop provably runs zero times (`fp_rbs_18_guaranteed_for_defines_body_vars`): the after-loop read has no SSA read reference.
7. **`info_exists_*` (14 tests, `tests/dataflow/existence_folds.rs`)**: I230 folds asked through `native_existence_result` (a captured native entry in direct compilation mode) report nothing, while `Analyser::analyse` of the same source under `tcl8.6` folds. Not decided whether the fixture or the code is wrong.
8. **Regex pattern through a variable** (`it_4 regex_patterns`, 5): `set str abc; set pat {^\d+$}; regexp $pat $str` records nothing. `Analyser::frozen_value_word` (`src/analyser/scope/constant_values.rs`) can supply the value on a release profile; the record also wants the defining literal's span, which it does not carry.
9. **Remaining bare-handler fixtures** in `src/analyser/handlers/tests/`: `interp_create_*` (interpreter state), `handle_interp_alias_*` (target recorded as `::set`, test expects `set`: decide which), `handle_namespace_path_records_a_braced_dollar_bearing_word`, `handle_namespace_import_consumes_only_one_force_flag` (tclsh raises `no namespace specified in import pattern "-force"` and imports nothing; neither the code nor the expectation matches that).
10. **Literal `interp eval child {proc foo {} {}}`** records no `::@interp@child::foo` under `tcl8.6` (it does under `tcl`): the site is a reached dispatch with no retained words, so the unreached-site fallback in `Analyser::procedure_words_for_analyser` (`src/analyser/handlers/procedures.rs`) does not apply. About eight `interp_*` and `safe_interp_*` tests.
11. **Not traced**: `it_4 unresolved_command` (16), `it_4 report_scoped_commands` (7), the four `w120_*` tests (`tcl::idna decode a` under `tcl9.0` draws W123 where W120 is wanted), vendor-profile gating (server e2e cause 3: `tcl-iapp`, `tcl-tmsh` publish nothing; `exec` in an iRule draws W123 where W002 is wanted), the `original_*_advice` alias-ordinal tests, most of `fp`.
12. **Superlinear analysis time in nesting depth** (server e2e cause 5): `if {1} {` nested N deep, `tcl8.6`: N=10 122 ms, 20 290 ms, 40 1,147 ms, 80 2,232 ms, nearly all in the cfg-ssa diagnostic stage. Not looked at; may belong to `analysis-performance`.

## Held work

- `../patches/handover-diagnostics-probe.patch`: not a fix. A file-driven probe test (`zz_probe_file`) appended to `src/analyser/diagnostics/tests.rs` as it was at `836282e8` (pre-split; on the restructured tree paste the test into any `tests/*.rs` module). It reads Tcl sources from `$ZZ_SRC` (cases separated by a line `----`), analyses each under every dialect in `$ZZ_DIALECT` and prints the diagnostics, procedures, scopes, renames, regex records and the merged `fp::codes` list; `ZZ_NATIVE=1` uses `native_existence_result`. Because it is a test-file edit it rebuilds in under a minute, and each new question after that costs no rebuild. Never committed: it trips `clippy` only by design choice (it is a scratch test) and must not be pushed.

No other held work: no stash, no side branch, no local commit.

## Open questions and rulings

Rulings this area works under (coordinator, 15 to 17 November):

- Display consumers may fall back to written words plus the registry; proof consumers stay strict. A diagnostic at a site the interpretation never reached reads the written word and the registry; at a reached but unproved site it stays silent unless tclsh confirms it for every binding the document could produce.
- A read after a possibly-empty loop is silent; only a provably empty collection warns (`w210_after_a_loop_follows_what_the_collection_proves`).
- A standalone document's global frame is open for W213 and W210, except for what the document itself proves (`unset_of_an_unbound_name_reports_where_the_document_proves_it`). W210 does not follow this yet: `puts $nope` at file level still fires, and many fixtures rely on it.
- The unreached-site registry fallback for `proc` is not widened in general (`a_dollar_named_procs_body_has_no_proven_home` stays).
- The recovered tail is an editor tier with registry-only count errors, in a separate pass that reads no fact of the walk; a prefix-declared procedure that is not rebound is checked for arity there.

Asked and unanswered:

- Whether file-level W210 should go silent now (a wide expectation change).
- Whether to keep `7a1d1983`, which records a procedure declared in the recovered tail of an unparsable document through the same fallback; the LSP side is building its own tail outline from written words.

## Hand-offs

Sent, waiting:

- `analyser-binding`: loop-body lookup state (`proc p {items body} {foreach item {a b} {uplevel 1 $body}}` under `tcl9.0` gives no `Eval` trait; 11 `param_traits` tests); declared-absence W210 after `foreach x $i {set y $x}` (`fp_rbs_17_guaranteed_foreach_defines_body_vars`); arity checked against the builtin where a namespaced user procedure is dispatched (`e003_shadow_is_namespace_scoped`, two `same_file_relative_qualified_call_*`); method bodies reporting no read-before-set (the `*_method_body_*` tests; `class-facts` has since landed part of this).
- `taint`: W201 reports nothing for `proc f {dir} {set path "$dir/file.txt"; return $path}`; the four fixtures also read an unset variable at file level and need the parameter form once the check reports.
- `values`: answered and fixed (`6983b2a5`, `bc044f8c`); `a_second_unset_is_a_definite_w213` passes.
- `irules-registry-tooling`: told of the `OptionPlacement` count change; `socket -server accept` drawing no E002 (the port is required) is a registry arity row, not sent as a fixture yet.

Received, not started: the server e2e session's causes 1, 2, 3 and 5 (items 4, 11 and 12 above; cause 2 is a list of structural checks that publish nothing: E005 unpaired switch pattern, execution-trace callback arity, W128 after `destroy`, W123 beside a created or imported name, W216 lost to W210 for `${arr($i)}`, I230 message spelling).

## Hazards

- A shallow fetch can cut history exactly at your base and make `git rebase origin/work` replay other people's commits. Check `git merge-base --is-ancestor <your base> origin/work` first and deepen (`--depth=200`) if it fails.
- Two concurrent or wide builds were killed for memory twice (`signal 9`, once restarting the container). Keep `CARGO_BUILD_JOBS=2` for the two-package selection and never edit a source file while a build of it is running.
- Changing the `-p` selection rebuilds most of the tree and leaves a second set of incremental caches; disk went to 6 GB free once. Remove the stale `target/debug/incremental/<crate>-*` directories of the selection you dropped.
- Background commands were given a 10-minute limit and killed mid-build twice; give a build its own long limit.
- Guessing which emitter produced a diagnostic cost three rebuilds. `TCL_LSP_TRACE_RBS_SOURCE_OWNER=1` (W210, W213) and `TCL_LSP_TRACE_JIM_SOURCE_OWNER=1` (source words, handler refusals) print the deciding stage without a rebuild.
- A fixture that reads an unset variable, calls a package command without `package require`, or runs under the generic `tcl` profile (`Analyser::new()` with `emit_document_cfg_ssa_diagnostics_for_test`, `codes_for`) proves nothing about a release. Several "failures" were only that.
- Some sources are analysed once per `(source, dialect)` and cached within a test process, so a debug print can appear fewer times than the calls.
- The segmenter does not always split a tail command off after a syntax cut (`set x [foo bar\n$cmd` stays one command); key recovery logic on the cut, not on `is_partial`.
- `owner-resolution` rejects a hand-built `::name`; use `crate::naming::normalise_qualified_name`. `clippy::too_many_lines` trips at 100 lines: extract a helper rather than grow a pass.
- A new failing name after splitting or renaming a test is not a regression; say so in the report with the old name.

## Oracle habits

`tclsh8.4`, `tclsh8.5`, `tclsh8.6`, `tclsh9.0` and `tclsh9.1` are on `PATH` (`/root/.local/bin`); `jimsh` is not installed in this container, so no Jim expectation was checked here. One line per release:

```bash
for v in 8.4 8.5 8.6 9.0 9.1; do echo '<script>; puts [list [catch {<command>} m] $m]' | tclsh$v; done
```

Wrap the command under test in `catch` and print the code and message, so an error is evidence instead of an abort; put the script and the output in the test's doc comment. For a diagnostic, run the fixture as the document would run (file level, or inside the procedure and with the call the fixture makes), since `upvar 1` and `uplevel 1` at file level are `bad level "1"`.
