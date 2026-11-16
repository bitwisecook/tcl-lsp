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

#[test]
fn commands_only_capture_keeps_current_original_regions_without_token_inventory() {
    let source = "proc p {} {greet arg}; set value [greet arg]; set data {inert arg}";
    let analysis = Analyser::new().analyse(source, "tcl8.6");
    let commands = SourceSyntaxStructure::capture_commands(source, &analysis).unwrap();
    let full = SourceSyntaxStructure::capture(source, &analysis).unwrap();
    assert_eq!(commands, full.commands());
    let greet = commands
        .iter()
        .filter(|command| command.texts.first().is_some_and(|head| head == "greet"))
        .count();
    assert_eq!(greet, 2);
    assert!(
        !commands
            .iter()
            .any(|command| command.texts.first().is_some_and(|head| head == "inert"))
    );
    assert!(
        SourceSyntaxStructure::capture_commands(
            &source.replace("greet arg", "other arg"),
            &analysis,
        )
        .is_none()
    );
    let mut missing = analysis.clone();
    missing.resolved_input = None;
    assert!(SourceSyntaxStructure::capture_commands(source, &missing).is_none());
    let mut mismatched = analysis;
    let config = mismatched.body_lexer_config.as_mut().unwrap();
    config.strict_quoting = !config.strict_quoting;
    assert!(SourceSyntaxStructure::capture_commands(source, &mismatched).is_none());
}

#[test]
fn logical_procedure_structure_uses_current_declared_body_word() {
    let source = "proc helper {value} {puts \"literal \\} ${value}\"}";
    let analysis = Analyser::new().analyse(source, "tcl");
    let image = SourceImage::document(source);
    let config = analysis.body_lexer_config.unwrap();
    let original = declared_bodies(source, &analysis, &image, config).unwrap();
    assert!(
        original.iter().any(
            |body| source.get(body.content.as_range()) == Some("puts \"literal \\} ${value}\"")
        )
    );
    let bodies = logical_declared_bodies(&analysis, &image).unwrap();
    assert!(
        bodies.iter().any(
            |body| source.get(body.content.as_range()) == Some("puts \"literal \\} ${value}\"")
        )
    );
    let mut changed = analysis.clone();
    changed
        .all_procs
        .get_mut("::helper")
        .unwrap()
        .params
        .clear();
    let bodies = logical_declared_bodies(&changed, &image).unwrap();
    assert!(bodies.is_empty());
    let declaration = changed.original_procedure_declarations().next().unwrap();
    assert_eq!(declaration.metadata().params.len(), 1);
    assert!(
        crate::original_declaration::OriginalDeclarationIdentity::for_procedure(
            "",
            source,
            &changed,
            declaration
        )
        .is_some()
    );
    let original = declared_bodies(source, &changed, &image, config).unwrap();
    assert!(
        original.iter().any(
            |body| source.get(body.content.as_range()) == Some("puts \"literal \\} ${value}\"")
        )
    );
    assert!(
        declared_bodies(source, &analysis, &SourceImage::document("changed"), config)
            .is_none_or(|bodies| bodies.is_empty())
    );
    assert!(
        logical_declared_bodies(&analysis, &SourceImage::document("changed"))
            .is_none_or(|bodies| bodies.is_empty())
    );
}

