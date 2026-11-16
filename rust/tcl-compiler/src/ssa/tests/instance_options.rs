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

//! Option writes linked to variables: selected, taint and instance options, and what catalogue roles may not donate.

use super::*;

fn logical_option_metadata_fixture(
    source: &str,
) -> (
    std::sync::Arc<tcl_registry::model::ContextRegistry>,
    crate::compilation_unit::CompilationUnit,
) {
    static OPTIONS: &[tcl_registry::hover::OptionSpec] = &[
        tcl_registry::hover::OptionSpec {
            name: "-global",
            value: tcl_registry::hover::OptionValue::global_var_name(),
            ..tcl_registry::hover::OptionSpec::DEFAULT
        },
        tcl_registry::hover::OptionSpec {
            name: "-local",
            value: tcl_registry::hover::OptionValue::var_name(),
            ..tcl_registry::hover::OptionSpec::DEFAULT
        },
        tcl_registry::hover::OptionSpec {
            name: "-new",
            value: tcl_registry::hover::OptionValue::global_var_name(),
            surface: Some(tcl_dialect::model::SpecSurface::TCL90_PLUS),
            ..tcl_registry::hover::OptionSpec::DEFAULT
        },
    ];
    let baseline = tcl_registry::model::ingress::static_context_for("tcl8.6");
    let mut registry = baseline
        .commands()
        .project_for_profile(tcl_dialect::DialectProfile::find("tcl8.6").unwrap());
    registry.insert(tcl_registry::CommandSpec {
        name: "scope-owner",
        options: OPTIONS,
        arity: tcl_registry::Arity::any(),
        ..tcl_registry::CommandSpec::DEFAULT
    });
    let context = std::sync::Arc::new(baseline.with_command_store(std::sync::Arc::new(registry)));
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
    let input = crate::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        std::sync::Arc::clone(&context),
        config,
    );
    let entry = crate::command_binding::SourceAnalysisEntry::for_logical_source(&input).unwrap();
    let unit = crate::compilation_unit::CompilationUnit::build_with_analysis_input(
        source,
        crate::compilation_unit::UnitBuildOptions {
            registry: context.commands(),
            config,
            dialect: Some(profile),
            defer_top_level: false,
            external_call_sites: None,
            declared_commands: None,
        },
        Some(&entry),
        &input,
    );
    (context, unit)
}

#[test]
fn selected_option_writes_keep_source_schema_scope_and_alias_ordinals() {
    use tcl_registry::VariableScope::{CurrentFrame, Global};
    for (source, expected) in [
        (
            "scope-owner -g value -local local",
            vec![(1, 1, Global), (3, 3, CurrentFrame)],
        ),
        (
            "interp alias {} scoped {} scope-owner -global; scoped value",
            vec![(1, 0, Global)],
        ),
        (
            "interp alias {} fixed {} scope-owner -global captured; fixed",
            vec![],
        ),
    ] {
        let (context, unit) = logical_option_metadata_fixture(source);
        let registry = context.commands();
        let metadata = unit
            .top_level
            .invocation_metadata_context_for_module(registry, &unit.ir_module)
            .unwrap();
        let tokens = unit
            .ir_module
            .top_level
            .statements
            .last()
            .unwrap()
            .tokens()
            .unwrap();
        let writes = original_command_option_writes(tokens, metadata, registry)
            .expect("authentic selected original source options");
        assert_eq!(
            writes
                .iter()
                .map(|write| (
                    write.effective_argument,
                    write.written_argument,
                    write.scope
                ))
                .collect::<Vec<_>>(),
            expected,
            "{source}"
        );
        assert!(
            SsaInvocationContext::supplied(registry, Some(metadata))
                .normal(tokens)
                .is_none(),
            "readonly option metadata cannot establish a Native handler"
        );
        let mut changed = tokens.clone();
        changed.argv_texts[0] = "scope-owner".into();
        changed.argv_texts.push("foreign".into());
        assert!(original_command_option_writes(&changed, metadata, registry).is_none());
    }
    for source in [
        "scope-owner $option value",
        "scope-owner {*}$options",
        "scope-owner -new value",
        "proc scope-owner args {}; scope-owner -global value",
    ] {
        let (context, unit) = logical_option_metadata_fixture(source);
        let metadata = unit
            .top_level
            .invocation_metadata_context_for_module(context.commands(), &unit.ir_module)
            .unwrap();
        let tokens = unit
            .ir_module
            .top_level
            .statements
            .last()
            .unwrap()
            .tokens()
            .unwrap();
        assert!(
            original_command_option_writes(tokens, metadata, context.commands()).is_none(),
            "unknown/unavailable/replaced option selection: {source}"
        );
    }
}

