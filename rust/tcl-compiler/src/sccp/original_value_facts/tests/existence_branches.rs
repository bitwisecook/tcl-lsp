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

//! Existence folds: the inventory of names a body defines or destroys, up-frame bodies and the entry and query they require.

use super::*;

#[test]
fn existence_inventory_keeps_literal_destination_names_and_unknown_targets() {
    let mut cfg = CfgFunction::new("source body", "entry");
    let entry = cfg.entry;
    for name in ["", "$literal", "${braced}", "$array(key)"] {
        cfg.blocks
            .get_mut(&entry)
            .unwrap()
            .statements
            .push(Statement::AssignConst {
                span: tcl_lexer::Span::new(0, 0),
                name: name.into(),
                name_braced: true,
                value: "1".into(),
                value_span: None,
            });
    }
    let registry = CommandRegistry::build_default()
        .project_for_profile(tcl_dialect::DialectProfile::find("tcl9.1").unwrap());
    let metadata = ExistenceMetadataInputs {
        registry: &registry,
        context: registry
            .profile()
            .map(tcl_registry::model::semantic::SemanticContext::for_profile)
            .map(Into::into),
    };
    let (defined, _, unknown) = scan_defined_and_unset(&cfg, metadata);
    assert_eq!(
        defined,
        FxHashSet::from_iter(
            ["", "$literal", "${braced}", "$array"]
                .into_iter()
                .map(str::to_owned)
        )
    );
    assert!(!unknown.writes);
    assert!(
        !defined.contains("literal") && !defined.contains("braced") && !defined.contains("array")
    );
    cfg.blocks
        .get_mut(&entry)
        .unwrap()
        .statements
        .push(Statement::AssignConst {
            span: tcl_lexer::Span::new(0, 0),
            name: "$target".into(),
            name_braced: false,
            value: "1".into(),
            value_span: None,
        });
    let (defined, _, unknown) = scan_defined_and_unset(&cfg, metadata);
    assert!(unknown.writes);
    assert!(!defined.contains("target"));
}

#[test]
fn source_destroy_inventory_keeps_original_empty_sigil_and_array_name_data() {
    let (context, unit) = logical_callback_unit("unset {} {$literal} {arr(key)}");
    let registry = context.commands();
    let metadata = ExistenceMetadataInputs {
        registry,
        context: crate::registry_invocation::InvocationMetadataContext::for_analysis_input(
            registry,
            unit.ir_module.source_metadata_input.as_ref().unwrap(),
        ),
    };
    let (_, unset, unknown) = scan_defined_and_unset(&unit.top_level.cfg, metadata);
    for name in ["", "$literal", "arr(key)", "arr"] {
        assert!(unset.contains(name), "missing original {name:?}: {unset:?}");
    }
    assert!(!unset.contains("literal"));
    assert!(unknown.destroys);
    let (_, _, refused) = scan_defined_and_unset(
        &unit.top_level.cfg,
        ExistenceMetadataInputs {
            registry,
            context: None,
        },
    );
    assert!(refused.writes && refused.destroys);
}

#[test]
fn nominal_destroy_inventory_uses_selected_roles_and_keeps_unknown_argv() {
    let registry = CommandRegistry::build_default()
        .project_for_profile(tcl_dialect::DialectProfile::find("tcl9.1").unwrap());
    let metadata = ExistenceMetadataInputs {
        registry: &registry,
        context: registry
            .profile()
            .map(tcl_registry::model::semantic::SemanticContext::for_profile)
            .map(Into::into),
    };
    let mut cfg = CfgFunction::new("explicit standalone", "entry");
    cfg.blocks
        .get_mut(&cfg.entry)
        .unwrap()
        .statements
        .push(Statement::Call {
            span: tcl_lexer::Span::new(0, 0),
            command: "::unset".into(),
            canonical_command: None,
            args: vec![
                String::new(),
                "array(key)".into(),
                "$computed".into(),
                "nul\0tail".into(),
            ],
            defs: Vec::new(),
            reads: Vec::new(),
            reads_own_defs: false,
            safe_on_uninit: false,
            tokens: None,
            foreach_groups: None,
        });
    let (_, unset, unknown) = scan_defined_and_unset(&cfg, metadata);
    assert!(unset.contains("") && unset.contains("array") && unset.contains("array(key)"));
    assert!(!unset.contains("computed") && !unset.contains("nul") && !unset.contains("nul\0tail"));
    assert!(unknown.destroys);
    cfg.metadata_context = crate::registry_invocation::OwnedInvocationMetadataContext::Unavailable;
    let (_, unset, unknown) = scan_defined_and_unset(&cfg, metadata);
    assert!(unset.is_empty() && unknown.writes && unknown.destroys);
}

