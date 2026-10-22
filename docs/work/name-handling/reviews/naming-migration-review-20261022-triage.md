# Triage: naming migration review (2026-10-22) against the current `work` tree

Reviewed report: [naming-migration-review-20261022.md](naming-migration-review-20261022.md), written against `fcb7927f02` (16 Oct). The working tree was replayed on 2026-10-22 from 10:30Z onwards, on `work` (snapshots `bc79dd0d4d` and `2fe1d48d97`) plus in-flight agent edits.

Evidence, scripts and logs: [triage-20261022/](triage-20261022/README.md). Live progress and resume steps: [../outstanding/review-triage.md](../outstanding/review-triage.md).

This report is updated after every probe run. Remaining post-R1 and latest-work obligations are stated explicitly.

## Validation snapshots

This is an interim triage report. The post-R1 rename replay and final latest-work check remain outstanding. No product code or permanent tests were changed. `make rust-check` and `make prep-pr` are unrun under the active `work` development-push exception.

| Evidence | Compiled/source snapshot | Result |
|---|---|---|
| `run6.log` main | `5bd5a503` | 24 observational probe functions completed; 3600 s cap reached during scaling before seams. Flat P25/50/100/200 and chains completed; lens P25 completed. |
| `run6.log` later existing-fold e2e | `90fe96e2` | 0 passed / 1 failed: DSL 1 expected `["5"]`, actual `[]`. |
| `run6-oracles.log` / `run6-oracle-summary.json` | replay recorded on `90fe96e2` of `5bd5a503` outputs | 96 files / 576 engine executions; 76 generated files / 126 matching-profile checks; zero unexpected matched results. |
| `run7-pending.log` | `0434e7e7` | Complete: three seams, N17 effects, R1 realm/result/seeding, N03 parser controls, chains and all three bounded lens sizes; every group exited 0. Temporary source/executable deleted. `run7-progress.log` is the earlier closed prefix. |
| `run7-pending-oracles.log` / `run7-pending-oracle-summary.json` | `0434e7e7` | 30 files / 180 executions; 10 generated files / 60 matched checks; zero unexpected result. N17 `good`, disabled inline `LOCAL`, disabled extraction `5|1` on all engines. |
| `run7-source-audit.log` | `0434e7e7` | Static excerpts for server-only seams, effect gate, arity ownership and scaling structures. |

The run7 harness passes mean each observational probe completed, not that every reported product answer is correct. R1 remains absent for plain `puts` and `puts [foo]` in both the retained realm and analyser result on C 8.6/9.0, before/after `core_surfaces::ensure`. Declaration-only and bare-call controls have both worlds. The refusal originates before analyser transport and is not repaired by that bootstrap; no post-fix rename matrix was triggered. The added N09 taint projection first failed before execution in `run8-n09` because its temporary probe used a crate-private helper (E0624). The corrected `run9-n09` on `128e1bb4` compiled and passed its one observational probe (64.04 s). Its exact dependency set for `::n::c` is `{::p, ::lib::p, ::n::c}`; leaf procedures have no dependencies. This is conservative widening, not a wrong definite global callee or a proof of precise end-to-end taint propagation.

Cloud setup and earlier failures are preserved in `cloud-setup.log`, `run5.log` and `run5-e2e-repeat.log`. Run5's main/scaling probes did not execute because compile-time fixtures were omitted from the selective checkout; those three fixture trees are restored. Its initial e2e diagnostic segment was lost when a tracked live log's inode changed on rebase, so only the stable repeat and subsequent run6 e2e are used to classify that failure. Initial six-engine input replay is `cloud-baseline-oracles.log` (19 files / 114 executions).

**Interim all-finding replay active on `60ce7009`:** `run10-progress.log` records the completed prefix through the server-shaped rename matrix (460.09 s, all six profiles), all observational groups exiting 0 with the same product results. Static audits: `run10-source-audit.log` on `60ce7009` and `run10-after-fix-source-audit.log` on rebased `6ebc499c`, where the new declared-footprint gate is present. Remote `fd5c12bf` now contains the N17 effect-gate fix, while R1 remains blocked; a fresh N17 replay is required after rebase. No run10 oracle result is claimed yet, and the still-running `run10-current.log` is intentionally untracked. The initial wrapper startup failure was fixed before compilation; it executed no tests (`run10-startup.log`).

## Method

