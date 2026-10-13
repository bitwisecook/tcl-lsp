// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

// Proposed additions to tests/wasm_real_link.rs; UNCOMPILED / UNEXECUTED.
// Use the actual Runtime/Wasmtime harness, not a software ABI provider stub.

#[test]
fn root_colon_publication_runs_its_own_compiled_table_entry() {
    let Some(runtime) = real_link_runtime() else {
        return;
    };
    let source = "proc :audit {a b} {return [expr {$a + $b}]}\n\
                  proc ordinary {a b} {return [expr {$a - $b}]}\n\
                  set colon_result [:audit 40 2]\n\
                  set ordinary_result [ordinary 10 3]\n";
    // compile_native_bound requires two actual table entries and a call to the
    // additive definition import. The bootstrap requires two runtime native
    // procedure dispatches before evaluating this query. Different body results
    // make binding the colon declaration to the wrong table slot observable.
    assert_eq!(
        run_real_native_proc(
            &runtime,
            "original_argv_colon_table",
            source,
            2,
            "list $colon_result $ordinary_result [info commands audit] [info args :audit]",
            2,
        ),
        "42 7 {} {a b}",
    );
}

#[test]
fn original_argv_descriptor_requires_current_manifest_and_rejects_prior_fingerprint() {
    use tcl_runtime_api::ManifestField;
    let Some(runtime) = real_link_runtime() else {
        return;
    };
    // This is the source-derived fingerprint of the inspected pre-additive
    // descriptor/layout table, retained with its source hash in abi-layout.json.
    // It is not a claim of executing an old Runtime binary or old module.
    const PRIOR_TABLE_FINGERPRINT: u32 = 0x034b_ab19;
    assert_ne!(
        tcl_runtime_api::codegen_abi::CODEGEN_ABI_VERSION,
        PRIOR_TABLE_FINGERPRINT,
    );
    let source = "proc :audit {} {return ORIGINAL}\nset answer [:audit]\n";
    let honest = compile_native_bound(source, 1);
    assert_eq!(check_manifest(&runtime, &honest), Ok(()));
    // A genuine newly compiled module with a foreign prior-table manifest is
    // passed to the actual pre-link identity comparator. Only that manifest
    // field changes; no fake Runtime identity or widened gate is supplied.
    let registry = CommandRegistry::build_default();
    let unit = CompilationUnit::build_for_dialect(source, &registry, false, "tcl9.0");
    let mut foreign = compile_wasm(
        &unit,
        &registry,
        WasmCompileOptions::runtime_linked().native_tier(),
    );
    foreign.manifest.as_mut().expect("actual emitter manifest").abi_version =
        PRIOR_TABLE_FINGERPRINT;
    assert!(matches!(
        check_manifest(&runtime, &foreign.to_bytes()),
        Err(LinkRefusal::Disagrees { field: ManifestField::AbiVersion, .. }),
    ));
    // Actual linking and call_indirect execution also validate the new four-i32
    // import signature; descriptor arithmetic alone cannot establish it.
    assert_eq!(
        run_real_native_proc(
            &runtime,
            "original_argv_current_manifest",
            source,
            1,
            "list $answer [info commands audit]",
            1,
        ),
        "ORIGINAL {}",
    );
}

#[test]
fn general_original_definition_materialises_its_body_once() {
    let Some(runtime) = real_link_runtime() else {
        return;
    };
    let source = "set reads 0\n\
                  proc body_value {} {incr ::reads; return {return MATERIALISED}}\n\
                  proc p {} [body_value]\n\
                  set answer [p]\n";
    assert_eq!(
        run_real_link_argv(
            &runtime,
            "original_argv_materialised_body",
            source,
            "list $answer $reads [string equal [info body p] {return MATERIALISED}]",
        ),
        "MATERIALISED 1 1",
    );
}

#[test]
fn general_original_definition_keeps_braced_body_text_inert() {
    let Some(runtime) = real_link_runtime() else {
        return;
    };
    let source = "set reads 0\n\
                  proc body_value {} {incr ::reads; return UNEXPECTED}\n\
                  proc p {} {return {[body_value]}}\n\
                  set answer [p]\n";
    assert_eq!(
        run_real_link_argv(
            &runtime,
            "original_argv_inert_body",
            source,
            "list [string equal $answer {[body_value]}] $reads [string equal [info body p] {return {[body_value]}}]",
        ),
        "1 0 1",
    );
}
