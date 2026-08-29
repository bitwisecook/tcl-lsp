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

//! `DIAMETER::disconnect` iRules command.
use crate::prelude::*;
use tcl_dialect::model::SpecSurface;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "DIAMETER::disconnect",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Sends Disconnect-Peer-Request to client or server based on context.",
            synopsis: &["DIAMETER::disconnect ORIGIN_HOST ORIGIN_REALM DIAMETER_DISCONNECT_CAUSE"],
            snippet: "This iRule command sends Disconnect-Peer-Request to the client (if run\non clientside) or to the server (if run on serverside).",
            source: "https://clouddocs.f5.com/api/irules/DIAMETER__disconnect.html",
            examples: "when DIAMETER_INGRESS {\n    # 2 = DO_NOT_WANT_TO_TALK_TO_YOU (RFC 6733 sec 5.4.3)\n    DIAMETER::disconnect \"bigip.core.example.com\" \"example.com\" 2\n}",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["DIAMETER", "MR"],
            also_in: &[],
            flow: false,
        }),
        forms: &[FormSpec {
            synopsis: "DIAMETER::disconnect ORIGIN_HOST ORIGIN_REALM DIAMETER_DISCONNECT_CAUSE",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::ConnectionControl,
            writes: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}
