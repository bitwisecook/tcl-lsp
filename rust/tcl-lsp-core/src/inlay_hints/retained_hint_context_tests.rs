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

use super::tests::whole_document_range;
use super::*;
use crate::original_invocation::selected_registry_words as selected_original_words;
use tcl_compiler::analyser::Analyser;

fn hints(
    source: &str,
    analysis: &AnalysisResult,
    profile: &'static tcl_dialect::DialectProfile,
    registry: &CommandRegistry,
) -> Vec<InlayHint> {
    inlay_hints_in_program(
        source,
        profile,
        LspRange {
            start_line: 0,
            start_character: 0,
            end_line: 100,
            end_character: 0,
        },
        Some(analysis),
        crate::definition::CallResolution {
            registry: Some(registry),
            program: None,
        },
        true,
        true,
    )
}

fn analysis_with_availability(source: &str, environment: &str) -> AnalysisResult {
    let profile = crate::profile_for_dialect("tcl9.1");
    let driver =
        tcl_registry::model::ingress::resolve_environment("tcl9.1").default_context_registry();
    let actual =
        tcl_registry::model::ingress::resolve_environment(environment).default_context_registry();
    let context = std::sync::Arc::new(
        actual.with_command_store(driver.commands().snapshot().shared_registry()),
    );
    let input = tcl_compiler::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        context,
        tcl_lexer::LexerConfig::for_profile(Some(profile)),
    );
    Analyser::new()
        .with_resolved_input(input)
        .analyse(source, profile.name)
}

fn logical_format_input(environment: &str) -> tcl_compiler::analyser::ResolvedAnalysisInput {
    let mut profile = crate::profile_for_dialect("f5-irules").clone();
    profile.name = "explicit-logical-format-hints";
    let profile = profile.intern();
    let driver =
        tcl_registry::model::ingress::resolve_environment("tcl9.1").default_context_registry();
    let actual =
        tcl_registry::model::ingress::resolve_environment(environment).default_context_registry();
    let context = std::sync::Arc::new(
        actual.with_command_store(driver.commands().snapshot().shared_registry()),
    );
    let config = tcl_lexer::LexerConfig {
        escapes: crate::profile_for_dialect("tcl8.6").grammar.escapes,
        ..tcl_lexer::LexerConfig::for_profile(Some(profile))
    };
    tcl_compiler::analyser::ResolvedAnalysisInput::new(profile, profile, context, config)
}

#[test]
fn logical_format_hints_keep_actual_context_and_full_source_currency() {
    // naming.core.original-inlay-retained-context
    // docs/design/analysis/name-resolution-proofs/original-inlay-retained-context.md
    let source = "clock scan 2020 -format {%Y}";
    for (environment, expected) in [("tcl9.1", true), ("tcl8.4", false)] {
        let input = logical_format_input(environment);
        let context = input.context_registry();
        let mut analysis = Analyser::new()
            .with_resolved_input(input.clone())
            .analyse(source, "presentation-only");
        assert!(analysis.allows_lexical_declaration_advice());
        analysis.command_invocations.clear();
        let out = hints(source, &analysis, input.unit_profile(), context.commands());
        assert_eq!(
            out.iter().any(|hint| hint.label == "year"),
            expected,
            "{environment}: {out:?}"
        );
        assert_eq!(
            hints(
                &source.replace("%Y", "%m"),
                &analysis,
                input.unit_profile(),
                context.commands()
            ),
            [] as [crate::inlay_hints::InlayHint; 0]
        );
        analysis.resolved_input = Some(tcl_compiler::analyser::ResolvedAnalysisInput::new(
            input.analyser_profile(),
            input.unit_profile(),
            std::sync::Arc::new(
                context.with_command_store(context.commands().snapshot().shared_registry()),
            ),
            input.lexer_config(),
        ));
        assert_eq!(
            hints(source, &analysis, input.unit_profile(), context.commands()),
            [] as [crate::inlay_hints::InlayHint; 0]
        );
    }
}