- **Oracles.** Each Tcl repro from the review (`triage-20261022/tcl/`) was run with `triage-20261022/oracle.sh <file>` on six engines: `tclsh8.4` (8.4.20), `tclsh8.5` (8.5.19), `tclsh8.6` (8.6.18), `tclsh9.0` (9.0.4), `tclsh9.1` (9.1.0) and Jim 0.84 (`tmp/jimtcl-oracle/jimsh`). Every oracle answer matches the review's expected outcome, including feature-unavailable rows: TclOO is missing on 8.4, 8.5 and Jim, and `namespace path` on 8.4 and Jim.
- **Probes.** Current production APIs were called from one temporary test, `rust/tcl-spectcl/tests/zz_review_probe.rs` (copy: `triage-20261022/zz_review_probe.rs.txt`).
  - It was built in the shared target with `CARGO_INCREMENTAL=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_PROFILE_DEV_DEBUG=0` and run with `--nocapture --test-threads=1`.
  - `tcl-spectcl` was chosen because one build links `tcl-compiler`, `tcl-lsp-core`, `tcl-spectcl`, `tcl-spec-hooks` and `tcl-engine-tclvm`.
  - Runs: run1 at 11:01Z (safety batch), run2 at 11:20Z, run3 from 11:28Z to 12:06Z, run4 from 12:13Z to 12:39Z (stopped before scaling when the triage moved to a cloud session). Cloud run6 uses `run6.sh`; the bounded follow-up uses `run-pending.sh` and `pending-probe.rs.txt`. The prepared final latest-work replay is `run-final.sh` with `final-probe.rs.txt`. Logs and hashes are in the evidence directory.
  - Runs 1–4 reported no crate compilation failures. Cloud run5 encountered omitted embedded fixture files in `tcl-test-support`; these checkout errors were repaired without product changes.
- **Edits.** Returned edits (rename `TextEdit`s, optimiser rewrites, minifier output) were applied to scratch copies (`triage-20261022/out/`), and each copy was run on the same six engines.
- **Rename entry point.** Rename was probed in two ways:
  - `rename_with_diagnosis(.., registry=None)` in run1 and run2;
  - `rename_in_program` with `registry = static_context_for(dialect).commands()` and a `ProgramExports` view from `WorkspaceIndex::export_snapshot()` in run3. This is the server's own call shape (`tcl-lsp-server/src/lib.rs`, `in_document_rename_edits`).
- **Not run.** `tcl-lsp-server` and its JSON-RPC end-to-end tests were not built. Server-only seams are judged by reading the code and are marked "(static)".
- **Status vocabulary.**
  - REPRODUCES: the same wrong result.
  - FIXED: the correct result.
  - CHANGED: different but still wrong, or now refuses or abstains.
  - NOT-APPLICABLE: the code was removed or replaced.
  - "Refused, pending R1": rename now refuses even plain static cases, because no completed command world is available. That gap is being fixed by compiler-names (R1), and these rows must be re-checked once rename produces edits again.

## Summary

