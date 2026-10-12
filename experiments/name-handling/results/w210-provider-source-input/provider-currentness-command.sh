source /tmp/name-env.sh
CARGO_TARGET_DIR=/workspace/tcl-lsp/tmp/name-target-issues timeout 300s cargo test --offline -j2 -p tcl-compiler --lib provider_fixtures::tests::provider_read_domain_requires_the_same_current_complete_input -- --exact --nocapture
