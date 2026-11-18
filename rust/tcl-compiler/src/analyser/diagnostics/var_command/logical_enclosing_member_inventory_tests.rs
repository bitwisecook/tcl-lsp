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
fn logical_receiver_inventory_uses_actual_frame_and_refuses_unknown_members() {
    // tclsh8.6.18/9.0.4: `my knwon` -> unknown method "knwon" (known is one
    // of the choices). With `mystery;` first, the definition script raises
    // invalid command name "mystery", C does not exist, and there is no
    // member inventory to propose from.
    for (effect, expected) in [("", true), ("mystery;", false)] {
        let source = format!(
            "oo::class create C {{{effect} method known {{}} {{}}; method body {{}} {{my knwon}}}}"
        );
        let result = Analyser::new().analyse(&source, "tcl");
        let proposes_known = result.diagnostics.iter().any(|d| {
            d.code == DiagCode::W308
                && d.message.contains("knwon")
                && d.fixes.iter().any(|fix| fix.new_text == "known")
        });
        assert_eq!(
            proposes_known, expected,
            "{source}: {:?}",
            result.diagnostics
        );
    }
}
