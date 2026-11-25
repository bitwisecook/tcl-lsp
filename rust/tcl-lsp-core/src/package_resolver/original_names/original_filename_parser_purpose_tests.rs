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
fn original_auto_index_filename_keeps_supplied_grammar_and_nested_command_bom() {
    // naming.package.original-reachability-source-controls
    // docs/design/analysis/name-resolution-proofs/original-package-reachability-source-controls.md
    // Loader filename syntax and counted original name only; no loading or variable value claim.
    let version = tcl_dialect::TclVersion::V9_1;
    let profile = tcl_dialect::DialectProfile::find(version.dialect_profile_name()).unwrap();
    let policy = NamePolicyProtocol::authored_tcl(version);
    let config = LexerConfig {
        leading_bom: tcl_lexer::LeadingBom::Skip,
        braced_var: tcl_lexer::BracedVarStyle::FirstClose,
        strict_quoting: true,
        ..LexerConfig::for_file_grammar(profile.grammar)
    };
    for (source, expected) in [
        (
            "\u{feff}set auto_index(p) {source [file join $dir p.tcl]}",
            true,
        ),
        (
            "\u{feff}set auto_index(p) {source [\u{feff}file join $dir p.tcl]}",
            false,
        ),
    ] {
        let entries =
            original_index_entries(source, Path::new("/library"), &|_| true, config, policy);
        assert_eq!(entries.len(), usize::from(expected), "{source}");
        if let Some(entry) = entries.first() {
            assert_eq!(entry.input.bytes(), b"p");
            assert_eq!(entry.file, Path::new("/library/p.tcl"));
        }
    }
    // This projection records authored path syntax; it does not evaluate this variable.
    let operand = "[file join $dir ${a{b} p.tcl]";
    let words = super::super::walk_command_words_with_config(operand, config);
    assert_eq!(words.len(), 1);
    assert_eq!(words[0].len(), 1);
    assert!(super::super::source_filename_with_config(operand, &words[0][0], config).is_some());
    let changed = LexerConfig {
        braced_var: tcl_lexer::BracedVarStyle::Tcl9Nesting,
        ..config
    };
    assert!(super::super::source_filename_with_config(operand, &words[0][0], changed).is_none());
}
