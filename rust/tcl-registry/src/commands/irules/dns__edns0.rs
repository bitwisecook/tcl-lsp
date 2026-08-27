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

//! `DNS::edns0` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "DNS::edns0",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Gets (v11.0+) and sets (v11.1+) the values of the edns0 pseudo-RR.",
            synopsis: &[
                "DNS::edns0 'remove' ('nsid' | 'subnet')?",
                "DNS::edns0 'exists' ('nsid' | 'subnet')?",
                "DNS::edns0 'do' (BOOLEAN)?",
                "DNS::edns0 'sz' (UNSIGNED_SHORT)?",
            ],
            snippet: "This iRules command gets (v11.0+) and sets (v11.1+) the values of the\nedns0 pseudo-RR.\n\nNote: This command requires the DNS Profile, which is only enabled as\npart of GTM or the DNS Services add-on.",
            source: "https://clouddocs.f5.com/api/irules/DNS__edns0.html",
            examples: "when DNS_REQUEST {\n  if { [DNS::edns0 exists] } {\n    log local0. [DNS::edns0 subnet address]\"\n  }\n}",
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
            synopsis: "DNS::edns0 'remove' ('nsid' | 'subnet')?",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::DnsState,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}
