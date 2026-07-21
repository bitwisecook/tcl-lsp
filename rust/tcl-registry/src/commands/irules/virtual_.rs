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

//! `virtual` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "virtual",
        traits: Traits::CSE_CANDIDATE.union(Traits::DIAGRAM_ACTION),
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns the name of the associated virtual server or selects another virtual server and an optional IP address and port to connect to.",
            synopsis: &[
                "virtual",
                "virtual (name | VIRTUAL_SERVER_OBJ) (IP_TUPLE | (IP_ADDR (PORT)?))?",
            ],
            snippet: "Returns the name of the associated virtual server that the connection\nis flowing through. In 9.4.0 and higher, it can be also used to route\nthe connection to another virtual server and an optional IP address\nand port, without leaving the BIG-IP.",
            source: "https://clouddocs.f5.com/api/irules/virtual.html",
            examples: "when HTTP_REQUEST {\n  log local0. \"Current virtual server name: [virtual name]\"\n}",
            return_value: "",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "virtual",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::ConnectionControl,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::Both,
            dialects: None,
        }],
        ..CommandSpec::DEFAULT
    }
}
