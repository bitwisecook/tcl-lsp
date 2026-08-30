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

//! `local_addr` iRules command.
use crate::prelude::*;
use tcl_dialect::model::SpecSurface;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "local_addr",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Deprecated: Use IP::local_addr instead.",
            synopsis: &["local_addr"],
            snippet: "Deprecated: Use IP::local_addr instead",
            source: "https://clouddocs.f5.com/api/irules/local_addr.html",
            examples: "when CLIENT_ACCEPTED {\n    log local0. \"client ip addr: [local_addr]\"\n}",
            return_value: "Returns the IP address being used in the connection. In the clientside context, this is the destination IP address from the client request (not necessarily the virtual IP address).",
        }),
        forms: &[FormSpec {
            synopsis: "local_addr",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::TcpState,
            reads: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        deprecated_replacement: Some("IP::local_addr"),
        deprecated_replacement_drop_in: true,
        ..CommandSpec::DEFAULT
    }
}
