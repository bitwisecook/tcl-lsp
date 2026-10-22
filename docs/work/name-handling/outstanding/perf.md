# Outstanding work: perf agent

Area: Analysis and SpecTcl pack-load cost.

Last updated: 2026-10-22 10:52 UTC

Maintained by the perf agent. It lists what this agent considers it must still complete. Update it whenever an item starts, finishes, changes or becomes blocked, and at least every 30 minutes while working. The coordinator commits and pushes it with each WIP snapshot.

## In progress

- Equality proof, after vs before, in the isolated `/dev/shm/exp` copy: AnalysisResult (`==`) over 102 corpus `.tcl` files, 15 `.irul` files and synthetic shapes. The fixed spectcl build has just finished; runs start now.

## Outstanding (in priority order)

1. tclIndex repro (`/usr/share/tcltk/tk8.6/tclIndex`, 253 `set auto_index(..) [list source [file join $dir f.tcl]]`). Confirm the memo batch makes it linear. If another hot path dominates, profile and report it.
2. Crate tests in exp, aiming for zero pass-to-fail: tcl-lexer lib; tcl-compiler lib filters (spectcl_source, original_name_value, formal_call_advice, realm, interprocedural, w216, w143); tcl-spectcl `eval_loader` and `golden_packs`.
3. Land the batch in ONE application. Re-check `git status` and diff exp against the tree for each file, and message the coordinator first. Files:
   - tcl-lexer `parse_cut.rs` and `source_map.rs`
   - tcl-compiler `parsing/spectcl_source/original_word.rs`
   - `command_binding/original_name_value.rs` and `command_binding/formal_call_advice.rs`
   - `realm.rs` and `interprocedural.rs`
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