/// A literal body resolves its actual selected-frame mutation; a dynamic
/// body retains unknown contents and presence on its reached continuation.
fn native_existence_unit(
    source: &str,
    registry: &CommandRegistry,
) -> crate::environment_ingress::RetainedNativeUnit {
    let profile = registry.profile().expect("selected native fixture profile");
    let (owner, captured) = crate::environment_ingress::captured_native_entry_with_owner(profile);
    let entry = crate::command_binding::SourceAnalysisEntry {
        native_entry: Some(std::sync::Arc::new(captured)),
        invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
        native_compilation: tcl_registry::native_compilation::NativeCompilationContext {
            mode: tcl_registry::native_compilation::NativeCompilationMode::Direct,
            ..Default::default()
        },
        ..crate::command_binding::SourceAnalysisEntry::default()
    };
    let unit = crate::compilation_unit::CompilationUnit::build_with_source_entry(
        source,
        crate::compilation_unit::UnitBuildOptions {
            registry,
            defer_top_level: false,
            config: tcl_lexer::LexerConfig::from_grammar(profile.grammar),
            dialect: Some(profile),
            external_call_sites: None,
            declared_commands: None,
        },
        &entry,
    );
    crate::environment_ingress::RetainedNativeUnit::new(unit, owner)
}

#[test]
fn existence_fold_resolves_literal_upframe_and_declines_dynamic_body() {
    let registry = CommandRegistry::build_default()
        .project_for_profile(tcl_dialect::DialectProfile::find("tcl8.6").unwrap());
    let cu = native_existence_unit(
        "proc f {} { uplevel 0 { set created 1 }; if {[info exists created]} { return yes } else { return no } }; f",
        &registry,
    );
    let f = cu.function("::f").expect("procedure analysed");
    assert!(
        f.cfg
            .blocks
            .values()
            .flat_map(|block| block.statements.iter())
            .any(|statement| matches!(statement, Statement::UpFrame { .. })),
        "the regression requires the static uplevel lowering path"
    );
    assert!(
        f.sccp.constant_branches.iter().any(|branch| branch.value),
        "the literal selected-frame store creates the queried cell: {:?}",
        f.sccp.constant_branches,
    );
    let dynamic = native_existence_unit(
        "proc f {script} {uplevel 0 $script; if {[info exists created]} {return yes} else {return no}}; f [mystery]",
        &registry,
    );
    assert!(
        dynamic
            .function("::f")
            .unwrap()
            .sccp
            .constant_branches
            .is_empty(),
        "a caller-supplied body may create or remove the queried cell",
    );
}

#[test]
fn existence_fold_resolves_nested_upframe_alias_unset_mutation() {
    // `uplevel 0` evaluates in this procedure's frame. The nested body
    // aliases that frame's parameter and unsets it, so Tcl observes the else
    // branch. The no-uplevel twin proves the branch is otherwise foldable;
    // this tests the resolved mutation rather than a whole-function barrier.
    let registry = CommandRegistry::build_default()
        .project_for_profile(tcl_dialect::DialectProfile::find("tcl8.6").unwrap());
    let stable = native_existence_unit(
        "proc f {local} { if {[info exists local]} { return yes } else { return no } }; f VALUE",
        &registry,
    );
    assert!(
        !stable
            .function("::f")
            .expect("control procedure analysed")
            .sccp
            .constant_branches
            .is_empty(),
        "the unmutated control must be foldable"
    );

    let mutated = native_existence_unit(
        "proc f {local} { uplevel 0 { uplevel 0 { upvar 0 local alias; unset alias } }; if {[info exists local]} { return yes } else { return no } }; f VALUE",
        &registry,
    );
    let f = mutated.function("::f").expect("mutated procedure analysed");
    let outer = f
        .cfg
        .blocks
        .values()
        .flat_map(|block| block.statements.iter())
        .find_map(|statement| match statement {
            Statement::UpFrame { body, .. } => Some(body),
            _ => None,
        })
        .expect("outer literal uplevel must lower to UpFrame");
    assert!(
        outer
            .statements
            .iter()
            .any(|statement| matches!(statement, Statement::UpFrame { .. })),
        "the nested literal uplevel must remain an UpFrame in its parent body"
    );
    let branches = &f.sccp.constant_branches;
    assert!(
        branches.iter().any(|branch| !branch.value),
        "the literal alias unset removes the selected local: {branches:?}",
    );
    assert!(branches.iter().all(|branch| !branch.value));
}