#[test]
fn logical_format_hints_keep_nested_escaped_words_and_refuse_captured_anchors() {
    // naming.core.original-inlay-retained-context
    // docs/design/analysis/name-resolution-proofs/original-inlay-retained-context.md
    let input = logical_format_input("tcl9.1");
    let context = input.context_registry();
    for source in [
        "format {%d} 7",
        r"format \x25d 7",
        r#"format "\x25d" 7"#,
        "proc p {} {format {%d} 7}",
        "interp alias {} fmt {} format; fmt {%d} 7",
    ] {
        let analysis = Analyser::new()
            .with_resolved_input(input.clone())
            .analyse(source, "presentation-only");
        let out = hints(source, &analysis, input.unit_profile(), context.commands());
        let formats = out
            .iter()
            .filter(|hint| hint.label == "int")
            .collect::<Vec<_>>();
        assert_eq!(formats.len(), 1, "{source}: {out:?}");
        let end = source.find('d').unwrap() + 1;
        let expected =
            LineIndex::new(source).position_at_utf16(u32::try_from(end).unwrap(), source);
        assert_eq!(
            (formats[0].position_line, formats[0].position_character),
            (expected.line, expected.character.get()),
            "{source}"
        );
    }
    for source in [
        "interp alias {} fmt {} format {%d}; fmt 7",
        "format $template 7",
        "proc format args {}; format {%d} 7",
        "format {a\\\n%d} 7",
    ] {
        let analysis = Analyser::new()
            .with_resolved_input(input.clone())
            .analyse(source, "presentation-only");
        assert!(
            hints(source, &analysis, input.unit_profile(), context.commands())
                .iter()
                .all(|hint| hint.label != "int"),
            "{source}"
        );
    }
}

#[test]
fn logical_format_escape_hints_use_the_complete_selected_grammar() {
    // naming.core.original-inlay-retained-context
    // docs/design/analysis/name-resolution-proofs/original-inlay-retained-context.md
    let source = r"format \x25d 7";
    let modern = logical_format_input("tcl9.1");
    let config = tcl_lexer::LexerConfig {
        escapes: crate::profile_for_dialect("tcl8.5").grammar.escapes,
        ..modern.lexer_config()
    };
    let legacy = tcl_compiler::analyser::ResolvedAnalysisInput::new(
        modern.analyser_profile(),
        modern.unit_profile(),
        modern.context_registry(),
        config,
    );
    for (input, expected) in [(modern, true), (legacy, false)] {
        let analysis = Analyser::new()
            .with_resolved_input(input.clone())
            .analyse(source, "presentation-only");
        assert_eq!(
            hints(
                source,
                &analysis,
                input.unit_profile(),
                input.context_registry().commands()
            )
            .iter()
            .any(|hint| hint.label == "int"),
            expected
        );
    }
}

#[test]
fn logical_procedure_parameter_hints_use_original_allocation_and_effective_topology() {
    for (source, expected) in [
        (
            "proc target {first second} {}; target ONE TWO",
            vec!["first:", "second:"],
        ),
        (
            "proc target {captured written} {}; interp alias {} alias {} target FIXED; alias VALUE",
            vec!["written:"],
        ),
        (
            "proc target {first {second DEFAULT} args} {}; target ONE",
            vec!["first:"],
        ),
        (
            "proc target {first {second DEFAULT} args} {}; target ONE TWO THREE",
            vec!["first:", "second:"],
        ),
    ] {
        let mut analysis = Analyser::new().analyse(source, "tcl");
        analysis.all_procs.clear();
        analysis.global_scope.procs.clear();
        analysis.command_invocations.clear();
        let start = u32::try_from(source.rfind(';').unwrap() + 1).unwrap();
        let out = hints(
            source,
            &analysis,
            analysis.resolved_profile().unwrap(),
            analysis.resolved_registry().unwrap(),
        );
        let labels = out
            .iter()
            .filter(|hint| {
                hint.kind == InlayHintKind::Parameter && hint.position_character >= start
            })
            .map(|hint| hint.label.as_str())
            .collect::<Vec<_>>();
        assert_eq!(labels, expected, "{source}: {out:?}");
        if source.contains("alias VALUE") {
            let actual = out.iter().find(|hint| hint.label == "written:").unwrap();
            let expected = LineIndex::new(source).position_at_utf16(
                u32::try_from(source.rfind("VALUE").unwrap()).unwrap(),
                source,
            );
            assert_eq!(
                (actual.position_line, actual.position_character),
                (expected.line, expected.character.get())
            );
            assert!(!out.iter().any(|hint| hint.label == "captured:"));
        }
    }
}

