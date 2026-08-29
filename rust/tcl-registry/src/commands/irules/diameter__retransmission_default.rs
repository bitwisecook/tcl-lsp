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

//! `DIAMETER::retransmission_default` iRules command.
use crate::prelude::*;
use tcl_dialect::model::{SpecSurface};
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "DIAMETER::retransmission_default",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Gets of sets the current connection's retransmission settings.",
            synopsis: &["DIAMETER::retransmission_default action"],
            snippet: "This command allows the setting or getting of the current\nconnection\\'s retransmission settings. All request messages on the\ncurrent connection will be initailized with the connection\\'s setings.\nThe messages\\'s settings may be changed with the\nDIAMETER::retransmission command.\n        \nGets the current connection\\'s retransmission action.\nPossible actions are:\n\n * \"disabled\" - request messages will not be queued for retransmission\n\n * \"busy\" - when retransmission is triggered for a request message an\n   answer message with a DIAMETER_TOO_BUSY result code will be\n   returned to the originator.",
            source: "https://clouddocs.f5.com/api/irules/DIAMETER__retransmission_default.html",
            examples: "when CLIENT_ACCEPTED {\n    DIAMETER::retransmission_default action busy\n}",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &[],
            also_in: &["CLIENT_ACCEPTED", "SERVER_CONNECTED"],
            flow: false,
        }),
        forms: &[FormSpec {
            synopsis: "DIAMETER::retransmission_default action",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::NetworkIo,
            reads: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}
