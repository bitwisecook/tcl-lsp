# Triage: naming migration review (2026-10-22) against the current `work` tree

Reviewed report: [naming-migration-review-20261022.md](naming-migration-review-20261022.md), written against `fcb7927f02` (16 Oct). The working tree was replayed on 2026-10-22 from 10:30Z onwards, on `work` (snapshots `bc79dd0d4d` and `2fe1d48d97`) plus in-flight agent edits.

Evidence, scripts and logs: [triage-20261022/](triage-20261022/README.md). Live progress and resume steps: [../outstanding/review-triage.md](../outstanding/review-triage.md).

This report is updated after every probe run. Rows marked **PENDING** are not settled.

Cloud resumption: 2026-10-22T13:17:02Z in a fresh, shallow selective checkout. Rust 1.99.1 and the exact six engines are provisioned; see [cloud-setup.log](triage-20261022/cloud-setup.log). Run5 completed with checkout-related failures before its main/scaling probes ran. All embedded fixtures and shared libraries are now available. The repeated existing live-fold e2e test fails as detailed below; the completed main snapshot and oracle replay are recorded below; isolated pending seam/lens checks and the post-R1 final replay remain outstanding. `make rust-check` and `make prep-pr` have not run in this evidence-only session.

## Cloud validation progress

**Run6 main snapshot (2026-10-22T14:16:15Z):** [run6-progress.log](triage-20261022/run6-progress.log) is a closed prefix snapshot from `5bd5a503`; the full main log and edit replay are now preserved. The main command completed 24 probe functions, then reached its 3600 s cap in scaling; the three seam functions were not reached. Current observations below supersede the older runs where stated. No final summary or completed migration is claimed.

- **2026-10-22T13:45:08Z:** 19 preserved Tcl inputs replayed on six exact engines: **114 executions, no timeouts**. Complete outputs, exit codes and hashes: [cloud-baseline-oracles.log](triage-20261022/cloud-baseline-oracles.log). Feature-unavailable rows are retained, including Jim's unsupported `interp alias` form; they are not incorrect oracle answers. N17's capture driver registers `good` on every engine.
- [cloud-source-audit.log](triage-20261022/cloud-source-audit.log) rechecks the static seams on `81a911af`. N11's existing-command early return and N13's four/five-round caps remain. These are source inspections, not server protocol executions.
- `run5.log` is the completed initial cloud build log (with an incomplete e2e diagnostic segment, as explained below). The initial main command exited 101 for omitted embedded fixture files; the scaling command also encountered missing includes before all three trees were restored, while the compiler continued building. No named probe result is inferred from these errors. Rebasing this tracked active log replaced its inode, so the e2e cargo diagnostic segment was lost; a stable repeat is recorded separately. The exact launched source is `run5-source.rs.txt`.
- The prepared `run6.sh` restores the expanded probe into the authorised temporary path, uses a fresh output directory, records completed-world presence, seeds Jim's compiled-in surfaces as production does, covers all six rename profiles and both sibling collision origins, and distinguishes direct hook answers from optimiser folding with/without declared pure effects. It also replays all findings, scaling and the existing fold e2e test. This is prepared work, not an executed result.

- **Live-fold repeat on `53ec43d1`: 0 passed / 1 failed.** The existing `a_pack_file_folds_a_call_site_in_the_optimiser` fails at `pack_source_e2e.rs:185`: DSL 1 expected `["5"]`, got `[]`. Log: [run5-e2e-repeat.log](triage-20261022/run5-e2e-repeat.log). This confirms the supported control does not fold. The direct/pure contrast remains pending.

**Completed run6 replay:** [run6.log](triage-20261022/run6.log), [run6-oracles.log](triage-20261022/run6-oracles.log), [summary](triage-20261022/run6-oracle-summary.json). There were **96 files / 576 engine executions**, including **76 generated outputs / 126 intended-profile checks**, with **0 unexpected matched results**. Every file ran on every engine; cross-family Unicode-prefix differences are not failures of a C-profile edit. All eight N17 drivers print `good` everywhere. The main executable was built at `5bd5a503`; the later e2e cargo command rebuilt after incoming compiler changes at `90fe96e2` and still fails with no O129. A complete final latest-code check is not claimed yet.

## Method

