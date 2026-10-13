# Existing MCP source and writer-hint controls

Three original controls ran once each directly on the selected validation-02
MCP test executable after the environment restart. No retained temporary or
experiment logs contained earlier execution outcomes for these controls.
No build, full MCP suite, literal-name repeat or interpreter oracle ran.

| Exact existing control | Result |
| --- | --- |
| `irule_gen::tests::called_helper_static_write_preserves_hot_event_for_multi_tmm` | 1 passed, 0 failed |
| `irule_gen::tests::executable_inventory_ignores_comments_and_data_but_keeps_live_substitutions` | 0 passed, 1 failed |
| `tools::original_event_source_tests::original_mcp_event_reports_keep_requested_analysis_and_ignore_reporting_maps` | 1 passed, 0 failed |

Each invocation executed one test and filtered 132. The executable-inventory
failure reaches the original assertion at `rust/tcl-mcp/src/irule_gen.rs:1307`:
`variables.static_vars` is `[]`, while the unchanged fixture expects `["hits"]`.
Its later multi-TMM assertion was not reached. The two passing controls do not
establish that all event or writer-hint consumers work.

[Validation](validation.json) and the three invocation JSON files retain exact
argv, times, exit codes and harness counts. Each control has full raw stdout,
stderr and an unchanged source excerpt; [source identities](control-sources.json)
retain the hashes. The selected [binary](binary.json) SHA256 stayed
`363856c3e8169e3d2720507b7bd11959c69dca83ffe18fa9d5e40e8be9986447`.

The before/after snapshots contain the same 5,307 files and 39 conservative
local packages as validation-02. Every retained hash and selected binary hash
matched before reuse and after each control; no files were added in the source
selection or changed during capture. VM is included; standalone Runtime is
excluded and may change independently. This source/header/manifest scope is
conservative and does not claim every captured file compiled.

The first preflight expanded the selection to every TOML file and stopped
before test execution on five fixture/provenance TOMLs absent from the earlier
snapshot. [That rejected selection](preflight-expanded-scope.json) is retained.
The corrected selection uses Rust/C/header files, `Cargo.toml`, and the retained
root configuration/lock files; it matches validation-02 with zero drift.
This was a selection mismatch, not evidence that the retained sources changed.

Sources, fixtures and assertions were not edited. Shared Cargo caches were not
built or mutated, and MCP source/cache ownership is released. The failed
inventory assertion remains an open implementation obligation. These are local
Rust consumer controls, with no F5 TMM/MCPD or Tcl dispatch conformance claim.
