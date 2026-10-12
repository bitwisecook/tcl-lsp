source /tmp/name-env.sh
CARGO_TARGET_DIR=/workspace/tcl-lsp/tmp/name-target-issues timeout 300s cargo test --offline -j2 -p tcl-compiler --lib script_binds::tests::declaration_read_advice_requires_original_unchanged_body_and_formals -- --exact --nocapture