| Finding / seam | Status | Owner | Current files |
|---|---|---|---|
| N01 colliding identities | CHANGED: `all_procs` still collapses; both definition targets FIXED on C 8.6/9.0/9.1; empty-tail link FIXED | unowned (closest compiler-names) | `tcl-compiler/src/analyser/types.rs:2415`, `analyser/handlers.rs` |
| N02 Jim colon policy | CHANGED: identities distinct; definition abstains under Jim | compiler-names (naming), lsp (definition) | `tcl-syntax/src/naming.rs`, `tcl-lsp-core/src/definition.rs` |
| N03 variable cursor/profile | C Tcl FIXED; Jim CHANGED (definition/references abstain and rename refuses) | lsp / compiler-names | `tcl-lsp-core/src/definition.rs`, original variable bindings |
| N04 imported-source rename | Refused, pending R1 | lsp (planner), compiler-names (R1 world) | `tcl-lsp-core/src/original_command_rename.rs`, `rename.rs` |
| N05 forced-import rename | Refused, pending R1 (references correct) | lsp, compiler-names | same |
| N06 collision checks | Refused, pending R1. Run6 class/proc and sibling cases refuse with UnknownWorld; methods lack a complete current member roster | lsp | same; `rename.rs` `cross_document_symbol_edits` |
| N07 speculative proc variable | CHANGED: safe refusal; definition no longer walks to global. Required post-R1 replay pending | lsp | `rename.rs`, `definition.rs` |
| N08 inliner subset oracle + nested lowering | FIXED (edits and unit inference) | — | — |
| N09 type/call/taint | Constructor/call graph FIXED; taint dependency resolver conservatively retains all source candidates | — | — |
| N10 OO class/relation | CHANGED: relations, qualified miss and path-based definition FIXED; path hover/type hierarchy still empty | lsp | `tcl-lsp-core/src/definition.rs:4260` (`resolve_class_target_at`) |
| N11 packages | CHANGED: `if {0}` provider classified `Conditional` and still selected; `merge_library_files` early return REPRODUCES (static); `auto_qualify` Unicode FIXED | lsp | `tcl-lsp-core/src/package_resolver.rs`, `package_resolver/reachability.rs`, `tcl-lsp-server/src/lib.rs:17823` |
| N12 incremental lifecycle | FIXED | — | — |
| N13 source environment | REPRODUCES (path not inherited); 5-round and 4-round caps present (static) | lsp, unowned (`analyser/state.rs`) | `tcl-compiler/src/analyser/state.rs:2197`, `tcl-lsp-core/src/workspace_index.rs:3354`, `tcl-lsp-server/src/lib.rs:28775` |
| N14 callback arity / semantic roles | FIXED (arity by reading the code; tokens executed) | — | — |
| N15 minifier compaction | CHANGED (refuses `MissingAlphaRenameContract`); output behaviour-preserving | lsp | `tcl-lsp-core/src/minify.rs`, `minify/original.rs` |
| N16 provider fallbacks | FIXED (4/4 rows) | — | — |
| N17 SpecTcl template | FIXED | — | — |
| N17 live hook | Fix landed in `fd5c12bf`, fresh verification PENDING. Pre-fix: direct/pure hooks work; undeclared effects suppress folding and existing e2e fails | compiler-codegen (`const_subst.rs`, per user decision 13:17Z); vm-runtime (host) | see detail |
| Seam `ns_var_global_fallback` | FIXED (static) | — | — |
| Seam `WorkspaceIndex::registry_for` | FIXED (static) | — | — |
| Seam `analyse_with_workspace_classes` | CHANGED (static): resolved input, resource inputs and base-analysis identity retained; source-entry parity not shown | lsp | `tcl-lsp-server/src/lib.rs:20036–20120` |
| Seam nested passthrough lowering | CHANGED: iRules retains generic calls and declines inlining; Tcl control inlines with parsed condition. Context forwarding FIXED (static) | compiler | `tcl-compiler/src/inline_uplevel.rs` |
| Seam `insert_rebound_candidates` | NOT-APPLICABLE (removed) | — | — |
| Seam `method_barrier` | FIXED (static) | — | — |
| Seam linked-editing self-name fallback | FIXED (static, plus executed probe) | — | — |
| Seam extract/inline refactors | CHANGED: explicitly disabled refusals with zero edits in both repros | lsp | `tcl-lsp-core/src/refactor/` |
| 4.1 scaling rows | Substantial flat/index/chain/lens costs REPRODUCE at the requested small sizes | perf / lsp | `workspace_index.rs`, `source_graph.rs`, `code_lens.rs` |

## Per-finding detail

Each row gives the evidence (probe call or command and the observed output), the oracle result, the owner and files, and the review's fix direction.

### N01: colliding / display identities

- **Status:** CHANGED. The `all_procs` display-key map still collapses two slots. Navigation now distinguishes both declarations; the empty-tail part remains FIXED.
- **Fresh run6:** on C 8.6, 9.0 and 9.1 the calls in `{p}]` and `{:p}]` target `(0,24)` and `(1,24)` respectively. This is corrected navigation despite the single `::a:::p` metadata entry; the map collision alone is not evidence that these current definition calls select the wrong entity.
- **Evidence:** `Analyser::new().analyse(src, "tcl8.6"|"tcl9.0")` on the review's `a:`/`:p` script (`tcl/n01.tcl`) gives `all_procs=["::a:::p"]`: one entry for two runtime slots. `diags=[]`, so the review's false unknown-command diagnostic is gone.
- **Oracle:** 8.6, 9.0 and 9.1 print `LEFT|RIGHT`. 8.4 and 8.5 reject `proc :p`. Jim prints `RIGHT|RIGHT`.
- **Empty tail:** for `tcl/n01b.tcl`, `WorkspaceIndex::command_links()` gives `("::a", "::n::")`. The target is preserved, so this part is FIXED. The oracle prints `EMPTY` on all C releases.
- **Owner:** unowned. `analyser/types.rs` (the `all_procs: HashMap<String, ProcDef>` key at line 2415) and `analyser/handlers.rs` (proc intake) are not on any agent's list; the closest owner is compiler-names. Both files are clean in `git status`.
- **Fix direction:** carry typed namespace/command identity from declaration intake; never reparse display text as identity.

