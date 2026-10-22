# Outstanding work: perf agent

Area: Analysis and SpecTcl pack-load cost.

Last updated: 2026-10-22 12:36 UTC

Maintained by the perf agent. It lists what this agent considers it must still complete. Update it whenever an item starts, finishes, changes or becomes blocked, and at least every 30 minutes while working. The coordinator commits and pushes it with each WIP snapshot.

## Resume

Evidence and tooling live in the repo at `docs/work/name-handling/handoff/reviews/merge-inventory-20261019/perf/`:

- `perf.patch`: the clean batch, diffed against the tree as of the last update.
- `batch/apply_batch.py ROOT [--zz]`: re-applies the batch by exact anchors to any tree. `--zz` adds the in-process old/new toggles. The `.rs.txt` blocks beside it hold the bodies it splices in.
- `tools/`: probe sources (`perfprobe_example.rs.txt`, `spectclprobe_example.rs.txt`), the gdb sampler `pmp.sh`, and `stacks.py`/`inc.py`/`cat.py` to aggregate samples.
- `evidence/`: `timings.md`, corpus lists and outputs, sampled stacks.

To rebuild the experiment (RAM-backed; keep /dev/shm under 3 GB and build with `-j2`):

1. Copy the tracked and untracked files of `rust/` into `/dev/shm/exp`: `(git ls-files rust; git ls-files --others --exclude-standard rust) | grep -v zz_ | tar -cf - -T - | tar -xf - -C /dev/shm/exp`.
2. Copy the root `Cargo.toml` and `Cargo.lock`, and symlink the other top-level dirs (specs, editors, runtime, ...).
3. Replace `/dev/shm/exp/rust/tcl-registry/tests/data` (839 MB) with a symlink to the tree's copy.
4. Run `python3 .../perf/batch/apply_batch.py /dev/shm/exp --zz`.
5. Copy the probe `.rs.txt` files to `/dev/shm/exp/rust/tcl-compiler/examples/zz_perfprobe.rs` and `zz_spectclprobe.rs`.
6. Build: `cd /dev/shm/exp && CARGO_INCREMENTAL=0 CARGO_TARGET_DIR=/dev/shm/perf-target CARGO_PROFILE_DEV_OPT_LEVEL=1 CARGO_PROFILE_DEV_DEBUG=line-tables-only cargo build -j2 -p tcl-compiler --example zz_perfprobe --example zz_spectclprobe`.

The runs:

- `ZZ_COMPARE=1 zz_perfprobe files <paths>` reports old1/new/old2 equality and timings per file. `ZZ_DIALECT=f5-irules` switches dialect. `zz_perfprobe calls|procs|upvar 10,20,40` runs the synthetic shapes. `TCL_LSP_TRACE_SOURCE_PHASES=1` prints analyser phases.
- `ZZ_COMPARE=1 zz_spectclprobe` compares load_embedded old vs new.
- `tools/pmp.sh out 40 0.2 -- <cmd>`, then `stacks.py out > out.stacks; inc.py out.stacks 50`, profiles.

## In progress

- AnalysisResult equality corpus. The first pass flagged 2/101 `.tcl` and 15/15 `.irul` mismatches. The same mismatch appears old-vs-old (ZZ_SAME=old): a second analysis in a warm process differs in retained registry/OnceLock fields (`<uninit>` vs initialised). It is a probe artifact, not a semantic change. Diagnostics are equal in every case. The probe is being reworked to run old1/new/old2 and compare new against the old runs.
- tclIndex (253 lines, opt-level 1): 208 s → 29 s after the `incomplete_body_metadata::header` memo. The rest is diffuse per-command whole-document work: SourceImage/SourceMap/LineIndex rebuilt per command (~20%) and re-segmentation per query (~25%). Still mildly super-linear (100 lines 7.9 s, 253 lines 29 s).

## Outstanding (in priority order)

1. tclIndex repro (`/usr/share/tcltk/tk8.6/tclIndex`, 253 `set auto_index(..) [list source [file join $dir f.tcl]]`). Confirm the memo batch makes it linear. If another hot path dominates, profile and report it.
2. Crate tests in exp, aiming for zero pass-to-fail: tcl-lexer lib; tcl-compiler lib filters (spectcl_source, original_name_value, formal_call_advice, realm, interprocedural, w216, w143); tcl-spectcl `eval_loader` and `golden_packs`.
3. Land the batch in ONE application. Re-check `git status` and diff exp against the tree for each file, and message the coordinator first. Files:
   - tcl-lexer `parse_cut.rs` and `source_map.rs`
   - tcl-compiler `parsing/spectcl_source/original_word.rs`
   - `command_binding/incomplete_body_metadata.rs`
   - `command_binding/original_name_value.rs` and `command_binding/formal_call_advice.rs`
   - `realm.rs` and `interprocedural.rs`: compiler agent's files. Send the hunks to the coordinator, diffed against the current tree.
4. origin/rust comparison timings: build rust-wt only after the exp target is deleted, then delete both.
5. Write the perf-NOTES.md handback (`docs/work/name-handling/handoff/reviews/merge-inventory-20261019/perf-NOTES.md`).
6. Next: the review's section 4.1/4.2 scaling shapes that are still super-linear on the LSP ordinary path. The coordinator will pass the confirmed list from the triage agent.

## Blocked (on whom, and what is needed)

- None.

## Done since the last update (with evidence: test counts, commit or snapshot)

- Root cause of the Analyser quadratic (gdb stack sampling):
  - `SourceCommandBindings::original_written_word_observations_at_span` rebuilds a binding for every prior dispatch offset on each W216/W143 query.
  - `declaration_caller_alias_arguments` re-surveys the callee for each call.
  - `realm.invocation_at_source` re-joins the n per-activation points.
- Root cause of the spectcl load_embedded quadratic:
  - `parse_cut::command_cut` builds a whole-source SourceImage per command.
  - `original_word::in_body` re-proves the full ancestor chain with full-byte image compares.
- Timings, opt-level 1, `p 1`×n:
  - before: n=40 3.5 s, n=80 14.8 s
  - after: n=40 1.4 s, n=80 2.9 s, n=160 ~7 s
  - upvar shape n=30: 39.6 s → 11.8 s
- load_embedded: 7.3 s → 1.9 s. With the coordinator-requested full-chain debug_assert it was 6.9 s, so that assert was replaced by the agreed inductive one.
- In-process equality so far: all synthetic shapes `eq=true`. PackSet sorted `{:#?}` lines identical (484,936).
