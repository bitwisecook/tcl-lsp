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

//! `DIAMETER::retransmission` iRules command.
use crate::prelude::*;
use tcl_dialect::model::SpecSurface;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "DIAMETER::retransmission",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Gets of sets the current message's retransmission settings.",
            synopsis: &["DIAMETER::retransmission action"],
            snippet: "This command allows the setting or getting of the current message\\'s\nretransmission settings.\n        \nGets the current message\\'s retransmission action.\nPossible actions are:\n\n* \"disabled\" - this request messages will not be queued for\n  retransmission\n\n* \"busy\" - when retransmission is triggered for this request message\n  an answer message with a 'busy' result code will be returned to the\n  originator.\n\n* \"unable\" - when retransmission is triggered for this request message\n  an answer message with a 'unable to deliver' result code will be\n  returned to the originator.",
            source: "https://clouddocs.f5.com/api/irules/DIAMETER__retransmission.html",
            examples: "when DIAMETER_INGRESS {\n    DIAMETER::retransmission action retransmit\n}",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["DIAMETER"],
            also_in: &[],
            flow: false,
        }),
        forms: &[FormSpec {
            synopsis: "DIAMETER::retransmission action",
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
