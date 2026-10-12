source /tmp/name-env.sh
CARGO_TARGET_DIR=/workspace/tcl-lsp/tmp/name-target-issues timeout 600s cargo test --offline -j2 -p tcl-compiler --lib command_binding::declaration_flow::tests::original_declaration_graph_keeps_zero_trip_and_no_match_successors -- --nocapture
