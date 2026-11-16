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

use super::SourceInvocationBinding;

fn bytecode_list_fixture(source: &str) -> (crate::compilation_unit::CompilationUnit, tcl_vm::Vm) {
    let context = tcl_registry::model::ingress::resolve_known_environment("tcl8.6")
        .unwrap()
        .default_context_registry();
    let registry = context.commands();
    let profile = registry.profile().unwrap();
    let config = tcl_lexer::LexerConfig::for_profile(Some(profile));
    let input = crate::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        std::sync::Arc::clone(&context),
        config,
    );
    let (owner, native) = crate::environment_ingress::captured_native_entry_with_owner(profile);
    let mut entry = crate::command_binding::SourceAnalysisEntry::for_supplied_source(
        registry,
        &input,
        config,
        Some(profile),
    );
    entry.native_entry = Some(std::sync::Arc::new(native));
    entry.native_compilation.mode =
        tcl_registry::native_compilation::NativeCompilationMode::BytecodeObject;
    let unit = crate::compilation_unit::CompilationUnit::build_with_analysis_input(
        source,
        crate::compilation_unit::UnitBuildOptions {
            registry,
            defer_top_level: false,
            config,
            dialect: Some(profile),
            external_call_sites: None,
            declared_commands: None,
        },
        Some(&entry),
        &input,
    );
    (unit, owner)
}

#[test]
fn compiled_list_head_original_owner_refuses_unknown_binding_join() {
    let context = tcl_registry::model::ingress::static_context_for("tcl8.6");
    let registry = context.commands();
    let (unit, _owner) =
        bytecode_list_fixture("set payload [gets stdin]; puts [list puts $payload]");
    let ssa = &unit.top_level.ssa;
    let parent = ssa
        .blocks
        .iter()
        .flat_map(|(block, data)| {
            (0..data.statements.len()).filter_map(move |index| {
                crate::ssa::SsaSourceView::at_statement(ssa, *block, index).source_tokens()
            })
        })
        .find(|tokens| tokens.argv_texts.first().is_some_and(|head| head == "puts"))
        .unwrap();
    let metadata = crate::registry_invocation::InvocationMetadataContext::for_module(
        registry,
        &unit.ir_module,
    )
    .unwrap();
    let mut joined =
        crate::word_subst::checked_original_argument_lifted_calls(parent, registry, metadata)
            .unwrap()
            .into_iter()
            .find_map(|child| child.tokens)
            .unwrap();
    assert_eq!(
        crate::registry_invocation::original_compiled_list_head(
            registry, metadata, parent, &joined,
        )
        .as_deref(),
        Some("puts")
    );
    joined
        .source_binding
        .as_mut()
        .unwrap()
        .join(&SourceInvocationBinding::default());
    assert!(
        crate::registry_invocation::original_compiled_list_head(
            registry, metadata, parent, &joined,
        )
        .is_none()
    );
}
