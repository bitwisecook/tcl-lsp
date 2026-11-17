# Hand-over: iRules, registry, SpecTcl and tooling

Written 17 November 2026 by the outgoing session. Everything this area did is on `work`; nothing is held locally. The detailed record, with every classification table, is the section "iRules, registry, SpecTcl and tooling area" of `docs/work/name-handling/current-state.md`.

## Scope

Owns the tests of `tcl-irules`, `tcl-registry`, `tcl-spectcl`, `tcl-sslictcl`, `tcl-cli`, `tcl-mcp`, `tcl-explorer`, `tcl-diagram`, `tcl-spec-studio`, and the iRules rule-loader part of `tcl-compiler` (`command_binding/rule_declaration.rs`, the rule-loader paths of the lowering and of `realm.rs`). Since 16 November it also owns four groups of the server end-to-end suite: `irules::*`, `spec_packs::*`, `sslictcl::*` and `jim::*` in `rust/tcl-lsp-server/tests/e2e/`.

Does not own: the compiler's walk, dataflow, taint, diagnostics, the execution crates, `tcl-lsp-core` and `tcl-lsp-server` code. A fix that needs one of those is a small separate commit announced to its owner, or a hand-off with a fixture. Two such commits were made by coordinator ruling: `048e5433` (analyser keeps a supplied input's vendor policy exact) and `3f7b64c1` (`tcl-lsp-core` declaration outline).

## How to measure

Environment, from the repository root:

```
source scripts/dev/agent-build-env.sh
export CARGO_PROFILE_TEST_OPT_LEVEL=1 CARGO_PROFILE_TEST_DEBUG=0 RUST_BACKTRACE=0
```

Do not export `CARGO_INCREMENTAL`. One cargo build at a time.

Tooling suite, always with the same ten packages so the build is reused (a different `-p` set rebuilds everything):

```
cargo nextest run -p tcl-compiler -p tcl-irules -p tcl-registry -p tcl-spectcl -p tcl-sslictcl \
  -p tcl-cli -p tcl-mcp -p tcl-explorer -p tcl-diagram -p tcl-spec-studio \
  --no-fail-fast --status-level fail --final-status-level none \
  -E 'not test(every_shape_of_inlined_body_answers_as_its_definition_does_at_every_site)'
```

The excluded test runs for 1,200 seconds. Add `& not package(tcl-compiler)` to skip the compiler's own 13,000 tests when the change cannot reach them. Cold build about 40 minutes, the compiler tests about 10 minutes, the nine other crates about 3 minutes. A compiler edit rebuilds in about 12 minutes; a registry or dialect edit, or a rebase over upstream, is a full rebuild.

Server groups, a second build with its own package set:

```
cargo nextest run -p tcl-compiler -p tcl-irules -p tcl-lsp-core -p tcl-lsp-server \
  --no-fail-fast --failure-output final \
  -E 'package(tcl-lsp-server) & binary(e2e) & (test(/^irules::/) | test(/^spec_packs::/) | test(/^sslictcl::/) | test(/^jim::/))'
```

Cold build about an hour, the 131 tests about 90 seconds. The suite is not deterministic: rerun a failure alone before believing it.

Comparison: extract failing names with

```
grep -E "^\s+(FAIL|TIMEOUT|SIGKILL|SIGABRT|SIGSEGV|SIGTERM)" LOG | sed -E 's/^\s+\S+ \[[^]]*\] \([^)]*\) //' | sort -u
```

and `comm` against the baseline list. Since the file split a test path gained a module, so compare by crate plus last path segment. Report "fixed N, new 0".

Gates before a push: `cargo fmt --all -- --check`, `cargo clippy --no-deps -p <crates> --all-targets -- -D warnings -A clippy::assert_is_empty` (`--no-deps` because an upstream crate may not be lint-clean), `make xtask-gates` (38 gates, about four minutes), and `cargo xtask pack-goldens` after any change to a spec or a port under `docs/design/spec-dsl-examples/`.

