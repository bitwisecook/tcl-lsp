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

fn anchor(source: &str, dialect: &str, target: &str) -> Option<tcl_lexer::Span> {
    let environment = tcl_registry::model::resolve_environment(dialect);
    let profile = environment.unit_profile();
    let input = crate::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        environment.default_context_registry(),
        tcl_lexer::LexerConfig::for_profile(Some(profile)),
    );
    let mut analyser = crate::analyser::Analyser::new().with_resolved_input(input);
    analyser.source = source.to_owned().into();
    analyser.narrow_to_read_var(
        tcl_lexer::Span::new(0, u32::try_from(source.len()).unwrap()),
        target,
    )
}

#[test]
fn variable_read_anchor_preserves_literal_name_and_complete_reference_geometry() {
    // naming.diagnostics.original-variable-name-anchor
    // docs/design/analysis/name-resolution-proofs/diagnostic-original-variable-name-anchor.md
    // Lexical anchor controls, not variable existence, W210 admission or
    // successful native reads. A braced scalar may contain an unmatched '('.
    for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        for (source, target, reference) in [
            ("puts $a ${a(b}", "a(b", "${a(b}"),
            ("puts ${literal} ${$literal}", "$literal", "${$literal}"),
            ("puts ${é}", "é", "${é}"),
            ("puts $a(k)", "a(k)", "$a(k)"),
            ("puts ${a(k)}", "a(k)", "${a(k)}"),
            ("puts ${}", "", "${}"),
            ("set marker 1; puts [list ${a(b}]", "a(b", "${a(b}"),
        ] {
            let start = u32::try_from(source.find(reference).unwrap()).unwrap();
            let expected =
                tcl_lexer::Span::new(start, start + u32::try_from(reference.len()).unwrap());
            assert_eq!(
                anchor(source, dialect, target),
                Some(expected),
                "{dialect}: {source:?}"
            );
        }
        for (source, target) in [
            ("puts $a", "a(b"),
            ("puts ${literal}", "$literal"),
            ("puts {${a(b}}", "a(b"),
            ("puts ${unclosed", "unclosed"),
            ("puts $a(", "a("),
        ] {
            assert_eq!(
                anchor(source, dialect, target),
                None,
                "{dialect}: {source:?}"
            );
        }
    }
}

#[test]
fn variable_read_anchor_uses_retained_braced_and_bare_name_grammar() {
    // naming.diagnostics.original-variable-name-anchor
    // docs/design/analysis/name-resolution-proofs/diagnostic-original-variable-name-anchor.md
    // Exact source/configuration discrimination, not a native cell or
    // completion observation. Bare Unicode is Jim syntax only.
    let source = "puts ${a{b}c}";
    for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "jim"] {
        assert_eq!(anchor(source, dialect, "a{b}c"), None);
        assert_eq!(
            anchor(source, dialect, "a{b"),
            Some(tcl_lexer::Span::new(5, 11))
        );
    }
    for dialect in ["tcl9.0", "tcl9.1"] {
        assert_eq!(
            anchor(source, dialect, "a{b}c"),
            Some(tcl_lexer::Span::new(5, 13))
        );
        assert_eq!(anchor(source, dialect, "a{b"), None);
    }
    for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        assert_eq!(anchor("puts $é", dialect, "é"), None);
    }
    assert_eq!(
        anchor("puts $é", "jim", "é"),
        Some(tcl_lexer::Span::new(5, 8))
    );
}
