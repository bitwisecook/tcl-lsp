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
fn original_dispatch_body_scans_keep_bom_content_and_selected_full_config() {
    // naming.core.original-dispatch-region-context
    // docs/design/analysis/name-resolution-proofs/core-original-dispatch-region-context.md
    // Source-only method reference geometry; no object or method execution.
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let mut registry = tcl_registry::CommandRegistry::build_default();
    let surface = registry.get("dict").unwrap().surface;
    registry.insert(tcl_registry::CommandSpec {
        name: "source_dispatch",
        arity: tcl_registry::Arity::at_least(1),
        traits: tcl_registry::Traits::TCLOO_SELF_DISPATCH,
        surface,
        ..tcl_registry::CommandSpec::DEFAULT
    });
    let registry = std::sync::Arc::new(registry);
    let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
    let context = std::sync::Arc::new(
        tcl_registry::model::ingress::static_context_for("tcl8.6").with_command_store(registry),
    );
    let input = tcl_compiler::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        context.clone(),
        config,
    );
    for (source, expected) in [
        ("source_dispatch selected", 1),
        ("\u{feff}source_dispatch selected", 0),
    ] {
        let analysis = tcl_compiler::analyser::Analyser::new()
            .with_resolved_input(input.clone())
            .analyse(source, profile.name);
        let body = tcl_lexer::Span::new(0, u32::try_from(source.len()).unwrap());
        assert_eq!(
            scan_my_method_sites(source, &analysis, &[body], "selected", None).len(),
            expected
        );
        assert_eq!(analysis.resolved_input.as_ref(), Some(&input));
        assert!(
            analysis
                .matches_original_source_image(&tcl_lexer::SourceImage::document(source), config)
        );
    }
    let mut first_close = config;
    first_close.braced_var = tcl_lexer::BracedVarStyle::FirstClose;
    let source = "puts ${a{b}; source_dispatch selected";
    let input =
        tcl_compiler::analyser::ResolvedAnalysisInput::new(profile, profile, context, first_close);
    let analysis = tcl_compiler::analyser::Analyser::new()
        .with_resolved_input(input.clone())
        .analyse(source, profile.name);
    let body = tcl_lexer::Span::new(0, u32::try_from(source.len()).unwrap());
    assert_eq!(
        scan_my_method_sites(source, &analysis, &[body], "selected", None).len(),
        1
    );
    let mut changed = analysis.clone();
    changed.body_lexer_config = Some(config);
    assert_eq!(
        scan_my_method_sites(source, &changed, &[body], "selected", None),
        [] as [tcl_lexer::Span; 0]
    );
    assert_eq!(
        scan_my_method_sites(
            &format!("{source}#changed"),
            &analysis,
            &[body],
            "selected",
            None
        ),
        [] as [tcl_lexer::Span; 0]
    );
}