#[test]
fn existence_folding_requires_actual_entry_and_selected_query() {
    let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
    let registry = CommandRegistry::build_default().project_for_profile(profile);
    let declaration = "proc f {local} {if {[info exists local]} {return yes} else {return no}}";
    let missing_entry = crate::compilation_unit::CompilationUnit::build_with_source_entry(
        &format!("{declaration}; f VALUE"),
        crate::compilation_unit::UnitBuildOptions {
            registry: &registry,
            defer_top_level: false,
            config: tcl_lexer::LexerConfig::from_grammar(profile.grammar),
            dialect: Some(profile),
            external_call_sites: None,
            declared_commands: None,
        },
        &crate::command_binding::SourceAnalysisEntry {
            unknown_entry: true,
            invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
            ..Default::default()
        },
    );
    assert!(missing_entry.function("::f").is_none());
    assert_missing_entry_retains_only_declared_query(declaration, &registry, profile);
    for source in [
        declaration.to_owned(),
        format!("rename info {{}}; proc info args {{return UNKNOWN}}; {declaration}; f VALUE"),
        format!("{declaration}; mystery; f VALUE"),
    ] {
        let unit = native_existence_unit(&source, &registry);
        assert!(
            unit.function("::f")
                .unwrap()
                .sccp
                .constant_branches
                .is_empty(),
            "unentered or unselected query must not fold: {source}"
        );
    }
}

fn assert_missing_entry_retains_only_declared_query(
    declaration: &str,
    registry: &CommandRegistry,
    profile: &tcl_dialect::DialectProfile,
) {
    let config = tcl_lexer::LexerConfig::from_grammar(profile.grammar);
    let bindings = crate::command_binding::SourceCommandBindings::analyse_with_options(
        declaration,
        config,
        registry,
        crate::command_binding::SourceAnalysisOptions {
            unknown_entry: true,
            invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
            native_compilation: crate::environment_ingress::authoring_native_compilation(),
            ..Default::default()
        },
    );
    let query = "info exists local";
    let offset = u32::try_from(declaration.find(query).unwrap()).unwrap();
    let segment =
        crate::segmenter::segment_commands_with_offset_and_config(query, offset, config).remove(0);
    let mut tokens = crate::ir::CommandTokens::from_segmented(
        &tcl_lexer::SourceMap::new(declaration),
        config,
        &segment,
    );
    bindings.stamp_original_tokens(&mut tokens);
    let binding = tokens.source_binding.as_ref().unwrap();
    let layout = binding
        .declaration_operand_layout_advice(&tokens)
        .expect("the original declaration retains conditional query geometry");
    assert!(!layout.closed_lookup());
    assert!(binding.unknown);
    assert!(binding.proved_execution_target().is_none());
    assert!(binding.compiler_lookup_state.is_none());
    let site = binding.invocation_site().unwrap();
    let lexical_body = bindings
        .conditional_body_entry_at(&site.source, site.offset)
        .expect("conditional lexical source is independent of actual entry");
    assert!(lexical_body.owns_source(&site.source, site.offset));
    assert!(!bindings.has_actual_procedure_entry_at(&site.source, site.offset));
    assert_eq!(
        binding.native_compilation_admission_selection(),
        tcl_registry::native_compilation::NativeCompilationSelection::Unknown
    );
}

#[test]
fn upframe_body_models_upvar_alias_unset_in_the_caller_frame() {
    // tclsh: `proc caller {} {set caller_value 1; f; info exists
    // caller_value}; proc f {} {uplevel 1 {upvar 0 caller_value alias;
    // unset alias}}; caller` returns 0. The literal UpFrame body therefore
    // carries a real caller-frame mutation, not merely a control-flow
    // wrapper, and is a separate vector from the nested local-frame test.
    let registry = CommandRegistry::build_default();
    let cu = crate::compilation_unit::CompilationUnit::build_for(
        "proc f {} { uplevel 1 { upvar 0 caller_value alias; unset alias } }",
        &registry,
        false,
    );
    let f = cu.function("::f").expect("procedure analysed");
    let body = f
        .cfg
        .blocks
        .values()
        .flat_map(|block| block.statements.iter())
        .find_map(|statement| match statement {
            Statement::UpFrame { body, .. } => Some(body),
            _ => None,
        })
        .expect("literal caller-frame body must lower to UpFrame");
    assert!(body.statements.iter().any(|statement| {
        matches!(statement, Statement::Call { command, .. } if command == "upvar")
    }));
    assert!(body.statements.iter().any(|statement| {
        matches!(statement, Statement::Call { command, .. } if command == "unset")
    }));
}
