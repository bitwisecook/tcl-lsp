# Unmeasured crates, 18 November 2026 (partial, taken at pause)

Command per crate, from `/home/user/tcl-lsp/rust`: `TCL_LSP_JIMSH=/home/user/tcl-lsp/tmp/jimtcl-oracle/jimsh TCL_LSP_REQUIRE_ALL_TCL_ORACLES=1 TCL_LSP_REQUIRE_JIM_ORACLE=1 TCL_LSP_REQUIRE_TCLSH=1 CARGO_PROFILE_TEST_OPT_LEVEL=1 CARGO_PROFILE_TEST_DEBUG=0 CARGO_PROFILE_TEST_INCREMENTAL=false cargo test -p <crate> --no-fail-fast` for workspace members, `cargo test --manifest-path rust/<crate>/Cargo.toml --no-fail-fast` for the excluded wasm crates and `zed-query-check`. Tree: `origin/work` at `366cda3b` (the tcl-spectcl cache fix), clean. One crate at a time. The script is `patches/unmeasured_sweep.py.txt`.

`web-shared` is not a crate: `rust/web-shared/` holds `site-update.js` only, no `Cargo.toml`.

"Run" counts the unit, integration and doc tests `cargo test` ran. "Left in /tmp" is the number of new `/tmp` entries around the crate's run, the sweep's own log excluded.

| Crate | Run | Passed | Failed | Ignored | Seconds | Failing tests or first error | Left in /tmp |
|---|---:|---:|---:|---:|---:|---|---:|
| bpf-tcl | 106 | 106 | 0 | 7 | 224 |  | 0 |
| bpf-tcl-codegen | 31 | 31 | 0 | 0 | 1 |  | 0 |
| bpf-tcl-ir | 82 | 82 | 0 | 0 | 5 |  | 0 |
| tcl-debugger | 19 | 18 | 1 | 0 | 53 | backend::tests::launch_survives_deeply_nested_control_flow | 0 |
| tcl-fuzz | 113 | 111 | 2 | 0 | 289 | wasm::tests::control_flow_program_runs; wasm::tests::simple_program_compiles_and_runs | 1 |
| tcl-host-c-abi | 3 | 3 | 0 | 0 | 0 |  | 0 |
| tcl-host-native | 11 | 11 | 0 | 0 | 0 |  | 0 |
| tcl-pkg | 70 | 70 | 0 | 0 | 77 |  | 0 |
| tcl-pkg-model | 53 | 53 | 0 | 0 | 9 |  | 0 |
| tcl-platform | 5 | 5 | 0 | 0 | 0 |  | 0 |
| tcl-regex | 32 | 32 | 0 | 0 | 2 |  | 0 |
| tcl-sandbox | 15 | 15 | 0 | 0 | 3 |  | 0 |
| tcl-userdirs | 2 | 2 | 0 | 0 | 0 |  | 0 |
| tcl-version | 2 | 2 | 0 | 1 | 0 |  | 0 |
| tcl-cli-support | 38 | 37 | 1 | 0 | 316 | highlight::tests::original_highlight_uses_actual_c_jim_and_irules_lexical_axes | 0 |
| tcl-test-support | 17 | 17 | 0 | 0 | 2 |  | 0 |
| tcl-engine-wasm | 54 | 51 | 3 | 0 | 89 | session::tests::original_typed_wasm_host_failure_preserves_prior_runtime_pending_cause; original_typed_host_failure_survives_wasm_callback_settlement_and_next_entry; original_typed_wasm_host_failure_i | 0 |
| zed-query-check | 2 | 2 | 0 | 0 | 8 |  | 0 |
| tcl-explorer-wasm | 2 | 2 | 0 | 0 | 312 |  | 0 |
| tcl-lsp-server-wasi | 27 | 27 | 0 | 0 | 430 |  | 0 |
| tcl-lsp-server-wasm | does not build for the host | - | - | - | - | error[E0428]: the name `to_jsonrpc_error` is defined multiple times | 0 |
| tcl-spec-studio-wasm | not run |  |  |  |  | the volume filled during the sweep (No space left on device); rerun |  |
| tcl-vm-wasm | not run |  |  |  |  | the volume filled during the sweep (No space left on device); rerun |  |
| bigip-query-wasm | not run |  |  |  |  | the volume filled during the sweep (No space left on device); rerun |  |

## Notes

- Failing (not triaged): `tcl-debugger` 1, `tcl-fuzz` 2, `tcl-cli-support` 1, `tcl-engine-wasm` 3. Names are in `lists/unmeasured-crates-failing-2026-11-18.txt`.
- `tcl-lsp-server-wasm` does not build for the host: it defines `to_jsonrpc_error` and `Decoder` twice (E0428, E0252). It targets wasm32; the first error is in `/tmp/sweep-tcl-lsp-server-wasm.log` (lost with the container). Whether CI builds it on the host is not checked.
- `tcl-spec-studio-wasm`, `tcl-vm-wasm` and `bigip-query-wasm` are **not run**: the disk filled (the sweep builds each excluded crate in its own `target/`). Rerun after clearing `rust/*/target`.
- Passing crates (bpf-tcl, bpf-tcl-codegen, bpf-tcl-ir, tcl-host-c-abi, tcl-host-native, tcl-pkg, tcl-pkg-model, tcl-platform, tcl-regex, tcl-sandbox, tcl-userdirs, tcl-version, tcl-test-support, zed-query-check, tcl-explorer-wasm, tcl-lsp-server-wasi) can go on the scoreboard as measured.
- The integration targets named for confirmation (`tcl-spectcl` `eval_loader` and `it_3`, `tcl-spec-studio` `it_3`/`it_4`/`it_5`, `f5-cli` `query`/`query_input`, `tcl-bigip` and `tcl-bigip-query` `integration`) are auto-discovered test targets, and `cargo nextest run -p <crate>` runs every target of a package. They were in the 08:00 run; I did not list the binaries to prove it.