#[test]
fn original_method_sites_keep_complete_words_nested_commands_and_current_input() {
    for (source, expected) in [
        ("$object method", ("object", "method", true)),
        ("\"$object\" method", ("object", "method", true)),
        ("${$literal} {with space}", ("$literal", "with space", true)),
        ("$array($index) method", ("array($index)", "method", true)),
        ("puts [$object method]", ("object", "method", true)),
        (
            "{object name} \"with space\"",
            ("object name", "with space", false),
        ),
        ("[self object] method", ("[self object]", "method", false)),
    ] {
        let analysis = tcl_compiler::analyser::Analyser::new().analyse(source, "tcl");
        let cursor = u32::try_from(source.rfind(expected.1).unwrap() + 1).unwrap();
        assert_eq!(
            original_method_site_at(source, &analysis, cursor),
            Some((expected.0.to_owned(), expected.1.to_owned(), expected.2)),
            "{source}"
        );
        let mut missing = analysis.clone();
        missing.resolved_input = None;
        assert!(original_method_site_at(source, &missing, cursor).is_none());
        assert!(original_method_site_at("changed", &analysis, cursor).is_none());
    }
    for (source, receiver, selector) in
        [("{} {}", "", ""), ("\"\" \"\"", "", ""), ("${} {}", "", "")]
    {
        let analysis = tcl_compiler::analyser::Analyser::new().analyse(source, "tcl");
        let cursor = u32::try_from(source.len() - 1).unwrap();
        assert_eq!(
            original_method_site_at(source, &analysis, cursor),
            Some((
                receiver.to_owned(),
                selector.to_owned(),
                source.starts_with('$')
            )),
            "{source}"
        );
    }
    let source = "puts seed; $object longMethod";
    for style in [
        tcl_dialect::BracedVarStyle::FirstClose,
        tcl_dialect::BracedVarStyle::Tcl9Nesting,
    ] {
        let analysis = cursor_analysis(source, style);
        let cursor = u32::try_from(source.find("longMethod").unwrap() + 1).unwrap();
        assert_eq!(
            original_method_site_at(source, &analysis, cursor),
            Some(("object".to_owned(), "longMethod".to_owned(), true))
        );
        let mut stale = analysis.clone();
        stale.body_lexer_config.as_mut().unwrap().braced_var =
            if style == tcl_dialect::BracedVarStyle::FirstClose {
                tcl_dialect::BracedVarStyle::Tcl9Nesting
            } else {
                tcl_dialect::BracedVarStyle::FirstClose
            };
        assert!(original_method_site_at(source, &stale, cursor).is_none());
    }
    let source = "puts {$object method}";
    let analysis = tcl_compiler::analyser::Analyser::new().analyse(source, "tcl");
    let cursor = u32::try_from(source.find("method").unwrap()).unwrap();
    assert_eq!(
        original_method_site_at(source, &analysis, cursor),
        Some(("puts".to_owned(), "$object method".to_owned(), false))
    );
    assert_eq!(
        crate::definition::definition(source, 0, cursor, &analysis),
        [] as [crate::definition::LspRange; 0]
    );
    for (source, needle) in [
        ("# $object method", "method"),
        ("expr {$object < $other}", "<"),
        ("$object $computed", "$computed"),
        ("{*}$object method", "method"),
    ] {
        let analysis = tcl_compiler::analyser::Analyser::new().analyse(source, "tcl");
        let cursor = u32::try_from(source.find(needle).unwrap()).unwrap();
        assert!(
            original_method_site_at(source, &analysis, cursor).is_none(),
            "{source}"
        );
    }
}

fn cursor_analysis(source: &str, style: tcl_dialect::BracedVarStyle) -> AnalysisResult {
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let context =
        tcl_registry::model::ingress::resolve_environment("tcl9.1").default_context_registry();
    let config = LexerConfig {
        braced_var: style,
        ..LexerConfig::for_profile(Some(profile))
    };
    Analyser::new()
        .with_resolved_input(ResolvedAnalysisInput::new(
            profile, profile, context, config,
        ))
        .analyse(source, "presentation-only")
}

