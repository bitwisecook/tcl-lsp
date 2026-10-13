# Unchanged baseline, with network access for dependency acquisition.
PATH=/workspace/.cargo/bin:$PATH CARGO_HOME=/workspace/.cargo RUSTUP_HOME=/workspace/.rustup CARGO_TARGET_DIR=/workspace/tcl-lsp/tmp/name-target-issues CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 timeout 1200s cargo test -j2 -p tcl-compiler --lib tcltest_ -- --nocapture

# Diagnostic-only instrumented exact original C8.4 control.
PATH=/workspace/.cargo/bin:$PATH CARGO_HOME=/workspace/.cargo RUSTUP_HOME=/workspace/.rustup CARGO_TARGET_DIR=/workspace/tcl-lsp/tmp/name-target-issues CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 TCL_LSP_TRACE_W210_FACTORY=1 timeout 600s cargo test --offline -j2 -p tcl-compiler --lib analyser::diagnostics::tests::tcltest_oracle_program_keeps_missing_reads_and_setup_completion -- --exact --nocapture
