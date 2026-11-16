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

use crate::analyser::Analyser;

#[test]
fn original_conditional_variable_reads_keep_distinct_absolute_opaque_operands() {
    // Implementation contract: naming.variable.conditional-registry-receiver-geometry
    // docs/design/analysis/name-resolution-proofs/conditional-registry-receiver-geometry.md
    let source = r"namespace eval n\uD800 {}
namespace exists n\uD800
namespace eval N {}
set ::N::v\uD800 1
set ::N::v\uD801 2
info exists ::N::v\uD800
info exists ::N::v\uD801";
    for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jimtcl"] {
        let mut analysis = Analyser::new().analyse(source, profile);
        let image = tcl_lexer::SourceImage::document(source);
        let config = analysis.body_lexer_config.unwrap();
        analysis.global_scope.variables.clear();
        analysis.command_invocations.clear();
        let mut symbols = Vec::new();
        for (spelling, bytes) in [
            (r"::N::v\uD800", b"::N::v\xed\xa0\x80".as_slice()),
            (r"::N::v\uD801", b"::N::v\xed\xa0\x81".as_slice()),
        ] {
            let offset = u32::try_from(source.rfind(spelling).unwrap()).unwrap();
            let reference = analysis
                .original_variable_symbol_in_source(&image, config, offset)
                .unwrap_or_else(|| panic!("{profile}: original info-exists receiver {spelling}"));
            assert!(!reference.is_declaration());
            assert_eq!(reference.original_name_input().bytes(), bytes);
            let declaration = analysis
                .original_variable_symbols
                .iter()
                .find(|row| row.is_declaration() && row.symbol() == reference.symbol())
                .unwrap_or_else(|| panic!("{profile}: corresponding opaque declaration"));
            assert_eq!(declaration.original_name_input().bytes(), bytes);
            symbols.push(reference.symbol().clone());
            assert!(
                analysis
                    .original_variable_symbol_in_source(
                        &tcl_lexer::SourceImage::document(
                            &source.replace("namespace eval N", "namespace eval M")
                        ),
                        config,
                        offset,
                    )
                    .is_none(),
                "{profile}: edited owner source is unavailable"
            );
        }
        assert_ne!(
            symbols[0], symbols[1],
            "{profile}: opaque keys stay distinct"
        );
    }
}

#[test]
fn original_conditional_variable_reads_preserve_shadow_and_unknown_scope_barriers() {
    // #2427: a bare undefined command no longer widens the command table; an autoload index entry in view keeps this device unbounded.
    // Implementation contract: naming.variable.conditional-registry-receiver-geometry
    // docs/design/analysis/name-resolution-proofs/conditional-registry-receiver-geometry.md
    for profile in ["tcl8.6", "tcl9.0", "jimtcl"] {
        for (source, operand) in [
            (
                "proc info {args} {return CUSTOM}; info exists ::target",
                "::target",
            ),
            (
                "set auto_index(unknown_command) {source lib.tcl}; unknown_command; namespace eval N {info exists bare}",
                "bare",
            ),
        ] {
            let analysis = Analyser::new().analyse(source, profile);
            let image = tcl_lexer::SourceImage::document(source);
            let config = analysis.body_lexer_config.unwrap();
            let offset = u32::try_from(source.rfind(operand).unwrap()).unwrap();
            assert!(
                analysis
                    .original_variable_symbol_in_source(&image, config, offset)
                    .is_none(),
                "{profile}: conditional advice cannot select {operand}"
            );
        }
    }
}
