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

use super::*;

fn analyse(
    source: &str,
) -> (
    SourceCommandBindings,
    tcl_lexer::LexerConfig,
    Option<crate::environment_ingress::CapturedNativeEntry>,
) {
    let profile = tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile();
    let registry = tcl_registry::CommandRegistry::build_default().project_for_profile(profile);
    let config = tcl_lexer::LexerConfig::from_grammar(profile.grammar);
    let frame = crate::var_resolve::VariableExecutionFrame::Procedure {
        namespace: "::".to_owned(),
        identity: "inventory-test".to_owned(),
    };
    let entry = Some(crate::environment_ingress::captured_native_entry(profile));
    let bindings = SourceCommandBindings::analyse_in_frame_with_options(
        source,
        &frame,
        config,
        &registry,
        super::super::SourceAnalysisOptions {
            native_entry: entry
                .as_ref()
                .map(crate::environment_ingress::CapturedNativeEntry::as_entry),
            invocation_dialect: Some(tcl_native_model::InvocationDialect::of_profile(profile)),
            native_compilation: tcl_registry::native_compilation::NativeCompilationContext {
                mode: tcl_registry::native_compilation::NativeCompilationMode::BytecodeObject,
                frame: tcl_registry::native_compilation::NativeCompilationFrame::ProcedureCode,
                ..Default::default()
            },
            ..Default::default()
        },
    );
    (bindings, config, entry)
}

#[test]
fn unknown_iterable_retains_the_entered_expression_read_inventory() {
    let source = "proc p {l} {foreach x $l {set y [expr {$x + 1}]}}";
    let (bindings, _, _owner) = analyse(source);
    assert!(
        bindings
            .variable_accesses
            .values()
            .flatten()
            .any(|access| { access.original_spelling == "$x" }),
        "points={:?} accesses={:?}",
        bindings
            .points
            .iter()
            .map(|point| (point.offset, &point.head, point.state.has_opaque_domain()))
            .collect::<Vec<_>>(),
        bindings
            .variable_accesses
            .values()
            .flatten()
            .map(|access| (&access.source, &access.original_spelling))
            .collect::<Vec<_>>()
    );
}

#[test]
fn command_enumeration_condition_retains_closed_full_chunk_admission() {
    let source = "if {[llength [info commands dict]] == 0} {list UNAVAILABLE} else {array set blocked {k OLD}; set d {first NEW second OTHER}; set ok BEFORE; set entered 0; set c [catch {dict update d first ok second blocked {set entered 1}} r]; list $c $ok $entered [catch {set blocked} v] $v}";
    for name in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let fixture = tcl_registry::model::ingress::static_context_for(name);
        let profile = fixture
            .commands()
            .profile()
            .expect("selected native fixture profile");
        let registry = tcl_registry::CommandRegistry::build_default().project_for_profile(profile);
        let config = tcl_lexer::LexerConfig::from_grammar(profile.grammar);
        let bindings = SourceCommandBindings::analyse_with_options(
            source,
            config,
            &registry,
            super::super::SourceAnalysisOptions {
                invocation_dialect: Some(tcl_native_model::InvocationDialect::of_profile(profile)),
                native_compilation: tcl_registry::native_compilation::NativeCompilationContext {
                    mode: tcl_registry::native_compilation::NativeCompilationMode::BytecodeObject,
                    frame: tcl_registry::native_compilation::NativeCompilationFrame::ScriptCode,
                    ..Default::default()
                },
                ..Default::default()
            },
        );
        assert!(
            !bindings.native_compilation_provider_required_at(0),
            "{name}"
        );
    }
}

#[test]
fn return_does_not_remove_later_compiler_admission() {
    let source = "return done; set never 1";
    let (bindings, config, _owner) = analyse(source);
    let offset = u32::try_from(source.find("set").unwrap()).unwrap();
    let binding = bindings.invocation_at_source("", offset);
    assert_eq!(
        binding.runtime_reachability(),
        SourceRuntimeReachability::NotEntered
    );
    assert!(binding.admitted_inline_invocation().is_some());
    assert!(binding.proved_execution_target().is_none());
    assert!(binding.lookup_command_word("set").unknown);
    let map = tcl_lexer::SourceMap::new(source);
    let segment = tcl_source::segment::segment_commands_with_offset_and_config(source, 0, config)
        .into_iter()
        .find(|segment| segment.span.start() == offset)
        .unwrap();
    let mut tokens = crate::ir::CommandTokens::from_segmented(&map, config, &segment);
    tokens.source_binding = Some(binding);
    assert!(
        crate::registry_invocation::native_operation_selection_plan(
            &tokens,
            config.escapes,
            tcl_syntax::word_rules::WordValueRules::from_grammar(
                &config.grammar_over(tcl_dialect::DialectProfile::find("tcl9.0").unwrap().grammar)
            ),
        )
        .unwrap()
        .is_some()
    );
}