#[test]
fn original_variable_cursor_keeps_selected_roots_and_complete_source_geometry() {
    // Implementation contract: naming.core.selected-variable-cursor-syntax
    // docs/design/analysis/name-resolution-proofs/selected-variable-cursor-syntax.md
    let source = concat!(
        "set seed café🙂\n",
        "puts ${a{b}c}\n",
        "puts ${scalar(open} ${scalar(open)tail} ${arr(k)} $arr($idx) ${cash$name} ${$n} ${café🙂}\n",
        "if {$seed} {puts $seed}\n",
        "puts \"$seed\" {literal $seed}\n",
        "# $seed\n",
    );
    for style in [
        tcl_dialect::BracedVarStyle::FirstClose,
        tcl_dialect::BracedVarStyle::Tcl9Nesting,
    ] {
        let analysis = cursor_analysis(source, style);
        assert!(analysis.allows_lexical_declaration_advice());
        for (needle, root, element) in [
            ("${scalar(open}", "scalar(open", "scalar(open"),
            (
                "${scalar(open)tail}",
                "scalar(open)tail",
                "scalar(open)tail",
            ),
            ("${arr(k)}", "arr", "arr(k)"),
            ("$arr($idx)", "arr", "arr"),
            ("$idx", "idx", "idx"),
            ("${cash$name}", "cash$name", "cash$name"),
            ("$name", "cash$name", "cash$name"),
            ("${$n}", "$n", "$n"),
            ("${café🙂}", "café🙂", "café🙂"),
            ("$seed}", "seed", "seed"),
            ("$seed\"", "seed", "seed"),
        ] {
            let cursor = u32::try_from(source.find(needle).unwrap() + 1).unwrap();
            let reference = original_variable_reference_at(source, &analysis, cursor)
                .unwrap_or_else(|| panic!("missing {needle:?} under {style:?}"));
            assert_eq!(reference.root, root, "{needle}");
            let original = &source[reference.whole_span.as_range()];
            assert_eq!(
                tcl_syntax::naming::element_var_name_for_style(original, style),
                element,
                "{needle}"
            );
            assert!(
                tcl_lexer::word_parts::whole_var_ref(
                    original.as_bytes(),
                    analysis.body_lexer_config.unwrap()
                )
                .unwrap()
                .is_some()
            );
            assert!(reference.whole_span.start() <= reference.token_span.start());
            assert!(reference.token_span.end() <= reference.whole_span.end());
        }
        let nested = u32::try_from(source.find("${a{b}c}").unwrap() + 2).unwrap();
        let reference = original_variable_reference_at(source, &analysis, nested).unwrap();
        assert_eq!(reference.root, if style.nests() { "a{b}c" } else { "a{b" });
        let tail = u32::try_from(source.find("}c}").unwrap() + 1).unwrap();
        assert_eq!(
            original_variable_reference_at(source, &analysis, tail).is_some(),
            style.nests()
        );
        for needle in ["literal $seed", "# $seed"] {
            let cursor =
                u32::try_from(source.find(needle).unwrap() + needle.find('$').unwrap() + 1)
                    .unwrap();
            assert!(original_variable_reference_at(source, &analysis, cursor).is_none());
        }
        assert!(
            original_variable_reference_at(&(source.to_owned() + " "), &analysis, nested).is_none()
        );
        let mut unavailable = analysis.clone();
        unavailable.resolved_input = None;
        assert!(original_variable_reference_at(source, &unavailable, nested).is_none());
        let mut foreign = analysis.clone();
        foreign.body_lexer_config.as_mut().unwrap().braced_var = if style.nests() {
            tcl_dialect::BracedVarStyle::FirstClose
        } else {
            tcl_dialect::BracedVarStyle::Tcl9Nesting
        };
        assert!(original_variable_reference_at(source, &foreign, nested).is_none());
    }
}

#[test]
fn original_cursor_declines_malformed_data_and_native_resolution_promotion() {
    // Implementation contract: naming.core.selected-variable-cursor-syntax
    // docs/design/analysis/name-resolution-proofs/selected-variable-cursor-syntax.md
    let source = "set seed 1\nputs ${missing\n";
    let analysis = cursor_analysis(source, tcl_dialect::BracedVarStyle::Tcl9Nesting);
    let cursor = u32::try_from(source.find("${missing").unwrap() + 2).unwrap();
    assert!(original_variable_reference_at(source, &analysis, cursor).is_none());
    let source = "proc if args {}\nif 1 {puts $seed}\n";
    let analysis = cursor_analysis(source, tcl_dialect::BracedVarStyle::Tcl9Nesting);
    let cursor = u32::try_from(source.find("$seed").unwrap() + 1).unwrap();
    assert!(original_variable_reference_at(source, &analysis, cursor).is_none());
    let source = "set seed 1\nputs $seed\n";
    let native = Analyser::new().analyse(source, "tcl8.6");
    let cursor = u32::try_from(source.find("$seed").unwrap() + 1).unwrap();
    assert!(original_variable_reference_at(source, &native, cursor).is_some());
    assert!(!native.allows_lexical_declaration_advice());
    assert!(
        crate::definition::substituting_var_at_position(source, &native, 1, 6, cursor).is_none()
    );
}