### N02: Jim colon policy

- **Status:** CHANGED.
- **Evidence for `tcl/n02.tcl`:**
  - Under `jim`: `all_procs=["::a:::p", "::a::p"]`. The identities are now distinct, but `definition()` from both `[a::p]` and `[a:::p]` returns `[]`, so navigation abstains.
  - Under `tcl8.6`: one key `::a::p`, and both calls go to line 2. That is correct, because C Tcl collapses the two.
- **Oracle:** C Tcl prints `THREE THREE`; Jim prints `TWO THREE`.
- **Owner:** compiler-names (`tcl-syntax/src/naming.rs` policy) and lsp (`tcl-lsp-core/src/definition.rs`).
- **Fix direction:** a mandatory family-aware naming policy (operation-specific resolution for Jim), so Jim navigation resolves instead of abstaining.

### N03: variable cursor/profile

- **Status:** C Tcl FIXED in the cursor/rename repros; Jim CHANGED to conservative abstention/refusal.
- **Fresh run6:** for `set caf A; set café B; puts $café`, all five C profiles edit the `caf` declaration and only the `$caf` prefix. Each matching-engine output is `Aé`. Under Jim, with core surfaces seeded as production does, definition/references are empty and rename refuses because the original variable cannot be proved. No Jim output edit is returned.
- **Nested braces:** C 8.6 selects `a{b`; C 9.0/9.1 select `a{b}`. The oracle prints `X}` on C 8.x/Jim and `Y` on C 9.x. A cursor on the literal `bar` suffix of `$foo-bar` gives no definition.
- **Fresh run7 parser debt check:** scalar names `a(b` and `a(b)c` remain intact under both `FirstClose` and `Tcl9Nesting`. The valid first-close `${a{b}(status)` script has no diagnostics on C 8.4/8.5/8.6/Jim, although the profileless `is_braced_indirect_array_ref` helper returns false. That helper alone is not evidence of a current diagnostic failure. The same script is invalid on C 9.x, as recorded by the oracles.
- **Source:** `template_variable_reference_at`, original-variable proofs and the shared style-aware array-name splitter. Remaining Jim provider coverage belongs to lsp/compiler-names.

### N04: imported-command source rename

- **Status:** refused, pending R1.
- **Fresh run6:** wildcard and exact forms refuse on all six profiles. `original_completed_command_world()` is false in every case; C profiles report UnknownWorld, Jim reports the original command unavailable. No edit was returned.
- **Evidence:** `rename_in_program` (run3) from `proc bar` to `baz` returns `REFUSED "unknown-completed-command-world …"` for both the wildcard (`tcl/n04w.tcl`) and the exact-import form, under tcl8.4, tcl8.6 and tcl9.0. Under Jim it refuses with "the original command selected at this cursor is unavailable …". No edit is produced, so nothing ran.
- **Oracle:** the original prints `1` on all six engines.
- **Owner:** lsp (`tcl-lsp-core/src/original_command_rename.rs`, which raises `UnknownWorld` at `occupied()`; also `rename.rs`), plus compiler-names for the completed command world (R1).
- **Fix direction:** one source-rename plan covering imports, exports and derived uses, or refusal of the whole edit. Re-check once R1 lands.

### N05: forced-import rename vs references

- **Status:** refused, pending R1.
- **Fresh run6:** all six profiles return only `(2,9)` from references and refuse rename with UnknownWorld. Simple `proc foo; puts [foo]` controls through `rename_with_diagnosis` also refuse on C 8.6/9.0/9.1; the server-shaped controls are later in this active run.
- **Evidence:** `references` from the `b::foo` declaration gives only `(2,9)`, which is correct. Rename is `REFUSED "unknown-completed-command-world …"`, both through `rename_with_diagnosis` without a registry and through `rename_in_program` with the registry and program view, under tcl8.4, tcl8.6, tcl9.0 and Jim.
- **Controls (run3):**
  - `proc foo {} {return A}; puts [foo]` refuses with "incomplete-original-command-coverage".
  - `set x [foo]` refuses the same way.
  - Only a bare `foo` call renames. Under Jim every proc rename refuses.
  - So the refusal is not specific to `-force`.
