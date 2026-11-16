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
fn original_body_reference_suppression_keeps_bom_in_the_selected_head() {
    // naming.source.original-configured-script-completeness
    // docs/design/analysis/name-resolution-proofs/source-original-configured-script-completeness.md
    // Unknown literal heads cannot grant a script role to inert argument data.
    let config = tcl_lexer::LexerConfig::for_file_grammar(
        crate::environment_ingress::resolve_environment("tcl9.1").grammar(),
    );
    assert!(!body_references_param(
        "namespace eval NS {puts $parameter}",
        "parameter",
        config
    ));
    assert!(!body_references_param(
        "\u{feff}namespace eval NS {puts $parameter}",
        "parameter",
        config
    ));
    assert!(body_references_param(
        "puts $parameter",
        "parameter",
        config
    ));
    assert!(!body_references_param("puts $another", "parameter", config));
}
