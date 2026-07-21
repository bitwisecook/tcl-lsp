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

//! `GENERICMESSAGE::route` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "GENERICMESSAGE::route",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Adds, deletes, or looks up message routes.",
            synopsis: &[
                "GENERICMESSAGE::route (add | delete | lookup) ((('virtual' VIRTUAL_SERVER_OBJ)",
            ],
            snippet: "The GENERICMESSAGE::route command allows you to add, delete, or lookup\nmessage routes.",
            source: "https://clouddocs.f5.com/api/irules/GENERICMESSAGE__route.html",
            examples: "when CLIENT_ACCEPTED {\n    GENERICMESSAGE::route add dst \"client-[IP::remote_addr]\" host \"[IP::remote_addr]:[TCP::remote_port]\"\n}",
            return_value: "",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "GENERICMESSAGE::route (add | delete | lookup) ((('virtual' VIRTUAL_SERVER_OBJ)",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::MessageState,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::Both,
            dialects: None,
        }],
        ..CommandSpec::DEFAULT
    }
}
