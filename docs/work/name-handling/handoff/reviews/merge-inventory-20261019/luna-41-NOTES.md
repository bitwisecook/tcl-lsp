# Luna-41 — LSP crates: measurement, root causes, fixes, clippy

Agent: Luna-41 (LSP). Scope: `rust/tcl-lsp-core`, `rust/tcl-lsp-db`, `rust/tcl-lsp-server`
and their tests; one design doc amendment
(`docs/design/analysis/name-resolution-proofs/original-method-candidates.md`).
Nothing committed by me. Patch: `luna-41.patch` in this directory (see "Patch" below).

Environment for every run: `CARGO_INCREMENTAL=0 CARGO_PROFILE_TEST_DEBUG=0`,
`TCL_LSP_REQUIRE_TCLSH=9.0,8.6`, `TCL_LSP_JIMSH=$PWD/tmp/jimtcl-oracle/jimsh`, one test
target at a time, `--test-threads` 1–3, logs reduced to test/panic lines under
`scratchpad/luna41/` (binaries deleted after use). The tree was shared with Sol-48/49,
compiler-names and Luna-42; every measurement is "HEAD + the working tree at that time",
so compiler fixes landing in parallel move numbers between runs (noted where it matters).

PLACEHOLDER-SECTIONS
