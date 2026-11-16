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
fn original_interpolation_literals_keep_bom_prefix_and_rendered_line_breaks() {
    // naming.source.original-configured-script-completeness
    // docs/design/analysis/name-resolution-proofs/source-original-configured-script-completeness.md
    // Authored interpolation syntax; no value/effect or Native observer receipt.
    let config = tcl_lexer::LexerConfig {
        leading_bom: tcl_lexer::LeadingBom::Skip,
        braced_var: tcl_lexer::BracedVarStyle::FirstClose,
        strict_quoting: true,
        ..tcl_lexer::LexerConfig::default()
    };
    let text = "\u{feff}#prefix\\n$x";
    assert_eq!(leading_literal_prefix_char(text, config), Some('\u{feff}'));
    assert!(literal_contains_crlf(text, config));
    assert_eq!(
        leading_literal_prefix_char("\u{feff}/$x", config),
        Some('\u{feff}')
    );
    assert!(!literal_contains_crlf("{\u{feff}#prefix\\n$x}", config));
    let joined = TaintLattice::tainted().with(TaintColour::CRLF_FREE | TaintColour::PATH_PREFIXED);
    let colour = interpolation_carve_out(text, joined, config);
    assert!(
        colour
            .colours
            .contains(TaintColour::TAINTED | TaintColour::NON_DASH_PREFIXED)
    );
    assert!(
        !colour
            .colours
            .intersects(TaintColour::CRLF_FREE | TaintColour::PATH_PREFIXED)
    );
    assert_eq!(
        colour,
        interpolation_carve_out(text, joined, config.nested())
    );
    assert_eq!(leading_literal_prefix_char("${a{b}", config), None);
    assert_eq!(config.leading_bom, tcl_lexer::LeadingBom::Skip);
}
