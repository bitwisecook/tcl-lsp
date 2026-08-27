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

//! `IP::protocol` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "IP::protocol",
        traits: Traits::PURE.union(Traits::CSE_CANDIDATE),
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns the IP protocol value.",
            synopsis: &["IP::protocol"],
            snippet: "Returns the IP protocol value. This command replaces the BIG-IP 4.X variable ip_protocol.\nFor a list of the IP protocol numbers, see /etc/protocols or the L<IANA protocol number list|http://www.iana.org/assignments/protocol-numbers/protocol-numbers.xml>",
            source: "https://clouddocs.f5.com/api/irules/IP__protocol.html",
            examples: "when CLIENT_ACCEPTED {\n  if { [IP::protocol] == 6 } {\n     pool tcp_pool\n  } else {\n     pool slow_pool\n  }\n}",
            return_value: "IP protocol",
        }),
        event_requires: Some(EventRequires {
            client_side: true,
            server_side: false,
            transport: None,
            profiles: &[],
            also_in: &[],
            flow: false,
        }),
        forms: &[FormSpec {
            synopsis: "IP::protocol",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::TcpState,
            reads: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}
