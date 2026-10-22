# Triage: naming migration review (2026-10-22) against the current `work` tree

**Final summary — 2026-10-22T19:57:30Z. Triage is complete.** Every N01–N17 finding and section 3.2/3.3 seam was rechecked on `45766fc3`, including the latest Jim/server integration. Final fetch `2831c535` has identical non-documentation content; exact tree IDs and cleanup checks are in `triage-20261022/final-verification.json`. No triage obligations remain.

N17 now passes the named existing pack-fold e2e (both DSL releases), all exact guarded-body combinations, live-hook controls and template cases. The fresh declared-effect/unclosed-argument regression passes, preserving the negative guards. Nested iRules lowering conservatively retains generic calls; the extract/inline repros explicitly disable with zero edits. All requested small scaling rows are measured, and the fresh smallest flat/lens point still has substantial cost.

After R1, ordinary C command worlds are completed. N04–N07 contested renames refuse safely; same-document N06 destination guards are exercised specifically. Import/workspace rename coverage, Jim navigation/diagnostic coverage, some OO providers, conditional package selection and source-context/convergence evidence gaps remain as detailed below. Safe refusal is not reported as completed feature support.

Final execution: **27 semantic observational probes + 1 scaling probe**, all five groups exit 0; named fold e2e **1/1**, declared-barrier regression **1/1**. Scratch sources ran on C Tcl 8.4–9.1 and pinned Jim: **107 files / 642 executions / 186 matching-profile checks, zero unexpected result**. Observational harness success does not turn the remaining limitations into fixed findings.

No product code or permanent tests were changed. Temporary probe source and all three test executables are deleted. Full server JSON-RPC integration and release gates `make rust-check` / `make prep-pr` remain **UNRUN**, under AGENTS.md's active-work development-push exception. This is completed review triage, not release approval.

Reviewed report: [naming-migration-review-20261022.md](naming-migration-review-20261022.md), against `fcb7927f02` (16 Oct). Evidence: [triage-20261022/](triage-20261022/README.md). Closed obligations: [../outstanding/review-triage.md](../outstanding/review-triage.md).

## Validation snapshots

| Evidence | Snapshot | Result |
|---|---|---|
| `run17-latest.log`, `final-probe.rs.txt` | main compiled `45766fc3`; e2e rebuilt `ff57cbcd`, same product source | 27 semantic + 1 scaling probes, all five groups exit 0; named existing pack-fold e2e passes 1/1 in 76.90s. Build 9m14s; warning retained in logs. |
| `run17-latest-oracles.log` / summary, `out/run17-latest/` | run17 generated scratch sources | 107 files / 642 engine executions, 86 generated files / 186 matching-profile checks, zero unexpected result. |
| `run17-declared-effects.log`, `run-declared-effects.sh` | `ff57cbcd` | Fresh existing declared-effect/unclosed-argument regression passes 1/1 in 82.20s; build/run/delete captured together. |
| `run17-source-audit.log`, `final-verification.json` | source identical through final fetch `2831c535` | Static seams inspected, no missing search patterns; N11 old shortcut identified as test-only. Source tree equality and cleanup recorded. |
| `run16-latest*`, `run16-declared-effects.log` | `2d1b85ad` / source-identical `0412d5e4` | Previous complete replay with the same dispositions; 642 executions / 186 matching checks, zero unexpected result. Output tree archived with per-file hashes. |
| `run14-*`, `run15-*` | `49b067ce`, `be53f58c` | Historical interrupted replay and failed integration build respectively. Run15 executed zero probes; corrected by e75345a7. Neither is a current semantic failure. |
| `run10-*`, `run11-*`, `run12-*`, `run13-*` | exact hashes in logs | Historical N17 progression: simple live hooks fixed before guarded execution recovered. Superseded by successful run16/run17 exact-body and existing e2e checks. |
| `run6.log`, `run7-pending.log` | `5bd5a503`, `0434e7e7` | Requested P25/50/100/200 flat, F250/500/1000/2000 chain and P25/50/100 lens rows. Larger rows remain historical; fresh run17 small measurements below. |

Harness passes mean observational probes completed, not that every product answer was correct. R1 landed in `ec80a189`/`79d4e834`; local run14, run16 and run17 confirm ordinary puts-call completed worlds on all five C profiles. Import-specific uncertainty and Jim coverage gaps remain. N04–N07 return no edits in the contested cases, so there is no unsafe proposed edit to execute. Returned control edits, variable-prefix edits, optimiser and minifier outputs were applied to scratch sources and executed.