- **Oracles.** Each Tcl repro from the review (`triage-20261022/tcl/`) was run with `triage-20261022/oracle.sh <file>` on six engines: `tclsh8.4` (8.4.20), `tclsh8.5` (8.5.19), `tclsh8.6` (8.6.18), `tclsh9.0` (9.0.4), `tclsh9.1` (9.1.0) and Jim 0.84 (`tmp/jimtcl-oracle/jimsh`). Every oracle answer matches the review's expected outcome, including feature-unavailable rows: TclOO is missing on 8.4, 8.5 and Jim, and `namespace path` on 8.4 and Jim.
- **Probes.** Current production APIs were called from one temporary test, `rust/tcl-spectcl/tests/zz_review_probe.rs` (copy: `triage-20261022/zz_review_probe.rs.txt`).
  - It was built in the shared target with `CARGO_INCREMENTAL=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_PROFILE_DEV_DEBUG=0` and run with `--nocapture --test-threads=1`.
  - `tcl-spectcl` was chosen because one build links `tcl-compiler`, `tcl-lsp-core`, `tcl-spectcl`, `tcl-spec-hooks` and `tcl-engine-tclvm`.
  - Runs: run1 at 11:01Z (safety batch), run2 at 11:20Z, run3 from 11:28Z to 12:06Z, run4 from 12:13Z to 12:39Z (stopped before scaling when the triage moved to a cloud session). The resume batch is `triage-20261022/run5.sh`. Logs are `triage-20261022/run*.log`.
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
| N07 speculative proc variable | Refused, pending R1. Definition no longer walks to the global | lsp | `rename.rs`, `definition.rs` |
| N08 inliner subset oracle + nested lowering | FIXED (edits and unit inference) | — | — |
| N09 type/call/taint | FIXED for executed constructor and call-graph repros; taint not independently executed | — | — |
| N10 OO class/relation | CHANGED: relations, qualified miss and path-based definition FIXED; path hover/type hierarchy still empty | lsp | `tcl-lsp-core/src/definition.rs:4260` (`resolve_class_target_at`) |
| N11 packages | CHANGED: `if {0}` provider classified `Conditional` and still selected; `merge_library_files` early return REPRODUCES (static); `auto_qualify` Unicode FIXED | lsp | `tcl-lsp-core/src/package_resolver.rs`, `package_resolver/reachability.rs`, `tcl-lsp-server/src/lib.rs:17823` |
| N12 incremental lifecycle | FIXED | — | — |
| N13 source environment | REPRODUCES (path not inherited); 5-round and 4-round caps present (static) | lsp, unowned (`analyser/state.rs`) | `tcl-compiler/src/analyser/state.rs:2197`, `tcl-lsp-core/src/workspace_index.rs:3354`, `tcl-lsp-server/src/lib.rs:28775` |
| N14 callback arity / semantic roles | FIXED (arity by reading the code; tokens executed) | — | — |
| N15 minifier compaction | CHANGED (refuses `MissingAlphaRenameContract`); output behaviour-preserving | lsp | `tcl-lsp-core/src/minify.rs`, `minify/original.rs` |
| N16 provider fallbacks | FIXED (4/4 rows) | — | — |
| N17 SpecTcl template | FIXED | — | — |
| N17 live hook | CHANGED: direct hooks correctly return `good`; explicit pure effects allow O129; undeclared effects suppress folding. Existing control e2e FAILS | compiler-codegen (`const_subst.rs`, per user decision 13:17Z); vm-runtime (host) | see detail |
| Seam `ns_var_global_fallback` | FIXED (static) | — | — |
| Seam `WorkspaceIndex::registry_for` | FIXED (static) | — | — |
| Seam `analyse_with_workspace_classes` | CHANGED (static): resolved input, resource inputs and base-analysis identity retained; source-entry parity not shown | lsp | `tcl-lsp-server/src/lib.rs:20036–20120` |
| Seam nested passthrough lowering | **PENDING** (run4 was inconclusive; run5 main did not execute; run6 now walks actual IR statements). Static: nested lowering now uses `analyse_script_at_site` and `entry.options()` | compiler | `tcl-compiler/src/inline_uplevel.rs:559–660` |
| Seam `insert_rebound_candidates` | NOT-APPLICABLE (removed) | — | — |
| Seam `method_barrier` | FIXED (static) | — | — |
| Seam linked-editing self-name fallback | FIXED (static, plus executed probe) | — | — |
| Seam extract/inline refactors | Safe in run4 (no behaviour-changing edit); whether this is a disabled refusal is **PENDING** the active run6 | lsp | `tcl-lsp-core/src/refactor/` |
| 4.1 scaling rows | **PENDING** run5 (run4 was stopped before scaling). Run3 measured 248 s to analyse a flat 250-proc file and 6.5 s to index it (review: 5.7 ms) | perf / lsp | `workspace_index.rs:7496`, `source_graph.rs:1627`, `code_lens.rs` |

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

