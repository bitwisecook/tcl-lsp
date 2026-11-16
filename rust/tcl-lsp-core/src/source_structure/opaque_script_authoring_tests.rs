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
use tcl_compiler::analyser::Analyser;

fn marked(marked: &str) -> (String, u32) {
    let cursor = u32::try_from(marked.find("<|>").unwrap()).unwrap();
    (marked.replacen("<|>", "", 1), cursor)
}

#[test]
fn retained_opaque_authoring_keeps_source_roles_separate_from_scripts() {
    let (source, cursor) = marked(
        "sslictcl 1\npolicy baseline {\n check bespoke {\n  predicate {\n   se<|>\n  }\n }\n}\n",
    );
    let analysis = Analyser::new().analyse(&source, "sslictcl");
    let config = analysis.body_lexer_config.unwrap();
    let context = analysis.resolved_input.as_ref().unwrap().context_registry();
    let authoring = SourceStructure::opaque_script_authoring_at(&source, &analysis, cursor)
        .expect("actual opaque role and current parent");
    assert_eq!(
        authoring
            .command_prefix(&source, &analysis, cursor)
            .unwrap()
            .word_index(),
        0
    );
    let image = SourceImage::document(&source);
    let body_content = authoring
        .region
        .content_for_authoring(&image, config, &context)
        .unwrap();
    let structure = SourceStructure::capture(&source, Some(&analysis), config).unwrap();
    assert!(
        !structure
            .scripts
            .iter()
            .any(|(span, _)| *span == body_content)
    );
    assert_opaque_member_purposes(&source, &analysis, cursor, body_content);
    let changed = source.replace("baseline", "changed_");
    assert!(
        authoring
            .command_prefix(&changed, &analysis, cursor)
            .is_none()
    );
    let foreign = Analyser::new().analyse(&source, "tcl8.6");
    assert!(
        authoring
            .command_prefix(&source, &foreign, cursor)
            .is_none()
    );
    let mut changed_config = Analyser::new().analyse(&source, "sslictcl");
    changed_config.body_lexer_config = Some(LexerConfig {
        strict_quoting: !config.strict_quoting,
        ..config
    });
    assert!(
        authoring
            .command_prefix(&source, &changed_config, cursor)
            .is_none()
    );
    let profile = analysis.resolved_profile().unwrap();
    let overlay =
        tcl_registry::registry_for_profile_with_overlay(profile, 0x0A0A_2026, |registry| {
            let mut row = registry.get("set").unwrap().clone();
            row.name = "opaque_authoring_control";
            registry.insert(row);
        });
    let foreign_store = analyse_document(&source, profile, overlay.as_ref(), config);
    assert!(
        authoring
            .command_prefix(&source, &foreign_store, cursor)
            .is_none()
    );
    assert!(
        SourceStructure::opaque_script_authoring_at(&source, &foreign_store, cursor)
            .unwrap()
            .command_prefix(&source, &foreign_store, cursor)
            .is_some()
    );
}

