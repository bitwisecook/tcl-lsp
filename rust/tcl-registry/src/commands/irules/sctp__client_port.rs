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

//! `SCTP::client_port` iRules command.
use crate::prelude::*;
use tcl_dialect::model::SpecSurface;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "SCTP::client_port",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns the SCTP port/service number of the specified client.",
            synopsis: &["SCTP::client_port"],
            snippet: "Returns the SCTP port/service number of the specified client. This command is equivalent to the command clientside { SCTP::remote_port }.\n\nSCTP::client_port\n    Returns the SCTP port/service number of the specified client.",
            source: "https://clouddocs.f5.com/api/irules/SCTP__client_port.html",
            examples: "when CLIENT_ACCEPTED {\n    if { [SCTP::client_port] > 1000 } {\n        pool slow_pool\n     }\n      else {\n         pool fast_pool\n       }\n}",
            return_value: "",
        }),
        forms: &[FormSpec {
            synopsis: "SCTP::client_port",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::TcpState,
            reads: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}