#[test]
fn logical_procedure_parameter_hints_keep_unknown_formals_and_source_currency_terminal() {
    for source in [
        "proc target {value} {}; unknown_mutation; target VALUE",
        "proc target {{optional DEFAULT} required} {}; target ONLY",
        "proc target {&value} {}; target VALUE",
        "proc target {args tail} {}; target FIRST SECOND",
        "proc target {value} {}; rename target {}; target VALUE",
    ] {
        let analysis = Analyser::new().analyse(source, "tcl");
        let start = u32::try_from(source.rfind(';').unwrap() + 1).unwrap();
        let out = hints(
            source,
            &analysis,
            analysis.resolved_profile().unwrap(),
            analysis.resolved_registry().unwrap(),
        );
        assert!(
            out.iter().all(
                |hint| hint.kind != InlayHintKind::Parameter || hint.position_character < start
            ),
            "{source}: {out:?}"
        );
    }
    let source = "proc target {value} {}; target VALUE";
    let original = Analyser::new().analyse(source, "tcl");
    let profile = original.resolved_profile().unwrap();
    let registry = original.resolved_registry().unwrap();
    assert!(
        hints(source, &original, profile, registry)
            .iter()
            .any(|hint| hint.label == "value:")
    );
    assert_eq!(
        hints(
            &source.replace("VALUE", "OTHER"),
            &original,
            profile,
            registry
        ),
        [] as [crate::inlay_hints::InlayHint; 0]
    );
    let mut changed = original.clone();
    changed.body_lexer_config.as_mut().unwrap().strict_quoting ^= true;
    assert_eq!(
        hints(source, &changed, profile, registry),
        [] as [crate::inlay_hints::InlayHint; 0]
    );
    changed = original.clone();
    changed.resolved_input = None;
    assert_eq!(
        hints(source, &changed, profile, registry),
        [] as [crate::inlay_hints::InlayHint; 0]
    );
}

#[test]
fn logical_parameter_hints_keep_actual_context_and_original_source_currency() {
    // naming.core.original-inlay-retained-context
    // docs/design/analysis/name-resolution-proofs/original-inlay-retained-context.md
    let source = "string cat A B";
    for (environment, expected) in [("tcl9.1", true), ("tcl8.4", false)] {
        let input = logical_format_input(environment);
        let context = input.context_registry();
        let mut analysis = Analyser::new()
            .with_resolved_input(input.clone())
            .analyse(source, "presentation-only");
        analysis.command_invocations.clear();
        let out = hints(source, &analysis, input.unit_profile(), context.commands());
        assert_eq!(
            out.iter().any(|hint| hint.kind == InlayHintKind::Parameter),
            expected,
            "{environment}: {out:?}"
        );
        assert_eq!(
            hints(
                "string cat C D",
                &analysis,
                input.unit_profile(),
                context.commands()
            ),
            [] as [crate::inlay_hints::InlayHint; 0]
        );
        analysis.resolved_input = Some(tcl_compiler::analyser::ResolvedAnalysisInput::new(
            input.analyser_profile(),
            input.unit_profile(),
            std::sync::Arc::new(
                context.with_command_store(context.commands().snapshot().shared_registry()),
            ),
            input.lexer_config(),
        ));
        assert_eq!(
            hints(source, &analysis, input.unit_profile(), context.commands()),
            [] as [crate::inlay_hints::InlayHint; 0]
        );
    }
}

#[test]
fn logical_parameter_hints_keep_nested_and_prefix_ordinals_without_captured_anchors() {
    // naming.core.original-inlay-retained-context
    // docs/design/analysis/name-resolution-proofs/original-inlay-retained-context.md
    let input = logical_format_input("tcl9.1");
    let context = input.context_registry();
    for (source, needle) in [
        ("string index held 1", "1"),
        ("proc p {} {string index held 1}", "1"),
        ("interp alias {} idx {} string index held; idx 1", "1"),
        (r"string \u0069ndex held 1", "1"),
    ] {
        let analysis = Analyser::new()
            .with_resolved_input(input.clone())
            .analyse(source, "presentation-only");
        let out = hints(source, &analysis, input.unit_profile(), context.commands());
        let actual = out
            .iter()
            .filter(|hint| hint.label == "charIndex:")
            .collect::<Vec<_>>();
        assert_eq!(actual.len(), 1, "{source}: {out:?}");
        let position = LineIndex::new(source).position_at_utf16(
            u32::try_from(source.rfind(needle).unwrap()).unwrap(),
            source,
        );
        assert_eq!(
            (actual[0].position_line, actual[0].position_character),
            (position.line, position.character.get())
        );
    }
    for source in [
        "interp alias {} len {} string length held; len",
        "proc string args {}; string index held 1",
        "string $selector held 1",
    ] {
        let analysis = Analyser::new()
            .with_resolved_input(input.clone())
            .analyse(source, "presentation-only");
        let offset = u32::try_from(source.rfind(';').map_or(0, |offset| offset + 1)).unwrap();
        let position = LineIndex::new(source).position_at_utf16(offset, source);
        assert!(
            hints(source, &analysis, input.unit_profile(), context.commands())
                .iter()
                .all(|hint| hint.kind != InlayHintKind::Parameter
                    || hint.position_line < position.line
                    || hint.position_line == position.line
                        && hint.position_character < position.character.get()),
            "{source}"
        );
    }
}

