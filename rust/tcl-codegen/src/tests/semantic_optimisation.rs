use tcl_compiler::semantic_optimisation::SemanticOptimisationConfig;

/// The `native-tier` group and `WasmCompileOptions::native_tier` are two
/// spellings of one set; a pass added to the option and not the group
/// would make the CLI and the Explorer quietly weaker than the harness.
#[test]
fn native_tier_group_matches_the_wasm_option() {
    let group = SemanticOptimisationConfig::from_names("native-tier").unwrap();
    let option = crate::codegen::wasm::WasmCompileOptions::hosted()
        .native_tier()
        .semantic_optimisations();
    assert_eq!(group, option);
}
