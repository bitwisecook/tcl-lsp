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
