# Hand-over: the `server` issue session (#1657, #2543, #2243)

Written 18 November 2026 by the outgoing `server` issue session (session_017u5g3HDHiPN1CimB7QiVSF),
paused on the owner's instruction. Start here. It replaces `infra.md` as the place to start; read
`infra.md` only for the older #2129/#1734 history.

## Issues

### #1657: closed

The server stopped answering under spec-pack churn. Closing comment: 6963434343, with headless evidence.
Four causes, one fix each, each with a test on `work`:

| Fix | Test (name it to find it) |
| --- | --- |
| Deep diagnostics pass kept polled while the fast tier publishes (futurelock), originally e54c33e2f051f20cccc4c9fa36af6461a8377731 | `a_parked_deep_pass_does_not_strand_the_lock_the_fast_tier_needs_1657` (tcl-lsp-server `src/tests/diagnostics/run.rs`) |
| Index seed polled while its currency check waits (`until_document_superseded`, futurelock), originally d3659a350ff173ca2f14749f0fa63a73b274059b | `a_seed_queued_on_the_map_is_polled_while_the_currency_check_waits_1657` (`source_publication/live_index/tests.rs`) |
| Evaluator epoch kept when a republished hook plan runs the same bodies: dd77e9313f0f2c196413d5540ea4b713d5a46611 | `an_unchanged_hook_plan_keeps_the_evaluator_epoch_1657` (tcl-spectcl `tests/hook_plan_republish.rs`) |
| Bounded diagnostics admission, one permit per analysis pass (`diagnostics/admission.rs`): 0f09b12e096894006635682fc77ab5b52051eed6 | `a_reload_over_many_documents_admits_at_most_the_bound_1657` (`src/tests/diagnostics/scheduling.rs`) |

**History note.** `work` was re-rooted at cbc50aa7 (a root commit with no parents). e54c33e2, d3659a35
and 446fa3ba (the first driver commit) are not ancestors of `work`, but their content is in it. The four
tests above and `scripts/dev/pack_churn_stress.py` are present on `work`.

**Stated in the closing comment:**
- the parent of e54c33e2 did not wedge in 7 runs;
- the CI `test-ext` witness was never obtained.

### #2543: open (memory growth under pack churn)

Split from #1657 and assigned here. The ruling in force: **keep it open and attribute the residue under
it**. It closes only when all of these hold:
- the 250-round run levels off;
- the two closing conditions below are met.

**Cause, attributed and fixed.** `CfgContext` and `FnLatticeKey` (tcl-lsp-db `function_lattices.rs`) were
keyed by `ProcedureId {scope, ordinal}`. The scope comes from the process-global `NEXT_SCOPE`
(`tcl-core-types/src/body_identity.rs`), so every rebuild of an unchanged document interned new keys and
new `FunctionUnit` memos. The server rebuilds on every pack reload, because the per-file configuration
alternates between two values and there are more open documents than the per-file memos hold.
The attribution is in comment 6964564160 and the inventory of id-carrying fields in 6964803633.

**Burn-down fixes**, all by `analysis-performance` (session_017u5g3LmJPuBQtB5DVx5VsA) and all on `work`:

| SHA | What |
| --- | --- |
| ac28944646906c8859df6fc20dd2b0819e024cef | Per-document lowered-module memo, keyed by the value of every lowering input (`tcl-compiler/src/compilation_unit/module_lowering_memo.rs`, `lowered_document_module` in tcl-lsp-db `memoised_units.rs`) |
| 43d017e072f45fc343775b049a704a14c080a47a | The document unit and the taint solve routed through the memo; analysis-context generations keyed by value, not `Arc` pointer |
| 8febb23f942c602ec189f7ee3fb905adcb2b0cfe | The module kept in a table outside salsa. This session found the cause: salsa's interned GC reclaims a LOW-durability `ModuleLoweringKey` untouched for 3 revisions, and the memo goes with it. Under server traffic the key was reclaimed and re-interned every few rounds. |
| 2a026629995ee8172ba5b7ecf18202ca9cc8d295 | The two closing conditions, below |

