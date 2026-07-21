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

//! `snatpool` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "snatpool",
        traits: Traits::DIAGRAM_ACTION,
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Assigns the specified SNAT pool or SNAT pool member to the current connection.",
            synopsis: &["snatpool SNAT_POOL_OBJ (member IP_ADDR)?"],
            snippet: "Causes the pool of addresses identified by <snatpool_name> to be used\nas translation addresses to create a SNAT.",
            source: "https://clouddocs.f5.com/api/irules/snatpool.html",
            examples: "when CLIENT_ACCEPTED {\n  if { [TCP::local_port] == 531 } {\n     snatpool chat_snatpool\n}\n  elseif { [TCP::local_port] == 25 } {\n     snatpool smtp_snatpool member 10.20.30.40\n }\n}",
            return_value: "",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "snatpool SNAT_POOL_OBJ (member IP_ADDR)?",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::SnatSelection,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::Server,
            dialects: None,
        }],
        ..CommandSpec::DEFAULT
    }
}
