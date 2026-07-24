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

//! `SSL::disable` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "SSL::disable",
        traits: Traits::DIAGRAM_ACTION,
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Disables SSL processing.",
            synopsis: &["SSL::disable (clientside | serverside)?"],
            snippet: "Disables SSL processing. This command is useful when using a virtual server that services both SSL and non-SSL traffic, or when you want to selectively re-encrypt traffic to pool members.\n\nNote: Disabling SSL on the serverside only applies before serverside connection has been established (SERVER_CONNECTED) or when the clientside of the connection is in a detached state (e.g., oneconnect, LB::detach).",
            source: "https://clouddocs.f5.com/api/irules/SSL__disable.html",
            examples: "when SERVER_CONNECTED {\n    if { $usessl == 0 } {\n        SSL::disable\n    }\n}",
            return_value: "SSL::disable [clientside | serverside] Disables SSL processing on one side of the LTM. Sends an SSL alert to the peer requesting termination of SSL processing.",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "SSL::disable (clientside | serverside)?",
            dialects: None,
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::SslState,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::Both,
            dialects: None,
        }],
        ..CommandSpec::DEFAULT
    }
}
