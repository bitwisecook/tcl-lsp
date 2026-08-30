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

//! `SCTP::mss` iRules command.
use crate::prelude::*;
use tcl_dialect::model::SpecSurface;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "SCTP::mss",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns the on-wire Maximum Segment Size (MSS) for an SCTP connection.",
            synopsis: &["SCTP::mss"],
            snippet: "Returns the on-wire Maximum Segment Size (MSS) for an SCTP connection.",
            source: "https://clouddocs.f5.com/api/irules/SCTP__mss.html",
            examples: "when CLIENT_ACCEPTED {\n        SCTP::collect\n        log local0.info \"Sctp local port is [SCTP::local_port]\"\n        log local0.info \"Sctp client port is [SCTP::client_port]\"\n        log local0.info \"Sctp mss is [SCTP::mss]\"\n        log local0.info \"sctp ppi is [SCTP::ppi]\"\n}",
            return_value: "",
        }),
        forms: &[FormSpec {
            synopsis: "SCTP::mss",
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
