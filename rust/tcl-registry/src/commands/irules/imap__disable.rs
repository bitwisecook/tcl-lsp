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

//! `IMAP::disable` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "IMAP::disable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Disable IMAP protocol handler.",
            synopsis: &["IMAP::disable"],
            snippet: "Disable IMAP protocol handler for IMAP message processing. This will disable detection of STARTTLS for IMAP.",
            source: "https://clouddocs.f5.com/api/irules/IMAP__disable.html",
            examples: "when CLIENT_ACCEPTED {\n                if { ([IP::addr [IP::client_addr] equals 10.0.0.0/8]) } {\n                    IMAP::disable\n                }\n            }",
            return_value: "",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "IMAP::disable",
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
