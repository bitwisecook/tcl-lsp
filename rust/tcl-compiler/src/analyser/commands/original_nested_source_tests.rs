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

fn nested_heads(source: &str, dialect: &str) -> Vec<CollectedHead> {
    let analysis = Analyser::new().analyse(source, dialect);
    let config = analysis.body_lexer_config.unwrap();
    let sm = SourceMap::new(source);
    let mut heads = Vec::new();
    let mut expressions = Vec::new();
    let last = tcl_source::segment::segment_commands_with_offset_and_config(source, 0, config)
        .pop()
        .unwrap();
    for token in last
        .all_tokens
        .iter()
        .filter(|token| token.kind == TokenType::Cmd)
    {
        collect_substitution_heads(&sm, &analysis, *token, config, &mut heads, &mut expressions);
    }
    heads
}

#[test]
fn named_receiver_inventory_uses_original_ordered_source_carriers() {
    // naming.source.original-class-instance-receiver
    // docs/design/analysis/name-resolution-proofs/source-original-class-instance-receiver.md
    for (source, marker, expected) in [
        (
            "oo::class create C {}; C create obj; obj ping",
            "obj ping",
            true,
        ),
        (
            "oo::class create C {}; C create obj; rename obj moved; moved ping",
            "moved ping",
            true,
        ),
        (
            "oo::class create C {}; C create obj; rename obj {}; obj ping",
            "obj ping",
            false,
        ),
        (
            "oo::class create C {}; C create obj; proc obj args {}; obj ping",
            "obj ping",
            false,
        ),
        (
            "oo::class create C {}; C create obj; rename C {}; obj ping",
            "obj ping",
            false,
        ),
    ] {
        let mut analyser = Analyser::new();
        let analysis = analyser.analyse(source, "tcl8.6");
        let offset = u32::try_from(source.rfind(marker).unwrap()).unwrap();
        let recorded = analyser.var_command_sites.iter().any(|site| {
            site.cmd_span.start() == offset
                && site.receiver == super::super::state::DispatchReceiver::InstanceCommand
        });
        assert_eq!(recorded, expected, "{source}");
        assert_eq!(
            crate::registry_invocation::source_structure::source_class_instance_words_at(
                source, &analysis, offset,
            )
            .is_some(),
            expected,
            "{source}",
        );
        assert!(analysis.created_instance_commands.is_empty());
    }
}

#[test]
fn direct_callbacks_retain_selected_schema_and_exact_prefix_producers() {
    // naming.source.original-editor-body-structure
    // docs/design/analysis/name-resolution-proofs/original-editor-body-structure.md
    for source in [
        "lsort -command {compare fixed} {b a}",
        "rename lsort sorter; sorter -command {compare fixed} {b a}",
        "interp alias {} sorter {} lsort -command {compare fixed}; sorter {b a}",
    ] {
        let analysis = Analyser::new().analyse(source, "tcl8.6");
        let callbacks = analysis
            .command_invocations
            .iter()
            .filter(|invocation| {
                invocation.name == "compare" && invocation.callback_arity.is_some()
            })
            .collect::<Vec<_>>();
        let [callback] = callbacks.as_slice() else {
            panic!("expected exactly one authentic callback: {source}");
        };
        let prefix = callback.original_callback_prefix.as_ref().unwrap();
        assert_eq!(prefix.name_input().bytes(), b"compare");
        assert_eq!(prefix.baked_argument_count(), 1);
        assert_eq!(callback.callback_arity, prefix.appended_arity());
        assert_eq!(callback.callback_baked_args, prefix.baked_argument_count());
        assert_eq!(
            callback.original_name_input.as_ref(),
            Some(prefix.name_input())
        );
        assert!(callback.argc.is_none() && !callback.rename_safe);
    }
    for source in [
        "proc lsort args {}; lsort -command {compare fixed} {b a}",
        "rename lsort {}; lsort -command {compare fixed} {b a}",
        "lsort -command $computed {b a}",
    ] {
        let analysis = Analyser::new().analyse(source, "tcl8.6");
        assert!(
            !analysis
                .command_invocations
                .iter()
                .any(|invocation| invocation.name == "compare"
                    && invocation.callback_arity.is_some()),
            "{source}"
        );
    }
}

