# Hand-over: dataflow (#2408, #1796)

Written 18 November 2026 by the `dataflow` issue session (https://claude.ai/code/session_017u5g3JkH4kppJ1LJpZQUus), paused on the owner's instruction. It replaces the 17 November hand-over. A fresh session in a clean container should be able to continue from this file alone.

## Issues

### #2408 (open): a module that rebinds a builtin has no transfer summaries

The owner's decision is in comment 6900373209: option B, where callee summaries feed the caller's walk as a fixpoint over the call graph, with option A as its sound floor. Under A, where a callee summary is missing, the walk continues under module-closed facts (`ModuleCommandMutations` plus `ModuleTraceFacts`). A caller cell's possible trace comes from `ModuleTraceFacts`, not from a blanket refusal.

The close criterion, from that comment:
1. The 8 `interprocedural::transfer::tests` pass. These are ledger rows 172–179 of `docs/work/test-scrub/ledger/dataflow-optimiser.tsv`.
2. The failing `cfg_builder` frame and `upvar_info` tests pass.
3. #2408's per-procedure gate and witness are in place. They landed in b2691d48 and pass.
4. #1796's `uplevel 3` and `w2→w1→outer` witnesses are in place.

State of the 8 transfer tests on `work`:

| Test | State |
| --- | --- |
| `a_barrier_has_no_summary` | passes; ledger `fixed` |
| `a_call_names_its_omitted_parameters_default` | passes; ledger `fixed` |
| `a_cycle_that_does_not_converge_is_may_bind` | passes; ledger `fixed` |
| `conditional_source_outer_places_do_not_issue_physical_steps` | passes; ledger `fixed` |
| `transfer_unused_formals_use_actual_variable_roots_and_metadata` | fails. The expectation is wrong: tclsh 8.4–9.1 report `info args p` → `a {$tag} é unused` and `info default p a d` → 1, `b`, so the body reads `a b`. Needs a FIX-EXPECTATION commit with that evidence. It is also not yet known why a release build gives `p` no summary at all. |
| `summaries_compose_through_two_callees` | fails. `reset`'s `unset v` through an upvar link evaluates as MayBound, because the store target is not placed. The values session offered (a) a var_symbol for the link local (ours to check) or (b) a place fallback (theirs). We chose (a) and asked them not to change `place`. |
| `a_name_argument_naming_an_outer_place_is_a_global_write` | fails. A `namespace eval ns {… proc w …}` makes every body a dynamic-name barrier (`uncalled_body_after_load` fails). Moved to us by the coordinator; see Rulings. |
| `a_rename_or_a_redefinition_moves_the_revision` | fails at `tests.rs` line 534 (`rename bump bump2`). Ledger `exposed`, KEEP-CONTRACT, cause `rename-not-a-command-mutation`. Moved to us; see Rulings. |

Option B itself has not landed. Investigation on the WIP branch shows that, in release builds, the 8 `value_transfer_witnesses::procedure_summaries` rows routed to us (ledger cause `summary-link-local-def`) and the issue's own `c`/`bump` repro fail for one reason:
- The declared-frame collector walks a called body as a declaration preview.
- Every preview point has `runtime_reachability == Conditional` (`command_binding/binding_projection.rs` ~190).
- So `proved_execution_target()` is `None` for `upvar`, `resolved_tokens_invocation` is `None`, and `dynamic_name_barrier` marks writes and destroys.
- `local_calls` then refuses the body, and the summary map is empty (`outer_steps` → None).

The collector falls back to a preview because `original_completed_root_state` is `None` whenever the root has an `Unknown` abrupt route. A top-level `puts`, or a read of a linked cell after the call (`bump n; set m $n`), produces one. `outcomes.rs::completed_normal_world` treats Unknown as possibly redefining commands.

The fix the owner's decision asks for: such a read's possible trace comes from `ModuleTraceFacts`, and the collector walks a called body as entered (after load, with variables widened) when the world is closed. On the WIP branch:
- `same_state_but_variables` (state_equality.rs) and `called_body_in_closed_world` (deferred_entries.rs) exist, approved by analyser-binding at 10:06Z on 18 Nov.
- They are not sufficient while `completed_normal_world` is `None`.
- analyser-binding's conditions for that change:
  1. Compare everything but `source_variables` and `variable_frame`.
  2. Keep observers when widening, with a test: `trace add variable n read {rename bump {}}` must not keep the summary.
  3. Leave the main walk untouched.

### #1796 (open): centralise compiler frame reach

Nothing has landed. Witnesses still to add, each with tclsh 8.4–9.1 evidence in the test:
- `deep`'s direct caller is untouched;
- `w2 → w1 → outer` closes exactly to `outer` (tclsh evidence `w2 w1 2` already gathered);
- `uplevel 3`.

They depend on release summaries for called procedures, which in turn depend on the preview/Conditional fix above.

## The burn-down's first-priority item: the declared-frame view must keep entry-independent facts

analyser-binding's switch replaces each procedure body with the body lowered in its declared frame. It fixes `try_and_errors::a_body_that_ends_in_an_error_may_have_failed_at_its_start` (the `info exists` fold), but measured 65 new failures in tcl-compiler, plus 1 in lsp-core (`refactor::inline_variable::tests::inline_keeps_the_following_line_indentation`), and 6 fixed.
- The list, grouped by lost fact, is `docs/work/test-scrub/declared-frame-switch.md` on `work`.
- The parked patch is at the bottom of that file: four lines in `retain_declared_frame_bodies` that copy each declared body over `self.module.procedures[id].body`. analyser-binding holds it locally as `park/declared-frame-switch`.
- It must not land until the view keeps those facts and measures new 0 on tcl-compiler and the lsp-core proof groups.

Groups: frames 9, cell/alias identity 11, interprocedural taint 12, value-transfer 11, formal/AOT proofs 9, folds 13.

**First fix (WIP branch, measured with the switch applied):**
- Within the body, `DeclaredFrameView` now admits only declared-frame points where the declared walk recorded a command in the declared frame (`own` offsets).
- Everywhere else (scripts the body runs in other frames: `uplevel`, `uplevel #0`), it keeps the main walk's joined points.
- A variable read belongs to the innermost command around it (`owns_read_at`).
- `reached` means `own` is non-empty.
- Result: 6 of the 65 pass (6 of 9 frame tests: `nested_relative_*`, `uplevel_hash_zero_*`, `original_global_source_write_keeps_empty_scalar_and_array_root`), and all 6 switch fixes still hold.

**What the other 59 need:** they are mostly not gaps in the view.
- Consumers outside lowering resolve a switched body's statements against `module.retained_source_bindings`, which is the main walk: `ssa/source_view/*`, `compilation_unit/module_facts.rs`, `lowering/script_bodies/original_bodies.rs`.
- The main walk never saw the declared activation the body now carries. Example: `ssa::cell_resolution_tests::scalar_alias_write_after_unset_reaches_both_spellings` gets no def for `set y NEW`.

**Merge approach (last step, on the WIP branch, built but not measured):**
- `SourceCommandBindings::adopt_declared_walk` (declared_frame_view.rs) copies into the main bindings every point the collector created inside a called body (index ≥ the collector's first index).
- It also replaces the main walk's reads in those bodies with the collector's, which join the declared contexts.
- `declared_frame_view` then works from the main bindings, and `declared_frame_walk` is no longer read.
- Measure it with and without the switch. Without the switch it must not regress `work`, because the main bindings now carry extra (sound) alternatives at those offsets.

Who should finish it: the view and collector are ours; the switch, the walk and the retained bindings are analyser-binding's. Tell them before pushing in their files.

## On `work` (all mine, each verified with `git merge-base --is-ancestor` when landed)

- b2691d480ed8d670a6db1d2d8a395ff9cd5599fb: keep the summaries a rebound builtin never reaches (#2408 gate, witness `a_shadowed_builtin_keeps_the_summaries_that_never_reach_it`)
- 3c52647f40ae7d027463fb7ea3100d7929f733a1: summarise a called procedure from its body in its own declared frame (`declared_frame_view`, `Module::declared_frame_bodies`, `CfgModule::declared_frame_procedures`)
- b2377be0cea002cab72f479c631b24f170267378: read a declared-frame body's variables in its own frame
- ba8469bb63e2a249e2c30c6a4518fa6879c36b1c: define a caller-linked local where a summary's body writes through it (summary SSA; `reads_linked_locals` removed)
- d5c22b337252df6d5fdb90d67177ac0769853dcf: find a statement's tokens in whichever lowering of its body holds it (repairs the loss from 3c52647f)
- ee8f214ccbea87f2a9cc4f4d6362b6519b842a67: walk every called body again in its declared frame (`declared_frame_walk` collector)
- 5b672fe35b0925702b023485b70103c3b68b23cd: leave the lowerer as it was found after lowering a declared-frame body
- f444f4b86108983c1876d73bf6a6f731a96b8829: read a control command lowered in place by the commands of its scripts
- 3c8bf397fc0a2b1c40b31576842f4221d7daf90b: define, destroy or may-bind a linked local through its link
- 63276a67a3db3611556bd175404720d53771cd76: treat a closed cycle's members as writing no outer place while solved
- fbf01c4c7acd6055b5404f4edd8836b810882114: transfer tests on release units, `revision_of` by name, witness `a_linked_place_is_preserved_may_bound_or_bound_as_the_body_says`, ledger rows 172–177

The last seven were measured against base at 0 new and 265 fixed, with 39/39 gates and `release_optimiser_safety` at 68/68.

## Only on branches

`issues/dataflow-2` at 429ef5dae2fe6183bf950e5fee6db7fc8fbe6363 is one WIP commit on fbf01c4c. It is **not verified** and not for `work`. It contains:
- the view fix (`own`/`commands`, `owns_read_at`) and the merge (`adopt_declared_walk`), in `command_binding/declared_frame_view.rs` and `deferred_entries.rs`;
- `same_state_but_variables` and `called_body_in_closed_world`;
- **the parked switch, applied only to measure**: marked `ZZSWITCH` in `lowering/declared_frame_bodies.rs`. Remove it before any commit;
- probes gated on env `ZZ`, all to remove:
  - `zz_diff` (state_equality.rs);
  - `zz_proof` (execution_proofs.rs);
  - `zz_resolve` (registry_invocation.rs);
  - eprintlns in dynamic_names.rs, procedure_calls.rs, transfer.rs, summarisation.rs, deferred_entries.rs and declared_frame_view.rs;
  - the tests `zz_probe_top_level_call` (transfer/tests.rs) and `zz_probe_c_bump` (it_2 procedure_summaries.rs).
  - `grep -rn 'ZZ\|zz_' rust/tcl-compiler` finds them all.

## How to measure

- Environment: `cd /home/user/tcl-lsp; source scripts/dev/agent-build-env.sh; export CARGO_PROFILE_TEST_OPT_LEVEL=1 RUST_BACKTRACE=0 CARGO_BUILD_JOBS=2`. Two jobs only: more caused an OOM kill (exit 137). Dev/test profile only, one cargo build at a time.
- Full suite, about 12 minutes of tests plus about 10 minutes of build:
  `cargo nextest run -p tcl-compiler --no-fail-fast --status-level fail --final-status-level none --failure-output never > run.log 2>&1; grep -E '^\s+FAIL' run.log | awk '{print $NF}' | sort -u > fail.txt`
  Then `comm -13 base.txt fail.txt` gives new failures and `comm -23` gives fixed ones.
- Baseline: the same command on unmodified `origin/work`. Check any candidate new failure by running it alone on a detached `origin/work` (`git checkout --detach origin/work`, then `-E 'test(=name) | …'`), because `work` moves.
- The switch list: build a filter from the backticked names in `declared-frame-switch.md` (`test(=path)` joined with ` | `) and run with the switch applied (the ZZSWITCH block).
- Optimiser safety: `cargo nextest run -p tcl-codegen --test it_3 -E 'test(/release_optimiser_safety::/)'`, expecting 68/68.
- Before every push to `work`: `cargo clippy -p tcl-compiler -p tcl-codegen --all-targets -- -D warnings -A clippy::assert_is_empty`, then `make xtask-gates` (39 gates, about 3 minutes), then `cargo fmt -p tcl-compiler` (not `--all`).
- Land: rebase onto `origin/work`, `cargo check --workspace`, push `HEAD:work`, then `git merge-base --is-ancestor <sha> origin/work`. Never trust a push without the ancestor check: `! [rejected]` once looked like success.
- Ledger: `python3 docs/work/test-scrub/tools/ledger_status.py --check docs/work/test-scrub/ledger`. A fix that turns a row green marks its status in the same push.

## Rulings in force

- Owner, comment 6900373209 (15 Nov): option B with A as the floor; the close criterion is above.
- Coordinator, via the parent, 18 Nov 08:09Z: the two analyser-binding rows (rename as a command mutation; the `namespace eval` barrier) are ours, as announced commits, each reviewed by analyser-binding before push. They come after option B and our own two tests, unless one blocks B.
  - **Rename:** analyser-binding's notes (18 Nov 08:09Z). The walk records `proc_rebound_names`, but `prepared_command_trust` (`compilation_unit/source_lowering.rs`) projects a CFG-prep lattice that never receives it. The real fix carries the walk's rename record into that lattice. Their shortcut (`original_publication_withdrawn` in `mutation_projection`) over-fires. Their witness test to keep is `a_renamed_or_deleted_procedure_is_no_longer_trusted`; its text is in their message, and the local branch is `park/proc-rename-trust` f65571a0, not on the remote.
  - **namespace eval:** only the diagnosis exists. Start where `dynamic_names` decides a barrier after a namespace-eval body; get tclsh evidence first.
- Coordinator, via the parent, 18 Nov 09:32Z: the declared-frame view work comes first, ahead of option B.
- analyser-binding, 18 Nov 10:06Z: we may change `state_equality.rs` and `deferred_entries.rs` for the collector, with the conditions listed under #2408.
- Values (earlier): option (a), the link-local var_symbol, is ours; values does not change `place`.

## Dead ends

- **Per-entry isolated collector walks** (each called body walked alone, keeping only points inside its span): the declared walk never interprets an `uplevel 1` script whose caller it cannot see, so the facts those tests expect exist only in the main walk's joined points. Recovered 5 of 65.
- **Filtering the view by frame alone** (the original `admits`): drops the body's own `uplevel` scripts.
- **`after_load` from `same_state` for called bodies:** fails on any top-level read or `puts` after a call.

## Next steps, in order

1. Resume only when told. Check out `issues/dataflow-2` at 429ef5da; work on a rebased copy, since `work` has moved.
2. Measure the merge (`adopt_declared_walk`) with the switch on the list of 71, then the full suite with and without the switch. If the merge regresses `work` without the switch, gate it behind the switch or route the consumers in `ssa/source_view`, `module_facts` and `original_bodies` through the view instead (agree with analyser-binding first).
3. Iterate group by group to new 0. Land the view and collector changes (without probes or ZZSWITCH) with tclsh witnesses for what the view keeps. Hand the switch to analyser-binding to land.
4. Option B floor: make `completed_normal_world` take a read's trace possibility from `ModuleTraceFacts`, so called bodies walk as entered in a closed world. Witnesses: the 8 `procedure_summaries` rows and #2408's `c`/`bump` repro on 8.4–9.1, plus analyser-binding's trace test.
5. Fix the `transfer_unused_formals` expectation (FIX-EXPECTATION, with tclsh evidence).
6. `summaries_compose`: option (a), a var_symbol for the link local, so `unset` through a link is placed.
7. The rename and namespace-eval rows (rulings above).
8. Close #2408 (state reason `completed`, a comment naming commits and witnesses, the footer). Then add the #1796 witnesses and close it.

## Hazards

- This worker dropped ordinary cross-session messages: a 09:32Z message arrived 40 minutes late. A successor should call `ReadNotifications` often, at least before and after every long build.
- Disk: the allowance filled once (ENOSPC). Remove `target/debug/incremental` and keep `CARGO_BUILD_JOBS=2`.
- `work` moves about 100 commits an hour. Re-fetch before every comparison and land quickly after measuring. Shallow fetches (`--depth=300`) can make `merge-base --is-ancestor` fail for older commits; fetch deeper to check them.
- Never edit source while a build is running.
- Commit trailers: `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>` and `Claude-Session: https://claude.ai/code/session_017u5g3JkH4kppJ1LJpZQUus`. No model identifiers anywhere else.
