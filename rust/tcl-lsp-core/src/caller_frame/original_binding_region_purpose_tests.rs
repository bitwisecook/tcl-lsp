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
fn original_binding_regions_keep_selected_frame_body_and_whole_file_purposes() {
    // naming.source.original-configured-script-completeness
    // docs/design/analysis/name-resolution-proofs/source-original-configured-script-completeness.md
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
    let source = "\u{feff}proc p {} {\u{feff}set x 1; puts $x}";
    let input = tcl_compiler::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        tcl_registry::model::ingress::context_for_profile(profile),
        config,
    );
    let analysis = tcl_compiler::analyser::Analyser::new()
        .with_resolved_input(input.clone())
        .analyse(source, profile.name);
    let (start, end, nested) = enclosing_frame_region(
        &analysis.global_scope,
        u32::try_from(source.find("$x").unwrap()).unwrap(),
        source,
    );
    assert!(nested);
    assert_eq!(
        binding_region_commands(source, start, end, config, nested)[0].name(),
        "\u{feff}set"
    );
    let (start, end, nested) = enclosing_frame_region(&analysis.global_scope, 0, source);
    assert!(!nested);
    assert_eq!(
        binding_region_commands(source, start, end, config, nested)[0].name(),
        "proc"
    );
    let body = "\u{feff}set x 1";
    assert_eq!(
        binding_region_commands(body, 0, body.len(), config, true)[0].name(),
        "\u{feff}set"
    );
    let mut first_close = config;
    first_close.braced_var = tcl_lexer::BracedVarStyle::FirstClose;
    let body = "puts ${a{b}; set x 1";
    assert_eq!(
        binding_region_commands(body, 0, body.len(), first_close, true).len(),
        2
    );
    assert!(binding_region_commands(body, 0, body.len(), config, true)[0].is_partial);
    assert_eq!(analysis.resolved_input.as_ref(), Some(&input));
    assert!(
        analysis.matches_original_source_image(&tcl_lexer::SourceImage::document(source), config)
    );
}
