# Hand-over: taint and iRules checks

Outgoing session `session_017u5g3HxGMgE8DmBYQoVvrC`, 17 November 2026. Written on `da039ed1`. The area's running notes are the "Taint and iRules checks area" section of [`current-state.md`](../current-state.md); this file replaces them as the place to start.

## Scope

Owned, all in `tcl-compiler`:

- `rust/tcl-compiler/src/taint.rs` and `src/taint/**` (sink warnings, word and substitution taint, propagation, setter constraints, source context).
- `src/taint_interproc.rs` and `src/taint_interproc/**` (procedure return summaries).
- `src/irules_checks.rs` and its modules, `src/uri_split.rs`, `src/analyser/irules_event_checks.rs`.
- Tests: lib `taint::*`, `taint_interproc::*`, `irules_checks::*`, `uri_split::*`, `analyser::irules_event_checks::*`; integration `tests/it_3/taint.rs`, `tests/it_4/taint_depth.rs`, `tests/it_3/side_effects_binding.rs`.

Not owned, though this area's tests wait on them:

- The execution walk, realm, `command_binding/**`, `registry_invocation/**` (analyser and binding). Three files there were changed by this area under a coordinator ruling, see Rulings.
- CFG, SSA, SCCP, lowering (dataflow). `tcl-registry` rows (tooling), except taint facts, which this area may declare with `make xtask-gates`.
- The rule loader and `tcl-irules`. Tk widget commands (tk-oo).

## How to measure

```bash
source scripts/dev/agent-build-env.sh
export CARGO_BUILD_JOBS=2 CARGO_PROFILE_TEST_OPT_LEVEL=1 RUST_BACKTRACE=0
# whole suite, before every push (build 15 to 40 minutes after a src edit, run 12 minutes)
cargo nextest run -p tcl-compiler -p tcl-irules -p tcl-lsp-core -p tcl-lsp-server \
  --no-fail-fast --status-level fail --final-status-level none --failure-output never \
  -E 'package(tcl-compiler) | package(tcl-irules)' > full.log 2>&1
grep -E "^\s+(FAIL|TIMEOUT|SIGABRT|SIGSEGV)" full.log \
  | sed -E 's/^\s+\S+ \[[^]]*\] \([^)]*\) //' | sort -u > fail.txt
# edit loop: the two taint integration files (seconds after a test-only edit, 2 to 7 minutes after a src edit)
cargo nextest run -p tcl-compiler -p tcl-irules -p tcl-lsp-core -p tcl-lsp-server \
  --test it_3 --test it_4 --no-fail-fast --status-level fail --final-status-level none \
  --failure-output final -E 'test(/^taint/)' 2>&1 | cut -c1-500
```

Keep the four-package selection; another selection rebuilds every dependency under different features and fills the disk. Compare two `fail.txt` lists with `comm -23 old new` (fixed) and `comm -13 old new` (new). The baseline list below was taken before the file split, so compare against it by the last path segment of each name. A name that is new but belongs to a test added upstream after the baseline is not a regression: check with `git log -S<test name>`.

Gates before a push, after the final rebase: `cargo fmt -p tcl-compiler -- --check`; `CARGO_PROFILE_TEST_INCREMENTAL=false cargo clippy -p tcl-compiler --all-targets -- -D warnings -A clippy::assert_is_empty`; `make xtask-gates` (38 gates) when `tcl-registry` or a gate-listed file is touched.

## Current numbers

Last whole-suite run: 16 November 2026, on the tree that became `6adf3d2f` (before the file split), 13,230 run, 2,257 failing. This area: 225 of those. `it_3` `taint` plus `it_4` `taint_depth`: 76 failing of 505 (170 before the handler work). No run has completed on the restructured tree; one was started and stopped by the pause.

By group, from the baseline list: `it_3` `taint::object_instance_taint_sources` 35, lib `taint::tests` 35, `irules_checks` 36, `uri_split::tests` 15, `irules_event_checks` 13, `taint_interproc` 13, `side_effects_binding` 9, `taint::instance_classes` 5, the rest of `it_3` `taint` and `it_4` `taint_depth` about 41, others 20.

## Baseline list