Earlier sparse-checkout build failures and stable historical fold failures remain reproducible in their logs; no current semantic verdict is inferred from failed setup or superseded executables. Losslessly compressed audits and output manifests preserve provenance.

## Method

- **Oracles.** Each Tcl repro from the review (`triage-20261022/tcl/`) was run with `triage-20261022/oracle.sh <file>` on six engines: `tclsh8.4` (8.4.20), `tclsh8.5` (8.5.19), `tclsh8.6` (8.6.18), `tclsh9.0` (9.0.4), `tclsh9.1` (9.1.0) and Jim 0.84 (`tmp/jimtcl-oracle/jimsh`). Every oracle answer matches the review's expected outcome, including feature-unavailable rows: TclOO is missing on 8.4, 8.5 and Jim, and `namespace path` on 8.4 and Jim.
- **Probes.** Current production APIs were called from one temporary test, `rust/tcl-spectcl/tests/zz_review_probe.rs` (current copy: `triage-20261022/final-probe.rs.txt`; earlier copies retained).
  - It was built in this checkout’s isolated target with `CARGO_INCREMENTAL=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_PROFILE_DEV_DEBUG=0` and run with `--nocapture --test-threads=1`.
  - `tcl-spectcl` was chosen because one build links `tcl-compiler`, `tcl-lsp-core`, `tcl-spectcl`, `tcl-spec-hooks` and `tcl-engine-tclvm`.
  - Runs: run1 at 11:01Z (safety batch), run2 at 11:20Z, run3 from 11:28Z to 12:06Z, run4 from 12:13Z to 12:39Z (stopped before scaling when the triage moved to a cloud session). Cloud run6 uses `run6.sh`; the bounded follow-up uses `run-pending.sh` and `pending-probe.rs.txt`. The current complete grouped replay uses `run-latest.sh` with `final-probe.rs.txt`. Logs and hashes are in the evidence directory.
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

## Summary

| Finding / seam | Status | Owner | Current files |
|---|---|---|---|
| N01 colliding identities | CHANGED: `all_procs` still collapses; both definition targets FIXED on C 8.6/9.0/9.1; empty-tail link FIXED | compiler-names / lsp | `tcl-compiler/src/analyser/types.rs`, `analyser/handlers.rs` |
| N02 Jim colon policy | CHANGED: identities distinct; definition abstains under Jim | compiler-names (naming), lsp (definition) | `tcl-syntax/src/naming.rs`, `tcl-lsp-core/src/definition.rs` |
| N03 variable cursor/profile | C Tcl FIXED; Jim CHANGED (definition/references abstain and rename refuses) | lsp / compiler-names | `tcl-lsp-core/src/definition.rs`, original variable bindings |
| N04 imported-source rename | CHANGED: post-R1 import cases still safely refuse with unavailable completed world | lsp (planner), compiler-names (R1 world) | `tcl-lsp-core/src/original_command_rename.rs`, `rename.rs` |
| N05 forced-import rename | CHANGED: references correct; post-R1 rename still refuses import-world uncertainty | lsp, compiler-names | same |
| N06 collision checks | Same-document class/method/proc-to-class guards FIXED; workspace cases refuse at both origins for coverage, no edits | lsp | same; `rename.rs` `cross_document_symbol_edits` |
| N07 speculative proc variable | CHANGED: safe refusal confirmed after R1; Jim W210 gap remains | lsp | `rename.rs`, `definition.rs` |
| N08 inliner subset oracle | FIXED (edits and unit inference) | — | — |
| N09 type/call/taint | Constructor/call graph FIXED; taint dependency resolver conservatively retains all source candidates | — | — |
| N10 OO class/relation | CHANGED: relations, qualified miss and path-based definition FIXED; path hover/type hierarchy still empty | lsp | `tcl-lsp-core/src/definition.rs` (`resolve_class_target_at`) |
| N11 packages | CHANGED: `if {0}` provider classified `Conditional` and still selected; `merge_library_files` is test-only; production request isolation unmeasured; `auto_qualify` Unicode FIXED | lsp | `tcl-lsp-core/src/package_resolver.rs`, `package_resolver/reachability.rs`, `tcl-lsp-server/src/lib.rs` |
| N12 incremental lifecycle | FIXED | — | — |
| N13 source environment | Namespace-only API limitation REPRODUCES; server inheritance unmeasured; 5-round/4-round caps remain (static) | compiler-names / lsp | `tcl-compiler/src/analyser/state.rs`, `tcl-lsp-core/src/workspace_index.rs`, `tcl-lsp-server/src/lib.rs` |
| N14 callback arity / semantic roles | FIXED (arity by reading the code; tokens executed) | — | — |
| N15 minifier compaction | CHANGED (refuses `MissingAlphaRenameContract`); output behaviour-preserving | lsp | `tcl-lsp-core/src/minify.rs`, `minify/original.rs` |
| N16 provider fallbacks | FIXED (4/4 rows) | — | — |
| N17 SpecTcl template | FIXED | — | — |
| N17 live hook | FIXED in requested repros: exact guarded body folds in all 8 contexts; named pack e2e passes; declared barriers preserved | — | `run17-latest.log`, `run17-declared-effects.log` |
| Seam `ns_var_global_fallback` | FIXED (static) | — | — |
| Seam `WorkspaceIndex::registry_for` | FIXED (static) | — | — |
| Seam `analyse_with_workspace_classes` | CHANGED (static): resolved input, resource inputs and base analysis retained; source-entry parity not shown | lsp | `tcl-lsp-server/src/lib.rs` |
| Seam nested passthrough lowering | CHANGED: iRules retains generic calls and declines inlining; Tcl control inlines with parsed condition. Context forwarding FIXED (static) | compiler | `tcl-compiler/src/inline_uplevel.rs` |
| Seam `insert_rebound_candidates` | NOT-APPLICABLE (removed) | — | — |
| Seam `method_barrier` | FIXED (static) | — | — |
| Seam linked-editing self-name fallback | FIXED (static, plus executed probe) | — | — |
| Seam extract/inline refactors | CHANGED: explicitly disabled refusals with zero edits in both repros | lsp | `tcl-lsp-core/src/refactor/` |
| 4.1 scaling rows | Substantial flat/index/chain/lens costs REPRODUCE at the requested small sizes | perf / lsp | `workspace_index.rs`, `source_graph.rs`, `code_lens.rs` |

