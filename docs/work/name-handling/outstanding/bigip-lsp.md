# Outstanding work: bigip-lsp agent

Area: editors/, BIG-IP/F5 crates and LSP modules.

Last updated: 2026-10-22T13:40Z

Maintained by the bigip-lsp agent. It lists what this agent considers it must still complete. Update it whenever an item starts, finishes, changes or becomes blocked, and at least every 30 minutes while working. The coordinator commits and pushes it with each WIP snapshot.

Full unit table and evidence: `docs/work/name-handling/handoff/reviews/merge-inventory-20261019/bigip-lsp-NOTES.md`.

## In progress

1. BIG-IP/APL pull diagnostics (coordinator-assigned; `current_diagnostics.rs`
   is mine until handed back): implemented — `DiagnosticPass.f5_model`,
   `DeliveryCtx::complete_f5_model_pass`, one shared `f5_model_document_report`
   for push and the direct pull, F5 reports exempt from the analysis-input and
   abstention-Context checks (the F5 report applies W109 abstention itself).
   New test `current_diagnostics::tests::f5_model_documents_answer_pull_with_their_model_report`
   — build+run in progress.
2. XC "Condition commands have no current original source owner" (f5-xc):
   diagnosing with a temporary probe test `zz_bigip_probe_condition_owner_gates`
   in `rust/f5-xc/src/translator.rs` (REMOVE before hand-back).

## Outstanding (in priority order)

1. Finish (1) and (2) above with tests; then the ILX handle diagnosis
   (`set handle [ILX::init …]; ILX::call $handle …` → `IlxMethodCall.target`
   None): fix if in tcl-irules, else repro + contract to the coordinator; check
   origin/rust.
2. Re-run e2e `issue1326_encoding::w109_disabled_still_abstains_for_bigip_and_apl_documents`,
   `commands::xc_translate_*`, `issue1707_ilx_methods`.
3. Clippy on tcl-bigip, tcl-bigip-query, tcl-registry, tcl-lsp-core,
   tcl-lsp-server, f5-xc, xtask (lib/bins/tests); fix findings on changed lines.
4. Re-run the 3 `xtask gen_editor_configs` tests (emacs README regenerated).
5. Hand back with the notes file current.

## Blocked (on whom, and what is needed)

- First publishDiagnostics >30 s in e2e: likely the quadratic analysis the perf
  agent is fixing — re-measure after its batch lands.

## Done since the last update (with evidence: test counts, commit or snapshot)

- Server e2e run and per-test attribution (notes file, "Verification" table).

All 35 BIG-IP units implemented (DONE 32, ALREADY 3); committed in WIP
snapshots up to 56893780b9 and later.

- ILX sibling input (core + server): core `ilx_navigation` 20/20 PASS.
- Partition-rename version identity: core `bigip_code_actions` 16/16; server
  `code_action_handler_bigip_conf_offers_partition_rename` PASS.
- Registry `when` partial layout + completion: registry slot tests 2/2,
  `tcl-registry --lib` 20 failures all in CI-5201 (0 new); `completion_residual`
  `when_*` 4/4 PASS.
- iRules semantic overlays on the document analysis: baseline recorded
  (`bigip-lsp-semantic-baseline.tsv`), new overlay tests PASS.
- Zed TMSH/BIG-IP split + `gen-bigip-grammars` generator and drift gate:
  xtask `gen_bigip_grammars` 4/4, `gen_zed_queries` 7/7, all six drift gates
  `--check` OK, `zed-query-check` 2/2.
- Server lib targets: 13 XC/dialect/code-action tests PASS; fixed HEAD's E0282
  in `fresh_analysis_driver`.
- `tcl-bigip` `conf_tokens` 22/22, `tcl-bigip-query` projection 3/3.
- `editors/emacs/README.md` regenerated (669c8e2277 renderer change was intended).

## Resume

- Pick up at "In progress". Tree state: all edits in the working tree / WIP
  snapshots; `git status --short rust/f5-xc rust/tcl-lsp-server` shows the
  current unit's files.
- Commands (one build at a time; build+run+delete the test executable in ONE
  command; check `df -h /` first, need >2.3 GB):
  - Server pull test: `CARGO_INCREMENTAL=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_PROFILE_DEV_DEBUG=0 cargo test -p tcl-lsp-server --lib -- current_diagnostics f5_ bigip`, then delete the `target/debug/deps/tcl_lsp_server-<hash>` the run printed.
  - XC probe: `cargo test -p f5-xc --lib -- zz_bigip_probe --nocapture 2>&1 | grep PROBE`, then delete `target/debug/deps/f5_xc-<hash>`.
  - e2e: `cargo test -p tcl-lsp-server --test e2e -- issue1707 xc_translate w109_disabled_still_abstains_for_bigip`, delete `target/debug/deps/e2e-<hash>`.
  - xtask drift: `cargo build -p xtask && target/debug/xtask gen-bigip-grammars --check` (also gen-zed-queries, gen-editor-extensions, gen-editor-configs).
- Evidence in the repo: `docs/work/name-handling/handoff/reviews/merge-inventory-20261019/bigip-lsp-NOTES.md`
  (unit table, verification), `bigip-lsp-semantic-baseline.tsv` (semantic-token
  baseline), and `bigip-lsp/` (e2e results 2026-10-22, core lib failure list,
  registry failure list, `emulate_gen.py`).
