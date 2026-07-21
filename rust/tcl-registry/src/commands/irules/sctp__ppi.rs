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

//! `SCTP::ppi` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "SCTP::ppi",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns or sets the SCTP payload protocol indicator.",
            synopsis: &["SCTP::ppi (PPI_ID)?"],
            snippet: "Returns or sets the SCTP payload protocol indicator.",
            source: "https://clouddocs.f5.com/api/irules/SCTP__ppi.html",
            examples: "when CLIENT_ACCEPTED {\n        SCTP::collect\n        log local0.info \"Sctp local port is [SCTP::local_port]\"\n        log local0.info \"Sctp client port is [SCTP::client_port]\"\n        log local0.info \"Sctp mss is [SCTP::mss]\"\n        log local0.info \"sctp ppi is [SCTP::ppi]\"\n}",
            return_value: "",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "SCTP::ppi (PPI_ID)?",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::TcpState,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Both,
            dialects: None,
        }],
        ..CommandSpec::DEFAULT
    }
}
