# Outstanding work: coordinator

Last updated: 2026-10-29T16:05Z

## Resume

STOPPED BY USER at ~16:15Z 29 Oct. All five roles and the 30-minute reminder stopped; no builds or hosted jobs running. Everything in WORK committed and pushed, including in-flight TEMP_*/TMP_* probes (remove before any CI dispatch or release carry). On resume: restart the five roles from their latest reports (the role rows in [current-state.md](../current-state.md) list each role's assigned-but-unfinished items), re-arm 30-minute reporting, depth-1 fetch before every push.

Assigned at the stop but not yet implemented:
- compiler-names: narrow Proc/definition admission in the AuthoredSimulation structure gate (root cause of empty cu.procedures); compatibility_name Debug spelling; value_transfer.rs AssignExpr gate; `proc continue` over-breadth; stack frame size.
- compiler-codegen: 9.0 registry select gaps (format, info errorstack, string is, string map, tailcall, clock+clockRead); branch-condition folds (scope note in user-decisions-20261029T1125Z.json); dict incr/with fallback; unused-set fold.
- vm-runtime: 3 deopt tests to an explicit profile (decision standalone_unit_tests_explicit_dialect) and the native_vm_with_output doc; clockRead opcode; registry index differential test and timings; mutation checks; lifetime bug; tcl-vm lib 271.
- lsp: reload_spec_packs(Config) never returning (did_change_configuration); 3 publish-path Elapsed; cross-file `source` references restore; classify 14 legacy rehoming tests; e2e sampling.
- compiler: taint correction-comment audit; class_factories definer slots; Tk facts; irules realm; chan configure taint.

## Resume (16:05Z refresh)

RUNNING (fourth restart resumed 08:00Z 29 Oct). WORK `/home/user/tcl-lsp` (`work`), release shallow clone `/home/user/release/tcl-lsp` (`release/lsp-naming` `83c8e08`, fixture cleanup 51/51). On user request at 16:05Z everything in the WORK tree was committed, including roles' in-flight TEMP_*/TMP_* diagnosis probes; remove them before any CI dispatch or release carry.

- Gate status: zero pass-to-fail NOT met. CI 5248 (`83a0b2351`) vs origin/rust 5203: 2,835 pass-to-fail, 10,299 unobserved, 216 absent; per-role lists in the session scratchpad `ci5248/roles/`, summary in `handoff/reviews/merge-inventory-20261019/coordinator-20261027/ci5248-vs-origin-rust-5203-pass-to-fail-summary.md`.
- User decisions this resume: MAY fixture correction; standalone units "correct all affected tests"; SpecTcl discovery fail-closed; iRules Tcl 8.4 objects; namespace121 approved; integer expression folds admitted everywhere (user-decisions-20261029T1125Z.json, with coordinator scope note for operand conversion; the user may narrow it).
- Root causes established: lsp scheduler lock-order deadlocks (fixed); path-admission db lock (fixed); rehoming guard aggregate-completeness (narrowed, pending review) exposing cross-file `source` reference loss (restore on source-view graph, no recorded decision retired it); empty `cu.procedures` and missing top-level IR/SSA facts (compiler-names); puts/IO completion domain Any (descriptors added); engine Currency compared per-thread errno (fixed); standalone-unit harnesses without a Tcl version (corrected via profile API); CLI start-up registry prerequisite scans (index applied, unmeasured).
- Possible user decisions pending facts: Tk taint under the open tk policy (28 rows); whether profile-less foreign modules must still run (3 deopt tests).
- Independent review batch before release: LSP B/B2/A, AuthoredSimulation admissions (compiler-names review note), integer/operand folds, completion descriptors, Currency fix, registry prerequisite index, C9 regexp follow-ups.
- Release cleanup: remove branch-only env-gated eprintln trace hooks (LOWER_SOURCE_PHASE, ORIGINAL_SOURCE_JOIN, SOURCE_CATALOGUE, N17G_LIVE, current_diagnostics and others), leftover committed probes (`zz_sol49_*`, `ZZ_NATIVE_PREFLIGHT_ASSESSMENT`), and all in-flight TEMP_*/TMP_* probes.
- Next: collect role fixes, compile-clean probe-free cohort, then pin refresh and hosted diagnostics, then a clean full ci.yml rerun; carry functional fixes to release and re-run release prep; final gates (section F, zero pass-to-fail, shipping Tcl performance, docs/comments, formatting/generated/packaging/squash-tree). No PR until every gate passes; no merge, tag or publish.

## Resume (2026-10-27 resume)

RESUMED 21:05Z in a fresh container. WORK `/home/user/tcl-lsp` (`work`), release shallow clone `/home/user/release/tcl-lsp` (`release/lsp-naming`). Packets restored and verified in the session scratchpad from the paused-checkpoint archive; /workspace and naming-proof-storage are absent.

- Done: pointer lint, release compatibility, Names ref_option, missing_fields_in_debug, assert_ne (WORK 527c653a8/111d0604c, release 531d338/a8ea94c). Release prep 40586145473 SUCCESS on 531d338; strict 40588176868 on 111d0604c running.
- Running: five roles. WIP snapshot 10b1125d5 holds unreviewed role edits.
- Next: coherent pin refresh across stale rosters on a reviewed cohort, then compiler Layout21/main170, corpus2, LSP startup/host and iRules dispatches; release cleanup commits family by family; 30-minute reports (trigger re-armed each cycle).
- Namespace121 question remains unanswered; all recorded decisions binding.

## Resume (pause snapshot)


USER PAUSED. No implementation, build, dispatch, archive intake or recurring reminder is authorised until the user resumes. This turn only preserves state and commits it. Read [pause-handoff-20261027.md](../pause-handoff-20261027.md) for exact source tips, all current results, reviewed/unapplied packets, incomplete source drafts, unanswered decision and first actions.

- Root alone canonical Git/source/dispatch. Canonical WORK `b41de429f886145f348381daf949fd6944357d4a`; release `0158859174cc4eca280dd5fa578e2a6a2f038d2a`; active checkout has frozen old product. Documentation checkpoint commits may advance WORK without changing product bytes.
- Five implementation roles stopped; no local role/build/timer process found. Hosted wave7fd already recorded terminal; no jobs cancelled. Previous /tmp helpers absent, no reminder armed.
- Current product latest fixes committed but not runtime validated; no prepared pointer/compatibility/upvar patch adopted during pause.
- Source cleanup44/51, seven remaining. All section F, baseline, shipping performance and final clean gates remain open. No PR is open. Six-hour target missed.
- Evidence and prepared source packets saved in [paused-checkpoint](../handoff/reviews/merge-inventory-20261019/coordinator-20261027/paused-checkpoint/packet-index.json); original ZIPs/full-Git/provider bytes stay in local proof storage. No assertion weakening, guard bypass or result donation.
- External BIGIP/perf stale; notified, no takeover. review-triage complete. Namespace121 choice unanswered; every recorded user decision remains binding.
- 30-minute snapshots resume only after new user instruction; re-arm then. No local builds below2GB; no unsafe target/executable deletion.