## Per-finding detail

The dispositions below were repeated in run17 unless explicitly labelled historical or static. Each row distinguishes executed production API behavior, oracle semantics and server-only evidence gaps.

### N01: colliding / display identities

- **Status:** CHANGED. The `all_procs` display-key map still collapses two slots. Navigation now distinguishes both declarations; the empty-tail part remains FIXED.
- **Executed result (run6, repeated in run17):** on C 8.6, 9.0 and 9.1 the calls in `{p}]` and `{:p}]` target `(0,24)` and `(1,24)` respectively. This is corrected navigation despite the single `::a:::p` metadata entry; the map collision alone is not evidence that these current definition calls select the wrong entity.
- **Evidence:** `Analyser::new().analyse(src, "tcl8.6"|"tcl9.0")` on the review's `a:`/`:p` script (`tcl/n01.tcl`) gives `all_procs=["::a:::p"]`: one entry for two runtime slots. `diags=[]`, so the review's false unknown-command diagnostic is gone.
- **Oracle:** 8.6, 9.0 and 9.1 print `LEFT|RIGHT`. 8.4 and 8.5 reject `proc :p`. Jim prints `RIGHT|RIGHT`.
- **Empty tail:** for `tcl/n01b.tcl`, `WorkspaceIndex::command_links()` gives `("::a", "::n::")`. The target is preserved, so this part is FIXED. The oracle prints `EMPTY` on all C releases.
- **Owner:** compiler-names / lsp. The legacy `all_procs: HashMap<String, ProcDef>` projection remains in `analyser/types.rs`; typed occurrence metadata now prevents the wrong navigation targets in this repro. This distinguishes display-map debt from current wrong identity settlement.
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
- **Executed result (run6, repeated in run17):** for `set caf A; set café B; puts $café`, all five C profiles edit the `caf` declaration and only the `$caf` prefix. Each matching-engine output is `Aé`. Under Jim, with core surfaces seeded as production does, definition/references are empty and rename refuses because the original variable cannot be proved. No Jim output edit is returned.
- **Nested braces:** C 8.6 selects `a{b`; C 9.0/9.1 select `a{b}`. The oracle prints `X}` on C 8.x/Jim and `Y` on C 9.x. A cursor on the literal `bar` suffix of `$foo-bar` gives no definition.
- **Fresh run7 parser debt check:** scalar names `a(b` and `a(b)c` remain intact under both `FirstClose` and `Tcl9Nesting`. The valid first-close `${a{b}(status)` script has no diagnostics on C 8.4/8.5/8.6/Jim, although the profileless `is_braced_indirect_array_ref` helper returns false. That helper alone is not evidence of a current diagnostic failure. The same script is invalid on C 9.x, as recorded by the oracles.
- **Source:** `template_variable_reference_at`, original-variable proofs and the shared style-aware array-name splitter. Remaining Jim provider coverage belongs to lsp/compiler-names.

