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

//! Logical scope advice: cross-namespace candidates and write-card correspondence.

use super::*;

#[test]
fn logical_cross_namespace_candidates_use_actual_namespace_owners() {
    fn erase_labels(scope: &mut Scope) {
        scope.name = "unrelated report".to_owned();
        for child in &mut scope.children {
            erase_labels(child);
        }
    }

    let source =
        "namespace eval N {set {a{b}c} ONE; namespace eval Nested {set value TWO}}\nputs $";
    let mut analysis = Analyser::new().analyse(source, "tcl");

    erase_labels(&mut analysis.global_scope);
    let image = tcl_lexer::SourceImage::document(source);
    let config = analysis.body_lexer_config.unwrap();
    let advice = analysis
        .logical_variable_scope_advice(&image, config, u32::try_from(source.len()).unwrap())
        .unwrap();
    assert_eq!(
        advice.cross_namespace_variable_names(),
        vec!["::N::Nested::value", "::N::a{b}c"]
    );
    assert!(
        analysis
            .logical_variable_scope_advice(
                &tcl_lexer::SourceImage::document(&format!("#{source}")),
                config,
                0
            )
            .is_none()
    );
    let mut missing = analysis.clone();
    missing.resolved_input = None;
    assert!(
        missing
            .logical_variable_scope_advice(&image, config, 0)
            .is_none()
    );
    let mut unavailable = analysis.clone();
    unavailable.global_scope.children[0].naming_scope = None;
    let advice = unavailable
        .logical_variable_scope_advice(&image, config, u32::try_from(source.len()).unwrap())
        .unwrap();
    assert_eq!(
        advice.cross_namespace_variable_names(),
        [] as [std::string::String; 0]
    );
}

#[test]
fn logical_write_card_correspondence_uses_original_word_without_native_recipe() {
    let source = "namespace eval N {variable x FIRST; set x AGAIN; puts $x}";
    let mut analyser = Analyser::new();
    analyser.analyse_and_retain_result_for_test(source, "tcl");
    let span_start = u32::try_from(source.find("set x AGAIN").unwrap() + "set ".len()).unwrap();
    let advice = analyser
        .result
        .original_variable_write_advice
        .iter()
        .find(|advice| advice.span().start() == span_start)
        .expect("actual Registry-selected Logical write card");
    let span = advice.span();
    let input = advice.original_name_input().clone();
    let image = tcl_lexer::SourceImage::document(source);
    let config = analyser.file_lexer_config();
    assert!(
        analyser
            .result
            .logical_original_word_input_matches_source(&image, config, span, &input)
    );
    let changed = tcl_lexer::SourceImage::document(&source.replace("AGAIN", "OTHER"));
    assert!(
        !analyser
            .result
            .logical_original_word_input_matches_source(&changed, config, span, &input)
    );
    let mut wrong = config;
    wrong.strict_quoting = !wrong.strict_quoting;
    assert!(
        !analyser
            .result
            .logical_original_word_input_matches_source(&image, wrong, span, &input)
    );
    let shifted = Span::new(span.start() + 1, span.end() + 1);
    assert!(
        !analyser
            .result
            .logical_original_word_input_matches_source(&image, config, shifted, &input)
    );
    let mut absent = analyser.result.clone();
    absent.resolved_input = None;
    assert!(!absent.logical_original_word_input_matches_source(&image, config, span, &input));
    let retained = analyser.result.resolved_input.as_ref().unwrap();
    let foreign = std::sync::Arc::new(retained.borrowed_context_registry().with_command_store(
        std::sync::Arc::new(tcl_registry::CommandRegistry::build_default()),
    ));
    let mut changed_store = analyser.result.clone();
    changed_store.resolved_input = Some(super::super::super::ResolvedAnalysisInput::new(
        retained.analyser_profile(),
        retained.unit_profile(),
        foreign,
        retained.lexer_config(),
    ));
    assert!(
        !changed_store.logical_original_word_input_matches_source(&image, config, span, &input)
    );
    let mut native = Analyser::new();
    native.analyse_and_retain_result_for_test(source, "tcl8.6");
    assert!(!native.result.logical_original_word_input_matches_source(
        &image,
        native.file_lexer_config(),
        span,
        &input
    ));
}
