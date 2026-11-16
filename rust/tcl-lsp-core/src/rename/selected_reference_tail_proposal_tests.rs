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
fn logical_reference_rename_declines_unrepresentable_qualified_brace_tail() {
    let nested = tcl_lexer::LexerConfig {
        braced_var: tcl_lexer::BracedVarStyle::Tcl9Nesting,
        ..Default::default()
    };
    let source = "${ns{::x}}";
    let span = tcl_lexer::Span::new(0, u32::try_from(source.len()).unwrap());
    assert!(build_var_ref_replacement(source, span, "new", nested).is_none());
    let first = tcl_lexer::LexerConfig {
        braced_var: tcl_lexer::BracedVarStyle::FirstClose,
        ..nested
    };
    let source = "${ns{::x}";
    let span = tcl_lexer::Span::new(0, u32::try_from(source.len()).unwrap());
    assert_eq!(
        build_var_ref_replacement(source, span, "new", first).as_deref(),
        Some("${ns{::new}")
    );
    let source = "$ns:::arr($index[command])";
    let span = tcl_lexer::Span::new(0, u32::try_from(source.len()).unwrap());
    assert_eq!(
        build_var_ref_replacement(source, span, "new", nested).as_deref(),
        Some("$ns:::new($index[command])")
    );
}
