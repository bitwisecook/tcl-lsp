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

//! `client_port` iRules command.
use crate::prelude::*;
use tcl_dialect::model::SpecSurface;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "client_port",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns the TCP port number/service of the specified client.",
            synopsis: &["client_port"],
            snippet: "Returns the TCP port number/service of the specified client. This is a BIG-IP version 4.X variable, provided for backward compatibility. You can use the equivalent 9.X command, TCP::client_port instead.",
            source: "https://clouddocs.f5.com/api/irules/client_port.html",
            examples: "",
            return_value: "client_port Returns the TCP port number/service of the specified client.",
        }),
        forms: &[FormSpec {
            synopsis: "client_port",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::TcpState,
            reads: true,
            connection_side: ConnectionSide::Client,
            ..SideEffect::DEFAULT
        }],
        deprecated_replacement: Some("TCP::client_port"),
        deprecated_replacement_drop_in: true,
        ..CommandSpec::DEFAULT
    }
}
