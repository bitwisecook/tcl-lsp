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

//! `LINK::lasthop` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "LINK::lasthop",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns the MAC address of the last hop.",
            synopsis: &["LINK::lasthop ('id' | 'type' | 'name')?"],
            snippet: "Returns the MAC address of the last hop.\nNote:\n  * In 11.4, you can extend LINK::lasthop with sub-commands to retrieve\n    the lasthop id, type, name, respectively. Without sub-command,\n    LINK::lasthop returns the MAC address as before.",
            source: "https://clouddocs.f5.com/api/irules/LINK__lasthop.html",
            examples: "when CLIENT_ACCEPTED {\n  set lastmac [LINK::lasthop]\n  session add uie [IP::client_addr] $lastmac 180\n}",
            return_value: "LINK::lasthop [id]",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "LINK::lasthop ('id' | 'type' | 'name')?",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::TcpState,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Both,
            dialects: None,
        }],
        ..CommandSpec::DEFAULT
    }
}
