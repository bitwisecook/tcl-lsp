# vm-runtime: verification of Sol-48 and SpecTcl execution (21 Oct)

Branch `work`, base `54256b96a3`. I made no commits. My changes are in the working tree,
and `vm-runtime.patch` holds `git diff 54256b96a3 --` my paths.

Owned paths: rust/tcl-vm, rust/tcl-vm-cli, rust/tcl-vm-wasm, rust/tcl-engine-tclvm,
rust/tcl-cmd-core, runtime/rust, rust/tcl-spectcl.

Environment for every run: `CARGO_INCREMENTAL=0 CARGO_PROFILE_TEST_DEBUG=0
CARGO_PROFILE_DEV_DEBUG=0 TCL_LSP_REQUIRE_TCLSH=9.0,8.6 TCL_LSP_JIMSH=$PWD/tmp/jimtcl-oracle/jimsh`.
SpecTcl targets also set `TCL_LSP_TCLSH90=tclsh9.0 TCL_REQUIRE_SPECTCL_COMPAT=1`.

Every binary was built and run alone, then deleted. Free disk sat between 1.2 and
2.8 GB all session, and builds waited whenever it fell below 2.5 GB.

Every "after" result includes whatever the compiler agent had in the working tree at
that moment, because tcl-compiler was rebuilt several times during the session.

PLACEHOLDER