#[test]
fn original_variable_cursor_retains_custom_reference_only_and_prefix_source_roles() {
    // Implementation contract: naming.core.selected-variable-cursor-syntax
    // docs/design/analysis/name-resolution-proofs/selected-variable-cursor-syntax.md
    fn reference_only(
        _: tcl_registry::InvocationArguments<'_>,
    ) -> Vec<(u8, tcl_registry::ScriptTiming)> {
        vec![(0, tcl_registry::ScriptTiming::ReferenceOnly)]
    }
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let mut registry = tcl_registry::CommandRegistry::build_default();
    registry.insert(tcl_registry::CommandSpec {
        name: "reference",
        arity: tcl_registry::Arity::exact(1),
        arg_roles: &[(0, tcl_registry::ArgRole::Body)],
        script_timing_resolver: Some(reference_only),
        ..tcl_registry::CommandSpec::DEFAULT
    });
    for (source, expected) in [
        ("reference {puts $value}", true),
        ("unknown {puts $value}", false),
        ("proc reference args {}\nreference {puts $value}", false),
        ("interp alias {} branch {} if 1\nbranch {puts $value}", true),
    ] {
        let analysis = analyse_document(
            source,
            profile,
            &registry,
            LexerConfig::for_profile(Some(profile)),
        );
        let cursor = u32::try_from(source.find("$value").unwrap() + 1).unwrap();
        let reference = original_variable_reference_at(source, &analysis, cursor);
        assert_eq!(
            reference.as_ref().map(|value| value.root),
            expected.then_some("value")
        );
        assert!(
            original_variable_reference_at(&(source.to_owned() + " "), &analysis, cursor).is_none()
        );
        let mut detached = analysis.clone();
        let input = detached.resolved_input.as_ref().unwrap();
        detached.resolved_input = Some(ResolvedAnalysisInput::new(
            profile,
            profile,
            Arc::new(
                input
                    .context_registry()
                    .with_command_store(registry.snapshot().shared_registry()),
            ),
            input.lexer_config(),
        ));
        assert!(original_variable_reference_at(source, &detached, cursor).is_none());
    }
}

#[test]
fn original_syntax_tokens_join_absolute_region_and_command_geometry() {
    // naming.cli.original-source-highlight-geometry
    // docs/design/analysis/name-resolution-proofs/cli-original-source-highlight-geometry.md
    let source = "set data {café🙂;\nnotACommand}\nif {1} {  puts [list café🙂]; puts $value  }";
    for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        let analysis = Analyser::new().analyse(source, dialect);
        let structure = SourceSyntaxStructure::capture(source, &analysis).unwrap();
        let body = structure
            .lexical_regions()
            .iter()
            .find(|region| {
                &source[region.span().as_range()] == "  puts [list café🙂]; puts $value  "
            })
            .unwrap_or_else(|| panic!("missing genuine syntax body for {dialect}"));
        let start = u32::try_from(source.find("puts [list").unwrap()).unwrap();
        assert!(
            body.tokens()
                .iter()
                .any(|token| token.kind == tcl_lexer::TokenType::Esc
                    && token.span == Span::new(start, start + 4))
        );
        assert!(body.tokens().iter().any(|token| {
            token.kind == tcl_lexer::TokenType::Var && &source[token.span.as_range()] == "$value"
        }));
        let nested = structure
            .lexical_regions()
            .iter()
            .find(|region| &source[region.span().as_range()] == "list café🙂")
            .unwrap();
        assert_eq!(&source[nested.tokens()[0].span.as_range()], "list");
        assert!(structure.commands().iter().any(|command| {
            command
                .argv
                .first()
                .is_some_and(|head| head.span == nested.tokens()[0].span)
        }));
        assert!(
            !structure
                .lexical_regions()
                .iter()
                .any(|region| &source[region.span().as_range()] == "café🙂;\nnotACommand")
        );
        for region in structure.lexical_regions() {
            assert!(
                region
                    .tokens()
                    .iter()
                    .all(|token| region.span().start() <= token.span.start()
                        && token.span.end() <= region.span().end()
                        && source.get(token.span.as_range()).is_some())
            );
        }
        assert!(SourceSyntaxStructure::capture(&(source.to_owned() + " "), &analysis).is_none());
    }
}

