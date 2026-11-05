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

//! Original substitution reads retain external cell effects.

use tcl_registry::side_effects::SideEffectTarget;
use tcl_spec_studio::infer::infer_from_body;

#[test]
fn global_substitution_and_command_reads_have_the_same_external_effect() {
    for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        for body in [
            "return $::seen",
            "return [set ::seen]",
            "global seen; return $seen",
        ] {
            let inferred = infer_from_body("", body, dialect);
            assert!(!inferred.pure, "{dialect}: {body}: {inferred:?}");
            assert!(
                inferred
                    .side_effects
                    .iter()
                    .any(|effect| effect.target == SideEffectTarget::Variable
                        && effect.reads
                        && !effect.writes),
                "{dialect}: {body}: {inferred:?}"
            );
        }
    }
}

#[test]
fn private_parameter_reads_and_literal_sigils_are_not_external_cell_effects() {
    for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        for (params, body) in [("seen", "return $seen"), ("", "return {$::seen}")] {
            let inferred = infer_from_body(params, body, dialect);
            assert!(inferred.pure, "{dialect}: {body}: {inferred:?}");
            assert!(
                !inferred
                    .side_effects
                    .iter()
                    .any(|effect| effect.target == SideEffectTarget::Variable && effect.reads),
                "{dialect}: {body}: {inferred:?}"
            );
        }
    }
}
