// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::interprocedural::test_body_identity::fixture_id_if_present;

use super::CompilationUnit;

/// Keep the genuine native interpreter alive through this whole-program CU.
/// This is the same live-entry/Direct fixture ingress as the AOT pipeline;
/// neither a dialect label nor an original source body issues that entry.
fn native_smoke_unit(source: &str) -> crate::environment_ingress::RetainedNativeUnit {
    let environment = tcl_registry::model::ingress::resolve_environment("tcl8.6");
    let profile = environment.unit_profile();
    let context = environment.default_context_registry();
    let (owner, captured) = crate::environment_ingress::captured_native_entry_with_owner(profile);
    let entry = crate::command_binding::SourceAnalysisEntry {
        native_entry: Some(std::sync::Arc::new(captured)),
        invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
        native_compilation: crate::environment_ingress::authoring_native_compilation(),
        ..Default::default()
    };
    let unit = CompilationUnit::build_with_context_registry(
        source,
        super::UnitBuildOptions {
            registry: context.commands(),
            defer_top_level: false,
            config: tcl_lexer::LexerConfig::from_grammar(profile.grammar),
            dialect: Some(profile),
            external_call_sites: None,
            declared_commands: None,
        },
        Some(&entry),
        std::sync::Arc::clone(&context),
    );
    crate::environment_ingress::RetainedNativeUnit::new(unit, owner)
}

/// The canonical "lex -> parse -> segment -> IR -> CFG -> codegen-ready"
/// pipeline on one tiny whole-program compilation. Callable procedure IR
/// requires the genuine live native entry retained by this fixture; source
/// document metadata alone retains declaration analysis independently.
#[test]
fn smoke_compile_canonical_snippet() {
    let src = "proc double {n} { return [expr {$n * 2}] }\nset r [double 21]\n";
    let unit = native_smoke_unit(src);
    assert!(
        fixture_id_if_present(&unit.procedures, "::double").is_some(),
        "expected a lowered ::double procedure, got {:?}",
        unit.procedures.keys().collect::<Vec<_>>()
    );
    assert!(
        !unit.top_level.cfg.blocks.is_empty(),
        "top-level script must lower to a non-empty CFG"
    );
}
