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
fn original_per_item_recovery_retains_the_whole_file_bom_boundary() {
    // naming.source.original-configured-script-completeness
    // docs/design/analysis/name-resolution-proofs/source-original-configured-script-completeness.md
    // The shell's original command stream, without execution admission.
    let source = "\u{feff}set x 1\nputs after\n";
    let mut prepared = Analyser::new();
    let input = prepared.prepare_analysis_input("tcl9.1").unwrap();
    let mut default = Analyser::new();
    let commands = default.per_item_setup(source, "tcl9.1");
    assert_eq!(commands[0].name(), "set");
    assert_eq!(default.result.resolved_input.as_ref(), Some(&input));
    assert_eq!(commands[0].argv[0].span.start(), 3);
    for bom in [tcl_lexer::LeadingBom::Skip, tcl_lexer::LeadingBom::Content] {
        let mut supplied = input.clone();
        supplied.config.leading_bom = bom;
        supplied.config.braced_var = tcl_lexer::BracedVarStyle::FirstClose;
        let mut selected = Analyser::new().with_resolved_input(supplied.clone());
        let commands = selected.per_item_setup(source, "display-only");
        assert_eq!(
            commands[0].name(),
            if bom == tcl_lexer::LeadingBom::Skip {
                "set"
            } else {
                "\u{feff}set"
            }
        );
        assert_eq!(selected.result.resolved_input.as_ref(), Some(&supplied));
        assert_eq!(
            selected.result.body_lexer_config,
            Some(supplied.lexer_config())
        );
        assert!(selected.result.matches_original_source_image(
            &tcl_lexer::SourceImage::document(source),
            supplied.lexer_config()
        ));
    }
}