#[test]
fn selected_taint_option_writes_require_taint_specific_registry_facts() {
    for (source, expected) in [
        ("entry .e -textvariable value", vec![(2, 2)]),
        ("scope-owner -global value", vec![]),
    ] {
        let (context, unit) = logical_option_metadata_fixture(source);
        let registry = context.commands();
        let metadata = unit
            .top_level
            .invocation_metadata_context_for_module(registry, &unit.ir_module)
            .unwrap();
        let tokens = unit
            .ir_module
            .top_level
            .statements
            .last()
            .unwrap()
            .tokens()
            .unwrap();
        let writes = original_command_taint_option_writes(tokens, metadata, registry)
            .expect("complete selected source command schema");
        assert_eq!(
            writes
                .iter()
                .map(|write| (write.effective_argument, write.written_argument))
                .collect::<Vec<_>>(),
            expected,
            "{source}"
        );
    }
}

#[test]
fn selected_instance_option_writes_keep_genuine_factory_and_written_receiver() {
    for (source, expected) in [
        ("entry .e; .e configure -textvariable value", vec![(2, 2)]),
        (
            "entry .e; interp alias {} configureEntry {} .e configure -textvariable; configureEntry value",
            vec![(2, 0)],
        ),
        (
            "entry .e; interp alias {} fixed {} .e configure -textvariable captured; fixed",
            vec![],
        ),
    ] {
        let (context, unit) = logical_option_metadata_fixture(source);
        let registry = context.commands();
        let metadata = unit
            .top_level
            .invocation_metadata_context_for_module(registry, &unit.ir_module)
            .unwrap();
        let tokens = unit
            .ir_module
            .top_level
            .statements
            .last()
            .unwrap()
            .tokens()
            .unwrap();
        let writes = original_instance_option_writes(tokens, metadata, registry)
            .expect("authentic original selected Registry instance descriptor");
        assert_eq!(
            writes
                .iter()
                .map(|write| (write.effective_argument, write.written_argument))
                .collect::<Vec<_>>(),
            expected,
            "{source}"
        );
        assert!(
            writes
                .iter()
                .all(|write| write.scope == tcl_registry::VariableScope::Global)
        );
        assert!(
            SsaInvocationContext::supplied(registry, Some(metadata))
                .normal(tokens)
                .is_none()
        );
        assert!(
            unit.top_level
                .ssa
                .blocks
                .values()
                .flat_map(|block| &block.statements)
                .all(|statement| statement.defs.is_empty()),
            "source option schema supplies no physical store: {source}"
        );
        let mut detached = tokens.clone();
        detached.source_binding = None;
        assert!(original_instance_option_writes(&detached, metadata, registry).is_none());
    }
    for source in [
        "entry .e; proc .e args {}; .e configure -textvariable value",
        "label .e; .e configure -textvariable value",
        "set w [entry .e]; unknown; $w configure -textvariable value",
    ] {
        let (context, unit) = logical_option_metadata_fixture(source);
        let metadata = unit
            .top_level
            .invocation_metadata_context_for_module(context.commands(), &unit.ir_module)
            .unwrap();
        let tokens = unit
            .ir_module
            .top_level
            .statements
            .last()
            .unwrap()
            .tokens()
            .unwrap();
        assert!(
            original_instance_option_writes(tokens, metadata, context.commands()).is_none(),
            "{source}"
        );
    }
    let (context, unit) = original_operand_metadata_fixture();
    let (block, index, _) = original_setter_definition_point(&unit.top_level);
    let tokens = SsaSourceView::at_statement(&unit.top_level.ssa, block, index)
        .source_tokens()
        .unwrap();
    let metadata = unit
        .top_level
        .invocation_metadata_context_for_module(context.commands(), &unit.ir_module)
        .unwrap();
    assert!(
        SsaInvocationContext::supplied(context.commands(), Some(metadata))
            .normal(tokens)
            .is_some(),
        "the independent actual Native setter/store eligibility remains available"
    );
}

