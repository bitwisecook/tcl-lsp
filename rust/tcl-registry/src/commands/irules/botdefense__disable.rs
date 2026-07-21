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

//! `BOTDEFENSE::disable` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "BOTDEFENSE::disable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Disables processing by Bot Defense on the connection.",
            synopsis: &["BOTDEFENSE::disable"],
            snippet: "Disables processing and blocking of the request by Bot Defense for the duration of the current TCP connection, or until BOTDEFENSE::enable is called.\nWhen called from events that occur before Bot Defense processing such as HTTP_REQUEST then the commands takes effect on the current request. Otherwise, if invoked in the BOTDEFENSE_REQUEST, BOTDEFENSE_ACTION or any other event that occurs after Bot Defense processing then the command will take effect only on the following request on the same connection.",
            source: "https://clouddocs.f5.com/api/irules/BOTDEFENSE__disable.html",
            examples: "# EXAMPLE: Disabling Bot Defense for a netmask of client IP addresses\nwhen CLIENT_ACCEPTED {\n    if {[IP::addr [IP::client_addr] equals 10.10.10.0/24]} {\n        BOTDEFENSE::disable\n    }\n}",
            return_value: "",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "BOTDEFENSE::disable",
            dialects: None,
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::AsmState,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::Client,
            dialects: None,
        }],
        ..CommandSpec::DEFAULT
    }
}
