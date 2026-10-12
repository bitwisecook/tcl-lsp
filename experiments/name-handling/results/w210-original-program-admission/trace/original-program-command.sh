source /tmp/name-env.sh
TCL_LSP_TRACE_W210_ORIGINAL=1 CARGO_TARGET_DIR=/workspace/tcl-lsp/tmp/name-target-issues timeout 600s cargo test --offline -j2 -p tcl-compiler --lib analyser::diagnostics::tests::tcltest_oracle_program_keeps_missing_reads_and_setup_completion -- --exact --nocapture