### N03: variable cursor / profile

- **Status:** C Tcl FIXED; Jim CHANGED.
- **Fresh run6:** C 8.6/9.0 still edit only `caf` and `$caf`; C 9.1 joins 9.0 in selecting `a{b}` for nested braces. Under Jim, with compiled-in core surfaces explicitly seeded as production does, definition and references for `$café` are both empty and rename refuses. The earlier Jim navigation result below is historical, not the current result.
- **Evidence for `set caf A; set café B; puts $café`:**
  - Under tcl8.6 and tcl9.0, `definition` gives `(0,4,7)` (the `caf` declaration), and references give `(0,4,7)` and `(2,5,9)`. Rename to `zz` edits `caf` and `$caf`; the edited file (`out/r_n03_tcl8.6.tcl`) prints `Aé` on all C Tcl releases.
  - Under Jim, `definition` gives `(1,4,8)` (`café`), and the rename output prints `B` on Jim.
- **Nested braces** (`tcl/n03b.tcl`): definition gives line 0 (`a{b`) under 8.6 and line 1 (`a{b}`) under 9.0. The oracle prints `X}` on 8.x and Jim, and `Y` on 9.x.
- **Literal suffix:** a cursor on `bar` in `$foo-bar` gives `def=[]`.
- **Source of the fix:** `template_variable_reference_at` (9d4f72634f, post-review).
- **Caveat:** with the registry and program view (run3), C Tcl rename succeeds, but Jim rename refuses with "cannot prove the original variable selected by this occurrence".

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

- **Status:** refused, pending R1.
- **Fresh run6:** definition is empty on all six profiles; no request edits the unrelated global. C profiles report W210 and refuse with `unreferenced-edit`; Jim reports no W210 and refuses because the original variable cannot be proved. This is a safe refusal, with a separate Jim analysis gap. The server-shaped matrix and required post-R1 replay remain pending.
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
- **Scope:** taint output was not independently executed. The review's demonstrated wrong definite call edge is corrected; this does not claim a complete new taint conformance matrix.

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
- **Owner:** lsp (`tcl-lsp-core/src/minify.rs`, `minify/original.rs`; the latter shows ` M` in `git status`, an in-flight edit).
- **Fix direction:** a shared transactional rename planner, or keep the server-side refusal.

### N16: provider fallbacks

- **Status:** FIXED, all four rows.
- **Signature help:** `namespace path ` under tcl8.4 gives `None`. Under tcl8.6, `namespace pat ` gives `namespace path ?namespaceList?`.
- **Alias hover:** a call to `a` inside `n` shows `proc ::n::a`.
- **Type definition:** `puts greet` gives `[]`.
- **Completion:** both `("f","::A::f")` and `("f","::B::f")` survive.

### N17: SpecTcl template and live hook

- **Template: FIXED.** Fresh run6 again registers only `good`, with no notices/load errors, in all 16 combinations of DSL 1/2.0, direct/helper, unbraced/braced and fast-path on/off. The capture driver independently registers `good` on all six pinned engines.
- **Live-hook substitution: FIXED in direct execution.** All eight run6 controls (two DSLs × braced/unbraced reference × declared/undeclared pure effects) install with no decline or notice and directly return `Some("good")`. The earlier wrong `bad` result is gone in these cases.
- **Optimiser behavior: REGRESSED for undeclared effects.** Some live hooks do fold; the effect declaration is the distinguishing condition:

| DSL / array reference forms | Effects declaration | Direct callback | `optimise_raw` O129 |
|---|---|---|---|
| 1 and 2.0; `$arr($a:b)` and `$arr(${a}:b)` | absent | `good` in all four cases | `[]` in all four cases |
| 1 and 2.0; both forms | `world_effects none` | `good` in all four cases | `["good"]` in all four cases |