#[test]
fn original_structure_retains_alias_body_and_bracket_geometry_after_ui_maps_clear() {
    // Implementation contract: naming.source.original-editor-body-structure
    // docs/design/analysis/name-resolution-proofs/original-editor-body-structure.md
    let source = concat!(
        "proc p {} {puts [list inner]}\n",
        "interp alias {} branch {} if 1\n",
        "branch {\n    puts [list chosen]\n    puts again\n}\n",
    );
    let mut analysis = Analyser::new().analyse(source, "tcl8.6");
    let config = analysis.body_lexer_config.unwrap();
    analysis.all_procs.clear();
    analysis.global_scope.procs.clear();
    analysis.global_scope.children.clear();
    let structure = SourceStructure::capture(source, Some(&analysis), config).unwrap();
    for name in ["list inner", "list chosen"] {
        let cursor = u32::try_from(source.find(name).unwrap()).unwrap();
        let span = structure.command_at(cursor).unwrap();
        assert_eq!(&source[span.as_range()], name);
    }
    let start = u32::try_from(source.find("{\n    puts").unwrap()).unwrap();
    assert!(structure.bodies.iter().any(|body| body.start() == start));
    assert!(
        SourceStructure::capture(&source.replace("again", "other"), Some(&analysis), config,)
            .is_none()
    );
    let mut foreign = config;
    foreign.strict_quoting = !foreign.strict_quoting;
    assert!(SourceStructure::capture(source, Some(&analysis), foreign).is_none());
}

fn assert_definition_cursor_region(
    captured: &SourceStructure,
    body: &tcl_compiler::registry_invocation::OriginalSourceScriptBody,
    expected_parent: &tcl_registry::definer::DefinitionBodyGrammar,
    context: &tcl_registry::model::ContextRegistry,
) {
    let region = captured
        .definition_cursor_regions
        .iter()
        .find(|region| region.span == body.content_span())
        .expect("the cursor walk must retain each source-issued body");
    let parent = region
        .parent
        .as_ref()
        .expect("the retained body region must keep its genuine source owner");
    assert!(region.members.is_some());
    assert!(std::ptr::eq(
        parent
            .source_words()
            .with_source_schema(context, |schema| {
                schema.authored_source_definition_body_grammar()
            })
            .flatten()
            .unwrap(),
        expected_parent
    ));
}

