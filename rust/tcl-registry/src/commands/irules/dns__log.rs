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

//! `DNS::log` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "DNS::log",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Controls log publisher in DNS log profile.",
            synopsis: &["DNS::log (MESSAGE)?"],
            snippet: "There are two version of this command.  DNS::log by itself returns a boolean indicating whether a DNS Logging Profile is configured in the DNS profile.  DNS::log with an argument logs a message to that log publisher.",
            source: "https://clouddocs.f5.com/api/irules/DNS__log.html",
            examples: "# Send one or more IP addresses for a response to an A query\n            # Use on an LTM virtual server with a DNS profile enabled\n            when DNS_REQUEST {\n                # Log query details\n                DNS::log \"DNS question name: [DNS::question name],\n                    DNS question class: [DNS::question class],\n                    DNS question type: [DNS::question type]\"\n\n                # Generate an answer with two A records",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["DNS"],
            also_in: &[],
            flow: false,
        }),
        forms: &[FormSpec {
            synopsis: "DNS::log (MESSAGE)?",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::LogIo,
            writes: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}
