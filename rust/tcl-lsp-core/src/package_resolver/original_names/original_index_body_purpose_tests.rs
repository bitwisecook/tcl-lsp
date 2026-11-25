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
fn original_auto_index_stored_script_keeps_body_bom_distinct_from_file_prologue() {
    // naming.package.original-reachability-source-controls
    // docs/design/analysis/name-resolution-proofs/original-package-reachability-source-controls.md
    // Stored loader syntax and original name identity; no loader execution grant.
    let version = tcl_dialect::TclVersion::V9_1;
    let profile = tcl_dialect::DialectProfile::find(version.dialect_profile_name()).unwrap();
    let policy = NamePolicyProtocol::authored_tcl(version);
    for bom in [tcl_lexer::LeadingBom::Skip, tcl_lexer::LeadingBom::Content] {
        let config = LexerConfig {
            leading_bom: bom,
            braced_var: tcl_lexer::BracedVarStyle::FirstClose,
            ..LexerConfig::for_file_grammar(profile.grammar)
        };
        for (source, expected) in [
            ("set auto_index(p) {source $dir/p.tcl}", true),
            ("set auto_index(p) {\u{feff}source $dir/p.tcl}", false),
            (
                "\u{feff}set auto_index(p) {source $dir/p.tcl}",
                bom == tcl_lexer::LeadingBom::Skip,
            ),
        ] {
            let entries =
                original_index_entries(source, Path::new("/library"), &|_| true, config, policy);
            assert_eq!(!entries.is_empty(), expected, "{bom:?}: {source:?}");
            if let Some(entry) = entries.first() {
                assert_eq!(entry.input.bytes(), b"p");
                assert_eq!(entry.file, Path::new("/library/p.tcl"));
            }
            assert_eq!(config.braced_var, tcl_lexer::BracedVarStyle::FirstClose);
        }
    }
}
