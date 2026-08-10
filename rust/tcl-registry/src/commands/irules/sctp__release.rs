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

//! `SCTP::release` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "SCTP::release",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Resumes processing and flushes collected data.",
            synopsis: &["SCTP::release (RELEASE_BYTES)?"],
            snippet: "Causes SCTP to resume processing the connection and flush collected data.",
            source: "https://clouddocs.f5.com/api/irules/SCTP__release.html",
            examples: "when CLIENT_ACCEPTED {\n  SCTP::collect 15\n}",
            return_value: "",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "SCTP::release (RELEASE_BYTES)?",
            dialects: None,
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::TcpState,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::Both,
            dialects: None,
        }],
        data_collection: Some(SCTP_RELEASE),
        ..CommandSpec::DEFAULT
    }
}