#[test]
fn original_builtin_hints_use_the_selected_availability_not_registry_profile() {
    // naming.core.original-inlay-retained-context
    // docs/design/analysis/name-resolution-proofs/original-inlay-retained-context.md
    let source = "string cat A B";
    let current = analysis_with_availability(source, "tcl9.1");
    let profile = current.resolved_profile().unwrap();
    let registry = current.resolved_registry().unwrap();
    let positive = hints(source, &current, profile, registry);
    let positions = positive
        .iter()
        .filter(|hint| hint.kind == InlayHintKind::Parameter)
        .map(|hint| hint.position_character)
        .collect::<Vec<_>>();
    // The synopsis names its first positional string; its variadic tail
    // contributes no additional parameter label.
    assert_eq!(positions, [11], "{positive:?}");
    assert!(positive.iter().any(|hint| hint.label == "string1:"));

    let older = analysis_with_availability(source, "tcl8.4");
    assert_eq!(
        older.resolved_registry().unwrap().profile().unwrap().name,
        "tcl9.1"
    );
    let segment = tcl_source::segment::segment_commands_with_offset_and_config(
        source,
        0,
        older.body_lexer_config.unwrap(),
    )
    .remove(0);
    let selected =
        selected_original_words(source, &older, &segment, older.resolved_registry().unwrap())
            .expect("same genuine Native source words and lookup still exist");
    let context = older.resolved_input.as_ref().unwrap().context_registry();
    assert_eq!(
        selected.with_source_schema(&context, |schema| schema.subcommand.kind()),
        Some(Some(
            tcl_registry::resolved_invocation::SubcommandResolutionKind::Unknown
        ))
    );
    assert!(
        hints(source, &older, profile, registry)
            .iter()
            .all(|hint| hint.kind != InlayHintKind::Parameter)
    );
}

#[test]
fn original_format_hints_use_selected_option_roles_under_the_actual_context() {
    // naming.core.original-inlay-retained-context
    // docs/design/analysis/name-resolution-proofs/original-inlay-retained-context.md
    let source = "clock scan 2020 -format {%Y}";
    let current = analysis_with_availability(source, "tcl9.1");
    let profile = current.resolved_profile().unwrap();
    let registry = current.resolved_registry().unwrap();
    let positive = hints(source, &current, profile, registry);
    assert_eq!(
        positive.iter().filter(|hint| hint.label == "year").count(),
        1,
        "{positive:?}"
    );
    let older = analysis_with_availability(source, "tcl8.4");
    let out = hints(source, &older, profile, registry);
    assert!(out.iter().all(|hint| hint.label != "year"), "{out:?}");
    let segment = tcl_source::segment::segment_commands_with_offset_and_config(
        source,
        0,
        older.body_lexer_config.unwrap(),
    )
    .remove(0);
    let selected =
        selected_original_words(source, &older, &segment, older.resolved_registry().unwrap())
            .expect("original words exist independently of unavailable format option");
    let context = older.resolved_input.as_ref().unwrap().context_registry();
    let formats = selected
        .with_source_schema(&context, schema_format_arguments)
        .expect("the actual context still admits clock scan source metadata");
    assert!(
        formats.is_none_or(|formats| formats.is_empty()),
        "the actual option grammar supplies no profile-derived format operand"
    );
}