- **Oracle:** the original prints `A` on six engines.
- **Owner and fix direction:** as N04. Reference enumeration and rename must project from the same settled binding.

### N06: rename collision checks

- **Status:** refused, pending R1.
- **Fresh run6 supersedes the older specific refusals below:** C 8.6/9.0/9.1 refuse class and proc→class with UnknownWorld, and method rename with `unknown-current-source-member-roster`. The general refusal keeps edits disabled but does not establish that the destination-specific collision check now runs. The six-profile sibling matrix also refuses at both origins: all C profiles give UnknownWorld; Jim gives UnknownWorld from the declaration and original-command-unavailable from the call. No edited sibling fixture is produced.
- **Evidence (`rename_in_program`, tcl8.6 and tcl9.0):**
  - Class `A`→`B`: `original-command-collision`.
  - Method `old`→`dest`: `original-member-collision`.
  - Proc `P`→class `C`: `original-command-collision`.
  - These are specific destination checks.
  - Control: a collision-free class rename `A`→`Z` refuses with "incomplete-original-command-coverage".
  - Local `p`→`q` with a sibling file defining `q`: core refuses (incomplete coverage). `cross_document_symbol_edits` returns 0 edits, and `workspace_symbol_rename_edits` returns `None`, because `WorkspaceIndex::allows_lexical_rename_advice()` is `false`. The declaration-origin server augmentation (`add_cross_document_rename_edits`) still has no destination check (static), but it is unreachable while that gate is false.
- **Oracle:** 8.6, 9.0 and 9.1 print `A|B`, `OLD DEST` and `P`.
- **Owner:** lsp (`rename.rs` `cross_document_symbol_edits`; server `add_cross_document_rename_edits` in `tcl-lsp-server/src/lib.rs`).
- **Fix direction:** one destination check, independent of where the rename starts. Re-check once R1 lands.

### N07: speculative proc-variable binding

- **Status:** CHANGED to safe refusal; required post-R1 replay remains outstanding.
- **Fresh run6:** definition is empty on all six profiles; no request edits the unrelated global. C profiles report W210 and refuse with `unreferenced-edit`; Jim reports no W210 and refuses because the original variable cannot be proved. This is a safe refusal, with a separate Jim analysis gap. The run6 server-shaped C matrix has a completed world and still refuses with `unreferenced-edit`, so this refusal is not caused by R1. The user-required post-R1 replay remains outstanding.
- **Evidence for `set g 1; proc p {} {puts $g}`:**
  - Definition from `$g` is now `[]`; it no longer walks out to the global. W210 is still reported.
  - Run1, without a registry: rename to `h` edits only the proc-local `$g`. The global is untouched, under tcl8.4, tcl8.6, tcl9.0 and Jim.
  - Run3, with the registry and program view: `REFUSED "unreferenced-edit: an edit falls outside the occurrences find-references reports"`. Under Jim it refuses with "cannot prove the original variable …".
- **Owner:** lsp (`rename.rs`, `definition.rs`).
- **Fix direction:** edit-producing providers need a proven occurrence-to-cell edge.

### N08: inliner subset oracle

- **Status:** FIXED.
- **Evidence:**
  - `optimise_with_dialect(N08, tcl8.4|tcl8.6|tcl9.0|jim)` returns no optimisations.
  - `optimise_source_multipass(..,5)` leaves the source unchanged, and it prints `LOCAL` on six engines (`out/n08_opt_*.tcl`).
  - `CompilationUnit::build_for_profile(N08, tcl8.6)`: the SCCP of `::n::c` mentions neither `GLOBAL` nor `LOCAL`, and the return type is `Overdefined`.
  - The review saw a wrong `GLOBAL` inference and a `LOCAL`→`GLOBAL` rewrite; neither appears.
- **Source of the fix:** `inline_uplevel.rs` selects the callee via `tokens.source_binding.proved_execution_target()` and checks the authored implementation. The nested literal body uses `analyse_script_at_site` and `entry.options()` (944d156922 and later, post-review).

### N09: compiler type / call / taint

