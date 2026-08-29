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

//! `TCP::release` iRules command.
use crate::prelude::*;
use tcl_dialect::model::{SpecSurface};
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "TCP::release",
        traits: Traits::DIAGRAM_ACTION,
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        data_collection: Some(TCP_RELEASE),
        hover: Some(HoverSnippet {
            summary: "Release data gathered by TCP::collect to the upper layer.",
            synopsis: &["TCP::release (LENGTH)?"],
            snippet: "Causes TCP to release and flush collected data, and allow other\nprotocol layers to resume processing the connection.\n\nReturns the number of bytes actually released. If specified, up to length bytes are released; the return value will tell you how many bytes actually were.",
            source: "https://clouddocs.f5.com/api/irules/TCP__release.html",
            examples: "when CLIENT_ACCEPTED {\n  TCP::collect 15\n}",
            return_value: "The number of bytes released.",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: Some("tcp"),
            profiles: &[],
            also_in: &["SIP_REQUEST", "SIP_REQUEST_SEND", "SIP_RESPONSE"],
            flow: false,
        }),
        forms: &[FormSpec {
            synopsis: "TCP::release (LENGTH)?",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::TcpState,
            writes: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}