#[test]
fn original_hint_metadata_keeps_the_generation_after_reporting_maps_are_cleared() {
    // naming.core.original-inlay-retained-context
    // docs/design/analysis/name-resolution-proofs/original-inlay-retained-context.md
    let source = "format {%d} 3";
    let mut analysis = analysis_with_availability(source, "tcl9.1");
    let profile = analysis.resolved_profile().unwrap();
    let retained_context = analysis.resolved_input.as_ref().unwrap().context_registry();
    let registry = retained_context.commands();
    let expected = hints(source, &analysis, profile, registry);
    assert!(
        expected.iter().any(|hint| hint.label == "int"),
        "{expected:?}"
    );
    analysis.command_invocations.clear();
    analysis.all_procs.clear();
    analysis.global_scope.procs.clear();
    analysis.global_scope.variables.clear();
    analysis.dialect = "foreign presentation label".to_owned();
    assert_eq!(hints(source, &analysis, profile, registry), expected);
    assert_eq!(
        hints("format {%x} 3", &analysis, profile, registry),
        [] as [crate::inlay_hints::InlayHint; 0]
    );

    let input = analysis.resolved_input.as_ref().unwrap();
    let context = input.context_registry();
    let foreign = tcl_compiler::analyser::ResolvedAnalysisInput::new(
        input.analyser_profile(),
        input.unit_profile(),
        std::sync::Arc::new(
            context.with_command_store(context.commands().snapshot().shared_registry()),
        ),
        input.lexer_config(),
    );
    analysis.resolved_input = Some(foreign);
    assert_eq!(
        hints(source, &analysis, profile, registry),
        [] as [crate::inlay_hints::InlayHint; 0]
    );
}

#[test]
fn original_inlay_hints_keep_retained_profile_registry_and_whole_source() {
    // naming.core.original-inlay-retained-context
    // docs/design/analysis/name-resolution-proofs/original-inlay-retained-context.md
    let source = "set value 42\nformat {%d} 3";
    let mut analysis = Analyser::new().analyse(source, "tcl8.6");
    analysis.global_scope.variables.clear();
    let profile = analysis.resolved_profile().unwrap();
    let registry = analysis.resolved_registry().unwrap();
    let expected = hints(source, &analysis, profile, registry);
    assert!(
        expected.iter().any(|hint| hint.label == ": int"),
        "{expected:?}"
    );
    assert!(
        expected.iter().any(|hint| hint.label == "int"),
        "{expected:?}"
    );
    assert_eq!(
        hints(
            source,
            &analysis,
            crate::profile_for_dialect("jim"),
            crate::registry_for_dialect("jim")
        ),
        expected,
    );
    assert_eq!(
        hints(&format!("{source} "), &analysis, profile, registry),
        [] as [crate::inlay_hints::InlayHint; 0]
    );
    let config = analysis.body_lexer_config.unwrap();
    let mut different = config;
    different.strict_quoting = !different.strict_quoting;
    analysis.body_lexer_config = Some(different);
    assert_eq!(
        hints(
            source,
            &analysis,
            profile,
            crate::registry_for_dialect("tcl8.6")
        ),
        [] as [crate::inlay_hints::InlayHint; 0]
    );
}

#[test]
fn original_format_hints_follow_actual_nested_roles_and_known_shadow() {
    // naming.core.original-inlay-retained-context
    // docs/design/analysis/name-resolution-proofs/original-inlay-retained-context.md
    let source = "proc p {} {format {%d} 3}";
    let analysis = Analyser::new().analyse(source, "tcl8.6");
    let out = hints(
        source,
        &analysis,
        crate::profile_for_dialect("jim"),
        crate::registry_for_dialect("jim"),
    );
    assert!(out.iter().any(|hint| hint.label == "int"), "{out:?}");
    let source = "proc format args {return ignored}\nproc p {} {format {%d} 3}";
    let analysis = Analyser::new().analyse(source, "tcl8.6");
    let out = hints(
        source,
        &analysis,
        crate::profile_for_dialect("tcl8.6"),
        crate::registry_for_dialect("tcl8.6"),
    );
    assert!(!out.iter().any(|hint| hint.label == "int"), "{out:?}");
}

#[test]
fn logical_type_hints_select_the_original_definition_version() {
    let source = "set value 42\nset value TEXT\nputs $value\n";
    let analysis = Analyser::new().analyse(source, "tcl");
    assert!(analysis.allows_lexical_declaration_advice());
    let hints = inlay_hints(
        source,
        analysis.resolved_profile().unwrap(),
        whole_document_range(source),
        Some(&analysis),
        analysis.resolved_registry(),
        true,
        false,
    );
    assert!(
        hints.iter().any(|hint| hint.kind == InlayHintKind::Type
            && hint.position_line == 0
            && hint.label == ": int"),
        "{hints:?}"
    );
    assert!(
        hints
            .iter()
            .all(|hint| hint.position_line != 0 || hint.label != ": str"),
        "{hints:?}"
    );
}