- **Status:** FIXED for the executed constructor and direct-call repros.
- **Fresh run6:** the nearer class gives `Object(::n::Foo)`. A nearer non-class procedure blocks global class inference, leaving `::n::c` `Overdefined`.
- The path fixture now has a `::n::c` summary with `direct_calls=["::lib::p"]`; the other summaries are `::lib::p` and `::p` with no direct calls. The earlier missing-caller result is no longer current. C Tcl's expected result is `LIB`.
- The legacy analyser `resolved_qualified_name` field is `None` for the `p` occurrence; the current call graph does not choose the wrong global procedure.
- **Scope:** taint lattices and warning outputs were not independently executed. The review's demonstrated wrong definite call edge is corrected; this does not claim a complete new taint conformance matrix.

- **Fresh run9 (`128e1bb4`):** constructor `::n::c` returns `Object(::n::Foo)`; a nearer non-class `::n::Foo` widens to `Overdefined`. Direct calls are only `::n::c → ::lib::p`. The public taint `resolved_callees` projection retains all three source ProcedureIds for `::n::c`, and none for either leaf. This eliminates the old wrong singleton from that projection by conservative widening; taint lattices and warning precision were not independently validated. Evidence: `run9-n09.log`.

### N10: OO class / relation resolution

- **Status:** CHANGED.
- **Fresh run6 relation site:** `supertypes("::B::Child")=["::A::Base"]`, matching C Tcl 8.6–9.1. FIXED.
- **Missing qualified class:** `superclass ::missing::Base` gives `[]`, without unrelated unique-tail rescue. FIXED.
- **Path-based class call:** `C new` inside `app` with path `::lib` now has definition `[0]`, the correct class declaration. Hover remains `None` and type hierarchy remains `[]`; these provider gaps persist.
- **Owner:** lsp. The old “classes never consult namespace path” code in `resolve_class_target_at` is behind a legacy advice gate. That comment is not proof that current definition ignores the path; the executed definition result now demonstrates otherwise.

### N11: package availability / provider

- **Status:** CHANGED.
- **Evidence:** the fixture registers P 1.0 unconditionally and P 2.0 inside `if {0}`.
  - `resolve_require` returns `[p1.tcl, p2.tcl]`.
  - `candidate_providers_for_require(.., Some(V8_6))` returns versions `1.0, 2.0`.
  - `resolve_require_for_profile(tcl8.6)` returns `[p1.tcl, p2.tcl]`.
  - `PackageInfo::availability(Some(V8_6))` gives 1.0 `Available` and 2.0 `Conditional`. The static `if {0}` is not evaluated to `Unavailable`, so p2 is still offered.
- **`auto_qualify`:** `("café","::")` gives `["café"]`. Unicode is preserved, so this part is FIXED.
- **`merge_library_files`** (`tcl-lsp-server/src/lib.rs:17823`): it still returns early when `workspace_command_exists` finds any candidate. REPRODUCES (static).
- **Owner:** lsp (`package_resolver.rs`, `package_resolver/reachability.rs`, `tcl-lsp-server/src/lib.rs`).
- **Fix direction:** filter unavailable registrations before selection, and keep provider/domain provenance in merged library facts.

### N12: incremental lifecycle invalidation

- **Status:** FIXED.
- **Evidence** (old and new texts as in the review, with the `rename` at the same offset): the warm old index gives `(false,false)`. After `replace_document` it gives `(true,true)`, the same as a fresh index on the new text, which also gives `(true,true)`.
- **Oracle:** `out/n12_old.tcl` prints `0` and `out/n12_new.tcl` prints `1`, on six engines.

### N13: source environment / convergence

- **Status:** REPRODUCES.
- **Evidence:** `Analyser::analyse_with_source_namespace("proc run {} {helper}", "tcl8.6", "::app")` resolves `helper` to `::app::helper`.
- **Oracle** (`tcl/n13p.tcl`, path `::lib`): `good` on 8.5–9.1.
- **Static:** `imported_constants` has `ROUND_CAP = 5` (`workspace_index.rs:3354`), and the server's `refresh_source_rehoming_locked` loops `for _round in 0..4` (`tcl-lsp-server/src/lib.rs:28775`). The seed is still a namespace string (`analyser/state.rs:2197`).
- **Owner:** lsp (`workspace_index.rs`, server), plus unowned `analyser/state.rs`.
- **Fix direction:** source views carry inherited path/import/environment state, with dependency-driven convergence and an explicit incomplete outcome.

### N14: callback arity and semantic roles