fn assert_original_definition_cursor_parent_chain(source: &str) {
    let analysis = Analyser::new().analyse(source, "spectcl");
    let config = analysis.body_lexer_config.unwrap();
    let input = analysis.resolved_input.as_ref().unwrap();
    let context = input.context_registry();
    let registry = context.commands();
    let document = registry.document_grammar().unwrap();
    let expected_pack = registry
        .authored_document_member_grammar(document, "speclib")
        .unwrap();
    let expected_command = registry
        .authored_document_member_grammar(expected_pack, "command")
        .unwrap();

    let root_command = segment_commands_with_offset_and_config(source, 0, config)
        .into_iter()
        .find(|command| command.name() == "speclib")
        .unwrap();
    let root_words =
        crate::original_invocation::source_registry_words(source, &analysis, &root_command)
            .unwrap();
    let mut root_bodies = root_words.source_script_bodies(&context);
    assert_eq!(root_bodies.len(), 1);
    let pack_body = root_bodies.remove(0);
    let pack_parent = pack_body
        .definition_parent_for(&context, None)
        .expect("the authored speclib body must issue its own source owner");
    assert!(pack_parent.matches_source(&SourceImage::document(source), config));
    assert!(std::ptr::eq(
        pack_parent
            .source_words()
            .with_source_schema(&context, |schema| {
                schema.authored_source_definition_body_grammar()
            })
            .flatten()
            .unwrap(),
        expected_pack
    ));

    let pack_source = source.get(pack_parent.content_span().as_range()).unwrap();
    let command = segment_commands_with_offset_and_config(
        pack_source,
        pack_parent.content_span().start(),
        config,
    )
    .into_iter()
    .find(|command| command.name() == "command")
    .unwrap();
    let command_words = crate::original_invocation::source_registry_words_in_definition_body(
        source,
        &analysis,
        &command,
        &pack_parent,
    )
    .unwrap();
    let mut command_bodies = command_words.source_script_bodies(&context);
    assert_eq!(command_bodies.len(), 1);
    let command_body = command_bodies.remove(0);
    let command_parent = command_body
        .definition_parent_for(&context, Some(&pack_parent))
        .expect("the authored command body must issue its actual child owner");
    assert!(command_parent.matches_source(&SourceImage::document(source), config));
    assert!(std::ptr::eq(
        command_parent
            .source_words()
            .with_source_schema(&context, |schema| {
                schema.authored_source_definition_body_grammar()
            })
            .flatten()
            .unwrap(),
        expected_command
    ));

    let captured = SourceStructure::capture_with_purpose(
        source,
        Some(&analysis),
        config,
        CapturePurpose::DefinitionCursor,
    )
    .unwrap();
    for (body, expected_parent) in [
        (&pack_parent, expected_pack),
        (&command_parent, expected_command),
    ] {
        assert_definition_cursor_region(&captured, body, expected_parent, &context);
    }
}

#[test]
fn original_definition_cursor_source_owners_reach_nested_spectcl_command_bodies() {
    // naming.core.original-definition-grammar-cursor
    // docs/design/analysis/name-resolution-proofs/core-original-definition-grammar-cursor.md
    // This control stops at the genuine selected source-owner boundary;
    // it does not depend on quick-fix diagnostics or candidate spelling.
    for source in [
        "speclib demo 1 {command demo {arty 1; arty 2; arg 0 -rle Body}}",
        "speclib demo 1 {\n command demo::x {\n  arty 1\n }\n}",
    ] {
        assert_original_definition_cursor_parent_chain(source);
    }
}

fn assert_original_definition_child_foreign_owners(
    source: &str,
    analysis: &AnalysisResult,
    command: &SegmentedCommand,
    parent: &tcl_compiler::registry_invocation::OriginalSourceScriptBody,
    config: tcl_lexer::LexerConfig,
) {
    let foreign = Analyser::new().analyse(source, "tcl8.6");
    let foreign_input = ResolvedAnalysisInput::new(
        analysis.resolved_profile().unwrap(),
        analysis.resolved_profile().unwrap(),
        foreign.resolved_input.as_ref().unwrap().context_registry(),
        config,
    );
    let foreign_context = Analyser::new()
        .with_resolved_input(foreign_input)
        .analyse(source, "spectcl");
    assert!(
        crate::original_invocation::source_registry_words_in_definition_body(
            source,
            &foreign_context,
            command,
            parent,
        )
        .is_none()
    );
    let mut foreign_config = (*analysis).clone();
    let mut different_config = config;
    different_config.strict_quoting = !different_config.strict_quoting;
    foreign_config.body_lexer_config = Some(different_config);
    assert!(
        crate::original_invocation::source_registry_words_in_definition_body(
            source,
            &foreign_config,
            command,
            parent,
        )
        .is_none()
    );
    assert!(
        crate::original_invocation::source_registry_words_in_definition_body(
            source, &foreign, command, parent,
        )
        .is_none()
    );
    assert!(crate::original_invocation::source_registry_words(source, analysis, command).is_none());
}