### N04: imported-command source rename

- **Status:** CHANGED to safe refusal, confirmed after R1 on `49b067ce` and repeated on `45766fc3`.
- Wildcard and exact imports refuse on all six profiles. C 8.4/8.5/8.6/9.0/9.1 report unavailable completed command world; Jim cannot prove the original command at the cursor. No declaration-only edit or stale import/call edit is offered.
- The same fresh executable gives a completed world for ordinary `proc foo; puts [foo]` on all five C profiles. Thus the general R1 blocker is repaired; import-specific world/coverage uncertainty remains.
- The preserved original wildcard input prints `1` on all six engines. There is no returned N04 edit to execute.
- **Owner:** compiler-names (import-specific facts) / lsp (transactional rename). Supported edits must cover imports/exports/derived uses; refusal contains the original unsafe behavior.

### N05: forced-import rename vs references

- **Status:** CHANGED to safe refusal, references correct after R1.
- Fresh run17 references include only local declaration `(2,9)` on all six profiles; rename refuses with UnknownWorld and returns no edit. The forced-import call is no longer renamed to the unrelated local implementation.
- Ordinary C `puts [foo]` controls now have a completed world but still refuse `incomplete-original-command-coverage`; bare `foo` renames declaration/call. `set x [foo]` also refuses coverage. Jim's ordinary control still lacks a completed world.
- The original forced-import input prints `A` on all engines. No N05 edit was returned for execution.
- **Owner:** lsp/compiler-names. Remaining limitation is import-specific closure and complete occurrence coverage, not the former broad R1 false guard.

### N06: rename collision checks

- **Same-document guards: FIXED in tested cases after R1.** On C 8.6/9.0/9.1, class A→B and proc P→existing class C refuse `original-command-collision`; method old→dest refuses `original-member-collision`. All have completed worlds and return zero edits. This now exercises destination-specific protection rather than the former broad world refusal.
- **Workspace cases: CHANGED / contained by refusal.** Both declaration and call origins refuse on all six profiles. All C variants report incomplete original coverage; Jim reports UnknownWorld from the declaration and original-command-unavailable from the call. `cross_document_symbol_edits` returns zero, workspace rename is unavailable, and lexical workspace rename advice remains false. This does not prove a workspace destination check becomes correct if coverage is later enabled.
- Collision-free class A→Z still refuses incomplete source coverage on all three OO profiles. The successful bare-call control and variable-prefix edits are replayed on all engines; no collision-case edit was returned to execute.
- **Owner:** lsp (`original_command_rename`, cross-document helpers and server augmentation). Preserve refusal until workspace/source coverage and destination checks can be proved together.

### N07: speculative proc-variable binding

- **Status:** CHANGED to safe refusal, required post-R1 check complete.
- Definition from `$g` is empty on all six profiles; the unrelated global is never edited. All C profiles report W210 and refuse `unreferenced-edit`; Jim has no W210 and refuses because the original variable cannot be proved. The Jim diagnostic gap remains.
- Server-shaped C cases have a completed world and still refuse for occurrence coverage, so this safety result is independent of the old R1 blocker. No N07 edit is returned to execute.
- **Owner:** lsp/compiler-names. An edit needs a proven occurrence-to-cell edge.

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
- **Executed result (run6, repeated in run17):** the nearer class gives `Object(::n::Foo)`. A nearer non-class procedure blocks global class inference, leaving `::n::c` `Overdefined`.
- The path fixture now has a `::n::c` summary with `direct_calls=["::lib::p"]`; the other summaries are `::lib::p` and `::p` with no direct calls. The earlier missing-caller result is no longer current. C Tcl's expected result is `LIB`.
- The legacy analyser `resolved_qualified_name` field is `None` for the `p` occurrence; the current call graph does not choose the wrong global procedure.
- **Scope:** taint lattices and warning outputs were not independently executed. The review's demonstrated wrong definite call edge is corrected; this does not claim a complete new taint conformance matrix.

