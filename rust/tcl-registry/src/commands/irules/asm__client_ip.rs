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

//! `ASM::client_ip` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "ASM::client_ip",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns the IP address of the end client that sent the request.",
            synopsis: &["ASM::client_ip"],
            snippet: "Returns the IP address of the end client that sent the request.\nNote that this IP address is not necessarily equal to the address\nreturned by the command IP::client_addr, which is the IP address of the\nimmediate client found in the IP header as received by BIG-IP. The\nlatter can be a proxy, in which case the end client IP address is\nextracted from one of the HTTP headers, typically, X-Forwarded-For.",
            source: "https://clouddocs.f5.com/api/irules/ASM__client_ip.html",
            examples: "when ASM_REQUEST_DONE {\n  log local0. \"Src IP: [IP::client_addr], End-client IP: [ASM::client_ip]\"\n}",
            return_value: "Returns the IP address of the end client that sent the request.",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["ASM"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[FormSpec {
            synopsis: "ASM::client_ip",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::AsmState,
            reads: true,
            connection_side: ConnectionSide::Client,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}
