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

//! `drop` iRules command.
use crate::prelude::*;
use tcl_dialect::model::SpecSurface;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "drop",
        traits: Traits::DIAGRAM_ACTION,
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Causes the current packet or connection to be dropped/discarded.",
            synopsis: &["drop"],
            snippet: "Causes the current packet or connection (depending on the context of\nthe event) to be dropped/discarded and the rule continues (no implied\nreturn). This command is identical to discard.\n\n**Warning**: After `drop`, the current iRule continues executing, and\nother iRules and later priorities in this event also run. This can\ncause TCL errors (e.g. `ASM::disable` on a dropped connection).\nAlways follow `drop` with `event disable all` and `return`.",
            source: "https://clouddocs.f5.com/api/irules/drop.html",
            examples: "when HTTP_REQUEST {\n  if { [IP::addr [IP::client_addr] equals 10.1.1.80] } {\n    drop\n    event disable all\n    return\n  }\n}",
            return_value: "",
        }),
        forms: &[FormSpec {
            synopsis: "drop",
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
