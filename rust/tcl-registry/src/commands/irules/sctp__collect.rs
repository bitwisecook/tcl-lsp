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

//! `SCTP::collect` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "SCTP::collect",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Collects the specified amount of content data.",
            synopsis: &["SCTP::collect (COLLECT_BYTES)?"],
            snippet: "Causes SCTP to start collecting the specified amount of content data. After collecting the data, event CLIENT_DATA will be triggered.\n\nSCTP::collect <length>\n    Causes SCTP to start collecting the specified amount of content data. The parameter specifies the minimum number of bytes to collect.\n\nSCTP::collect\n    When length is not specified, CLIENT_DATA will be triggered for every received packet. To stop collecting data, use SCTP::release.",
            source: "https://clouddocs.f5.com/api/irules/SCTP__collect.html",
            examples: "when CLIENT_ACCEPTED {\n  SCTP::collect 15\n}",
            return_value: "",
        }),
        forms: &[FormSpec {
            synopsis: "SCTP::collect (COLLECT_BYTES)?",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::TcpState,
            writes: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        data_collection: Some(SCTP_COLLECT),
        ..CommandSpec::DEFAULT
    }
}
