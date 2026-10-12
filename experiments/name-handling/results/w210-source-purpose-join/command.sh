source /tmp/name-env.sh
CARGO_TARGET_DIR=/workspace/tcl-lsp/tmp/name-target-issues TCL_LSP_W210_SOURCE_JOIN=1 timeout 300s cargo test --offline -j2 -p tcl-compiler --lib analyser::diagnostics::tests::tcltest_shared_frame_keeps_genuine_undefined_reads_and_data_opaque -- --exact --nocapture