#[test]
fn logical_type_hints_refuse_a_same_named_definition_from_another_body() {
    let source = "proc first {} {set value 42}\nproc second {} {set value TEXT}\n";
    let mut analysis = Analyser::new().analyse(source, "tcl");
    assert!(analysis.allows_lexical_declaration_advice());
    let scopes = &mut analysis.global_scope.children;
    assert_eq!(scopes.len(), 2);
    let foreign_span = scopes[1].variables["value"].definition_span;
    scopes[0]
        .variables
        .get_mut("value")
        .unwrap()
        .definition_span = foreign_span;
    let hints = inlay_hints(
        source,
        analysis.resolved_profile().unwrap(),
        whole_document_range(source),
        Some(&analysis),
        analysis.resolved_registry(),
        true,
        false,
    );
    assert!(
        hints
            .iter()
            .all(|hint| hint.kind != InlayHintKind::Type || hint.position_line != 0),
        "{hints:?}"
    );
    assert_eq!(
        hints
            .iter()
            .filter(|hint| hint.kind == InlayHintKind::Type && hint.position_line == 1)
            .map(|hint| hint.label.as_str())
            .collect::<Vec<_>>(),
        vec![": str"],
        "{hints:?}"
    );
}

#[test]
fn logical_type_hints_keep_repeated_declarations_on_their_original_body_ids() {
    let source = "proc repeated {} {set value 42}\nproc repeated {} {set value TEXT}\n";
    let analysis = Analyser::new().analyse(source, "tcl");
    assert!(analysis.allows_lexical_declaration_advice());
    let registry = analysis.resolved_registry().unwrap();
    let hints = inlay_hints(
        source,
        analysis.resolved_profile().unwrap(),
        whole_document_range(source),
        Some(&analysis),
        Some(registry),
        true,
        false,
    );
    let labels = |line| {
        hints
            .iter()
            .filter(|hint| hint.kind == InlayHintKind::Type && hint.position_line == line)
            .map(|hint| hint.label.as_str())
            .collect::<Vec<_>>()
    };
    assert!(labels(0).contains(&": int"), "{hints:?}");
    assert!(!labels(0).contains(&": str"), "{hints:?}");
    assert!(labels(1).contains(&": str"), "{hints:?}");
    assert!(!labels(1).contains(&": int"), "{hints:?}");
}

#[test]
fn logical_type_hints_keep_constructed_colon_scopes_and_original_source() {
    // naming.analysis.constructed-key-consumer-projections
    // docs/design/analysis/name-resolution-proofs/constructed-key-consumer-projections.md
    let source = "namespace eval Ω {\nproc :pair {} {set x 42}\nproc pair {} {set x 3.5}\n}\n";
    let analysis = Analyser::new().analyse(source, "tcl");
    assert!(analysis.allows_lexical_declaration_advice());
    let registry = analysis.resolved_registry().unwrap();
    let hints = inlay_hints(
        source,
        analysis.resolved_profile().unwrap(),
        whole_document_range(source),
        Some(&analysis),
        Some(registry),
        true,
        false,
    );
    let labels = |line| {
        hints
            .iter()
            .filter(|hint| hint.kind == InlayHintKind::Type && hint.position_line == line)
            .map(|hint| hint.label.as_str())
            .collect::<Vec<_>>()
    };
    assert!(labels(1).contains(&": int"), "{hints:?}");
    assert!(!labels(1).contains(&": double"), "{hints:?}");
    assert!(labels(2).contains(&": double"), "{hints:?}");
    assert!(!labels(2).contains(&": int"), "{hints:?}");
    let changed = format!("# changed\n{source}");
    assert_eq!(
        inlay_hints(
            &changed,
            analysis.resolved_profile().unwrap(),
            whole_document_range(&changed),
            Some(&analysis),
            Some(registry),
            true,
            false,
        ),
        [] as [crate::inlay_hints::InlayHint; 0]
    );
    assert_eq!(
        inlay_hints(
            source,
            analysis.resolved_profile().unwrap(),
            whole_document_range(source),
            None,
            Some(registry),
            true,
            false,
        ),
        [] as [crate::inlay_hints::InlayHint; 0]
    );
}