## Current numbers

All before the file restructuring; nothing was run on the restructured tree.

| Group | Failing | Run on | Date |
| --- | --- | --- | --- |
| `tcl-cli` | 42 | `631529c9` | 16 November |
| `tcl-diagram` | 14 | `631529c9` | 16 November |
| `tcl-explorer` | 14 | `631529c9` | 16 November |
| `tcl-mcp` | 3 | `631529c9` | 16 November |
| `tcl-irules` | 3 | `631529c9` | 16 November |
| `tcl-spectcl` | 29 | full run after `638bc3b6` | 16 November |
| `tcl-spec-studio` | 1 | same run | 16 November |
| `tcl-sslictcl` | 1 | same run | 16 November |
| `tcl-registry` | 0 | same run | 16 November |
| server `irules`, `spec_packs`, `sslictcl`, `jim` | 33 of 131 | base of `7736f284` | 16 November |

Later commits changed the list without a full run: `tcl-spectcl` `workspace_packs::a_manifest_that_places_nothing_does_not_lift_its_pack` was fixed (so 28), two duplicate tool tests were removed and two `tcl-spectcl` clusters tabled (both passing).

## Baseline list

`docs/work/name-handling/lists/irules-registry-tooling-failing-2026-11-16.txt`, 140 lines: the union of the runs above, in nextest's `binary test::path` form, pre-restructure paths. It still lists the `workspace_packs` test fixed afterwards.

## Queue

In order. Paths are as they were before the split; find the item with `grep -rn 'fn <name>' rust/<crate>/src` or the crate's plan under `docs/work/file-split/plans/`.

1. **Re-measure on the restructured tree.** Run both suites, write a new list, and update the scoreboard row at the top of `current-state.md`.
2. **Row-level pack notices sit at 0:0 in the editor** (open question 1 below). Test: `spec_packs::granting_trust_reloads_and_installs_the_bodies`. Fixture: an untrusted workspace with a pack whose command has a `const_fold` body; the notice "`const_fold` is dormant" must be on the hook's row. The loader records the row (`rust/tcl-spectcl/src/hooks.rs` `dormant_hooks`, `rust/tcl-spectcl/src/pack.rs` `PackNotice::dormant`; `tcl-spectcl` `workspace_trust` passes). `rust/tcl-lsp-server/src/lib.rs` `whole_line_range` positions only a notice with a `SpecPackNoticeSubject`, and its tests forbid falling back to the line. Proposed: a row subject kind in `rust/tcl-lsp-core/src/code_actions/spec_notice.rs`, issued by the loader for the hook row and carried through `HookDecl`, `DormantHook` and `PackNotice`; also covers refused stamps, refused declarations and pack-text backing.
3. **Blocks two levels down in a declaration document** (open question 2). Tests: `spec_packs::a_packs_blocks_are_folding_ranges`, `a_pack_statement_word_hovers_from_the_spectcl_surface`, `a_dropped_property_offers_a_did_you_mean_fix`, and `tcl-spec-studio` `tests::pack_formatting_uses_the_shared_spectcl_profile`. Fixture: `speclib demo 1 {\ncommand foo {\nsynopsis {foo value}\narity 1\n}\n}`: `tcl fmt --dialect spectcl` indents the `speclib` body and not the `command` body; a `hover { … }` inside `command` has no fold. Cause as traced: `SourceStructure::capture` in `rust/tcl-lsp-core/src/source_structure.rs` walks document definition bodies only for the `DefinitionCursor` purpose (`definition_cursor_document_bodies`). The outline had the same gap and was fixed in `3f7b64c1` by reading a member with `source_registry_words_in_definition_body` under `definition_parent_for`; that is the pattern.
4. **Cut the eight duplicated `value_transfers_cli` tests to one row per verb once they pass** (ruling 6). They are listed in `current-state.md` under "Cross-crate overlap".
5. **iRules quick-fix tests** in `rust/tcl-lsp-server/tests/e2e/irules.rs`: the helper `taint_fix(lsp, sink, code)` builds a real rule and hands back the published diagnostic. Waiting on the taint area (see Hand-offs); when a code is published with a fix, its test passes without a change here. If a test still fails with the diagnostic published and a fix offered, the fixture's expected text is the thing to check.
6. **Not reduced:** server `spec_packs::a_closed_file_under_a_pack_extension_is_indexed` (go-to-definition into a never-opened `.irulex` file is `null`), `environment_lifecycle::unchanged_documents_follow_folder_pack_and_stub_generations`, the two latency timeouts (`an_open_pack_keeps_its_notices_while_it_is_edited`, `the_bundled_upf_body_resolver_reaches_editor_diagnostics_and_tokens`); `tcl-cli` `pkg_verbs::pkg_discover_*` (2) and `analyser_only_verbs_install_the_current_projects_spec_pack_overlay`; `tcl-explorer` `source_function_rows_preserve_duplicate_names_and_named_top`; the three `tcl-irules` tests and the `tcl-sslictcl` golden (`x509_projection_matches_golden`).
7. **Build configurations the coordinator found broken, in this area, not started:** `tcl-spec-studio-wasm` does not compile (`src/dsl_highlight.rs:313-314`, E0308 and E0061); `tcl-lsp-server-wasi`, `bigip-query-wasm` and `bigip-report-gen/wasm` fail clippy.

