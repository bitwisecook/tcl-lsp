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

//! `discard` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "discard",
        traits: Traits::DIAGRAM_ACTION,
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Causes the current packet or connection to be dropped/discarded.",
            synopsis: &["discard"],
            snippet: "Causes the current packet or connection (depending on the context of\nthe event) to be dropped/discarded and the rule continues (no implied\nreturn). This command is identical to drop.\n\n**Warning**: After `discard`, the current iRule continues executing,\nand other iRules and later priorities in this event also run. This\ncan cause TCL errors. Always follow `discard` with `event disable\nall` and `return`.",
            source: "https://clouddocs.f5.com/api/irules/discard.html",
            examples: "when HTTP_REQUEST {\n  if { [IP::addr [IP::client_addr] equals 10.1.1.80] } {\n    discard\n    event disable all\n    return\n  }\n}",
            return_value: "",
        }),
        forms: &[FormSpec {
            synopsis: "discard",
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