fn original_definition_document_parents(
    source: &str,
    analysis: &AnalysisResult,
    roots: &[SegmentedCommand],
    context: &tcl_registry::model::ContextRegistry,
) -> Vec<tcl_compiler::registry_invocation::OriginalSourceScriptBody> {
    roots
        .iter()
        .map(|command| {
            crate::original_invocation::source_registry_words(source, analysis, command)
                .unwrap()
                .source_script_bodies(context)
                .remove(0)
                .definition_parent_for(context, None)
                .unwrap()
        })
        .collect::<Vec<_>>()
}

#[test]
fn original_document_child_schema_requires_actual_parent_complete_argv_and_context() {
    let source = "speclib demo 1 {command demo {arty 1}}\nspeclib other 1 {command other {arty 2}}";
    let analysis = Analyser::new().analyse(source, "spectcl");
    let config = analysis.body_lexer_config.unwrap();
    let context = analysis.resolved_input.as_ref().unwrap().context_registry();
    let roots = segment_commands_with_offset_and_config(source, 0, config);
    let parents = original_definition_document_parents(source, &analysis, &roots, &context);
    assert_eq!(parents.len(), 2);
    let pack_source = &source[parents[0].content_span().as_range()];
    let command = segment_commands_with_offset_and_config(
        pack_source,
        parents[0].content_span().start(),
        config,
    )
    .remove(0);
    let child = crate::original_invocation::source_registry_words_in_definition_body(
        source,
        &analysis,
        &command,
        &parents[0],
    )
    .unwrap();
    assert!(matches!(
        &child.source,
        crate::original_invocation::OriginalRegistrySource::DefinitionMember(_)
    ));
    assert_eq!(child.source_script_bodies(&context).len(), 1);
    assert_eq!(
        child.source_script_bodies_for(
            &context,
            tcl_compiler::registry_invocation::OriginalSourceScriptPurpose::PotentialEvaluation
        ),
        [] as [tcl_compiler::registry_invocation::OriginalSourceScriptBody; 0]
    );
    assert!(
        crate::original_invocation::source_registry_words_in_definition_body(
            source,
            &analysis,
            &command,
            &parents[1],
        )
        .is_none()
    );
    let mut truncated = command.clone();
    truncated.argv.pop();
    truncated.texts.pop();
    truncated.word_fragments.pop();
    truncated.single_token_word.pop();
    assert!(
        crate::original_invocation::source_registry_words_in_definition_body(
            source,
            &analysis,
            &truncated,
            &parents[0],
        )
        .is_none()
    );
    let mut partial = command.clone();
    partial.is_partial = true;
    assert!(
        crate::original_invocation::source_registry_words_in_definition_body(
            source,
            &analysis,
            &partial,
            &parents[0],
        )
        .is_none()
    );
    let stale = source.replace("arty 1", "arty 9");
    assert!(
        crate::original_invocation::source_registry_words_in_definition_body(
            &stale,
            &analysis,
            &command,
            &parents[0],
        )
        .is_none()
    );
    let mut missing = analysis.clone();
    missing.resolved_input = None;
    assert!(
        crate::original_invocation::source_registry_words_in_definition_body(
            source,
            &missing,
            &command,
            &parents[0],
        )
        .is_none()
    );
    assert_original_definition_child_foreign_owners(
        source,
        &analysis,
        &command,
        &parents[0],
        config,
    );
}

