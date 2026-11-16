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

use super::original_store_metadata;
use crate::analyser::ResolvedAnalysisInput;
use crate::compilation_unit::{CompilationUnit, UnitBuildOptions};
use std::sync::Arc;

fn original_unit() -> (
    Arc<tcl_registry::model::ContextRegistry>,
    CompilationUnit,
    tcl_vm::Vm,
) {
    let context =
        tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
    let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
    let config = tcl_lexer::LexerConfig::for_profile(Some(profile));
    let input = ResolvedAnalysisInput::new(profile, profile, Arc::clone(&context), config);
    let source = "set {$literal} FIRST; set {$literal} SECOND";
    let (native_owner, captured) =
        crate::environment_ingress::captured_native_entry_with_owner(profile);
    let entry = crate::command_binding::SourceAnalysisEntry {
        native_entry: Some(Arc::new(captured)),
        invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
        native_compilation: crate::environment_ingress::authoring_native_compilation(),
        ..Default::default()
    };
    let unit = CompilationUnit::build_with_analysis_input(
        source,
        UnitBuildOptions {
            registry: context.commands(),
            defer_top_level: false,
            config,
            dialect: Some(profile),
            external_call_sites: None,
            declared_commands: None,
        },
        Some(&entry),
        &input,
    );
    (context, unit, native_owner)
}

#[test]
fn original_store_points_keep_actual_module_function_and_source_owners() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // This authentic original point validates metadata only. A native cell,
    // literal contents, adjacency and callbacks remain separate store gates.
    let (context, unit, _native_owner) = original_unit();
    let input = unit.top_level.source_metadata_input().unwrap();
    let profile = input.unit_profile();
    let config = input.lexer_config();
    let function = &unit.top_level;
    let (&block, data) = function
        .cfg
        .blocks
        .iter()
        .find(|(_, block)| !block.statements.is_empty())
        .unwrap();
    let tokens = function
        .cfg
        .source_tokens_at(block, 0)
        .expect("original setter vector survives structural lowering");
    assert!(!data.statements[0].span().is_empty());
    assert!(original_store_metadata(function, tokens, context.commands(), None).is_some());
    let module = crate::interprocedural::ModuleProcedures::of_unit(&unit, context.commands());
    assert!(original_store_metadata(function, tokens, context.commands(), Some(&module)).is_some());
    let older = Arc::new(
        tcl_registry::model::ingress::static_context_for("tcl8.4")
            .with_command_store(Arc::clone(context.commands())),
    );
    assert!(Arc::ptr_eq(context.commands(), older.commands()));
    for facet in 0..4 {
        let mut changed = function.clone();
        match facet {
            0 => changed.source_metadata_input = None,
            1 => {
                changed.source_metadata_input = Some(ResolvedAnalysisInput::new(
                    profile,
                    profile,
                    Arc::clone(&older),
                    config,
                ));
            }
            2 => {
                changed.source_metadata_input = Some(ResolvedAnalysisInput::new(
                    profile,
                    profile,
                    tcl_registry::model::ingress::resolve_environment("tcl9.1")
                        .default_context_registry(),
                    config,
                ));
            }
            3 => changed.source_config.strict_quoting = !changed.source_config.strict_quoting,
            _ => unreachable!(),
        }
        assert!(
            original_store_metadata(&changed, tokens, context.commands(), None).is_none(),
            "changed function facet {facet}"
        );
        assert!(
            original_store_metadata(&changed, tokens, context.commands(), Some(&module)).is_none(),
            "changed module/function facet {facet}"
        );
    }
    let mut changed = tokens.clone();
    changed.word_exprs.pop();
    assert!(
        original_store_metadata(function, &changed, context.commands(), Some(&module)).is_none()
    );
    let mut stale = unit.clone();
    stale.ir_module.top_level_namespace = "::stale".into();
    let stale_module =
        crate::interprocedural::ModuleProcedures::of_unit(&stale, context.commands());
    assert!(
        original_store_metadata(function, tokens, context.commands(), Some(&stale_module))
            .is_none()
    );
}
