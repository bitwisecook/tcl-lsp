# LSP agent (after Luna-41) — measurements, fixes, routed causes

Owned paths: `rust/tcl-lsp-core`, `rust/tcl-lsp-db`, `rust/tcl-lsp-server` (not the BIG-IP
modules). I committed nothing; the coordinator's WIP snapshots carry my edits.
Run conditions: `CARGO_INCREMENTAL=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_PROFILE_DEV_DEBUG=0`,
`TCL_LSP_REQUIRE_TCLSH=9.0,8.6`, `TCL_LSP_JIMSH=$PWD/tmp/jimtcl-oracle/jimsh`,
`--test-threads=2`, every test executable deleted right after its run. All numbers are
"HEAD + shared working tree at that moment"; compiler agents landed changes between runs.

(Draft — updated at hand-back.)
