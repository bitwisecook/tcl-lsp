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

//! `reject` iRules command.
use crate::prelude::*;
use tcl_dialect::model::SpecSurface;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "reject",
        traits: Traits::DIAGRAM_ACTION,
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Causes the connection to be rejected.",
            synopsis: &["reject"],
            snippet: "Causes the connection to be rejected, returning a reset as appropriate\nfor the protocol. Subsequent code in the current event in the current\niRule or other iRules on the VS are still executed prior to the reset\nbeing sent.\n\n**Warning**: After `reject`, the current iRule continues executing,\nand other iRules on the VS also run. This can cause TCL errors\n(e.g. `ASM::disable` on a rejected connection). Always follow\n`reject` with `event disable all` and `return`.\n\nIf the VS is using FastHTTP, reject commands will not work, at least\nunder 11.3.0.",
            source: "https://clouddocs.f5.com/api/irules/reject.html",
            examples: "when CLIENT_ACCEPTED {\n  if { [TCP::local_port] != 443 } {\n    reject\n    event disable all\n    return\n  }\n}",
            return_value: "",
        }),
        forms: &[FormSpec {
            synopsis: "reject",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::ConnectionControl,
            writes: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}