fn assert_opaque_member_purposes(
    source: &str,
    analysis: &AnalysisResult,
    cursor: u32,
    body_content: Span,
) {
    use tcl_compiler::registry_invocation::source_structure::source_registry_words_in_definition_body_at;
    use tcl_compiler::registry_invocation::{
        OriginalSourceDefinitionMemberDisposition as MemberDisposition, OriginalSourceScriptPurpose,
    };
    let config = analysis.body_lexer_config.unwrap();
    let context = analysis.resolved_input.as_ref().unwrap().context_registry();
    let image = SourceImage::document(source);
    let structure = SourceStructure::capture_with_purpose(
        source,
        Some(analysis),
        config,
        CapturePurpose::DefinitionCursor,
    )
    .unwrap();
    let offset = u32::try_from(source.find("predicate").unwrap()).unwrap();
    let region = structure
        .definition_cursor_regions
        .iter()
        .filter(|region| region.span.start() <= offset && offset < region.span.end())
        .max_by_key(|region| region.depth)
        .unwrap();
    let parent = region.parent.as_ref().unwrap();
    assert_eq!(parent.source_words().command(), "check");
    let members = region.members.as_ref().unwrap();
    members.complete_original_commands().unwrap();
    let original = members.original_command_words_at(offset).unwrap();
    let member = members.member_keyword(original).unwrap();
    assert_eq!(member.keyword, "predicate");
    assert_eq!(member.arg_roles, &[(0, ArgRole::OpaqueScript)]);
    assert_eq!(
        members
            .argument_words_for_role(original, ArgRole::OpaqueScript)
            .unwrap(),
        vec![&original[1]]
    );
    assert_eq!(
        members
            .argument_words_for_role(original, ArgRole::Body)
            .unwrap(),
        [] as [&tcl_lexer::NativeWord; 0]
    );
    assert_eq!(
        members.checked_script_bodies(original).unwrap(),
        [] as [tcl_compiler::registry_invocation::OriginalSourceDefinitionMemberScriptBody; 0]
    );
    assert!(matches!(
        members.script_body_disposition(&context, original),
        MemberDisposition::CompleteMember(bodies)
            if bodies.is_empty()
    ));
    let opaque = members
        .opaque_script_authoring_regions(&context, original)
        .unwrap();
    assert_eq!(opaque.len(), 1);
    assert_eq!(
        opaque[0].content_for_authoring(&image, config, &context),
        Some(body_content)
    );
    // Non-block rows retain their member role, without a block descriptor.
    assert!(
        source_registry_words_in_definition_body_at(source, analysis, offset, parent).is_none()
    );
    let descriptor = context.commands().get(member.keyword).unwrap();
    assert_eq!(descriptor.arg_roles, &[(0, ArgRole::OpaqueScript)]);
    assert!(descriptor.definition_body.is_none());
    let words = parent.source_words();
    let syntax = words
        .source_script_bodies_for_syntax_coverage(&context)
        .unwrap();
    assert!(
        syntax
            .iter()
            .any(|body| body.content_span() == parent.content_span())
    );
    assert!(
        syntax
            .iter()
            .all(|body| body.content_span() != body_content)
    );
    assert!(
        words
            .source_script_bodies_for(&context, OriginalSourceScriptPurpose::Syntax)
            .iter()
            .all(|body| body.content_span() != body_content)
    );
    assert_eq!(
        words.source_script_bodies_for(&context, OriginalSourceScriptPurpose::PotentialEvaluation),
        [] as [tcl_compiler::registry_invocation::OriginalSourceScriptBody; 0]
    );
    assert!(
        words
            .source_script_bodies_for_mutation_coverage(&context)
            .is_none()
    );
    assert_opaque_not_script(source, analysis, cursor, body_content);
}

fn assert_opaque_not_script(source: &str, analysis: &AnalysisResult, cursor: u32, content: Span) {
    let config = analysis.body_lexer_config.unwrap();
    for purpose in [
        CapturePurpose::Readonly,
        CapturePurpose::SyntaxCompaction,
        CapturePurpose::CheckedSyntax,
        CapturePurpose::DefinitionCursor,
    ] {
        let captured =
            SourceStructure::capture_with_purpose(source, Some(analysis), config, purpose).unwrap();
        assert!(!captured.scripts.iter().any(|(span, _)| *span == content));
    }
    assert!(matches!(
        SourceStructure::definition_grammar_at(source, analysis, cursor),
        SourceDefinitionGrammar::Unavailable(DefinitionGrammarUnavailable::UnrepresentedInterior)
    ));
}

#[test]
fn opaque_authoring_refuses_unselected_or_cooked_operands() {
    for input in [
        "set data {se<|>}",
        "policy baseline {check bespoke {$head {se<|>}}}",
        "policy baseline {check bespoke {{*}predicate {se<|>}}}",
        "policy baseline {check bespoke {predicate \"s\\x65<|>\"}}",
        "policy baseline {check bespoke {predicate {se<|>} extra}}",
        "policy baseline {check bespoke {message {se<|>}}}",
        "policy baseline {check bespoke {predicate {*}{se<|>}}}",
    ] {
        let (source, cursor) = marked(input);
        let analysis = Analyser::new().analyse(&source, "sslictcl");
        assert!(
            SourceStructure::opaque_script_authoring_at(&source, &analysis, cursor).is_none(),
            "{input}"
        );
    }
}
