source /tmp/name-env.sh
CARGO_TARGET_DIR=/workspace/tcl-lsp/tmp/name-target-issues timeout 600s cargo test --offline -j2 -p tcl-compiler --lib tcltest_aliases_keep_genuine_undefined_reads_and_result_data_opaque -- --nocapture
