// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Offline compiler coverage for verbatim, licensed Tcl programs that perform
//! real jobs. Fixture provenance and complete upstream licence texts live in
//! `fixtures/real_jobs/README.md`.

use tcl_compiler::codegen::wasm::{WasmCompileOptions, compile_wasm};
use tcl_compiler::compilation_unit::CompilationUnit;
use tcl_compiler::lowering::lower_to_ir;
use tcl_registry::model::ingress::static_context_for;
use tcl_registry::model::semantic::SemanticContext;

const SQAWK_ASSEMBLE: &str = include_str!("fixtures/real_jobs/sqawk/assemble.tcl");
const ODO_CHECKSUM: &str = include_str!("fixtures/real_jobs/odo-miner/checksum.tcl");

struct Fixture {
    name: &'static str,
    source: &'static str,
}

const FIXTURES: &[Fixture] = &[
    Fixture {
        name: "Sqawk tools/assemble.tcl",
        source: SQAWK_ASSEMBLE,
    },
    Fixture {
        name: "Odo miner src/miner/checksum.tcl",
        source: ODO_CHECKSUM,
    },
];

#[test]
fn real_jobs_lower_and_keep_semantics_around_opaque_regions() {
    let registry = static_context_for("tcl8.6").commands();

    for fixture in FIXTURES {
        // Exercise the public lowering API independently of CompilationUnit so
        // a future unit-builder shortcut cannot hide a source-parser failure.
        let ir = lower_to_ir(fixture.source, registry);
        assert!(
            ir.top_level.statements.len() > 1,
            "{} should remain a multi-statement real program",
            fixture.name,
        );

        let unit = CompilationUnit::build_for_dialect(fixture.source, registry, false, "tcl8.6");
        assert_eq!(
            unit.top_level.semantic_facts.context(),
            Some(SemanticContext::for_environment("tcl8.6")),
            "{} must retain the explicitly selected profile",
            fixture.name,
        );

        let mixed_region = unit.all_body_function_units().find(|function| {
            let executable = function.semantic_facts.executable();
            executable.function().is_some()
                && executable.opaque_regions().next().is_some()
                && (executable.invocations().next().is_some()
                    || executable.lowered_operations().next().is_some())
        });
        let mixed_region = mixed_region.unwrap_or_else(|| {
            panic!(
                "{} should retain ordinary semantic operations on at least one side of an opaque region",
                fixture.name,
            )
        });
        let executable = mixed_region.semantic_facts.executable();
        assert!(
            executable.completion_inputs().count() > executable.opaque_regions().count(),
            "{} must not turn the whole mixed function into one decline",
            fixture.name,
        );
    }
}

#[test]
fn real_jobs_compile_with_the_canonical_wasm_api() {
    let registry = static_context_for("tcl8.6").commands();

    for fixture in FIXTURES {
        let unit = CompilationUnit::build_for_dialect(fixture.source, registry, false, "tcl8.6");
        let mut wasm = compile_wasm(&unit, registry, WasmCompileOptions::hosted());
        let bytes = wasm.to_bytes();

        // `WasmModule::to_bytes` is the library's validated binary encoder;
        // pin both mandatory preamble fields and a real exported entry point,
        // rather than relying on a command-line validator being installed.
        assert_eq!(
            bytes.get(..8),
            Some(&[0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00][..]),
            "{} did not produce a WebAssembly 1 module",
            fixture.name,
        );
        assert!(
            wasm.to_wat().contains(r#"(export "::top")"#),
            "{} should export the compiled top-level entry",
            fixture.name,
        );
    }
}
