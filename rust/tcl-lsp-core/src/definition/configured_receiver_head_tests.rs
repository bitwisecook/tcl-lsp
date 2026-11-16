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
use tcl_compiler::analyser::Analyser;

#[test]
fn configured_method_heads_require_the_whole_variable_operand() {
    // naming.core.original-source-instance-completion
    // docs/design/analysis/name-resolution-proofs/core-original-source-instance-completion.md
    let method = |head: &str, config| {
        let source = format!("{head} longMethod\n");
        let character = u32::try_from(
            source[..source.find("longMethod").unwrap()]
                .encode_utf16()
                .count(),
        )
        .unwrap();
        instance_method_at_cursor(&source, 0, character, config)
    };
    let first = tcl_lexer::LexerConfig {
        braced_var: tcl_dialect::BracedVarStyle::FirstClose,
        ..tcl_lexer::LexerConfig::default()
    };
    let mut nested = first;
    nested.braced_var = tcl_dialect::BracedVarStyle::Tcl9Nesting;
    for config in [first, nested] {
        assert_eq!(
            method("${箱}", config),
            Some(("箱".to_owned(), "longMethod".to_owned(), true))
        );
        assert_eq!(
            method("$arr(k)", config),
            Some(("arr(k)".to_owned(), "longMethod".to_owned(), true))
        );
        for head in ["${x}suffix", "$x-suffix", "${x}[suffix]", "${x", "$箱"] {
            assert!(method(head, config).is_none(), "{head}");
        }
    }
    assert_eq!(
        method("${a{b}c}", nested),
        Some(("a{b}c".to_owned(), "longMethod".to_owned(), true))
    );
    assert!(method("${a{b}c}", first).is_none());
    assert_eq!(
        method("${a{b}", first),
        Some(("a{b".to_owned(), "longMethod".to_owned(), true))
    );
    assert!(method("${a{b}", nested).is_none());
    assert_eq!(
        method(r"${a\}b}", nested),
        Some((r"a\}b".to_owned(), "longMethod".to_owned(), true)),
    );
    assert!(method(r"${a\}b}", first).is_none());
}

#[test]
fn actual_method_navigation_does_not_promote_compound_variable_syntax() {
    // naming.core.original-source-instance-completion
    // docs/design/analysis/name-resolution-proofs/core-original-source-instance-completion.md
    for dialect in ["tcl8.6", "tcl9.1"] {
        for (head, present) in [
            ("${箱}", true),
            ("${箱}suffix", false),
            ("${箱}[list suffix]", false),
        ] {
            let source = format!(
                "oo::class create C {{method longMethod {{}} {{}}}}\nset {{箱}} [C new]\n{head} longMethod\n"
            );
            let analysis = Analyser::new().analyse(&source, dialect);
            let line = source.lines().nth(2).unwrap();
            let character = u32::try_from(
                line[..line.find("longMethod").unwrap()]
                    .encode_utf16()
                    .count(),
            )
            .unwrap();
            let locations = definition(&source, 2, character, &analysis);
            assert_eq!(
                !locations.is_empty(),
                present,
                "{dialect}: {head}: {locations:?}"
            );
            if present {
                assert!(locations.iter().all(|location| location.start_line == 0));
            }
        }
    }
}
