source /tmp/name-env.sh
CARGO_TARGET_DIR=/workspace/tcl-lsp/tmp/name-target-issues timeout 300s cargo test --offline -j2 -p tcl-lsp-server --test e2e imported_tcltest_test_structure_recognised -- --nocapture