#[test]
fn nested_source_bodies_follow_original_moves_alias_prefixes_and_shadow_barriers() {
    // naming.source.original-editor-body-structure
    // docs/design/analysis/name-resolution-proofs/original-editor-body-structure.md
    for source in [
        "rename switch choose; set result [choose key {key {puts nested}}]",
        "interp alias {} choose {} switch --; set result [choose key {key {puts nested}}]",
    ] {
        let heads = nested_heads(source, "tcl8.6");
        assert!(heads.iter().any(|head| head.name == "puts"), "{source}");
        assert!(
            !heads.iter().any(|head| head.name == "key"),
            "a case pattern is data"
        );
    }
    let source = "proc switch args {}; set result [switch key {key {puts inert}}]";
    let heads = nested_heads(source, "tcl8.6");
    assert!(heads.iter().any(|head| head.name == "switch"));
    assert!(!heads.iter().any(|head| head.name == "puts"));
}

#[test]
fn nested_source_bodies_use_the_actual_dialect_availability() {
    // naming.source.original-editor-body-structure
    // docs/design/analysis/name-resolution-proofs/original-editor-body-structure.md
    let source = "set result [dict for {key value} {} {puts nested}]";
    assert!(
        nested_heads(source, "tcl8.6")
            .iter()
            .any(|head| head.name == "puts")
    );
    assert!(
        !nested_heads(source, "tcl8.4")
            .iter()
            .any(|head| head.name == "puts")
    );
}

#[test]
fn nested_source_callbacks_keep_the_original_input_baked_count_and_purpose() {
    // naming.source.original-structured-script-timing
    // docs/design/analysis/name-resolution-proofs/original-structured-script-timing.md
    let source = "proc compare args {}; set result [lsort -command {compare fixed} {a b}]";
    let heads = nested_heads(source, "tcl8.6");
    let head = heads.iter().find(|head| head.name == "compare").unwrap();
    let callback = head.callback.as_ref().unwrap();
    assert_eq!(callback.name_input().bytes(), b"compare");
    assert_eq!(callback.baked_argument_count(), 1);
    assert_eq!(
        callback.appended_arity(),
        Some(tcl_registry::AppendedArity::Exactly(2))
    );
    assert_eq!(source.get(head.span.as_range()), Some("compare"));
    assert!(head.argc.is_none());
}

fn reference_only_script(
    _arguments: tcl_registry::InvocationArguments<'_>,
) -> Vec<(u8, tcl_registry::ScriptTiming)> {
    vec![(0, tcl_registry::ScriptTiming::ReferenceOnly)]
}

#[test]
fn nested_source_inventory_keeps_reference_only_scripts_out_of_dispatch() {
    // naming.source.original-script-region-purpose
    // docs/design/analysis/name-resolution-proofs/original-script-region-purpose.md
    let source = "set result [puts {format nested}]";
    for reference_only in [true, false] {
        let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
        let generation = tcl_registry::model::context_for_profile(profile);
        let mut registry = tcl_registry::CommandRegistry::build_default();
        registry.insert(tcl_registry::CommandSpec {
            name: "puts",
            arity: tcl_registry::Arity::exact(1),
            arg_roles: &[(0, tcl_registry::ArgRole::Body)],
            arg_role_layout_resolver: None,
            arg_role_count_resolver: None,
            arg_role_resolver: None,
            arg_role_resolver_roles: &[],
            script_timing_resolver: reference_only.then_some(reference_only_script),
            ..generation
                .context()
                .resolve_spec(generation.commands(), "puts")
                .unwrap()
                .clone()
        });
        let context =
            std::sync::Arc::new(generation.with_command_store(std::sync::Arc::new(registry)));
        let selected = context
            .context()
            .resolve_spec(context.commands(), "puts")
            .unwrap();
        assert_eq!(selected.arg_roles, &[(0, tcl_registry::ArgRole::Body)]);
        assert_eq!(selected.script_timing_resolver.is_some(), reference_only);
        let config = LexerConfig::for_file_grammar(profile.grammar);
        let input =
            super::super::input::ResolvedAnalysisInput::new(profile, profile, context, config);
        let analysis = Analyser::new()
            .with_resolved_input(input)
            .analyse(source, profile.name);
        let sm = SourceMap::new(source);
        let segment =
            tcl_source::segment::segment_commands_with_offset_and_config(source, 0, config)
                .pop()
                .unwrap();
        let token = *segment
            .all_tokens
            .iter()
            .find(|token| token.kind == TokenType::Cmd)
            .unwrap();
        let mut heads = Vec::new();
        let mut expressions = Vec::new();
        collect_substitution_heads(&sm, &analysis, token, config, &mut heads, &mut expressions);
        assert!(
            heads.iter().any(|head| head.name == "format"),
            "readonly syntax stays visible"
        );
        let mut potential = Vec::new();
        collect_substitution_segments(&sm, &analysis, token, config, &mut potential);
        assert_eq!(
            potential
                .iter()
                .any(|segment| segment.texts.first().is_some_and(|head| head == "format")),
            !reference_only
        );
    }
}