- The ordinary `mylib::strlen` control emits no optimisation even with a returned value. The existing `a_pack_file_folds_a_call_site_in_the_optimiser` independently fails for DSL 1: expected `["5"]`, got `[]` (0 passed / 1 failed; `run5-e2e-repeat.log`). This is not a claim that the hook engine or every possible live hook is dead.
- **Static cause supported by the controlled contrast:** `const_subst.rs::fold_retained_at_depth` rejects `requires_world_barrier()`, while undeclared world effects receive a conservative unknown/re-entrant footprint. Requiring old packs to add a pure declaration is not the selected compatibility fix; the user assigned compiler-codegen the scoped gate correction.
- **Owner:** compiler-codegen (`const_subst.rs`), per the recorded 13:17Z user decision. No product change was made by triage. The eight generated execution drivers await the fresh output replay after the active run finishes.

### Section 3.2 / 3.3 seams

- **`ns_var_global_fallback`: FIXED (static).** It reads `resolved_profile()` first and falls back to the catalogue string only without resolved input (`analyser/types.rs:4286`; 944d156922).
- **`WorkspaceIndex::registry_for`: FIXED (static).** It borrows the document's retained input registry, guarded by its semantic key (`workspace_index.rs:3727`, `fn registry` at 1479; 8aa4dfea79).
- **`analyse_with_workspace_classes`: CHANGED (static).**
  - Retained: `base.resolved_input`, `resource_analyser_inputs`, and a cache keyed on fingerprint, the base analysis `Arc` and resource inputs.
  - Not visibly forwarded: the source-analysis entry or seed.
  - No parity test was found.
  - Location: `tcl-lsp-server/src/lib.rs:20036–20120`. Owner: lsp.
- **Nested passthrough lowering:** **PENDING.** The run4 probe (`CompilationUnit::build_for_profile`, then counting `op: Contains` in `Debug` of `ir_module`) found no `Contains`/`Eq` ops even for the direct condition, so the count pattern was wrong.
  - The iRules case produced no inlined `Block` (the `run {...}` call inside `when` is not inlined).
  - The Tcl case inlined one `Block`.
  - The probe was rewritten for run5 to list all ops and `if` condition node kinds, and to try `call run {...}` and a proc-body form under `f5-irules`.
  - Static: nested lowering now passes `entry.options()` and the site-analysed bindings (`inline_uplevel.rs`, `try_inline_callsite`).
- **`insert_rebound_candidates`: NOT-APPLICABLE.** Removed in cf8f1bc85b and replaced by `collect_namespace_resolution` (`command_binding.rs`).
- **`method_barrier`: FIXED (static).** The global-first and first-tail `class_head_component` was removed (944d156922). `hierarchy_components` unions only retained relation candidates and widens on unknowns; test `retained_dispatch_does_not_select_an_unrelated_class_tail`.
- **Linked-editing self-name fallback: FIXED (static).** `OriginalLinkedSourceSpelling::capture` requires `invocation_targets_declaration` or a call-declaration match. The literal fallback survives only as a `#[cfg(test)]` helper. Executed (run4): for `proc greet {} {greet}` plus an unrelated `n::greet`, `linked_editing_ranges` from the declaration gives `[(0,5),(0,15)]`, the declaration and its own resolved self-call only.
- **Extract/inline refactors** (run4):
  - `inline_proc` on `[p]` in `::n::c`, with `::p` and `::n::p` both defined, returned "Inline proc 'p'" (N08 shape).
  - `extract_variable(.., "result")` was offered where `result` already exists.
  - Applying either one's `edits` left the source unchanged (`out/seam_inline.tcl` prints `LOCAL`; `out/seam_extract.tcl` prints `5|1` on six engines). Both are therefore probably `disabled` refusals, which a `Refactoring` with no edits signals.
  - No unsafe edit was observed. Run5 prints `disabled` to confirm.

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

Code lenses at **25 procedures** took **6,049.759 ms**, after **61,798.1 ms** pre-analysis, and returned **25 lenses**. The combined batch reached its one-hour cap during the later lens work, so no P50/P100 lens time is inferred. `run-pending.sh` will measure those cases separately and print analysis completion before starting the lens query, with explicit per-case budgets. Because compiler changes arrived during run6, that follow-up refreshes the complete small lens series.

Static inspection still finds the linear live-procedure/class/link scan in `workspace_command_exists_for_call` and the root-path vectors in `RunOrder`. Full server/release latency and RSS remain unmeasured. Logs and source hashes distinguish this snapshot from later implementation fixes.
