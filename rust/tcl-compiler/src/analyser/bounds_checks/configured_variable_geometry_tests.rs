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
fn analytical_roots_and_scalar_names_never_truncate_or_accept_partial_braces() {
    let grammar = tcl_dialect::LexerGrammar::default();
    let config = tcl_lexer::LexerConfig::from_grammar(grammar);
    for source in ["$i", "${i}"] {
        assert_eq!(var_scalar_name(source, config).as_deref(), Some("i"));
        assert_eq!(strict_scalar_name(source, config).as_deref(), Some("i"));
    }
    assert_eq!(
        var_scalar_name("$arr(index)", config).as_deref(),
        Some("arr")
    );
    assert!(strict_scalar_name("$arr(index)", config).is_none());
    for source in ["${missing", "$name}", "$ns::value", "$café", "$a$b"] {
        assert!(var_scalar_name(source, config).is_none(), "{source}");
        assert!(strict_scalar_name(source, config).is_none(), "{source}");
    }
}

#[test]
fn actual_condition_configuration_reaches_both_counter_projection_paths() {
    let grammar = tcl_dialect::LexerGrammar::default();
    let mut config = tcl_lexer::LexerConfig::from_grammar(grammar);
    config.strict_quoting = !config.strict_quoting;
    assert_eq!(
        extract_counter_name("${i} < 10", &grammar, config).as_deref(),
        Some("i")
    );
    assert_eq!(
        parse_simple_for_cond_configured("${i} < 10", &grammar, config),
        Some(("i".to_owned(), "<".to_owned(), 10))
    );
    assert!(parse_simple_for_cond_configured("$ns::i < 10", &grammar, config).is_none());
    assert!(extract_counter_name("[next] < $i", &grammar, config).is_none());
    assert!(parse_simple_for_cond_configured("${missing < 10", &grammar, config).is_none());
}