#[test]
fn source_user_class_setters_join_original_moves_aliases_and_whole_substitutions() {
    // naming.source.original-class-constructor-call
    // docs/design/analysis/name-resolution-proofs/source-original-class-constructor-call.md
    for source in [
        "oo::class create C {}; rename C Held; set object [Held \"new\"]",
        "oo::class create C {}; rename C Held; interp alias {} make {} Held new; set object [make]",
    ] {
        let mut analyser = Analyser::new();
        let result = analyser.analyse(source, "tcl8.6");
        assert_eq!(
            result.instance_classes.get("object").map(String::as_str),
            Some("::C"),
            "{source}"
        );
        let offset = u32::try_from(source.find("set object").unwrap()).unwrap();
        let construction =
            crate::registry_invocation::source_structure::source_handle_construction_at(
                source, &result, offset,
            )
            .unwrap();
        let call = crate::registry_invocation::source_structure::source_constructor_call_at(
            source,
            &result,
            construction.construction().words[0].span().start(),
        )
        .unwrap();
        assert!(call.constructor_shape(&result).is_some());
        assert_eq!(
            call.class_declaration()
                .source_class(&result)
                .unwrap()
                .metadata()
                .qualified_name,
            "::C"
        );
    }
    for source in [
        "oo::class create C {}; proc set args {}; set object [C new]",
        "oo::class create C {}; set object [C new; format changed]",
        "oo::class create C {}; set object \"prefix[C new]\"",
        "oo::class create C {}; set object [C new] surplus",
        "oo::class create C {}; proc C args {}; set object [C new]",
    ] {
        let result = Analyser::new().analyse(source, "tcl8.6");
        assert!(!result.instance_classes.contains_key("object"), "{source}");
    }
}

#[test]
fn pending_constructor_heads_preserve_real_variable_and_selector_geometry() {
    // naming.source.original-produced-command-prefix
    // docs/design/analysis/name-resolution-proofs/original-produced-command-prefix.md
    for (source, expected) in [
        (
            r"set object [$class create name]",
            Some(("class", "create", "$class")),
        ),
        (
            r"set object [${class with space} n\ew]",
            Some(("class with space", "new", "${class with space}")),
        ),
        (r"set object [${class}Suffix create name]", None),
        (r"set object [$class(index) create name]", None),
        (r"set object [$class $selector name]", None),
    ] {
        let result = Analyser::new().analyse(source, "tcl8.6");
        let construction =
            crate::registry_invocation::source_structure::source_handle_construction_at(
                source, &result, 0,
            )
            .unwrap();
        let pending = class_var_head_constructor_subst(&construction);
        assert_eq!(
            pending.as_ref().map(|(class, method, span)| (
                class.as_str(),
                method.as_str(),
                &source[span.as_range()]
            )),
            expected,
            "{source}"
        );
    }
}

