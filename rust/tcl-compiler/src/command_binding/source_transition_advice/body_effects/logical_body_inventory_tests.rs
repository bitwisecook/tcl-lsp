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

use crate::command_binding::SourceCommandTransitionObligation;
use crate::registry_invocation::source_structure::{OriginalRegistrySource, source_registry_words};

fn analyse(source: &str) -> crate::analyser::AnalysisResult {
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let input = crate::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        tcl_registry::model::ingress::resolve_environment("tcl9.1").default_context_registry(),
        tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
    );
    crate::analyser::Analyser::new()
        .with_resolved_input(input)
        .analyse(source, profile.name)
}

#[test]
fn logical_immediate_body_schema_keeps_original_geometry_and_unknown_effects_separate() {
    // naming.source.original-editor-body-structure
    // docs/design/analysis/name-resolution-proofs/original-editor-body-structure.md
    for source in [
        "switch $subject one {set local 1} default {set other 2}",
        "if $condition {set local 1}",
        "foreach item $values {set local 1}",
    ] {
        let analysis = analyse(source);
        let input = analysis.resolved_input.as_ref().unwrap();
        let config = input.lexer_config();
        let context = input.context_registry();
        let root = tcl_source::segment::segment_commands_with_offset_and_config(source, 0, config)
            .remove(0);
        let parent = source_registry_words(source, &analysis, &root).unwrap();
        let bodies = parent.source_script_bodies(&context);
        assert!(!bodies.is_empty(), "{source}");
        for body in bodies {
            let span = body.content_span();
            let child = tcl_source::segment::segment_commands_with_offset_and_config(
                &source[span.as_range()],
                span.start(),
                config,
            )
            .remove(0);
            let words = source_registry_words(source, &analysis, &child).expect(source);
            let OriginalRegistrySource::SourceTransitions(advice) = words.source() else {
                panic!("missing own conditional Logical body: {source}");
            };
            assert_eq!(advice.logical_source_input(), Some(input));
            assert_eq!(advice.logical_source_body(), Some(&body));
            assert_eq!(
                words.roles(),
                Some([(0, tcl_registry::ArgRole::VarWrite)].as_slice())
            );
            assert!(
                advice.obligations().contains(
                    &SourceCommandTransitionObligation::ConditionalLogicalBodyApplicability
                )
            );
            assert!(
                advice
                    .obligations()
                    .contains(&SourceCommandTransitionObligation::UnknownEarlierMutation)
            );
            assert!(!words.operands_preserve_source_lookup());
            assert!(
                !advice.matches_source(&tcl_lexer::SourceImage::native(source.as_bytes()), config)
            );
        }
    }
}

#[test]
fn logical_immediate_body_schema_keeps_child_mutations_and_foreign_owners_terminal() {
    // naming.source.original-editor-body-structure
    // docs/design/analysis/name-resolution-proofs/original-editor-body-structure.md
    let source = "switch $subject one {rename set gone; set blocked 1}";
    let analysis = analyse(source);
    let config = analysis.resolved_input.as_ref().unwrap().lexer_config();
    let offset = u32::try_from(source.find("set blocked").unwrap()).unwrap();
    let segment = tcl_source::segment::segment_commands_with_offset_and_config(
        &source[usize::try_from(offset).unwrap()..source.len() - 1],
        offset,
        config,
    )
    .remove(0);
    assert!(source_registry_words(source, &analysis, &segment).is_none());
    let source = "if $condition {set local 1}";
    let mut analysis = analyse(source);
    let offset = u32::try_from(source.find("set local").unwrap()).unwrap();
    let segment = tcl_source::segment::segment_commands_with_offset_and_config(
        &source[usize::try_from(offset).unwrap()..source.len() - 1],
        offset,
        config,
    )
    .remove(0);
    assert!(source_registry_words(source, &analysis, &segment).is_some());
    assert!(source_registry_words(&format!("{source} "), &analysis, &segment).is_none());
    analysis.body_lexer_config = Some(tcl_lexer::LexerConfig {
        strict_quoting: !config.strict_quoting,
        ..config
    });
    assert!(source_registry_words(source, &analysis, &segment).is_none());
    let native = crate::analyser::Analyser::new().analyse(source, "tcl9.1");
    if let Some(words) = source_registry_words(source, &native, &segment)
        && let OriginalRegistrySource::SourceTransitions(advice) = words.source()
    {
        assert!(advice.logical_source_input().is_none());
        assert!(
            !advice
                .obligations()
                .contains(&SourceCommandTransitionObligation::ConditionalLogicalBodyApplicability)
        );
    }
}
#[test]
fn logical_root_substitutions_keep_alias_point_without_future_body_applicability() {
    // naming.source.original-editor-body-structure
    // docs/design/analysis/name-resolution-proofs/original-editor-body-structure.md
    let source = "interp alias {} make {} list set x; eval [make $value]";
    let analysis = analyse(source);
    let input = analysis.resolved_input.as_ref().unwrap();
    let offset = u32::try_from(source.find("make $value").unwrap()).unwrap();
    let segment = tcl_source::segment::segment_commands_with_offset_and_config(
        "make $value",
        offset,
        input.lexer_config(),
    )
    .remove(0);
    let words = source_registry_words(source, &analysis, &segment).unwrap();
    let OriginalRegistrySource::SourceTransitions(advice) = words.source() else {
        panic!("missing genuine original alias substitution point");
    };
    assert_eq!(advice.logical_source_input(), Some(input));
    assert!(advice.logical_source_body().is_none());
    assert!(advice.original_head().native_input().is_none());
    assert!(
        !advice
            .obligations()
            .contains(&SourceCommandTransitionObligation::ConditionalLogicalBodyApplicability)
    );
    assert!(
        !advice
            .obligations()
            .contains(&SourceCommandTransitionObligation::DeferredLogicalBodyApplicability)
    );
    assert_eq!(words.command(), "list");
    assert_eq!(
        words
            .arguments()
            .first()
            .and_then(|word| word.literal_bytes()),
        Some(b"set".as_slice())
    );
    assert_eq!(
        words
            .arguments()
            .get(1)
            .and_then(|word| word.literal_bytes()),
        Some(b"x".as_slice())
    );
}

#[test]
fn logical_root_substitutions_keep_earlier_mutations_and_owner_refusals() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    let source = "interp alias {} make {} list set x; eval [make $value]";
    let baseline = analyse(source);
    let offset = u32::try_from(source.find("make $value").unwrap()).unwrap();
    let config = baseline.body_lexer_config.unwrap();
    let segment =
        tcl_source::segment::segment_commands_with_offset_and_config("make $value", offset, config)
            .remove(0);
    assert!(source_registry_words(source, &baseline, &segment).is_some());
    let changed = "interp alias {} make {} list set x; eval [list [rename make {}] [make $value]]";
    let changed_analysis = analyse(changed);
    let offset = u32::try_from(changed.find("make $value").unwrap()).unwrap();
    let blocked =
        tcl_source::segment::segment_commands_with_offset_and_config("make $value", offset, config)
            .remove(0);
    assert!(source_registry_words(changed, &changed_analysis, &blocked).is_none());
    assert!(source_registry_words(&format!("{source} "), &baseline, &segment).is_none());
    let mut missing = baseline.clone();
    missing.resolved_input = None;
    assert!(source_registry_words(source, &missing, &segment).is_none());
    let mut config_changed = baseline;
    config_changed.body_lexer_config = Some(tcl_lexer::LexerConfig {
        strict_quoting: !config.strict_quoting,
        ..config
    });
    assert!(source_registry_words(source, &config_changed, &segment).is_none());
}
