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

//! `DIAMETER::retransmit` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "DIAMETER::retransmit",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Triggers the request associated to the current answer message for retransmission.",
            synopsis: &["DIAMETER::retransmit (disabled | busy | unable | retransmit) (NOTE)?"],
            snippet: "This iRule command triggers the request in the retransmission queue\nthat is associated with the current answer message for\nretransmission. This command will fail the current message is a\nrequest or if there is not an associated request message in the\nretransmission queue.",
            source: "https://clouddocs.f5.com/api/irules/DIAMETER__retransmit.html",
            examples: "when DIAMETER_EGRESS {\n    if { [DIAMETER::is_response] && ![DIAMETER::is_retransmission] } {\n        log local0. \"reason [DIAMETER::retransmission_reason]\"\n        DIAMETER::retransmit\n    }\n}",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["DIAMETER"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "DIAMETER::retransmit (disabled | busy | unable | retransmit) (NOTE)?",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::NetworkIo,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::Both,
            dialects: None,
        }],
        ..CommandSpec::DEFAULT
    }
}
