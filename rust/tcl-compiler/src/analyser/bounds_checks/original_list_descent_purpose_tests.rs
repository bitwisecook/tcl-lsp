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
fn original_list_descent_keeps_file_bom_and_extracted_body_marks_distinct() {
    // naming.source.original-configured-script-completeness
    // docs/design/analysis/name-resolution-proofs/source-original-configured-script-completeness.md
    // This lexical list-length projection grants no getter, store or frame.
    let profile = tcl_dialect::DialectProfile::find("tcl9.0").unwrap();
    let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
    let context = BoundsMetadataContext::Standalone(None);
    for (source, expected) in [
        ("\u{feff}set xs {a b}; lset xs 9 v", Some(2)),
        ("if 1 {\u{feff}set xs {a b}; lset xs 9 v}", None),
        ("if 1 {set xs {a b}; lset xs 9 v}", Some(2)),
    ] {
        let before = u32::try_from(source.find("lset").unwrap()).unwrap();
        assert_eq!(
            infer_list_length_from_recent_set(source, "xs", before, context, config),
            expected,
            "{source:?}"
        );
    }
    let source = "if 1 {puts ${a{b}; set xs {a b}; lset xs 9 v}}";
    let before = u32::try_from(source.find("lset").unwrap()).unwrap();
    let mut first_close = config;
    first_close.braced_var = tcl_lexer::BracedVarStyle::FirstClose;
    assert_eq!(
        infer_list_length_from_recent_set(source, "xs", before, context, first_close),
        Some(2)
    );
    assert_eq!(
        infer_list_length_from_recent_set(source, "xs", before, context, config),
        None
    );
}