- **Status:** FIXED.
- **Semantic tokens:** `semantic_tokens::full` on the review's `A::touch`/`B::touch` script marks `plain` as type `string` with no modifiers. It is no longer a variable declaration. The oracle prints `plain|0`.
- **Callback arity (static):** the tail-bucketed `project_command_arities` was removed in 3e29714009 and replaced by per-slot `original_command_signatures` and `original_lookup_command_arities` (`tcl-lsp-db/src/lib.rs`).

### N15: minifier compaction

- **Status:** CHANGED (refuses).
- **Evidence:** both `minify_with_profile` and `minify_with_analysis` (the call the server's `minifyDocument` makes) were run for both repros, under tcl8.6, tcl9.0 and Jim, for the Compact and Aggressive tiers, isolated and not. Every run returned refusal `MissingAlphaRenameContract`, plus three more refusals for Aggressive. The symbol map is empty and the output is separator compaction only.
- **Execution:** all 48 outputs print `OLD|NEW` or `NEW` on six engines.
- **Residual:** the legacy `compact_names` path survives behind `allows_lexical_declaration_advice()` in `minify/original.rs`. These inputs did not reach it.
- **Owner:** lsp (`tcl-lsp-core/src/minify.rs`, `minify/original.rs`).
- **Fix direction:** a shared transactional rename planner, or keep the server-side refusal.

### N16: provider fallbacks

- **Status:** FIXED, all four rows.
- **Signature help:** `namespace path ` under tcl8.4 gives `None`. Under tcl8.6, `namespace pat ` gives `namespace path ?namespaceList?`.
- **Alias hover:** a call to `a` inside `n` shows `proc ::n::a`.
- **Type definition:** `puts greet` gives `[]`.
- **Completion:** both `("f","::A::f")` and `("f","::B::f")` survive.

### N17: SpecTcl template and live hook

The declared-footprint-only barrier correction landed in `fd5c12bf`; the following contrast describes pre-fix executions. The rebased run10 e2e and a newly built focused live-hook matrix must verify the correction before this row is settled on current work.

- **Template: FIXED.** Fresh run6 again registers only `good`, with no notices/load errors, in all 16 combinations of DSL 1/2.0, direct/helper, unbraced/braced and fast-path on/off. The capture driver independently registers `good` on all six pinned engines.
- **Live-hook substitution: FIXED in direct execution.** All eight run6 controls (two DSLs × braced/unbraced reference × declared/undeclared pure effects) install with no decline or notice and directly return `Some("good")`. The earlier wrong `bad` result is gone in these cases.
- **Pre-fix optimiser behavior (`5bd5a503` / `60ce7009`): REGRESSED for undeclared effects.** Some live hooks do fold; the effect declaration is the distinguishing condition:

| DSL / array reference forms | Effects declaration | Direct callback | `optimise_raw` O129 |
|---|---|---|---|
| 1 and 2.0; `$arr($a:b)` and `$arr(${a}:b)` | absent | `good` in all four cases | `[]` in all four cases |
| 1 and 2.0; both forms | `world_effects none` | `good` in all four cases | `["good"]` in all four cases |

- The ordinary `mylib::strlen` control emits no optimisation even with a returned value. The existing `a_pack_file_folds_a_call_site_in_the_optimiser` independently fails for DSL 1: expected `["5"]`, got `[]` (0 passed / 1 failed; `run5-e2e-repeat.log`). This is not a claim that the hook engine or every possible live hook is dead.
- **Static cause supported by the controlled contrast:** `const_subst.rs::fold_retained_at_depth` rejects `requires_world_barrier()`, while undeclared world effects receive a conservative unknown/re-entrant footprint. Requiring old packs to add a pure declaration is not the selected compatibility fix; the user assigned compiler-codegen the scoped gate correction.
- **Owner:** compiler-codegen (`const_subst.rs`), per the recorded 13:17Z user decision. No product change was made by triage. All eight run6 execution drivers printed `good` on all six engines. Run7 repeats the same effect contrast; all eight fresh drivers also print `good` on all six engines.

### Section 3.2 / 3.3 seams

- **`ns_var_global_fallback`: FIXED (static).** It reads `resolved_profile()` first and falls back to the catalogue string only without resolved input (`analyser/types.rs:4286`; 944d156922).
- **`WorkspaceIndex::registry_for`: FIXED (static).** It borrows the document's retained input registry, guarded by its semantic key (`workspace_index.rs:3727`, `fn registry` at 1479; 8aa4dfea79).
- **`analyse_with_workspace_classes`: CHANGED (static).**
  - Retained: `base.resolved_input`, `resource_analyser_inputs`, and a cache keyed on fingerprint, the base analysis `Arc` and resource inputs.
  - Not visibly forwarded: the source-analysis entry or seed.
  - No parity test was found.
  - Location: `tcl-lsp-server/src/lib.rs:20036–20120`. Owner: lsp.
- **Nested passthrough lowering: CHANGED / conservative decline.** Fresh run7 walks actual `Statement` nodes from every top-level/procedure/body-unit root. Both iRules forms (`call run` in an event and `run` in a proc) retain generic `if`/dispatcher calls, with no inlined block and no typed direct condition. The old `Binary(Contains)` direct versus `Raw` inlined mismatch is therefore not present on this path; it would overstate the evidence to claim that specialised iRules inlining is working. The ordinary Tcl control produces one inlined `Block` and a parsed `Binary(StrEq)` direct condition; the literal inlined `if` is retained generically.
  - Static context forwarding is repaired: `try_inline_callsite` obtains site-analysed bindings with `analyse_script_at_site`, calls `set_source_analysis_options(entry.options())`, and lowers with those bindings. A declined specialisation is permitted by the lowering contract; no product fix was made by triage.
- **`insert_rebound_candidates`: NOT-APPLICABLE.** Removed in cf8f1bc85b and replaced by `collect_namespace_resolution` (`command_binding.rs`).
- **`method_barrier`: FIXED (static).** The global-first and first-tail `class_head_component` was removed (944d156922). `hierarchy_components` unions only retained relation candidates and widens on unknowns; test `retained_dispatch_does_not_select_an_unrelated_class_tail`.
- **Linked-editing self-name fallback: FIXED (static).** `OriginalLinkedSourceSpelling::capture` requires `invocation_targets_declaration` or a call-declaration match. The literal fallback survives only as a `#[cfg(test)]` helper. Executed (run4): for `proc greet {} {greet}` plus an unrelated `n::greet`, `linked_editing_ranges` from the declaration gives `[(0,5),(0,15)]`, the declaration and its own resolved self-call only.
- **Extract/inline refactors: explicit disabled refusals.** Fresh run7 reports:
  - `inline_proc`: zero edits; `disabled = "the copied command's implementation is not preserved in the caller namespace"`.
  - `extract_variable(.., "result")`: zero edits; `disabled = "The selection is not a pure original complete word at a reached local source point"`.
  - These repros are contained by disabled actions; they are not silently offered successful zero-edit transformations. Their scratch sources are unchanged. This does not claim every extract/inline feature is globally disabled or broken.

### Section 4.1 scaling rows

**REPRODUCES substantial costs in the unoptimised test profile.** Run6's serial measurements below use wall time (`Instant`), not the original review's process-CPU medians or a shipping-server build. Index/query columns are minima of three; analysis is measured once. These fixtures include real current analysis; the review's metadata-only index input is not a direct absolute-time comparator.

| Flat procedures | Analysis ms | Index ms | 100 missing call-aware queries ms |
|---:|---:|---:|---:|
| 25 | 23,489.8 | 1,043.491 | 1.480 |
| 50 | 73,747.7 | 3,332.122 | 2.364 |
| 100 | 237,256.1 | 5,018.812 | 5.314 |
| 200 | 630,311.1 | 10,880.502 | 16.610 |

| Source-chain files | Build ms (minimum of three) |
|---:|---:|
| 250 | 134.775 |
| 500 | 318.125 |
| 1,000 | 1,396.401 |
| 2,000 | 5,055.848 |

Fresh run7 uses a separate process/budget per lens size on `0434e7e7`, with analysis completion explicitly logged before the query:

| Procedures | Pre-analysis ms | Lens query ms | Returned lenses |
|---:|---:|---:|---:|
| 25 | 35,242.482 | 6,093.848 | 25 |
| 50 | 150,409.102 | 43,231.733 | 50 |
| 100 | 280,831.269 | 134,609.430 | 100 |

Fresh source-chain minima (F250/500/1000/2000) are 70.989 / 252.495 / 1,085.463 / 5,692.710 ms. Run6's older P25 lens query was 6,049.759 ms after 61,798.1 ms analysis; its combined one-hour cap yielded no larger lens result, so no time is inferred from that cap.

Static inspection still finds the linear live-procedure/class/link scan in `workspace_command_exists_for_call` and the root-path vectors in `RunOrder`. Full server/release latency and RSS remain unmeasured. Logs and source hashes distinguish this snapshot from later implementation fixes.