#[test]
fn source_named_constructors_do_not_publish_permanent_speculative_commands() {
    // naming.source.original-class-constructor-call
    // docs/design/analysis/name-resolution-proofs/source-original-class-constructor-call.md
    let source =
        "oo::class create C {}; rename C Held; interp alias {} make {} Held create; make object";
    let result = Analyser::new().analyse(source, "tcl8.6");
    assert_eq!(
        result.instance_classes.get("object").map(String::as_str),
        Some("::C")
    );
    assert!(result.created_instance_commands.is_empty());
    for source in [
        "Unknown create object",
        "oo::class create C {}; proc C args {}; C create object",
        "oo::class create C {}; rename C {}; C create object",
        "oo::class create C {self method create args {return ordinary}}; C create object",
    ] {
        let result = Analyser::new().analyse(source, "tcl8.6");
        assert!(!result.instance_classes.contains_key("object"), "{source}");
        assert!(
            !result.created_instance_commands.contains("object"),
            "{source}"
        );
    }
}
#[test]
fn source_pending_constructor_values_do_not_select_native_reporting_class_labels() {
    // naming.source.original-class-constructor-call
    // docs/design/analysis/name-resolution-proofs/source-original-class-constructor-call.md
    let source = "oo::class create C {}; set class C; set object [$class new]";
    for dialect in [
        "tcl8.4",
        "tcl8.5",
        "tcl8.6",
        "tcl9.0",
        "tcl9.1",
        "jim",
        "f5-irules",
    ] {
        let mut analyser = Analyser::new();
        analyser.result = analyser.analyse(source, dialect);
        assert!(
            !analyser
                .resolved_analysis_input()
                .has_logical_source_name_context(),
            "{dialect}"
        );
        assert!(
            analyser.resolve_user_class_at("C", 0).is_none(),
            "{dialect}"
        );
        assert!(
            !analyser.result.instance_classes.contains_key("object"),
            "{dialect}"
        );
    }
    let mut logical = Analyser::new();
    logical.result = logical.analyse("oo::class create C {}", "tcl");
    assert!(
        logical
            .resolved_analysis_input()
            .has_logical_source_name_context()
    );
    assert_eq!(
        logical.resolve_user_class_at("C", 0).as_deref(),
        Some("::C")
    );
    logical.result.resolved_input = None;
    logical.result.lexical_declaration_advice = true;
    assert!(logical.resolve_user_class_at("C", 0).is_none());
    assert!(logical.resolve_user_class_in("C", &[]).is_none());
}
#[test]
fn logical_reported_class_lookup_respects_lexical_candidates_without_native_identity() {
    // naming.source.original-class-constructor-call
    // docs/design/analysis/name-resolution-proofs/source-original-class-constructor-call.md
    let source = "oo::class create Global {}; namespace eval ns {oo::class create Local {}; namespace eval inner {set marker 1}}";
    let mut analyser = Analyser::new();
    analyser.result = analyser.analyse(source, "tcl");
    let local = u32::try_from(source.find("oo::class create Local").unwrap()).unwrap();
    let inner = u32::try_from(source.find("set marker").unwrap()).unwrap();
    assert_eq!(
        analyser.resolve_user_class_at("Local", local).as_deref(),
        Some("::ns::Local")
    );
    assert_eq!(
        analyser.resolve_user_class_at("Global", inner).as_deref(),
        Some("::Global")
    );
    assert!(
        analyser.resolve_user_class_at("Local", inner).is_none(),
        "no ancestor or unique-tail fallback"
    );
    analyser.result.resolved_input = None;
    analyser.result.lexical_declaration_advice = true;
    assert!(analyser.resolve_user_class_at("Global", inner).is_none());
}
#[test]
fn analyser_hook_metadata_uses_retained_context_without_stashed_store_donation() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    let profile = tcl_dialect::DialectProfile::find("tcl9.0").unwrap();
    let generation = tcl_registry::model::context_for_profile(profile);
    let mut registry = tcl_registry::CommandRegistry::build_default();
    registry.insert(tcl_registry::CommandSpec {
        analyser_hook: None,
        ..generation
            .context()
            .resolve_spec(generation.commands(), "set")
            .unwrap()
            .clone()
    });
    let context = std::sync::Arc::new(generation.with_command_store(std::sync::Arc::new(registry)));
    let input = super::super::ResolvedAnalysisInput::new(
        profile,
        profile,
        context,
        LexerConfig::for_file_grammar(profile.grammar),
    );
    let mut analyser = Analyser::new().with_resolved_input(input);
    analyser.context = None;
    analyser.registry = Some(std::sync::Arc::new(
        tcl_registry::CommandRegistry::build_default(),
    ));
    assert!(
        analyser
            .resolve_analyser_hook_call("set", &["name".into(), "value".into()])
            .is_none()
    );
    let old_profile = tcl_dialect::DialectProfile::find("tcl8.4").unwrap();
    let input = super::super::ResolvedAnalysisInput::new(
        old_profile,
        old_profile,
        tcl_registry::model::context_for_profile(old_profile),
        LexerConfig::for_file_grammar(old_profile.grammar),
    );
    let mut analyser = Analyser::new().with_resolved_input(input);
    analyser.context = None;
    analyser.registry = Some(std::sync::Arc::new(
        tcl_registry::CommandRegistry::build_default(),
    ));
    assert!(!analyser.head_is_self_dispatch_keyword("my"));
}
