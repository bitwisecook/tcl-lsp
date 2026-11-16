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
use crate::analyser::ResolvedAnalysisInput;

#[test]
fn original_condition_child_keeps_bom_alias_and_actual_expression_base() {
    // naming.diagnostics.original-matcher-output-retention
    // docs/design/analysis/name-resolution-proofs/diagnostic-original-matcher-output-retention.md
    // Selected conditional matcher/output schema only, no executed matcher or output cell contents.
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let context =
        crate::environment_ingress::resolve_environment("tcl8.6").default_context_registry();
    let config = tcl_lexer::LexerConfig {
        leading_bom: tcl_lexer::LeadingBom::Skip,
        braced_var: tcl_lexer::BracedVarStyle::FirstClose,
        strict_quoting: true,
        ..tcl_lexer::LexerConfig::for_file_grammar(profile.grammar)
    };
    let input = ResolvedAnalysisInput::new(profile, profile, context.clone(), config);
    assert!(input.has_logical_source_name_context());
    let source =
        "\u{feff}interp alias {} \u{feff}match {} regexp\nif {![\u{feff}match x y v]} {puts $v}";
    let mut analyser = Analyser::new().with_resolved_input(input.clone());
    analyser.result = analyser.analyse(source, "presentation");
    let image = tcl_lexer::SourceImage::document(source);
    let plan = tcl_lexer::native_script_words_in(
        image.clone(),
        tcl_lexer::Span::new(0, u32::try_from(source.len()).unwrap()),
        config,
    )
    .unwrap();
    let word = &plan.commands.last().unwrap().words[1];
    let region = word.content_span().unwrap();
    let expression = crate::expr_parser::parse_expr_with_grammar(
        &source[region.as_range()],
        &config.grammar_over(profile.grammar),
    );
    let (selected, negated) = analyser
        .original_condition_matcher(&expression, Some(region.start()))
        .unwrap();
    assert!(negated);
    assert_eq!(selected.command(), "regexp");
    assert_eq!(
        selected_matcher_no_match_outputs(&selected, &context),
        ["v"]
    );
    assert_eq!(word.config(), config);
    assert_eq!(word.image(), &image);
    assert_eq!(analyser.result.resolved_input.as_ref(), Some(&input));
    assert!(
        analyser
            .original_condition_matcher(&expression, None)
            .is_none()
    );
    assert!(
        analyser
            .original_condition_matcher(&expression, region.start().checked_add(1))
            .is_none()
    );
    let retained = analyser.result.clone();
    analyser.result.body_lexer_config = Some(config.nested());
    assert!(
        analyser
            .original_condition_matcher(&expression, Some(region.start()))
            .is_none()
    );
    analyser.result = retained.clone();
    analyser.result.resolved_input = None;
    assert!(
        analyser
            .original_condition_matcher(&expression, Some(region.start()))
            .is_none()
    );
    analyser.result = retained;
    analyser.source = source.replace(" x y v", " x z v").into();
    assert!(
        analyser
            .original_condition_matcher(&expression, Some(region.start()))
            .is_none()
    );
}