- **Run9 (`128e1bb4`), repeated in run17:** constructor `::n::c` returns `Object(::n::Foo)`; a nearer non-class `::n::Foo` widens to `Overdefined`. Direct calls are only `::n::c → ::lib::p`. The public taint `resolved_callees` projection retains all three source ProcedureIds for `::n::c`, and none for either leaf. This eliminates the old wrong singleton from that projection by conservative widening; taint lattices and warning precision were not independently validated. Evidence: `run9-n09.log`.

### N10: OO class / relation resolution

- **Status:** CHANGED.
- **Relation site (run6, repeated in run17):** `supertypes("::B::Child")=["::A::Base"]`, matching C Tcl 8.6–9.1. FIXED.
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
- **Server scope (static):** `merge_library_files` and its `ensure_autoload_indexed` caller are `#[cfg(test)]`. Their any-existing-command shortcut does **not** establish a current production bug. Production `ensure_logical_library_candidates` starts from checked source coordinates, publishes selected library analyses, then revalidates caller text, resolver revision and the exact request before reading current candidates. `required_library_files` uses each source requirement/preference and resolved profile. The review’s multi-request provider-isolation scenario was not executed; no full server fix or regression is claimed.
- **Owner:** lsp (`package_resolver.rs`, `package_resolver/reachability.rs`, `tcl-lsp-server/src/lib.rs`).
- **Remaining evidence gap:** the public API probe still offers the conditional registration. A server multi-request test must establish provider eligibility and provenance for the actual requesting view; the test-only helper cannot answer that.

### N12: incremental lifecycle invalidation

- **Status:** FIXED.
- **Evidence** (old and new texts as in the review, with the `rename` at the same offset): the warm old index gives `(false,false)`. After `replace_document` it gives `(true,true)`, the same as a fresh index on the new text, which also gives `(true,true)`.
- **Oracle:** `out/n12_old.tcl` prints `0` and `out/n12_new.tcl` prints `1`, on six engines.

### N13: source environment / convergence

- **Status:** REPRODUCES in the explicit namespace-only API repro; server integration remains unmeasured.
- **Evidence:** `Analyser::analyse_with_source_namespace("proc run {} {helper}", "tcl8.6", "::app")` resolves `helper` to `::app::helper`.
- **Oracle** (`tcl/n13p.tcl`, path `::lib`): `good` on 8.5–9.1.
- **Static:** `imported_constants` has `ROUND_CAP = 5` (`workspace_index.rs`), and the server's `refresh_source_rehoming_locked` loops `for _round in 0..4` (`tcl-lsp-server/src/lib.rs`). The seed is still a namespace string (`analyser/state.rs`).
- **Scope:** the call supplies only a namespace string, so its result demonstrates that API’s context limitation. It does not execute a full source-point server request, nor establish that every newer source-entry path loses inherited context.
- **Owner:** compiler-names (`analyser/state.rs` source inputs) / lsp (`workspace_index.rs`, server).
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

- **Status: FIXED in the requested current repros (run17).** Both DSL releases register the expected template value across all 16 combinations. All five original live-hook controls fold `good` or `5`. All eight DSL/reference/effects combinations return and optimise to `good`, including absent explicit effects.
- **Exact guarded strlen body now works:** all eight DSL1/2 × optimiser None/Some(tcl8.6) × return/no-return cases return direct `Some(5)` and emit O129 `5`; installation has no decline. This is the body used by the existing pack test, not just an unconditional length substitute.
- **Named existing test:** `pack_source_e2e::a_pack_file_folds_a_call_site_in_the_optimiser` passes 1/1 in 76.90s on `ff57cbcd`, product source identical to `45766fc3`; it asserts both DSL releases. Fresh existing `a_live_fold_keeps_declared_effects_and_unclosed_arguments` passes 1/1 in 82.20s (`run17-declared-effects.log`). Declared writes/callbacks and unclosed arguments remain barriers.
- **Ground truth:** `tcl/n17-ascii-hook.tcl` prints `ascii=1|hook=5` on C 8.4–9.1 and pinned Jim. Fresh folded drivers are included in the 186 successful matching-profile checks.
- **Historical distinction:** `fd5c12bf` repaired the overly broad footprint barrier. Later run12/run13 still installed the guarded body but returned no fold; the separately attributed owner trace reported missing native compiler preflight provider refusal. Those failures no longer reproduce after the reviewed integration used by run16 and run17. This does not establish which individual internal change caused the successful guarded execution, or justify bypassing a provider guard.
- No product/test edits were made by triage. Earlier 4/4 `variable_reference_boundaries` execution is retained in `run11-variable-boundaries.log`; the relevant negative regression was rebuilt and rerun fresh above.

### Section 3.2 / 3.3 seams

