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

use tcl_lexer::{
    Lexer, LexerConfig, ParseIssueKind, first_parse_cut, group_commands, parse_cuts_in,
};

#[test]
fn cuts_keep_the_first_nested_error_in_each_command() {
    let source = "puts [list {a}b]\nputs ok\nputs [list \"c\"d]";
    let config = LexerConfig::default();
    let tokens = Lexer::with_config(source, config).tokenise_all().unwrap();
    let commands = group_commands(&tokens, source, config);
    let cuts = parse_cuts_in(&commands, &tokens, source, config).unwrap();
    assert_eq!(cuts.len(), 2);
    assert_eq!(cuts[0], first_parse_cut(source, config).unwrap());
    assert_eq!(cuts[0].command, 0);
    assert_eq!(cuts[0].kind, ParseIssueKind::ExtraAfterBrace);
    assert_eq!(cuts[1].command, 2);
    assert_eq!(cuts[1].kind, ParseIssueKind::ExtraAfterQuote);
    assert_eq!(source.as_bytes()[cuts[1].offset as usize], b'd');
}

#[test]
fn jim_quote_suffix_stays_accepted_while_brace_suffix_is_rejected() {
    let source = "puts [list \"a b\"c]\nputs [list {a b}c]";
    let config = LexerConfig::for_dialect("jim");
    let tokens = Lexer::with_config(source, config).tokenise_all().unwrap();
    let commands = group_commands(&tokens, source, config);
    let cuts = parse_cuts_in(&commands, &tokens, source, config).unwrap();
    assert_eq!(cuts.len(), 1, "{cuts:?}");
    assert_eq!(cuts[0].command, 1);
    assert_eq!(cuts[0].kind, ParseIssueKind::ExtraAfterBrace);
}