#[test]
fn linked_option_original_values_preserve_literal_dollars_and_element_roots() {
    let (context, unit) = logical_metadata_fixture(
        "info exists {$literal}; info exists {items(key)}; info exists {(key)}; info exists {}",
    );
    let registry = context.commands();
    let metadata = unit
        .top_level
        .invocation_metadata_context_for_module(registry, &unit.ir_module)
        .unwrap();
    for (statement, expected) in unit.ir_module.top_level.statements.iter().zip([
        Some("::$literal"),
        Some("::items"),
        Some("::"),
        None,
    ]) {
        let tokens = statement.tokens().unwrap();
        assert_eq!(
            linked_option_definition_name(tokens, 1, true, metadata, registry).as_deref(),
            expected,
        );
        assert!(original_global_root_frame(tokens, metadata, registry).is_none());
        let mut changed = tokens.clone();
        changed.argv_texts[2] = "different_name".to_owned();
        assert!(linked_option_definition_name(&changed, 1, true, metadata, registry).is_none());
        let mut detached = tokens.clone();
        detached.source_binding = None;
        assert!(linked_option_definition_name(&detached, 1, true, metadata, registry).is_none());
    }
}

#[test]
fn logical_linked_option_unicode_nul_retains_complete_qualified_name_data() {
    for (source, expected) in [
        (
            "info exists \"original_scope::literal\\u0000tail\"",
            "::original_scope::literal\0tail",
        ),
        (
            "info exists \"original_scope::items\\u0000tail(key)\"",
            "::original_scope::items\0tail",
        ),
    ] {
        let (context, unit) = logical_metadata_fixture(source);
        let registry = context.commands();
        let metadata = unit
            .top_level
            .invocation_metadata_context_for_module(registry, &unit.ir_module)
            .unwrap();
        let tokens = unit.ir_module.top_level.statements[0].tokens().unwrap();
        assert_eq!(
            linked_option_definition_name(tokens, 1, true, metadata, registry).as_deref(),
            Some(expected)
        );
        let binding = tokens.source_binding.as_ref().unwrap();
        let input = binding.original_written_name_input(tokens, 2).unwrap();
        let key = input.original_word_key().unwrap();
        assert!(key.bytes().contains(&0));
        assert_ne!(
            key.policy()
                .recipe()
                .combined_variable_input(key.bytes())
                .root()
                .selected(),
            key.bytes()
        );
        assert!(
            SsaInvocationContext::supplied(registry, Some(metadata))
                .normal(tokens)
                .is_none()
        );
    }
}

#[test]
fn top_label_and_option_definition_cannot_supply_global_frame_ownership() {
    let registry = tcl_registry::model::ingress::static_context_for("tk").commands();
    let mut function = Function::new("::top", "entry");
    let block = function.blocks.get_mut(&function.entry).unwrap();
    block.statements.push(call(
        Span::new(0, 34),
        "entry",
        &[".e", "-textvariable", "value"],
        &["::value"],
    ));
    assert!(!mirror_top_level_global_option_defs(
        &mut function,
        registry
    ));
    let Statement::Call { defs, .. } = &function.blocks[&function.entry].statements[0] else {
        panic!("original option statement");
    };
    assert_eq!(defs, &["::value".to_owned()]);
}

#[test]
fn catalogue_widget_option_roles_do_not_donate_physical_store_proof() {
    let registry = tcl_registry::model::ingress::static_context_for("tk").commands();
    let arguments = [".e", "-textvariable", "value"];
    let option = registry
        .get("entry")
        .unwrap()
        .options
        .iter()
        .find(|option| option.name == "-textvariable")
        .unwrap();
    assert!(
        matches!(option.value, tcl_registry::hover::OptionValue::Takes(argument)
            if argument.role == tcl_registry::ArgRole::VarWrite)
    );
    assert_eq!(
        registry.option_variable_scope("entry", &arguments, 2, registry.own_surface_query()),
        Some(tcl_registry::VariableScope::Global)
    );
    let unit = crate::compilation_unit::CompilationUnit::build_standalone_for(
        "entry .e -textvariable value",
        registry,
        false,
    );
    assert!(
        unit.top_level
            .ssa
            .blocks
            .values()
            .flat_map(|block| &block.statements)
            .all(|statement| statement.defs.is_empty())
    );
}