[`../lists/taint-failing-2026-11-16.txt`](../lists/taint-failing-2026-11-16.txt): the 225 failing names of this area from that run, in the pre-split module paths.

## Queue

In order. Fixtures are under `f5-irules` unless a dialect is named, through `find_taint_warnings_for_cu`; in `tests/it_3/taint.rs` an `f5-irules` fixture with no `when` is wrapped in `when HTTP_REQUEST {…}` by `rule_source`.

1. **Apply and verify the `uri_split` unit-test helper** (held patch 1). 15 `uri_split::tests` build a unit from a registry with no profile, so the handler function is never visited. Expect most of the 15 to pass; the check itself works in handlers since `6b9999ee`.
2. **Confirm `HTTP::redirect` in a handler.** The tooling session reports that `when HTTP_REQUEST {set url [HTTP::uri]; HTTP::redirect $url}` publishes no IRULE3004 through the server. My probe had `set h [HTTP::host]; HTTP::redirect $h` reporting; `HTTP::uri` carries `PATH_PREFIXED`, which is `REDIRECT_SAFE`, so silence may be right. Check before changing anything.
3. **T102 and T103 in handlers.** `set h [HTTP::header Host]; regexp $h test` reports nothing. `option_injection_arguments` (`src/taint/sink_warnings/option_injection.rs`) has a Native branch and a resolved branch (`conditional_option_arguments_with_metadata_context`), no hosted one. A hosted accessor would mirror `original_hosted_scalar_store_facts` (`src/registry_invocation/hosted_structure.rs`): hosted metadata, `original_vendor_registry_words`, `with_source_schema(.., authored_source_possible_option_arguments)`. That file is the analyser area's; ask or keep it one small commit. Tests: `t102_suppression` (3), `sink_suppression_matrix` (2), `non_dash_prefixed_interpolation::literal_dash_prefix_warns`, server `irules::t102*`.
4. **A rule's own procedure called from a handler.** `proc html_safe {x} {return [HTML::encode $x]}` then `set safe [html_safe $raw]; HTTP::respond 200 content $safe` warns; `proc log_addr {addr} {log local0. $addr}` called with an `IP::client_addr` value warns twice. The call has no hosted row and no proved target, so `classify_substitution_taint` (`src/taint/substitution_taint.rs`) returns plain taint before `interproc_call_taint` (`src/taint/definition_taint.rs`) is asked; `ctx.source.procedure_ids` is the lookup to make work for a hosted handler. Tests: `interprocedural_colours` (3), `interprocedural_taint::tainted_return_from_helper_reaches_sink`.
5. **A sink argument that is itself a command substitution.** `HTTP::respond 200 content [HTML::encode $raw]` warns on `raw` although the encoder consumes it; `HTTP::respond 200 content [HTTP::query]` reports nothing; `if {[string match "/api/*" [HTTP::uri]]}` finds no URI operand (`check_expr_command` in `src/uri_split.rs`). Not traced. Tests: `t106_double_encoding::double_encode_nested_no_intermediate_fires`, `t100_sink_family` (2), `irule3103_string_match` in conditions, server T106.
6. **`expr $n` after a sanitising helper** under `tcl8.6`: `proc safe_len {x} {return [string length $x]}; set n [safe_len $raw]; expr $n` reports T100 on `n` while `puts $n` is clean. The sink path recomputes the value without the summary. Test: `interprocedural_taint::helper_sanitiser_suppresses_taint`.
7. **W201** reports nothing for `proc f {dir} {set path "$dir/file.txt"; return $path}` under `tcl8.6` (diagnostics session's fixture; they keep their four fixtures). Not started.
8. **`irules_checks` (36), `irules_event_checks` (13), lib `taint::tests` (35), `taint_interproc::tests` (11).** Not classified beyond the first message. `irules_checks` already reads handler statements through assistance (`original_irules_flow_inventory`); its failures are `catch`, `foreach` and `switch` bodies in handlers, static-variable writes (`set static::x`, which the hosted store leaves as a call) and IRULE4004. `taint::tests::hosted_taint_diagnostics_use_current_source_metadata_without_native_handler_claims` should be close to passing now and is the first to look at.
9. **IRULE3103 over a proved value** (`irule3103_edge_cases` 3, `setter_constraints::irule3103_reads_a_proven_operand`): needs a value for a non-literal handler store, which SCCP does not get by design. Coordinate with dataflow (`PassContext::eval_expression_at` has an F5 fallback).
10. **Collapse the per-rule fixtures into tables** (`tests/it_3/taint.rs` is 4,800 lines of one-shape tests: source, sink, sanitiser, expected code). The coordinator asked for it; not started. Do it once the red set is small, and keep every assertion.

Waiting on other areas, do not start: Tk widget commands (35 + 5), special variables and the unanswered-command walk, `side_effects_binding` (9). See Hand-offs.

## Held work

Both patches apply cleanly to `da039ed1` (restructured tree); neither is applied on `work`.

- `../patches/handover-taint-uri-split-test-helper.patch`: `uri_split::tests::warnings_for_dialect` builds from `static_context_for("f5-irules").commands()` with `CompilationUnit::build_for` and wraps a fixture that has no `when` in an `HTTP_REQUEST` handler. Compiles on the pre-split tree; never run to completion (two runs were stopped by pauses). Verify with the lib tests `uri_split::`.
- `../patches/handover-taint-special-variable-seed.patch`: `seed_entry_taints` (`src/taint/propagation.rs`) colours the root-global cells of the registry's special variables when the unit has no captured native entry, which a supplied document never has. Compiled and probed on the pre-split tree; the hunk in `propagation.rs` was re-applied by hand to the restructured file and is not compiled there. It changes no result today: at file level `eval $argv` is unresolved by the walk, and in a procedure every unrepresented read is already treated as possibly tainted (`proc p {} {global other; set a $other; eval $a}` reports T100 for any global). Apply it only together with the walk change in Hand-offs, with a table test over `argv`, `argv0`, `env`.

Nothing else is held. A stash named "WIP on probe-hosted" in the old container was probe code only.

## Open questions and rulings

Rulings from the coordinator that this area works under:

- The handler store with a substituted value (H1) was this area's to implement in `command_binding`, as one small commit, telling the analyser session which functions changed. Done in `333c6952`.
- Hosted metadata may supply getter colours and sanitiser transforms **to the taint diagnostics only**; no rewrite, fold or proof consumer may read them. `hosted-source-taint-descriptor.md` says so, and each claim that rests on F5 behaviour is BIG-IP check 16 in `bigip-name-checks.md`. Add a check there for any new claim.
- iRules fixtures run in a handler with commands TMM has. A fixture that mixed dialects is fixed, not deleted; an expectation is inverted only with interpreter evidence in the test comment.
- A handler body is advice: never entered, no Native handler. Keep new code behind the hosted row or `hosted_entry`.
- No file over 2,000 lines; new code goes in the module it belongs to. Colon trims go through `tcl_syntax::naming`.

Unanswered:

- Eight tests assert a combination no dialect can run: a shell or network sink (`exec`, `socket`, `http::geturl`) fed by an iRules getter (`exec_with_shell_atom_suppressed`, `exec_with_port_suppressed`, `ip_atom_clears_exec_t100`, `port_atom_clears_exec_t100`, `fqdn_atom_clears_exec_t100`, `socket_validated_*`, `http_geturl_fqdn_clears`), and `double_regex_quote` uses `regex::quote`, which TMM does not have. I left them untouched and asked the coordinator for a ruling; none came. Removing a test was refused by the permission system here, rightly; do not delete them.
- Whether IRULE1005 and IRULE1006 stay conditional advice with no action subject (the server's quick fix needs one). The server session was to take it to the coordinator.

## Hand-offs

Sent, waiting:

- Analyser and binding (`session_017u5g3J3rxgBLfx8ijRxPu9`): (a) the fresh world of a supplied document holds no special variables, so `eval $argv` at file level is unresolved (tclsh 8.4.20 to 9.1.0: `argv`, `argv0`, `env` exist); (b) `set line [gets stdin]; foo $line; eval $line`: an unanswered command leaves no sink candidate for anything after it; (c) Tk widget commands. Acknowledged as theirs, queued, not fixed.
- tk-oo (`session_017u5g3HwS13Bq9AgkSKJ68y`): fixtures `package require Tk; entry .user; set value [.user get]; eval $value`, the `ttk::entry` form and the handle form. First half on branch `issues/tk-oo-autoload-wip`. When it lands, the 35 `object_instance_taint_sources` fixtures also need `package require Tk` and a public way to build a unit with the Tk provider (`provider_fixtures` is `pub(crate)`).
- Hosted-walk author (`session_017u5g3KRpbHebfAxzbMbnAe`): a braced reference in a handler template has no SSA read reference. `set addr [IP::client_addr]; set msg "src=${addr}:80"; log local0. $msg` warns where `$addr` does not: `SsaSourceView::read_reference` is `None` for both the access span and the component span. Four tests (`crlf_free` 2, `interpolation_colour_invalidation::crlf_free_preserved_without_literal`, one more of the same shape). No reply.

Received, not started:

- Tooling (`session_017u5g3LbRZi6td44Nr3JkaV`), server `irules::*` results: IRULE3003 and T100 publish with a fix; IRULE3001, IRULE3002, T106 publish with no quick fix; not published: the braced read `${raw}`, `HTTP::redirect $url`, `puts $raw` (T101), `regexp $pat $data` (T102, T103), IRULE6001, S110. Queue items 2, 3 and 5 cover most; T101 for `puts` in a handler, IRULE6001 and S110 are not looked at.
- Tooling: `tcl-diagram` `hosted_diagram_procedures_require_the_current_body_card` needs a hosted declaration body unit for a rule procedure; the handler store does not create one.
- Diagnostics: W201, queue item 7.

## Hazards

- **A src edit costs 5 to 40 minutes.** `tcl-compiler`'s tests depend on `tcl-lsp-core`, so every compiler edit rebuilds it. Batch src edits; iterate on test-only edits (`--test it_3` rebuilds in seconds).
- **Probe through the integration test, not the lib.** A temporary `#[test] fn zz_probe` at the end of `tests/it_3/taint.rs` that prints `warns(src, IR)` shows `eprintln!` traces from the library and rebuilds fast. Remove every trace and probe before committing (`grep -n "ZZ\|zz_probe"`).
- **Clippy after the final rebase.** My `classify_substitution_taint` passed, then crossed the 100-line limit when an upstream commit added lines to it on rebase, and broke everyone's lint gate. `unsafe` is forbidden in tests too; use `temp_env`.
- **Disk.** 8 GB free is normal here. `rm -rf target/debug/incremental` frees 14 GB and costs one full rebuild. Never start a second cargo build.
- **Vacuous passes.** Many suppression tests pass only because nothing is reported at all. After any change that makes a sink report, expect such tests to fail and check each fixture before blaming the change (ten did when handler sinks first reported).
- **A fixture helper can hide the unit.** `CommandRegistry::build_default()` with `load_irules()` or `project_for_profile` is not the document's command world; `static_context_for("f5-irules").commands()` with `CompilationUnit::build_for` is. A unit built the first way has no visited handler function.
- **Background waits.** A foreground wait is cut at 10 minutes and a background one at 30; write results to a file and poll for a marker line.
- **Messages arrive late.** Cross-session messages reached me up to eight hours after they were sent. Read notifications before each push.
- `git checkout rust` is ambiguous here (a branch named `rust` exists); write `git checkout -- rust`.

## Oracle habits

- Reference interpreters: `/root/.local/bin/tclsh8.4`, `tclsh8.5`, `tclsh8.6`, `tclsh9.0`, `tclsh9.1` (patch levels 8.4.20, 8.5.19, 8.6.18, 9.0.4, 9.1.0). Put the script in a file and run each: `for t in /root/.local/bin/tclsh8.* /root/.local/bin/tclsh9.*; do $t script.tcl; done`. Record the script and the printed line in the test comment.
- Taint is a policy, so tclsh settles only the premise of a test: whether a command exists (`catch {HTTP::uri} r; puts $r` prints `invalid command name "HTTP::uri"`), what a command returns (`string length` an integer), whether a variable exists at file level.
- F5 has no interpreter here. A claim about TMM goes into `bigip-name-checks.md` as a numbered check with the request to send and what to log; the code follows Tcl 8.4 or the registry row until it is answered. Whether a command exists in TMM is read from `rust/tcl-registry/src/commands/irules/`.
- No `jimsh` is installed in this container.