**Tests on `work`** (tcl-lsp-db `tests/it_5/memory_growth.rs`):
- 3fff2d039a73e93af9f60c6e9c23b87ac415214c: `an_unchanged_document_reanalysed_under_alternating_configuration_interns_nothing_new_2543`, the bounding test.
- a78bcdc0a39cece0254473d6b09d2ee94591401f: `an_uncertifiable_body_reanalysed_under_alternating_configuration_interns_nothing_new_2543`, which pins the fallback tier.
- 261f4c35ec3e61ddad41470ab3fe2060fdf4a1b5: `equal_procedure_ordinals_never_share_a_lattice_between_different_analyses_2543`, the collision guard. It requires two documents and the 8.6/8.4 `dict` case to match a fresh database.
- Added by `analysis-performance`:
  - `more_documents_than_the_memo_caps_…_plateau_2543`
  - `many_revisions_between_configuration_flips_keep_the_lowered_module_2543`
  - `past_capacity_only_the_least_recently_used_module_goes`
  - `a_dropped_database_releases_its_retained_modules`

**Measurements.** Comments 6963439501 (before) and 6976846727 (after). The details are under *How to measure*.

| Run | Before (0f09b12e era) | ac289446 | 43d017e0 | 8febb23f |
| --- | --- | --- | --- | --- |
| 40 documents, round 60: CfgContext / FnLatticeKey | 316 / 803 | 78 / 177 | — | — |
| 40 documents, round 200: CfgContext / FnLatticeKey / ModuleLoweringKey | — | 219 / 515 / 29 | 202 / 432 / 53 | 64 / 166 / 58, flat from round 150 |
| 40 documents, trimmed heap at round 100 → 200 | 3.09 → 4.04 GB | 2.40 → 2.88 GB | 2.44 → 3.16 GB | 2.18 → 2.45 GB |
| 183 documents, RSS at rounds 150 / 250 | runs A 7.76 / 7.72 GB, B 9.12 / 9.24 GB | — | — | 6.30 / 6.48 GB |
| 10 documents, trimmed heap at round 100 → 200 | — | — | — | 1,045 → 1,097 MB; counts flat at 17 / 34 |

2a026629 has **not** been measured yet.

**The residue.** With every salsa count flat, the trimmed heap still grows:
- about 2.7 MB a round at 40 documents;
- about 0.5 MB a round at 10 documents;
- about 1.9 MB a round of RSS at 183 documents.

Attribution so far:
- **Massif at 10 documents on 8febb23f** (depth 30, snapshots at rounds 100 and 200; the data is on `issues/server-2`). The useful heap is 444 → 541 MB, so it is **live data, not fragmentation**. The growth is all under command-binding analysis:
  - `SourceCommandBindings::walk_source` and its callees, about +49 MB;
  - `realm::document_realm_bindings_with_resolved_input` and `Analyser::document_command_realm`, +26 MB;
  - `ModuleCommandBindings::original_registry_command_paths` and `RegistryMetadataSlotIndex::new`, +19 MB;
  - salsa `deep_verify_edges`, +10 MB;
  - `file_project_source_facts`, +9 MB.
- `msroots.py` finds no salsa query frame above most of it. That is probably stack truncation at depth 30. A depth-150 run was taken, but its round-200 snapshot raced the shutdown and came out empty; the fixed rerun was killed by the pause.
- **Caveat:** under valgrind the server lags the driver, so the round numbers are approximate.
- **Not yet known:** which structure keeps these alive. Candidates: per-document command realms or `ModuleCommandBindings` cached per configuration generation, or the analyser's per-item and incremental paths. Those build through `analyse_per_item`, not `CompilationUnit`, and nothing in the lattice-key work touched them.

