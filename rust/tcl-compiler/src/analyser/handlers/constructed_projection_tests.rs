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
fn computed_creation_placeholder_preserves_constructed_procedure_holder() {
    // Implementation contract: naming.analysis.constructed-key-consumer-projections
    // docs/design/analysis/name-resolution-proofs/constructed-key-consumer-projections.md
    // Only a metadata placeholder is constructed; no creation is executed.
    for (procedure, expected) in [
        ("p", "::${ns}::C"),
        ("::a::p", "::a::${ns}::C"),
        ("::a::::p", "::a::${ns}::C"),
        ("::::::", ":::::${ns}::C"),
        ("::é::λ", "::é::${ns}::C"),
    ] {
        let mut candidate = ParameterisedCreation {
            proc_qname: procedure.to_owned(),
            params: Vec::new(),
            name_word: "${ns}::C".to_owned(),
            call_off: 0,
            control_path: Vec::new(),
        };
        assert_eq!(
            Analyser::unresolved_class_name(&candidate).as_deref(),
            Some(expected)
        );
        candidate.name_word = "C".to_owned();
        assert!(Analyser::unresolved_class_name(&candidate).is_none());
    }
}