## Held work

None. No local commit, stash, side branch or patch file. There are no `handover-irules-registry-tooling-*.patch` files.

## Open questions and rulings

Unanswered:

1. Row-level pack notices: have the loader issue a row subject (my recommendation), or let the server fall back to the recorded line, which its tests forbid?
2. Should this area make the two-levels-down `SourceStructure` change (queue item 3), or does the LSP features area?

Rulings in force:

1. Never open a pull request; push to `work` only, after fetch and rebase.
2. Classify each failure as a code regression, a fixture that is not what it claims, an expectation wrong for the dialect (prove with tclsh and say so in the test), or two tests that contradict. Never weaken an assertion, never `#[ignore]`, no `#[allow]`, no known-failure lists.
3. Pack discovery fails closed: an unreadable manifest or directive selects nothing and borrows no scan. The workspace test was changed to agree; recorded in `current-state.md` for the owner to review.
4. A diagram, and any tool, resolves the document's analysis input once and builds the unit and the analysis from it. The analyser keeps a supplied input exact. Do not change the lowering's standalone input for hosted profiles; handler entry moves separately.
5. A test that only wants a release's behaviour names that release; the lenient plain-Tcl profile with a release's availability is the analyser area's family.
6. Keep the eight failing `value_transfers_cli` duplicates while they fail; cut to one row per verb when they pass.
7. A supplied Tcl 8.4 input is a native release and records no logical source input.
8. F5 questions that need a device become numbered checks in `docs/work/name-handling/bigip-name-checks.md`, never a guess.
9. File rules from the restructuring: no Rust file over 2,000 lines, new code in a named module, imports from the defining module (`docs/work/file-split/README.md`). Colon trims in production code go through `tcl_syntax::naming`.

## Hand-offs

Sent, waiting on the other area (fixtures are in the messages and in `current-state.md`):