**Closing conditions sent to `analysis-performance`**, from this session's review of 8febb23f (the parent made them conditions):
1. *The capacity cliff.* At 512 entries the table did a clear-all, which brings the full leak back on a workspace with more than 512 (document, configuration) pairs. 2a026629 now evicts the LRU entry. **Still open:** the parent asked for a test that runs past the cap and shows CfgContext/FnLatticeKey plateau. The test added (`past_capacity_only_…`) checks only the order of a synthetic table. I asked for a settable capacity and a run at cap 8 with 12 documents × 2 configurations. No reply had arrived at the pause.
2. *Entries outliving their database.* The table was process-global, keyed by a per-database salsa id. 2a026629 makes it a salsa singleton input on the database, and `a_dropped_database_releases_its_retained_modules` shows release. **Met.**

### #2243: parked

The memoised and uncached compiler checks differed on T100 for `$argc` in tcllib 2.0 `imap4.tcl`. Today
both paths report 0 checks for the whole file, so their agreement says nothing. The analysis walk never
reaches the reads. The last comment is 6949881698. The minimal fixture (`tcl8.6`):

```tcl
if {[info script] eq $argv0} {
    set argc [gets stdin]
    lassign $argv server user pass
    if {$argc > 3} {puts big}
    puts "reached argc=$argc server=$server"
}
```

- **Fixture behaviour:** memo and uncached both give 0 checks. Removing the `lassign`, or moving the body to top level, restores T100 on `$argc`. Any `lassign` in a nested body (`if`, `proc`, `foreach`) ends the walk.
- **tclsh check:** tclsh 8.6.18, 9.0.4 and 9.1.0 all run the body to the end (`echo 7 | tclsh f.tcl h u p` prints `big`, then `reached argc=7 server=h`).
- **Owner:** the walk, `analyser-binding` (session_017u5g3HAPLcJWWX93pMpk32), which has the fixture.
- **What it waits on:** that walk fact. When it lands, re-run `compiler_check_memo_matches_uncached_imap4_argc` with the injected-taint control, then decide.
- `analysis-performance` doesn't expect its object-handle comparison fault to bear on #2243. It will re-run imap4 after its fix.

### Multi-second probe latency: not tracked by any issue

The driver's `tcl-lsp.getEffectiveConfig` probe has peaked at 20.8 s (run A on 0f09b12e) and 17.9 s
(183 documents on 8febb23f). Run B on 0f09b12e peaked at 1.9 s.

- Neither 20.8 s nor 17.9 s run injected anything: no trim and no memory command. gdb only attaches on a wedge, of which there were none. So the latency is the server's own. Comment 6976881904 on #2543 says so.
- 3a82a00a now records `[round, seconds]` for every probe over `--slow-probe` (default 2 s) in the `end` event. No run has used it at 183 documents yet.
- The owner has been asked through the parent whether it gets its own issue.

## On `work`: this session's commits

