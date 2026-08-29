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

//! `IP::version` iRules command.
use crate::prelude::*;
use tcl_dialect::model::{SpecSurface};
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "IP::version",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns the IP version of a connection.",
            synopsis: &["IP::version"],
            snippet: "Returns the IP version of a connection. When called in a clientside event, this command returns the IP version for the clientside connection. When called in a serverside event, this command returns the IP version for the serverside connection.",
            source: "https://clouddocs.f5.com/api/irules/IP__version.html",
            examples: "when CLIENT_ACCEPTED {\n   log local0. \"Client [IP::client_addr], VS: [IP::local_addr],\\\n      \\[IP::version\\]: [IP::version], \\[IP::protocol\\]: [IP::protocol]\"\n}",
            return_value: "IP version of a connection",
        }),
        forms: &[FormSpec {
            synopsis: "IP::version",
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
