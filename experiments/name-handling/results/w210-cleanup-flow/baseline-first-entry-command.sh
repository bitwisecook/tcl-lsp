source /tmp/name-env.sh
CARGO_TARGET_DIR=/workspace/tcl-lsp/tmp/name-target-issues timeout 600s cargo test --offline -j2 -p tcl-compiler --lib command_binding::declaration_flow::tests::declared_loop_first_entry_does_not_supply_runtime_zero_trip_proof -- --nocapture
