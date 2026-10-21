# vm-runtime: Sol-48 verification and SpecTcl execution (21 Oct)

## Scope and setup

Branch `work`, starting at `54256b96a3`. I made no commits myself. The coordinator's ~30-minute WIP
snapshots (for example `6afd791002`) committed my edits as they stood. Every change is described
below. `vm-runtime.patch` is `git diff 54256b96a3 --` over the paths I touched.

Owned paths:
- rust/tcl-vm
- rust/tcl-vm-cli
- rust/tcl-vm-wasm
- rust/tcl-engine-tclvm
- rust/tcl-cmd-core
- runtime/rust
- rust/tcl-spectcl

At the coordinator's request I also edited:
- `docs/design/spec-dsl-examples/oo-class.tclspec`
- two `rust/tcl-spec-studio` tests (that crate has no owner)

Run environment:
- Every run: `CARGO_INCREMENTAL=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_PROFILE_DEV_DEBUG=0
  TCL_LSP_REQUIRE_TCLSH=9.0,8.6 TCL_LSP_JIMSH=$PWD/tmp/jimtcl-oracle/jimsh`.
- SpecTcl targets also: `TCL_LSP_TCLSH90=tclsh9.0 TCL_REQUIRE_SPECTCL_COMPAT=1`, as in
  `make test-spectcl-compat`.
- One test binary at a time; each binary was deleted after its run.
- Free disk sat between 1.2 and 2.8 GB all session. Builds waited for ≥ 2.5 GB, and later
  for ≥ 2 GB, as the coordinator directed.
- tcl-compiler and tcl-registry were rebuilt by other agents throughout. Each "after" result
  therefore includes their working tree at that moment.

RESULTS
