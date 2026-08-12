// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Architecture guard for the single public WASM code-generation pipeline.

use std::path::{Path, PathBuf};

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn source(root: &Path, relative: &str) -> String {
    std::fs::read_to_string(root.join(relative))
        .unwrap_or_else(|error| panic!("read {relative}: {error}"))
}

#[test]
fn production_consumers_call_the_canonical_codegen_api() {
    let root = workspace_root();
    for relative in [
        "rust/tcl-cli/src/commands/compile.rs",
        "rust/tcl-explorer/src/serialise.rs",
        "rust/tcl-fuzz/src/linked_wasm.rs",
        "rust/tcl-fuzz/src/wasm.rs",
        "rust/tcl-fuzz/src/wasm_diff.rs",
        "rust/tcl-mcp/src/tools.rs",
        "rust/tcl-compiler/examples/emit_wasm.rs",
    ] {
        let contents = source(&root, relative);
        assert!(
            contents.contains("compile_wasm("),
            "{relative} must call the canonical compile_wasm API"
        );
    }
}

#[test]
fn no_production_surface_exposes_a_legacy_codegen_selector() {
    let root = workspace_root();
    let forbidden = [
        concat!("wasm_codegen", "_module"),
        concat!("wasm_codegen", "_compilation_unit"),
        concat!("compile_literal", "_safe_wasm"),
        concat!("LiteralSafe", "Wasm"),
        concat!("Wasm", "Backend"),
        concat!("Tree", "Walker"),
        concat!("--", "backend"),
    ];
    for relative in [
        "rust/tcl-compiler/src/codegen/wasm/mod.rs",
        "rust/tcl-cli/src/cli.rs",
        "rust/tcl-cli/src/commands/compile.rs",
        "rust/tcl-explorer/src/serialise.rs",
        "rust/tcl-fuzz/src/linked_wasm.rs",
        "rust/tcl-fuzz/src/wasm.rs",
        "rust/tcl-fuzz/src/wasm_diff.rs",
        "rust/tcl-mcp/src/tools.rs",
    ] {
        let contents = source(&root, relative);
        for name in forbidden {
            assert!(
                !contents.contains(name),
                "{relative} retains forbidden public codegen surface {name}"
            );
        }
    }

    let public_module = source(&root, "rust/tcl-compiler/src/codegen/wasm/mod.rs");
    assert!(public_module.contains("compile_wasm"));
    assert!(public_module.contains("WasmCompilation"));
    assert!(public_module.contains("WasmCodegenPlan"));
}

#[test]
fn wasm_has_one_module_emitter_for_semantic_and_general_plans() {
    let root = workspace_root();
    let wasm_dir = root.join("rust/tcl-compiler/src/codegen/wasm");
    assert!(
        !wasm_dir.join("executable.rs").exists(),
        "the former parallel executable-IR emitter must not return"
    );

    let backend = source(&root, "rust/tcl-compiler/src/codegen/wasm/backend.rs");
    let pipeline = source(&root, "rust/tcl-compiler/src/codegen/wasm/pipeline.rs");
    let semantic_plan = source(&root, "rust/tcl-compiler/src/codegen/wasm/semantic_plan.rs");
    let production = format!("{backend}\n{pipeline}\n{semantic_plan}");
    assert_eq!(
        production.matches("fn emit_wasm(").count(),
        1,
        "WASM must have exactly one module-emission entry"
    );
    assert_eq!(
        production.matches("WasmModule::new()").count(),
        1,
        "semantic and general plans must share one module constructor"
    );
    assert!(!production.contains("emit_wasm_generic"));
    assert!(!production.contains("WasmCodegenPlan::Compatibility"));
    assert!(!production.contains("WasmCompatibilityReason"));
    assert!(backend.contains("WasmEmissionMode::SemanticInvoke"));
    assert!(backend.contains("WasmEmissionMode::General"));
}

#[test]
fn architecture_doc_does_not_advertise_removed_backend_choices() {
    let root = workspace_root();
    let document = source(&root, "docs/design/compiler/wasm-codegen.md");
    for stale in [
        concat!("--backend ", "tree-walker"),
        concat!("--backend ", "generic-invoke"),
        concat!("--backend ", "vm"),
        concat!("wasm_codegen", "_compilation_unit"),
        concat!("compile_literal", "_safe_wasm"),
    ] {
        assert!(
            !document.contains(stale),
            "WASM architecture document advertises removed choice {stale}"
        );
    }
}

#[test]
fn architecture_docs_do_not_advertise_unimplemented_package_or_completion_coverage() {
    let root = workspace_root();
    let codegen = source(&root, "docs/design/compiler/wasm-codegen.md");
    assert!(
        !codegen.contains("The optional package linker scans"),
        "WASM documentation must not advertise the future package-driven linker"
    );
    assert!(
        !codegen.contains("full Tcl completion flow explicit"),
        "bounded executable IR must not be documented as complete Tcl completion coverage"
    );
    assert!(codegen.contains("There is currently no compiler package-require scan"));
    assert!(codegen.contains("Already-lowered operations and opaque"));
}

#[test]
fn semantic_aot_contract_keeps_code_changing_passes_default_off() {
    let root = workspace_root();
    let contract = source(&root, "docs/design/compiler/semantic-aot-optimisation.md");
    for required in [
        "Every new code-changing semantic AOT pass is disabled by default",
        "Each control must be independently disableable",
        "Word evaluation precedes the dispatch guard",
        "`TclType::Int` does not mean WASM `i32` or `i64`",
        "Command-specific knowledge belongs in `CommandRegistry`",
    ] {
        assert!(
            contract.contains(required),
            "semantic AOT contract lost required invariant: {required}"
        );
    }
}