#[test]
fn literal_colon_namespace_retains_its_entered_declaration_points() {
    let source =
        "namespace eval : {proc p {} {namespace export exposed; return [namespace current]}}";
    let (bindings, _, _owner) = analyse(source);
    let offset = u32::try_from(source.find("proc p").unwrap()).unwrap();
    let binding = bindings.invocation_at_source("proc", offset);
    assert_eq!(
        binding.runtime_reachability(),
        SourceRuntimeReachability::Reached
    );
    assert_eq!(binding.lookup_namespace, ":::");
    assert!(binding.proved_handler_target().is_some(), "{binding:#?}");
    assert!(bindings.points.iter().any(|point| {
        point.head.as_deref() == Some("namespace")
            && point.offset == u32::try_from(source.find("namespace export").unwrap()).unwrap()
    }));
}

#[test]
fn literal_colon_procedure_inventory_uses_the_published_slot_without_relookup() {
    let source = "namespace eval : {proc p {} {return [namespace current]}}";
    for name in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        let fixture = tcl_registry::model::ingress::static_context_for(name);
        let profile = fixture
            .commands()
            .profile()
            .expect("selected native fixture profile");
        let registry = tcl_registry::CommandRegistry::build_default().project_for_profile(profile);
        let bindings = SourceCommandBindings::analyse_with_options(
            source,
            tcl_lexer::LexerConfig::from_grammar(profile.grammar),
            &registry,
            super::super::SourceAnalysisOptions {
                invocation_dialect: Some(tcl_native_model::InvocationDialect::of_profile(profile)),
                native_compilation: tcl_registry::native_compilation::NativeCompilationContext {
                    mode: tcl_registry::native_compilation::NativeCompilationMode::Direct,
                    frame: tcl_registry::native_compilation::NativeCompilationFrame::ScriptCode,
                    ..Default::default()
                },
                ..Default::default()
            },
        );
        let owner_local = matches!(name, "tcl8.6" | "tcl9.0" | "tcl9.1");
        let expected_key = if owner_local { ":::::p" } else { "::p" };
        let expected_namespace = if owner_local { ":::" } else { "::" };
        assert!(
            bindings.deferred.values().any(|body| {
                body.identity == expected_key && body.namespace == expected_namespace
            }),
            "{name}: {:?}",
            bindings
                .deferred
                .values()
                .map(|body| (&body.identity, &body.namespace))
                .collect::<Vec<_>>()
        );
    }
}

#[test]
fn expanded_head_stamps_the_original_invocation_site_including_its_marker() {
    let source = "set command {set result READY}; {*}$command";
    let (bindings, config, _owner) = analyse(source);
    let map = tcl_lexer::SourceMap::new(source);
    let segments = tcl_source::segment::segment_commands_with_offset_and_config(source, 0, config);
    let segment = segments.last().unwrap();
    let mut tokens = crate::ir::CommandTokens::from_segmented(&map, config, segment);
    assert_ne!(
        tokens.argv[0].start(),
        tokens.words()[0].source().span.start()
    );
    bindings.stamp_original_tokens(&mut tokens);
    let proof = tokens.source_binding.as_ref().unwrap();
    assert_eq!(
        proof.invocation_site().unwrap().offset,
        segment.span.start()
    );
    assert_eq!(proof.evaluated_command_word(), Some("set"));
    assert_eq!(
        proof.runtime_reachability(),
        SourceRuntimeReachability::Reached
    );
}

#[test]
fn dynamically_compiled_unentered_arm_remains_in_parent_compiler_inventory() {
    let source = "set flag 0; if {$flag} {set never 1}; set reached 2";
    let (bindings, config, _owner) = analyse(source);
    let offset = u32::try_from(source.find("set never").unwrap()).unwrap();
    let binding = bindings.invocation_at_source("", offset);
    assert_eq!(
        binding.runtime_reachability(),
        SourceRuntimeReachability::NotEntered
    );
    assert!(binding.admitted_inline_invocation().is_some());
    let map = tcl_lexer::SourceMap::new(source);
    let segment = tcl_source::segment::segment_commands_with_offset_and_config(source, 0, config)
        .into_iter()
        .find(|segment| segment.span.start() == u32::try_from(source.find("if").unwrap()).unwrap())
        .unwrap();
    let mut tokens = crate::ir::CommandTokens::from_segmented(&map, config, &segment);
    bindings.stamp_original_tokens(&mut tokens);
    assert!(tokens.nested_bindings.iter().any(|(site, proof)| {
        *site == offset
            && proof.admitted_inline_invocation().is_some()
            && proof.runtime_reachability() == SourceRuntimeReachability::NotEntered
    }));
}