#[test]
fn source_member_disposition_prevents_unavailable_layout_global_fallback() {
    let source = "oo::class create C {\nmethod good {} {puts ordinary}\nif $condition {puts global}\nmethod missing {}\nmethod cooked {} \"puts\\x20cooked\"\nsuperclass Base\n}\n";
    let analysis = Analyser::new().analyse(source, "tcl");
    let syntax = SourceSyntaxStructure::capture(source, &analysis).unwrap();
    let head = |needle| u32::try_from(source.find(needle).unwrap()).unwrap();
    let member = syntax
        .definition_member_at(head("method good"))
        .unwrap()
        .unwrap();
    assert_eq!(member.script_bodies().len(), 1);
    assert!(
        syntax
            .definition_member_at(head("if $condition"))
            .unwrap()
            .is_none()
    );
    assert!(
        syntax
            .definition_member_at(head("puts ordinary"))
            .unwrap()
            .is_none()
    );
    for needle in ["method missing", "method cooked"] {
        assert!(syntax.definition_member_at(head(needle)).is_err());
    }
    assert_eq!(
        syntax
            .definition_member_at(head("superclass Base"))
            .unwrap()
            .unwrap()
            .script_bodies(),
        []
    );
}

#[test]
fn original_structure_uses_genuine_definition_wrappers_and_clears_ordinary_bodies() {
    // Implementation contract: naming.source.original-editor-body-structure
    // docs/design/analysis/name-resolution-proofs/original-editor-body-structure.md
    let source = "oo::class create C {self self {method café {} {puts wrapper}}; if 1 {method café {} {puts immediate}}; method m {} {method ordinary {} {puts data}}}";
    let analysis = Analyser::new().analyse(source, "tcl8.6");
    let config = analysis.body_lexer_config.unwrap();
    let structure = SourceStructure::capture(source, Some(&analysis), config).unwrap();
    let wrapper = u32::try_from(source.find("{method café").unwrap()).unwrap();
    assert!(structure.bodies.iter().any(|body| body.start() == wrapper));
    for body in ["puts wrapper", "puts immediate"] {
        let offset = u32::try_from(source.find(body).unwrap()).unwrap();
        assert_eq!(
            source.get(structure.command_at(offset).unwrap().as_range()),
            Some(body)
        );
    }
    let offset = u32::try_from(source.find("puts data").unwrap()).unwrap();
    assert_ne!(
        source.get(structure.command_at(offset).unwrap().as_range()),
        Some("puts data")
    );
    let compact =
        SourceStructure::capture_for_syntax_compaction(source, &analysis, config).unwrap();
    assert!(
        !compact.bodies.iter().any(|body| body.start() == wrapper),
        "a readonly wrapper source role cannot grant syntax-compaction authority"
    );
    assert!(
        SourceStructure::capture(
            &source.replace("wrapper", "changed"),
            Some(&analysis),
            config
        )
        .is_none()
    );
    let mut foreign = config;
    foreign.strict_quoting ^= true;
    assert!(SourceStructure::capture(source, Some(&analysis), foreign).is_none());
}

#[test]
fn original_case_and_lambda_geometry_skip_data_and_keep_dynamic_subject() {
    // Implementation contract: naming.source.original-editor-body-structure
    // docs/design/analysis/name-resolution-proofs/original-editor-body-structure.md
    let source = concat!(
        "switch -- $choice {\n x {\n  puts [list arm]\n }\n}\n",
        "apply {{} {\n puts [list lambda]\n}}\n",
        "list {puts [list data]}\n",
    );
    let analysis = Analyser::new().analyse(source, "tcl8.6");
    let structure =
        SourceStructure::capture(source, Some(&analysis), analysis.body_lexer_config.unwrap())
            .unwrap();
    for name in ["list arm", "list lambda"] {
        let cursor = u32::try_from(source.find(name).unwrap()).unwrap();
        assert_eq!(
            &source[structure.command_at(cursor).unwrap().as_range()],
            name,
        );
    }
    let cursor = u32::try_from(source.find("list data").unwrap()).unwrap();
    assert_ne!(
        &source[structure.command_at(cursor).unwrap().as_range()],
        "list data",
    );
}