- Taint: no diagnostic published for `HTTP::respond 200 content ${raw}`, `HTTP::redirect $url`, `puts $raw`, `regexp $pat $data` as a statement, the three `IRULE6001` forms, the S110 payload round trips; `IRULE3001`, `IRULE3002` and `T106` are published with no fix offered; `IRULE1005`/`1006` are conditional advice with no action subject (the taint area asked the coordinator for a decision); `tcl-diagram` `hosted_diagram_procedures_require_the_current_body_card` (no hosted declaration body unit, so `bodyId` is null).
- Analyser: `source_registry_words` answers `None` under the lenient profile with Tcl 9.0 availability; `registry_words_for_original_call` selects `throw` under a Tcl 8.4 availability context (`tcl-diagram` `original_diagram_metadata_keeps_actual_availability_and_source_currency`); the walk drops a built-in E003 after a proved pack E002 (`spec_packs::a_workspace_pack_subcommand_arity_error_is_published`); Jim document diagnostics (W307 and others on the Jim program, in `tcl-cli`, `tcl-spectcl` and server `jim`); the `tcl-cli` `diag` and `tcl-mcp` policy rows.
- LSP features: the rename planner edits a use and not its alias-captured definition (`interp alias {} assign {} set fixed; assign 1; list $fixed`, so `tcl minimize` writes a broken program) and plans nothing for `foreach {first second} …`; `document_symbols_from_analysis` gives four function cards for two hosted procedures; completion in an empty handler body has no `HTTP::header`.
- Dataflow: the `value_transfers_cli` group (SCCP leaves values `overdefined`, summaries not folded on Tcl 8.4), the `tcl-explorer` SCCP views, `tcl-diagram` completion rows.
- Execution: `tcl-spectcl` `codegen_stamps` (a pack reference body is not inlined), the bare CR lost in a braced word, `native_normal_store`, `pack_source_e2e`.

Received and not started: nothing. The analyser diagnostics area told me of its `tcl-registry` change `1da27230` (`OptionPlacement` on `InvocationOptions`); no action was needed.

## Hazards

- Disk. The container cannot hold the tooling build and the server build together. It reached 0 MB once and lost a run. Run `rm -rf target/debug/incremental` between builds and watch `df`.
- A different `-p` set is a different build. Keep to the two package sets above.
- Do not edit a crate while a build that compiles it is running if you care which version ran.
- `cargo fmt --all` reformats other areas' files. Format your own files with `rustfmt --edition 2024 <files>` and leave the rest.
- Temporary `eprintln!` probes are how most causes were found; remove them before committing (`git diff` for `PROBE`).
- I marked the CLI, diagram and explorer groups as waiting on the `return [expr …]` fix without checking; they did not move when it landed. Read a failure's output before attributing it.
- I misread a failing line once (`tcl-explorer` supplied input) and reported the wrong field. Check which assertion the line number is.
- A mechanical module move rewrote a string literal in a test (`use crate::render_sweep::prelude::*;`). After the file split, look for the same.
- Tests that flip between runs: server `spec_packs::a_pack_added_after_startup_reloads_and_re_analyses_open_documents` and `a_pack_const_fold_body_folds_a_call_site_in_the_optimiser`.
- Kept on purpose, do not collapse: `jim_generation_zero` (each test needs a fresh process), `smoke_explore_reports_summary_for_tiny_snippet` (the Makefile's smoke tier selects `smoke_*` by name).
- A commit message built in a shell variable lost the blank line before its trailers; write the message with the blank line in place.

## Oracle habits

Interpreters are at `~/.local/bin/tclsh8.4`, `tclsh8.5`, `tclsh8.6`, `tclsh9.0`, `tclsh9.1` (patch levels 8.4.20, 8.5.19, 8.6.18, 9.0.4, 9.1.0). Feed a script on stdin and print `[info patchlevel]` with the answer, wrapping the command in `catch` so an error message is the result:

```
for v in 8.4 8.6 9.0; do ~/.local/bin/tclsh$v <<'TCL'
catch {package require -exact json 1.2 2.0} m; puts "[info patchlevel]: $m"
TCL
done
```

Record the releases and the answer in the test's comment. For Jim, `tcl_test_support::locate_jimsh` finds an interpreter when one is installed; I did not run one. A built CLI is the quickest probe of our own behaviour: copy `target/debug/tcl` aside and run `diag`, `opt`, `fmt`, `diff`, `symbols` or `spec import` with `--dialect` and `--source`.
