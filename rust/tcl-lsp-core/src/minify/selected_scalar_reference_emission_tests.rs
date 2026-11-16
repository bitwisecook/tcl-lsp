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
fn logical_scalar_emission_uses_selected_proposals_and_original_component_boundaries() {
    for (dialect, source, expected) in [
        ("tcl8.4", "${a{b}c}", "${a{b}"),
        ("tcl9.0", "${a{b}c}", "${a{b}c}"),
        ("tcl9.0", "${a{b}}", "${a{b}}"),
        ("tcl8.6", "${$n}", "${$n}"),
        ("tcl8.6", "${}", "${}"),
        ("tcl8.6", "${x}tail", "${x}"),
        ("tcl8.6", "${x}::tail", "${x}"),
        ("tcl9.0", "${x}::tail", "${x}"),
        ("tcl8.6", "${x}(k)", "${x}"),
        ("tcl8.6", "$array($index)", "$array($index)"),
        ("tcl8.6", "${array(key)}", "${array(key)}"),
    ] {
        let config = LexerConfig::for_dialect(dialect);
        let tokens = Lexer::with_config(source, config).tokenise_all().unwrap();
        let token = tokens
            .iter()
            .find(|token| token.kind == TokenType::Var)
            .copied()
            .unwrap();
        let end = tcl_lexer::word_parts::scan_var_ref(
            source.as_bytes(),
            token.span.start() as usize,
            config,
        )
        .unwrap()
        .unwrap()
        .next;
        let result =
            reconstruct_variable_reference(&SourceMap::new(source), token, &source[end..], config)
                .unwrap();
        assert_eq!(result, expected, "{dialect}: {source}");
    }
    let source = "${a{b}c}";
    let token = Lexer::with_config(source, LexerConfig::for_dialect("tcl8.4"))
        .tokenise_all()
        .unwrap()[0];
    assert!(
        reconstruct_variable_reference(
            &SourceMap::new(source),
            token,
            "",
            LexerConfig::for_dialect("tcl9.0")
        )
        .is_none()
    );

    let source =
        "puts ${x}tail; puts ${x}(k); puts $array($index); puts ${x}::tail; puts \"${x}::tail\"";
    let analysis = tcl_compiler::analyser::Analyser::new().analyse(source, "tcl");
    let config = analysis.body_lexer_config.unwrap();
    let result = lexical_default(source, &analysis).unwrap();
    assert_eq!(
        tcl_compiler::var_refs::scan_var_ref_forms_braced_with_config(&result, config),
        tcl_compiler::var_refs::scan_var_ref_forms_braced_with_config(source, config)
    );
}
