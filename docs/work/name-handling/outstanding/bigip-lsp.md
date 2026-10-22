# Outstanding work: bigip-lsp agent

Area: editors/, BIG-IP/F5 crates and LSP modules.

Last updated: 2026-10-22T11:10Z

Maintained by the bigip-lsp agent. It lists what this agent considers it must still complete. Update it whenever an item starts, finishes, changes or becomes blocked, and at least every 30 minutes while working. The coordinator commits and pushes it with each WIP snapshot.

Full unit table and evidence: `docs/work/name-handling/handoff/reviews/merge-inventory-20261019/bigip-lsp-NOTES.md`.

## In progress

- Triage of server e2e failures (run 10:35–10:59Z): `issue1707_ilx_methods`
  12 of 16 fail (incl. definition/hover paths this work did not change, and the
  new closed-sibling test), `commands::xc_translate_{items_are_status_tagged,
  returns_terraform_and_json_documents,console_format_returns_pasteable_documents}`,
  `commands::diagram_*`, `issue1326_encoding::w109_disabled_still_abstains_for_bigip_and_apl_documents`
  and others fail; messages were not captured. Needs one re-run of the failing
  e2e tests with output (waiting for ≥2 GB free on /).

## Outstanding (in priority order)

1. Re-run the failing e2e tests with output; fix whatever the BIG-IP/ILX/XC changes broke, attribute the rest.
2. Clippy (`-D warnings`) on tcl-bigip, tcl-bigip-query, tcl-registry,
   tcl-lsp-core, tcl-lsp-server, xtask — never run (disk).
3. Re-run the 3 `xtask gen_editor_configs` tests after the emacs README
   regeneration (`gen-editor-configs --check` already OK).
4. Hand the final report back (SubagentHandback) with the notes file current.

## Blocked (on whom, and what is needed)

- Items 1–3 need ≥2 GB free on / per build; the shared disk is at ~1.5 GB.
- Not in this agent's scope, recorded for owners: `tests::workspace_folder_add_reschedules_non_xc_document_too`
  hangs (plain-Tcl folder-add rescheduling; lsp agent); 7 core `semantic_tokens`
  integration failures in sslictcl/spectcl/TclOO bodies and
  `tcl-bigip links::irule_links_follow_reached_helpers_but_not_dormant_procedures`
  (helper-reach in tcl-irules/compiler) — not caused by BIG-IP changes.
- Follow-ups that need a different change: Zed `bigip.conf` basename (the Zed
  generator writes extensions only); `tree-sitter-bigip` grammar's own module
  list (needs `tree-sitter generate` + rev bump).

## Done since the last update (with evidence: test counts, commit or snapshot)

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
