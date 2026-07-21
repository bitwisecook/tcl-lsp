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

//! `DNS::ptype` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "DNS::ptype",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns the type of the DNS packet.",
            synopsis: &["DNS::ptype"],
            snippet: "This iRules command returns the type of the DNS packet.\n\nNote: This command requires the DNS Profile, which is only enabled as\npart of GTM or the DNS Services add-on.",
            source: "https://clouddocs.f5.com/api/irules/DNS__ptype.html",
            examples: "OMAIN response is going to be sent,\n            # instead attach a record to resolve to.\n            when DNS_RESPONSE {\n                if { [DNS::ptype] == \"NXDOMAIN\" } {\n                    DNS::header rcode NOERROR\n                    DNS::answer insert \"[DNS::question name]. 60 [DNS::question class] [DNS::question type] 192.168.1.245\"\n                }\n            }",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["DNS"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "DNS::ptype",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::DnsState,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Both,
            dialects: None,
        }],
        ..CommandSpec::DEFAULT
    }
}