| SHA | One line |
| --- | --- |
| e37b816848eaac1a8f9c74fda2ef9c15771c055c | pack_churn_stress: options for wedge and memory attribution runs |
| dd77e9313f0f2c196413d5540ea4b713d5a46611 | Keep the evaluator epoch when a published hook plan runs the same bodies (#1657) |
| 0f09b12e096894006635682fc77ab5b52051eed6 | Admit a bounded number of documents to diagnostics analysis at once (#1657) |
| 367f64dce803f32e78c8585522db085dfd358adf | pack_churn_stress: report the probe latency distribution and CPU ticks |
| 3fff2d039a73e93af9f60c6e9c23b87ac415214c | #2543 bounding test (cherry-picked by analysis-performance, authorship kept) |
| 261f4c35ec3e61ddad41470ab3fe2060fdf4a1b5 | #2543 collision guard test |
| a78bcdc0a39cece0254473d6b09d2ee94591401f | #2543 fallback-tier bounding test |
| 3a82a00a4a931dec914faeec11a06a17bfd8eadb | pack_churn_stress: record the round of every slow probe (#2543) |

Present by content only, after the re-root at cbc50aa7: e54c33e2, d3659a35 and 446fa3ba (see #1657).

## Only on branches

`issues/server-2` at 673e7b940311dda14194bb6bf8c6c3fc8721cdf3: `scripts/dev/wip/server/`. Unfinished
measurement tools are listed in its README:
- `debug-memory.patch` and `debug_memory.rs`, the temporary `tcl-lsp.debugMemory` salsa-count command;
- `cfg_log_instrument.py`, the lowering and CfgContext log;
- the massif wrapper, `snap_at.py`, `msdiff.py` and `msroots.py`;
- the depth-30 massif snapshots in `data/`.

None of it is ever to be committed to `work`.

## How to measure

- **Build.**
  - Command: from `rust/`, `CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_OPT_LEVEL=2 CARGO_PROFILE_DEV_DEBUG=0 cargo build -p tcl-lsp-server --bin tcl-lsp-server`. Never `--release`.
  - Takes 10–35 minutes on 4 cores.
  - Copy `target/debug/tcl-lsp-server` out, because the next build overwrites it.
  - For salsa counts, first `git apply scripts/dev/wip/server/debug-memory.patch` and copy `debug_memory.rs` to `rust/tcl-lsp-server/src/commands/`. Revert both after copying the binary out.
- **Driver.** `python3 scripts/dev/pack_churn_stress.py --server-bin BIN --workdir DIR --log DIR/run.jsonl …`. It copies the fixture workspace and opens documents up to `--max-documents` (default every small `.tcl` file, 183 here). Each round:
  - writes or deletes `.tcl-lsp/extensions.tclspec`, with one fixed body: create, then change with identical bytes, then delete;
  - sends `workspace/didChangeWatchedFiles`;
  - meanwhile, a probe thread sends `tcl-lsp.getEffectiveConfig` every 0.5 s, with a 120 s timeout as the wedge criterion.

  Options:
  - `--rounds`
  - `--rss-every N`
  - `--malloc-trim` (calls `malloc_trim(0)` through gdb before each reading, giving `trimmed_kib`)
  - `--memory-command tcl-lsp.debugMemory` (salsa `structs`/`queries` counts as [count, bytes])
  - `--gdb` (stacks on a wedge)
  - `--slow-probe`
  - `--probe-timeout`
  - `--distinct-bodies`
  - `--reopen`
  - `--close-at-end`
- **Commands used for the tables above.**
  - 40 documents, counts: `--rounds 60 --max-documents 40 --rss-every 10 --memory-command tcl-lsp.debugMemory`.
  - 40 documents, trimmed heap: `--rounds 200 --max-documents 40 --rss-every 25 --malloc-trim --memory-command tcl-lsp.debugMemory`.
  - 183 documents: `--rounds 250 --rss-every 10 --gdb`.
  - 10 documents: as for 40 with `--max-documents 10`.
- **Reading the results.** From the JSONL:
  - `rss` events give `rss_kib`, `trimmed_kib`, `threads` and `cpu_ticks`; CPU in cores is Δticks/100/Δt.
  - `memory` events carry `report.structs["CfgContext"|"FnLatticeKey"|"ModuleLoweringKey"]` and `report.queries["alloc::sync::Arc<tcl_compiler::compilation_unit::FunctionUnit>"]`.
  - The `end` event carries `answered`, `latency{median,p95,max}` and `slow`.
  - Salsa interned counts are high-water slot counts; they never go down.
- **Massif.** `OUT=… SERVER=… scripts/dev/wip/server/massif-server.sh` as `--server-bin`. Run `snap_at.py LOG PREFIX 100 200` alongside, with `--rounds` well beyond the last snapshot (a snapshot takes about 2 minutes, and the driver must not shut the server down meanwhile). Then `msdiff.py` and `msroots.py`. The full workload is not feasible under valgrind; 10 documents is.
- **Never concurrent:** a measurement and a cargo build; two measurements; anything during a trimmed-heap run. One cargo build at a time.

## Rulings in force

- #2543 stays open and the residue is attributed under it (parent, 18 November).
- The two closing conditions above.
- `analysis-performance` owns the memo/key code. This session supplies tests, review and measurement, and does not edit `function_lattices.rs`, `memoised_units.rs` or the compiler capsule.
- Fix shape (A) for #2543: scope-free keys with rebinding, not a deterministic scope.
- #1657 closed on headless evidence. The diagnostics permit covers one analysis pass and is never held across a re-debounce.
- No `--release`; dev or opt-level-2 dev only. Never force-push `work`. WIP goes to `issues/server-2`, synced with a `git commit-tree` merge.
- Before each push to `work`:
  - `cargo check` of the touched crates and their dependents;
  - clippy `-D warnings -A clippy::assert_is_empty`;
  - `make xtask-gates`;
  - zero new failures against a baseline.
- Read `docs/work/test-scrub/README.md`. A fix updates its ledger row's `status`. This session has no ledger of its own; the #2543 rows are in `analysis-performance.tsv`, including rows 37 and 39.

## Dead ends

- **Salsa backdating.** Re-execution swapping in a new module under a flat key was ruled out: the log showed no (file, key) pair lowered twice.
- **Allocator fragmentation as the residue.** Massif's useful heap grows too, so the residue is live data.
- **An idle wedge from `malloc_trim` injection.** One 5-document run wedged with threads on a glibc arena lock. Three runs without injection were clean, so it was treated as a gdb artefact and disclosed on #1657.
- **`a_genuinely_unknown_command_still_draws_w123`.** server-e2e-db claimed 0f09b12e regressed it. The test fails alone with and without the commit, so the claim was withdrawn.
- **Restoring cached fallback units onto a rebuilt module.** It is lossy: `PointResolveContexts::restore_source_tokens` marks argument points unknown. So the fix lowers modules by value instead.

## Next steps, in order

1. Measure 2a026629 with the 40-document trimmed run (200 rounds, counts). Confirm the counts stay flat and the residue is unchanged.
2. Press `analysis-performance` for the past-cap plateau test (condition 1), or take a refusal to the parent.
3. Attribute the residue:
   - redo the depth-150 massif run at 10 documents, with snapshots at 100 and 200 and at least 300 rounds;
   - run `msroots.py` to name the owning query or structure;
   - send the owner a fixture.
   If it's in the analyser's per-item or realm caches, the owner is probably `analysis-performance` or `analyser-binding`; ask the parent.
4. With the residue fixed, run 183 documents for 250 rounds with `--slow-probe 2`. Post the table and the slow-probe rounds on #2543, and close it if it levels off and both conditions are met.
5. #2243: re-check when `analyser-binding` lands the nested-`lassign` walk fact.

## Hazards

- **Disk.** The `target/` directory reaches 25 GB, and the allowance runs out mid-build ("No space left on device"). Remove `target/debug/incremental`, and use `find target/debug/build -mindepth 2 -maxdepth 2 -type d -mmin +240 -exec rm -rf {} +` for stale hash directories. Keep only the server binaries you need (about 175 MB each).
- **Server RSS.** Under the 183-document churn the server holds 6–9 GB. On a 15 GB box, run nothing else alongside.
- **valgrind.** The server lags the driver, so probes are slow; set `--probe-timeout 1800`.
- **A shared `CARGO_TARGET_DIR`.** Building another worktree into it serves stale rlibs (AGENTS #1052).
- **Known failing tests that are not this session's regressions:**
  - tcl-lsp-db `memory_growth::edit_session_memory_growth_plateaus` (noisy; numbers in ledger row 39) and `interned_slots_stay_bounded_across_an_edit_session`;
  - `tests::check_solve::proc_summary_cascade_reused_on_unrelated_edit` and `taint_cascade_reused_on_unrelated_edit` (they fail identically before #2543's fix);
  - tcl-lsp-server e2e `a_genuinely_unknown_command_still_draws_w123`, which fails alone.

  Other server e2e flakes were not catalogued by name in this session.
