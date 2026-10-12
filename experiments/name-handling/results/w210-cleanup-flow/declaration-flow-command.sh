source /tmp/name-env.sh
CARGO_TARGET_DIR=/workspace/tcl-lsp/tmp/name-target-issues timeout 300s cargo test --offline -j2 -p tcl-compiler --lib command_binding::declaration_flow::tests:: -- --nocapture
