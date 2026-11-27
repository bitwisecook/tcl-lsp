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

fn assert_original_nested_eval_activation(
    source: &str,
    expected: &str,
    materialised: bool,
    frame: &tcl_names::variable::VariableExecutionFrame,
    config: tcl_lexer::LexerConfig,
    context: &tcl_registry::model::ContextRegistry,
    options: super::super::SourceAnalysisOptions<'_>,
) {
    let image = tcl_lexer::SourceImage::document(source);
    let bindings = SourceCommandBindings::analyse_image_in_frame_with_options(
        &image,
        frame,
        config,
        context.commands(),
        options,
    )
    .unwrap();
    let origin = Arc::clone(bindings.root_origin.as_ref().unwrap());
    let site = CommandAllocationSite {
        source: Arc::clone(&origin),
        offset: 0,
    };
    let entered = bindings.executed_script_at(&site, 0).unwrap();
    assert_eq!(entered.text.try_text().unwrap(), expected);
    if materialised {
        assert_eq!(entered.base(), 0);
        assert!(matches!(
            entered.mapping,
            tcl_names::origin::ExecutedScriptMapping::Materialised
        ));
        assert_ne!(entered.origin, origin);
    }
    let body_rows = bindings
        .declaration_layouts
        .get(&CommandAllocationSite {
            source: Arc::clone(&entered.origin),
            offset: entered.base(),
        })
        .unwrap();
    let activation = body_rows
        .iter()
        .find(|row| {
            matches!(
                row.entry.as_ref(),
                OriginalDiagnosticFrameEntry::ScriptActivation { .. }
            )
        })
        .unwrap();
    assert_eq!(activation.entry.source(), entered);
    assert_eq!(activation.entry.frame().layout(), frame);
    assert_eq!(activation.config, config);
    assert_eq!(activation.words[0].legacy_text(), "set");
    assert!(
        matches!(activation.entry.as_ref(), OriginalDiagnosticFrameEntry::ScriptActivation {
    config: actual, ..
} if *actual == config)
    );
    assert!(activation.snapshot.state.baseline.native_entry.is_none());
    assert!(
        bindings
            .declaration_layouts
            .get(&site)
            .unwrap()
            .iter()
            .any(|row| matches!(
                row.entry.as_ref(),
                OriginalDiagnosticFrameEntry::RootScript { .. }
            ))
    );
}

#[test]
fn original_nested_parser_keeps_entered_root_frame_separate_from_whole_input() {
    // naming.source.original-configured-script-completeness
    // docs/design/analysis/name-resolution-proofs/source-original-configured-script-completeness.md
    // A genuine eval observation supplies its frame; NestedScript supplies grammar only.
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let context =
        tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry();
    let config = tcl_lexer::LexerConfig {
        leading_bom: tcl_lexer::LeadingBom::Skip,
        strict_quoting: true,
        braced_var: tcl_lexer::BracedVarStyle::FirstClose,
        ..tcl_lexer::LexerConfig::for_file_grammar(profile.grammar)
    };
    let input =
        crate::analyser::ResolvedAnalysisInput::new(profile, profile, Arc::clone(&context), config);
    let frame = tcl_names::variable::VariableExecutionFrame::Global;
    let options = super::super::SourceAnalysisOptions::for_logical_source(&input).unwrap();
    for (source, expected, materialised) in [
        ("eval {set x VALUE; list $x}", "set x VALUE; list $x", false),
        ("eval {set} {x VALUE}", "set x VALUE", true),
    ] {
        assert_original_nested_eval_activation(
            source,
            expected,
            materialised,
            &frame,
            config,
            &context,
            options,
        );
    }

    let body_at_zero = tcl_lexer::SourceImage::document("set x VALUE");
    let nested = SourceCommandBindings::analyse_image_in_frame_with_purpose(
        &body_at_zero,
        &frame,
        config,
        context.commands(),
        options,
        tcl_lexer::SourceSegmentationPurpose::NestedScript,
    )
    .unwrap();
    assert!(nested.points.iter().any(|point| point.offset == 0));
    assert!(
        nested
            .declaration_layouts
            .values()
            .flatten()
            .all(|row| !matches!(
                row.entry.as_ref(),
                OriginalDiagnosticFrameEntry::RootScript { .. }
            ))
    );
    assert_eq!(
        nested.original_script_activations,
        [] as [std::sync::Arc<
            crate::command_binding::declaration_layout::OriginalDiagnosticFrameEntry,
        >; 0]
    );
    let whole = SourceCommandBindings::analyse_image_in_frame_with_options(
        &body_at_zero,
        &frame,
        config,
        context.commands(),
        options,
    )
    .unwrap();
    assert!(
        whole
            .declaration_layouts
            .values()
            .flatten()
            .any(|row| matches!(
                row.entry.as_ref(),
                OriginalDiagnosticFrameEntry::RootScript { .. }
            ))
    );
}
