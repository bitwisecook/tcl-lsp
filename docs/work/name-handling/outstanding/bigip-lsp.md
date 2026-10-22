# Outstanding work: bigip-lsp agent

Area: editors/, BIG-IP/F5 crates and LSP modules.

Last updated: 2026-10-22T14:05Z

Maintained by the bigip-lsp agent. It lists what this agent considers it must still complete. Update it whenever an item starts, finishes, changes or becomes blocked, and at least every 30 minutes while working. The coordinator commits and pushes it with each WIP snapshot.

Full unit table and evidence: `docs/work/name-handling/handoff/reviews/merge-inventory-20261019/bigip-lsp-NOTES.md`.

## In progress

None — this session is wrapping up; work continues in a fresh cloud session
from the coordinator's next snapshot (branch `work`).

## Outstanding (in priority order)

1. XC `if` conditions (f5-xc, mine): the standalone and document translators
   refuse every `if` with "Condition commands have no current original source
   owner" (`translator.rs` `walk_statement` → `SourceExpressionAdvice::command_owners_available`;
   the known CI-5201 f5-xc failures; e2e `commands::xc_translate_{items_are_status_tagged,returns_terraform_and_json_documents,console_format_returns_pasteable_documents}`).
   NOT STARTED beyond a probe: paste `merge-inventory-20261019/bigip-lsp/xc-condition-owner-probe.rs.txt`
   into `mod original_expression_owner_tests`, run it, and find which gate fails
   (`expression_operand` / `original_single_source_command_substitution` /
   `tcl_compiler::source_graph::registry_words_for_original_substitution`'s
   module/input/registry/bindings gates / `source_registry_words_at` /
   `source_xc_advice_allowed`). Fix the source-owner route in f5-xc if the
   cause is there; if it is a compiler gate, send the coordinator a repro +
   contract. Add tests (the existing `getter_source_advice_retains_original_positions_input_and_module`
   is the model), then remove the probe.
2. ILX variable handle: `set handle [ILX::init p e]; ILX::call $handle m` →
   `IlxMethodCall.target` is None (12/16 `issue1707_ilx_methods` e2e fail;
   definition/hover identically). Diagnose in `tcl_irules::ilx`
   (`ilx_method_calls_from_analysis`); fix if in tcl-irules, else repro +
   contract for the compiler agent. Check whether it passes on origin/rust.
3. Re-run e2e: `issue1326_encoding::w109_disabled_still_abstains_for_bigip_and_apl_documents`
   (should now pass with the F5 pull report), `commands::xc_translate_*`,
   `issue1707_ilx_methods` (incl. new `a_closed_sibling_rule_is_scanned_under_its_own_input`).
4. `current_diagnostics::tests::actual_direct_and_push_rewrites_retain_their_captured_document_source`
   failed in the same run ("actual worker must retain an applicable rewrite");
   it is a plain-Tcl optimiser-rewrite test on paths the F5 change does not
   take (`f5_model` is false for it; the abstention check only moved after the
   receipt lookup) — confirm it fails without the F5 change, else fix.
5. Clippy (`cargo clippy -p tcl-bigip -p tcl-bigip-query -p tcl-registry -p tcl-lsp-core -p tcl-lsp-server -p f5-xc -p xtask --lib --bins --tests`);
   fix findings on lines this work changed. Never completed (disk/locks).
6. Re-run the 3 `xtask gen_editor_configs` tests (emacs README regenerated;
   `gen-editor-configs --check` OK).
7. Hand `current_diagnostics.rs` back to the lsp agent (via the coordinator)
   once the F5 pull work is verified end to end.

## Blocked (on whom, and what is needed)

- First publishDiagnostics >30 s in e2e: likely the quadratic analysis the perf
  agent is fixing — re-measure after its batch lands.

## Done since the last update (with evidence: test counts, commit or snapshot)

- BIG-IP/APL pull diagnostics (`textDocument/diagnostic` and the
  current-diagnostics command): one F5 model report code for push and direct
  pull (`f5_model_document_report` in lib.rs), the push completes its pass with
  the delivered result id (`DeliveryCtx::complete_f5_model_pass`), F5 passes
  carry `DiagnosticPass.f5_model` and are exempt from the analysis-input and
  abstention-Context reborrow checks (the F5 report applies the W109
  abstention itself). `deliver_if_current` is now `#[cfg(test)]`.
  New test `f5_model_documents_answer_pull_with_their_model_report` PASS
  (BIG-IP `.scf` and APL: direct pull == push == reborrow, current command
  `complete`). 32/33 in the `current_diagnostics f5_ bigip` filter
  (`bigip-lsp/server-lib-current-diagnostics-bigip-20261022.txt`).
- Earlier: all 35 BIG-IP units implemented (see notes); server e2e attributed
  (`bigip-lsp/e2e-20261022-results.txt`).

## Resume

- Start from the coordinator's snapshot of this state (branch `work`). Nothing
  is half-edited: every file compiles (the tcl-lsp-server lib test build of
  2026-10-22 13:50Z succeeded with the F5 pull change); the XC probe test was
  removed from `rust/f5-xc/src/translator.rs` and saved as evidence.
- Next steps: "Outstanding" 1 → 7, in order.
- Commands (CARGO_INCREMENTAL=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_PROFILE_DEV_DEBUG=0;
  build+run+delete each test executable in one command):
  - XC probe: `cargo test -p f5-xc --lib -- zz_bigip_probe --nocapture 2>&1 | grep PROBE`
  - f5-xc lib: `cargo test -p f5-xc --lib`
  - Server targets: `cargo test -p tcl-lsp-server --lib -- current_diagnostics f5_ bigip xc_`
  - e2e: `cargo test -p tcl-lsp-server --test e2e -- issue1707 xc_translate w109_disabled_still_abstains_for_bigip`
  - core: `cargo test -p tcl-lsp-core --lib -- ilx_navigation irules_overlay_tests`;
    `cargo test -p tcl-lsp-core --test completion_residual -- when_`
  - xtask: `cargo build -p xtask && for v in gen-bigip-grammars gen-zed-queries gen-editor-extensions gen-editor-configs gen-tmlanguage-keywords; do target/debug/xtask $v --check; done`;
    `cargo test -p xtask -- gen_bigip_grammars gen_zed_queries editor_extensions gen_editor_configs`
  - `cd rust/zed-query-check && cargo test`
- Evidence in the repo: `docs/work/name-handling/handoff/reviews/merge-inventory-20261019/bigip-lsp-NOTES.md`
  (unit table, verification, attribution), `bigip-lsp-semantic-baseline.tsv`,
  and `bigip-lsp/` (e2e results, server current-diagnostics results, core lib
  and registry failure lists, `emulate_gen.py`, `xc-condition-owner-probe.rs.txt`).
