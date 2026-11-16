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

#[test]
fn w214_uses_original_tight_name_spans_and_quoted_markers() {
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let input = crate::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        tcl_registry::model::ingress::context_for_profile(profile),
        tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
    );
    assert!(
        tcl_registry::InvocationDialect::of_profile(profile)
            .authored_name_policy()
            .is_some()
    );
    assert!(input.has_logical_source_name_context());
    let source = r#"proc p {"as" value extra} {return 1}"#;
    let analysis = crate::analyser::Analyser::new()
        .with_resolved_input(input)
        .analyse(source, profile.name);
    let hints = analysis
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.code == tcl_core_types::DiagCode::W214)
        .map(|diagnostic| &source[diagnostic.span.as_range()])
        .collect::<Vec<_>>();
    assert_eq!(hints, ["value", "extra"]);
}