#[test]
fn catalogue_instance_options_do_not_manufacture_live_widget_handlers() {
    let registry = tcl_registry::model::ingress::static_context_for("tk").commands();
    let invocation = registry
        .resolve_instance_invocation(
            "entry",
            ".e",
            &["configure", "-textvariable", "value"],
            registry.own_surface_query(),
        )
        .expect("catalogue instance assistance");
    assert!(
        invocation
            .semantics
            .traits
            .contains(tcl_registry::Traits::CONFIGURES_INSTANCE_OPTIONS)
    );
    let unit = crate::compilation_unit::CompilationUnit::build_standalone_for(
        "entry .e; .e configure -textvariable value",
        registry,
        false,
    );
    assert!(
        unit.top_level
            .ssa
            .blocks
            .values()
            .flat_map(|block| &block.statements)
            .all(|statement| statement.defs.is_empty())
    );
}

#[test]
fn build_ssa_top_level_ordinary_global_set_has_no_bare_alias() {
    let reg = default_registry();
    let mut func = Function::new("::top", "entry");
    let entry = func.entry;
    func.blocks
        .get_mut(&entry)
        .unwrap()
        .statements
        .push(Statement::AssignConst {
            span: Span::new(0, 13),
            name: "::value".into(),
            name_braced: false,
            value: "1".into(),
            value_span: None,
        });
    func.blocks.get_mut(&entry).unwrap().terminator = Some(make_return());

    let ssa = build_ssa(&func, &reg);
    let absolute = ssa.var_symbol("::value").expect("global spelling interned");
    assert_eq!(
        ssa.blocks[&entry].statements[0].defs.get(&absolute),
        Some(&1)
    );
    assert_eq!(ssa.var_symbol("value"), None);
}

#[test]
fn qualified_catalogue_option_keeps_scope_assistance_without_a_store() {
    let registry = tcl_registry::model::ingress::static_context_for("tk").commands();
    let arguments = [".e", "-textvariable", "::ns::value"];
    assert_eq!(
        registry.option_variable_scope("entry", &arguments, 2, registry.own_surface_query()),
        Some(tcl_registry::VariableScope::Global)
    );
    let unit = crate::compilation_unit::CompilationUnit::build_standalone_for(
        "namespace eval ns {}; entry .e -textvariable ::ns::value",
        registry,
        false,
    );
    assert!(
        unit.top_level
            .ssa
            .blocks
            .values()
            .flat_map(|block| &block.statements)
            .all(|statement| statement.defs.is_empty())
    );
    assert_eq!(unit.top_level.ssa.var_symbol("value"), None);
}

#[test]
fn build_ssa_global_option_without_absolute_def_has_no_bare_alias() {
    let reg = default_registry();
    let mut func = Function::new("::top", "entry");
    let entry = func.entry;
    func.blocks.get_mut(&entry).unwrap().statements.push(call(
        Span::new(0, 34),
        "entry",
        &[".e", "-textvariable", "value"],
        &[],
    ));
    func.blocks.get_mut(&entry).unwrap().terminator = Some(make_return());

    let ssa = build_ssa(&func, &reg);
    assert_eq!(ssa.var_symbol("::value"), None);
    assert_eq!(ssa.var_symbol("value"), None);
}

#[test]
fn procedure_widget_catalogue_does_not_write_global_or_local_values() {
    let registry = tcl_registry::model::ingress::static_context_for("tk").commands();
    let unit = crate::compilation_unit::CompilationUnit::build_standalone_for(
        "proc make_entry {} {entry .e -textvariable value; .e configure -textvariable value}",
        registry,
        false,
    );
    let function = unit.function("::make_entry").unwrap();
    assert!(
        function
            .ssa
            .blocks
            .values()
            .flat_map(|block| &block.statements)
            .all(|statement| statement.defs.is_empty())
    );
}
