# Outstanding work: bigip-lsp agent

Area: editors/, BIG-IP/F5 crates and LSP modules.

Last updated: 2026-10-22T14:22Z

Maintained by the bigip-lsp agent. Keep this snapshot and Resume current, and
push at least every 30 minutes and after each finished item. Full unit table:
`../handoff/reviews/merge-inventory-20261019/bigip-lsp-NOTES.md`.

## In progress

Building server e2e and current-diagnostics reruns after the XC fix; local
log `/tmp/bigip-server-selected.log`. Clippy `-D warnings` is also running in independent `/workspace/bigip-clippy-target`;
logs `/tmp/bigip-clippy.jsonl` and `/tmp/bigip-clippy-build.log`. The ILX
baseline/current comparison is complete; its missing compiler contract is below.

## Outstanding (in priority order)

1. Restore ILX variable-handle source navigation after the compiler supplies
   the constructor/setter/read relation described under Blocked. Keep the
   current negative assertions and separate source candidates from live handles.
2. Re-run server e2e `commands::xc_translate_*`, `issue1707_ilx_methods`
   (including `a_closed_sibling_rule_is_scanned_under_its_own_input`) and
   `issue1326_encoding::w109_disabled_still_abstains_for_bigip_and_apl_documents`.
   Also re-run `current_diagnostics::tests::f5_model_documents_answer_pull_with_their_model_report`.
3. Confirm `current_diagnostics::tests::actual_direct_and_push_rewrites_retain_their_captured_document_source`
   fails without the F5 pull change, or fix an attributable regression. The
   exact F5 change is in `c9dc3c2892` (current_diagnostics.rs and only its F5
   lib.rs hunks). Temporary reverse-and-restore testing must retain assertions.
4. Clippy `-D warnings` on the owned crates and editor generators, including
   tcl-bigip, tcl-bigip-io, tcl-bigip-query, bigip-query-wasm, tcl-irules, f5-xc,
   f5-cli, tcl-f5mku, bigip-report-gen, tcl-lsp-core, tcl-lsp-server, xtask and
   the previously changed tcl-registry lines. Fix owned findings; route other
   owners' blockers without editing their files.
5. Re-run the three `xtask gen_editor_configs` tests. Its generated Emacs README
   was previously regenerated and `gen-editor-configs --check` passed.
6. After the F5 pull e2e is confirmed, record `current_diagnostics.rs handed back`
   here for the coordinator/LSP agent. It remains on loan until then.

## Blocked (on whom, and what is needed)

- ILX: the compiler owner must supply an authenticated original source
  constructor/setter/later-read relation for remote handles. The current
  tcl-irules producer deliberately writes `target: None`; this is not a failed
  compiler lookup. Its contract and tests explicitly forbid the former local
  `HandleScope` inference. Repro, exact cause, existing-owner analogy, F5
  provenance and required contract:
  `../handoff/reviews/merge-inventory-20261019/bigip-lsp/ilx-handle-owner-repro.md`.
  Baseline `origin/rust` (`fcb7927f02`) literal-handle test: 1/1 PASS; current
  tests retaining the refusal: 2/2 PASS. No assertion was weakened. BIG-IP owns
  wiring the new source relation into navigation once available.
- Previous e2e first `publishDiagnostics` exceeded 30 s on the shared tree.
  Re-measure here; do not raise budgets. The perf batch has not been admitted
  by this agent.

## Done since the last update

- XC condition source ownership FIXED in f5-xc. Expression AST ends are
  inclusive; `expression_operand` incorrectly treated them as half-open and
  clipped `]`. It now calls the shared `tcl_syntax::expr::expression_span`.
  The saved probe confirmed current source/input/configuration/Registry and
  retained bindings, then `operand false`. The new ASCII/Unicode/nested-getter
  regression fails before the fix and passes after. Full f5-xc lib: **31/31
  PASS**, including all previous source/registry refusal assertions (no edits
  to those assertions). Evidence in `bigip-lsp/`: `xc-condition-owner-probe-result-20261022.txt`,
  `xc-range-before-20261022.txt`, `xc-lib-after-20261022.txt`. Probe removed;
  every test executable was built, run and deleted in one command.
- ILX comparison: `ilx-baseline-20261022.txt` (baseline 1/1) and
  `ilx-current-20261022.txt` (current 2/2). This establishes the deliberate
  contract change, not restored navigation or a passing e2e run.
- Earlier F5 pull implementation and all 35 BIG-IP units are in the notes.
  Historical F5 unit filter was 32/33; its plain-Tcl rewrite failure still
  needs the attribution above. New server runs and clippy are not yet green.

## Resume

- Active shallow `work` checkout: `/workspace/bigip-lsp`. Independent
  `origin/rust` baseline: `/workspace/bigip-baseline` at `fcb7927f02`.
  Rust 1.99.1, pinned C Tcl 9.0.4/8.6.18 and Jim 0.84 are built and verified.
  No temporary probe is left in the source tree.
- Temporary `/tmp/bigip-env.sh` exports the required oracle variables and
  CARGO_INCREMENTAL=0 plus test/dev debug=0, then sources agent-build-env.sh.
  `/tmp/bigip-test.py` builds, runs and deletes each test executable within one
  command. Build targets are independent between checkouts.
- Next: finish the running server and clippy checks; rewrite attribution
  with/without the F5 change; standalone bigip-query-wasm clippy; three generator
  tests; hand the diagnostics file back.
  Keep unresolved/compiler-owned items explicit. Never open a PR; the PR to
  rust remains gated on external review section F.
- The first server build needed 76 literal oracle snapshots excluded by the
  sparse checkout. They and 141 further literal fixture inputs are now
  materialised from HEAD; no code or assertion was changed. The restarted
  server build has reached compiler/VM dependencies. No server result yet.
- Validation above used the initial `9cd5a75b` product source plus the XC fix.
  Fetched updates through `5bd5a503` changed coordination/evidence only. Required
  repository/host gates remain pending under the authorised development-push
  exception; these results do not grant release sign-off.
