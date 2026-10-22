# Outstanding work: perf agent

Area: Analysis and SpecTcl pack-load cost.

Last updated: 2026-10-22 12:47 UTC

Maintained by the perf agent. It lists what this agent considers it must still complete. Update it whenever an item starts, finishes, changes or becomes blocked, and at least every 30 minutes while working. The coordinator commits and pushes it with each WIP snapshot.

## Resume

The session moved to its own cloud session at the user's decision. All /dev/shm state (exp copy, target, rust-wt) has been deleted, and nothing is applied to the tree.

Repo evidence: `docs/work/name-handling/handoff/reviews/merge-inventory-20261019/perf/`, with the summary in `../perf-NOTES.md` beside it.

- `perf.patch`: the whole batch, `git apply`-able against the tree as of the last update. It is the concatenation of:
  - `perf-owned.patch` (6 files, perf's own): tcl-lexer `parse_cut.rs` and `source_map.rs`; `parsing/spectcl_source/original_word.rs`; `command_binding/original_name_value.rs`, `formal_call_advice.rs`, `incomplete_body_metadata.rs`.
  - `perf-realm-interprocedural.patch` (2 files, compiler agent's `realm.rs` and `interprocedural.rs`): hand to the coordinator.
- `batch/apply_batch.py ROOT [--zz]`: re-applies the batch by exact anchors. Use it if the patch stops applying, then regenerate the patches with `diff -u`. `--zz` adds the in-process old/new toggles for probes: `tcl_lexer::ZZ_OLD` / `zz_old()` (also env `ZZ_OLD_ENV=1`) and `tcl_compiler::realm::ZZ_NO_MEMO`.
- `tools/`: `perfprobe_example.rs.txt` and `spectclprobe_example.rs.txt` (tcl-compiler examples), `pmp.sh` (gdb sampler), and `stacks.py` / `inc.py` / `cat.py` (aggregation).
- `evidence/`: `timings.md`, corpus lists, run-1 corpus outputs, sampled stacks.

State of each fix:

| Fix | Written | Exactness | Proven |
|---|---|---|---|
| original_name_value memo | yes | pure per-site memo | synthetic + tclIndex `==` |
| formal_call_advice survey memo | yes | pure per-callee memo | synthetic + tclIndex `==` |
| realm invocation_at_source memo | yes | head ignored by callee | synthetic + tclIndex `==` |
| incomplete_body header memo | yes | pure, image-retaining key | tclIndex `==` |
| parse_cut image hoist | yes | same bytes and channel | PackSet equal |
| SourceImage ptr_eq | yes | ptr_eq implies equal bytes | PackSet equal |
| original_word link-only check | yes | invariant by construction; new unit test | PackSet equal |

None of the crate tests has been run yet.

Exact commands on a fresh machine (no /dev/shm limits needed there, but build with `-j2`):

1. `git apply docs/work/name-handling/handoff/reviews/merge-inventory-20261019/perf/perf.patch` in a scratch worktree, or run `apply_batch.py <worktree> --zz` for toggled probes.
2. Copy the probe sources into `rust/tcl-compiler/examples/zz_perfprobe.rs` and `zz_spectclprobe.rs`.
3. Build: `CARGO_PROFILE_DEV_OPT_LEVEL=1 CARGO_PROFILE_DEV_DEBUG=line-tables-only cargo build -j2 -p tcl-compiler --example zz_perfprobe --example zz_spectclprobe`.
4. Equality: `ZZ_COMPARE=1 target/debug/examples/zz_perfprobe files $(cat .../perf/evidence/corpus.txt)`, then the same with `ZZ_DIALECT=f5-irules` over `corpus_irules.txt`. Expect `eq=true` (new == old1 or old2). `det=false` lines flag warm-process nondeterminism, not the batch.
5. `ZZ_COMPARE=1 target/debug/examples/zz_spectclprobe`.
6. Tests, each run twice (plain, then `ZZ_OLD_ENV=1`); diff the pass/fail sets:
   - `cargo test -j2 -p tcl-lexer --lib`
   - `cargo test -j2 -p tcl-compiler --lib -- spectcl_source original_name_value formal_call_advice realm interprocedural w216 w143`
   - `cargo test -j2 -p tcl-spectcl --test eval_loader --test golden_packs`

## In progress

- None. Handed back for the move to a cloud session.

## Outstanding (in priority order)

1. Three-way corpus equality rerun (resume step 4).
2. Crate tests with and without `ZZ_OLD_ENV` (step 6), aiming for zero pass-to-fail.
3. origin/rust timings for the same inputs: `git worktree add <dir> origin/rust`, copy the probe without the `ZZ_*` toggles, then time `calls 10,20,40,80`, the tclIndex prefixes, and load_embedded.
4. Land: message the coordinator, apply `perf-owned.patch` in one go after `git status`, and pass `perf-realm-interprocedural.patch` to the compiler agent via the coordinator.
5. tclIndex residual (253 lines, 29 s at opt-level 1, mildly super-linear): per-command whole-document SourceImage/SourceMap/LineIndex rebuilds and re-segmentation. Candidate fix: one cached document SourceImage on the Analyser. Those are compiler-names / unowned files; send a patch.
6. `incomplete_body_operand_layout_advice` still loops over every earlier site per command (now ~7% of tclIndex samples). Index the sites if needed.
7. The review's section 4.1/4.2 scaling shapes: wait for the coordinator's confirmed list.

## Blocked (on whom, and what is needed)

- `realm.rs` / `interprocedural.rs` hunks: compiler agent, via the coordinator.

## Done since the last update (with evidence: test counts, commit or snapshot)

- Saved the clean split patches, apply script, probes, tools, timings, corpus outputs and stacks under `perf/`. Wrote `perf-NOTES.md`.
- Deleted /dev/shm/perf-target, /dev/shm/exp and the rust-wt worktree. The `zz_perfprobe.rs` tree probe was already deleted.
