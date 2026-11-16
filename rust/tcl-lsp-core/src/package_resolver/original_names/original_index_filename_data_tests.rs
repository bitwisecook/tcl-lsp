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
fn original_index_filename_candidates_preserve_oracle_literal_data_and_join_components() {
    const ORACLES: [&str; 6] = [
        include_str!("../../../tests/data/original_index_filenames/8.4.20.stdout"),
        include_str!("../../../tests/data/original_index_filenames/8.5.19.stdout"),
        include_str!("../../../tests/data/original_index_filenames/8.6.18.stdout"),
        include_str!("../../../tests/data/original_index_filenames/9.0.4.stdout"),
        include_str!("../../../tests/data/original_index_filenames/9.1.0.stdout"),
        include_str!("../../../tests/data/original_index_filenames/jim.stdout"),
    ];
    for output in ORACLES {
        for (label, result) in [
            ("spaces", "737061636573"),
            ("quotes", "71756f746573"),
            ("sigil", "736967696c"),
            ("dash", "64617368"),
            ("empty_component", "656d7074795f636f6d706f6e656e74"),
            ("nested_quotes", "6e65737465645f71756f746573"),
        ] {
            assert!(
                output
                    .lines()
                    .any(|line| line == format!("RESULT|{label}|0|{result}"))
            );
        }
    }
    for version in tcl_dialect::TclVersion::ALL {
        let config = LexerConfig::from_grammar(
            tcl_registry::InvocationDialect::for_version(version).lexer_grammar,
        );
        for (loader, filename) in [
            (
                r"[list source [file join $dir { spaced.tcl }]]",
                " spaced.tcl ",
            ),
            (
                r#"[list source [file join $dir {"quoted".tcl}]]"#,
                "\"quoted\".tcl",
            ),
            (
                r"[list source [file join $dir {cash$name.tcl}]]",
                "cash$name.tcl",
            ),
            (
                r"[list source [file join $dir -literal.tcl]]",
                "-literal.tcl",
            ),
            (r"[list source [file join $dir {} empty.tcl]]", "empty.tcl"),
            (
                r#"[list source [file join $dir {a b} {part".tcl}]]"#,
                "a b/part\".tcl",
            ),
            (r"{source { spaced.tcl }}", " spaced.tcl "),
            (r#"{source {"quoted".tcl}}"#, "\"quoted\".tcl"),
            (r"{source a\ b.tcl}", "a b.tcl"),
            (r#"{source "$dir/ spaced.tcl "}"#, " spaced.tcl "),
            (r#"[list source "$dir/\"quoted\".tcl"]"#, "\"quoted\".tcl"),
            (r"{source {$literal.tcl}}", "$literal.tcl"),
        ] {
            let source = format!("set {{auto_index( spaced key )}} {loader}");
            let expected = Path::new("/library").join(filename);
            let entries = original_index_entries(
                &source,
                Path::new("/library"),
                &|path| path == expected,
                config,
                NamePolicyProtocol::authored_tcl(version),
            );
            assert_eq!(entries.len(), 1, "{version:?}: {source}");
            assert_eq!(entries[0].file, expected);
            assert_eq!(entries[0].input.bytes(), b" spaced key ");
        }
    }
}

#[test]
fn original_index_source_operand_grammar_and_real_producers_refuse_guessed_filenames() {
    for version in tcl_dialect::TclVersion::ALL {
        let config = LexerConfig::from_grammar(
            tcl_registry::InvocationDialect::for_version(version).lexer_grammar,
        );
        for (option, accepted) in [
            ("-encoding utf-8", version != tcl_dialect::TclVersion::V8_4),
            (
                "-nopkg",
                matches!(
                    version,
                    tcl_dialect::TclVersion::V9_0 | tcl_dialect::TclVersion::V9_1
                ),
            ),
            ("-encoding utf-8 -nopkg", false),
        ] {
            let source = format!(
                "set auto_index(p) [list source {option} [file join $dir {{ spaced.tcl }}]]"
            );
            let entries = original_index_entries(
                &source,
                Path::new("/library"),
                &|path| path == Path::new("/library/ spaced.tcl "),
                config,
                NamePolicyProtocol::authored_tcl(version),
            );
            assert_eq!(
                entries.len(),
                usize::from(accepted),
                "{version:?}: {source}"
            );
        }
        for source in [
            "set auto_index(p) [list unknown $dir/p.tcl]",
            "set auto_index(p) [list source $unknown]",
            "set auto_index(p) [list source [file join $dir $unknown]]",
            "set auto_index(p) [list source [file join $dir p.tcl] EXTRA]",
            "set auto_index(p) [list source {*}$unknown]",
            "proc list args {return OTHER}; set auto_index(p) [list source [file join $dir p.tcl]]",
            "proc file args {return OTHER}; set auto_index(p) [list source [file join $dir p.tcl]]",
            "interp alias {} joiner {} file join; set auto_index(p) [list source [joiner $dir p.tcl]]",
            "interp alias {} builder {} list source; set auto_index(p) [builder [file join $dir p.tcl]]",
        ] {
            assert!(
                original_index_entries(
                    source,
                    Path::new("/library"),
                    &|_| true,
                    config,
                    NamePolicyProtocol::authored_tcl(version),
                )
                .is_empty(),
                "{version:?}: {source}"
            );
        }
    }
}