- **`ns_var_global_fallback`: FIXED (static).** It reads `resolved_profile()` first and falls back to the catalogue string only without resolved input (`analyser/types.rs`; 944d156922).
- **`WorkspaceIndex::registry_for`: FIXED (static).** It borrows the document's retained input registry, guarded by its semantic key (`workspace_index.rs`, `fn registry` at 1479; 8aa4dfea79).
- **`analyse_with_workspace_classes`: CHANGED (static).**
  - Retained: `base.resolved_input`, `resource_analyser_inputs`, and a cache keyed on fingerprint, the base analysis `Arc` and resource inputs.
  - Not visibly forwarded: the source-analysis entry or seed.
  - No parity test was found.
  - Location: `tcl-lsp-server/src/lib.rs`. Owner: lsp.
- **Nested passthrough lowering: CHANGED / conservative decline.** Run7 and run17 walk actual `Statement` nodes from every top-level/procedure/body-unit root. Both iRules forms (`call run` in an event and `run` in a proc) retain generic `if`/dispatcher calls, with no inlined block and no typed direct condition. The old `Binary(Contains)` direct versus `Raw` inlined mismatch is therefore not present on this path; it would overstate the evidence to claim that specialised iRules inlining is working. The ordinary Tcl control produces one inlined `Block` and a parsed `Binary(StrEq)` direct condition; the literal inlined `if` is retained generically.
  - Static context forwarding is repaired: `try_inline_callsite` obtains site-analysed bindings with `analyse_script_at_site`, calls `set_source_analysis_options(entry.options())`, and lowers with those bindings. A declined specialisation is permitted by the lowering contract; no product fix was made by triage.
- **`insert_rebound_candidates`: NOT-APPLICABLE.** Removed in cf8f1bc85b and replaced by `collect_namespace_resolution` (`command_binding.rs`).
- **`method_barrier`: FIXED (static).** The global-first and first-tail `class_head_component` was removed (944d156922). `hierarchy_components` unions only retained relation candidates and widens on unknowns; test `retained_dispatch_does_not_select_an_unrelated_class_tail`.
- **Linked-editing self-name fallback: FIXED (executed and static).** `OriginalLinkedSourceSpelling::capture` requires `invocation_targets_declaration` or a call-declaration match. The literal fallback survives only as a `#[cfg(test)]` helper. Executed again in run17: for `proc greet {} {greet}` plus an unrelated `n::greet`, `linked_editing_ranges` from the declaration gives `[(0,5),(0,15)]`, the declaration and its own resolved self-call only.
- **Extract/inline refactors: explicit disabled refusals.** Run7 and run17 report:
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

Run14 output storage: `triage-20261022/out/run14-final.tar.gz` retains all 71 generated files (68 root Tcl outputs plus package fixtures); `run14-final.manifest.json` verifies every byte. The interrupted output tree is archived to keep evidence below 5 MB; the current complete replay will retain its fresh outputs directly.

Evidence storage: historical run6 output files are losslessly archived as `out/run6.tar.gz` with a per-file SHA-256 manifest; prior oracle logs remain unchanged. This preserves room for the complete latest-tree raw outputs under the 5 MB evidence limit.

Fresh run16 and run17 check the smallest flat/lens point and full small source-chain range again:

| Workload | Run16 `2d1b85ad` wall ms | Latest run17 `45766fc3` wall ms |
|---|---:|---:|
| P25 flat analysis / index / 100 missing queries | 23,226.7 / 559.647 / 1.591 | 35,913.0 / 574.821 / 1.111 |
| F250 / F500 / F1000 / F2000 chain build | 76.024 / 352.855 / 813.077 / 3,669.177 | 61.876 / 228.197 / 902.135 / 4,691.509 |
| P25 lens pre-analysis / query / count | 22,906.4 / 5,100.675 / 25 lenses | 29,251.4 / 4,784.873 / 25 lenses |

Index/query/chain values are minima of three; analysis and lens values are single measurements. The helper named `cpu_ms` uses `Instant`, so these are wall times, not CPU medians. Larger flat/lens rows above belong to historical run6/run7 snapshots and are not relabelled as latest measurements. Substantial costs persist at the current small point; no causal speedup/regression, release latency or RSS conclusion is inferred from these unoptimised wall-time samples.

Run16 outputs are losslessly archived in `out/run16-latest.tar.gz` with a per-file manifest after all 186 checks completed; all 104 files were byte-verified before removing the historical raw tree. Run17 retains the latest raw outputs.
