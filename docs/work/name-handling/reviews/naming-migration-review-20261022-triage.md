# Triage: naming migration review (2026-10-22) against the current `work` tree

Reviewed report: [naming-migration-review-20261022.md](naming-migration-review-20261022.md), written against `fcb7927f02` (16 Oct). The working tree was replayed on 2026-10-22 from 10:30Z to 12:00Z, on top of HEAD `bc79dd0d4d` plus in-flight agent edits.

## Method

- **Oracles.** Each Tcl repro from the review was executed on `tclsh8.4` (8.4.20), `tclsh8.5` (8.5.19), `tclsh8.6` (8.6.18), `tclsh9.0` (9.0.4), `tclsh9.1` (9.1.0) and `tmp/jimtcl-oracle/jimsh` (Jim 0.84). Every oracle answer matches the review's expected outcome. Feature-unavailable rows (TclOO on 8.4/8.5/Jim, `namespace path` on 8.4/Jim) are noted as such.
- **Probes.** The current production APIs were driven from one temporary test file, `rust/tcl-spectcl/tests/zz_review_probe.rs`, built in the shared target with `CARGO_INCREMENTAL=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_PROFILE_DEV_DEBUG=0` and run with `--nocapture --test-threads=1`. It has been deleted. Its essential calls are named in each row. `tcl-spectcl` was chosen because it links `tcl-compiler`, `tcl-lsp-core`, `tcl-spectcl`, `tcl-spec-hooks` and `tcl-engine-tclvm` in one build. No crate failed to compile during the triage.
- **Executing edits.** Returned edits (rename `TextEdit`s, optimiser `Optimisation`s, minifier output, refactor edits) were applied to scratch copies, which were executed on all six engines.
- **Protocol coverage.** No server-level (JSON-RPC) test was built: `tcl-lsp-server` was not built, because of disk limits. Server-side seams are judged statically and say so.
- **Status vocabulary.**
  - REPRODUCES: the same wrong result.
  - FIXED: the correct result.
  - CHANGED: different but still wrong, or now refuses or abstains.
  - NOT-APPLICABLE: the code was removed or replaced.
  - Static inspection, where used, is marked "(static)".

## Summary

| Finding | Status | Owner (files) |
|---|---|---|
| N01 colliding identities | REPRODUCES (slot collapse); the empty-tail part is FIXED | unowned: `tcl-compiler/src/analyser/types.rs`, `handlers.rs` (closest owner compiler-names) |
| N02 Jim colon policy | CHANGED (identity distinct, navigation abstains) | lsp / compiler-names |
| N03 variable cursor/profile | FIXED | — |
| N04 imported source rename | PENDING-RUN3 | lsp |
| N05 forced-import rename | PENDING-RUN3 | lsp |
| N06 collision checks | PENDING-RUN3 | lsp |
| N07 speculative proc variable | FIXED | — |
| N08 inliner subset oracle | FIXED | — |
| N09 type/call/taint | constructor FIXED; call graph CHANGED (abstains) | unowned (`interprocedural.rs`) |
| N10 OO relation/class selection | CHANGED: relation site and missing class FIXED; `namespace path` class selection REPRODUCES | lsp (`definition.rs`) |
| N11 package availability/provider | PENDING-RUN3 (`auto_qualify` FIXED; `merge_library_files` early return REPRODUCES, static) | lsp |
| N12 incremental lifecycle | FIXED | — |
| N13 source environment | REPRODUCES (inherited path lost; 5-round and 4-round caps still present) | lsp, plus unowned `analyser/state.rs` |
| N14 callback arity / semantic roles | FIXED (arity static, tokens executed) | — |
| N15 minifier compaction | CHANGED (refuses) | lsp |
| N16 provider fallbacks | FIXED (all four rows) | — |
| N17 SpecTcl template / live hook | template FIXED; hook PENDING-RUN3 | — |

PENDING-RUN3: the details table below holds the current evidence for these rows.
