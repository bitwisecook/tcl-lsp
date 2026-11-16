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

fn input() -> crate::analyser::ResolvedAnalysisInput {
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let config = tcl_lexer::LexerConfig {
        leading_bom: tcl_lexer::LeadingBom::Skip,
        strict_quoting: true,
        braced_var: tcl_lexer::BracedVarStyle::FirstClose,
        ..tcl_lexer::LexerConfig::for_file_grammar(profile.grammar)
    };
    crate::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry(),
        config,
    )
}

#[test]
fn original_binding_purpose_retains_full_input_and_separates_cached_interpretations() {
    // naming.source.original-configured-script-completeness
    // docs/design/analysis/name-resolution-proofs/source-original-configured-script-completeness.md
    // The same original Document can have independently selected parser purposes.
    // Neither interpretation issues Native admission or a fresh body activation.
    let input = input();
    let config = input.lexer_config();
    let context = input.context_registry();
    let registry = context.commands();
    let image = tcl_lexer::SourceImage::document("\u{feff}puts ${a{b}");
    let frame = crate::var_resolve::VariableExecutionFrame::Global;
    let options = SourceAnalysisOptions::for_logical_source(&input).unwrap();
    let whole_key = source_analysis_cache::SourceAnalysisCacheKey::at_entry(
        &image,
        &frame,
        config,
        registry,
        options,
        tcl_lexer::SourceSegmentationPurpose::WholeInput,
    )
    .unwrap();
    let body_key = source_analysis_cache::SourceAnalysisCacheKey::at_entry(
        &image,
        &frame,
        config,
        registry,
        options,
        tcl_lexer::SourceSegmentationPurpose::NestedScript,
    )
    .unwrap();
    assert!(whole_key != body_key);
    for purpose in [
        tcl_lexer::SourceSegmentationPurpose::WholeInput,
        tcl_lexer::SourceSegmentationPurpose::NestedScript,
        tcl_lexer::SourceSegmentationPurpose::WholeInput,
    ] {
        let bindings = SourceCommandBindings::analyse_image_in_frame_with_purpose(
            &image, &frame, config, registry, options, purpose,
        )
        .unwrap();
        assert_eq!(bindings.lexer_config, Some(config));
        let point = bindings.points.first().unwrap();
        assert_eq!(point.state.logical_source_name_advice_input(), Some(&input));
        let offset = if purpose == tcl_lexer::SourceSegmentationPurpose::WholeInput {
            3
        } else {
            0
        };
        let head = if offset == 3 { "puts" } else { "\u{feff}puts" };
        assert_eq!(point.offset, offset);
        let binding = bindings.invocation_at_source(head, offset);
        let (command, tokens) = binding.original_recorded_command().unwrap();
        assert_eq!(command.texts[0], head);
        assert_eq!(command.texts[1], "${a{b}");
        assert_eq!(
            binding.original_lexer_config_for_tokens(&tokens),
            Some(config)
        );
        let original = crate::registry_invocation::original_native_compiler_words(
            &image,
            tokens.words(),
            offset,
            config,
        )
        .unwrap();
        assert_eq!(original.len(), 2);
        assert!(
            original
                .iter()
                .all(|word| word.config() == config && word.image() == &image)
        );
        let stale = tcl_lexer::LexerConfig {
            braced_var: tcl_lexer::BracedVarStyle::Tcl9Nesting,
            ..config
        };
        assert!(
            crate::registry_invocation::original_native_compiler_words(
                &image,
                tokens.words(),
                offset,
                stale,
            )
            .is_none()
        );
        assert!(point.state.baseline.native_entry.is_none());
        assert!(point.state.baseline.execution_name_policy.is_none());
        let mut foreign = tokens.clone();
        foreign.synthetic = Some(crate::ir::SyntheticMarker::IterationBindings(None));
        assert!(binding.original_lexer_config_for_tokens(&foreign).is_none());
    }
}
