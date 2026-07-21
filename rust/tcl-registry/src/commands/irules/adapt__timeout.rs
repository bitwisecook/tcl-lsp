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

//! `ADAPT::timeout` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "ADAPT::timeout",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Sets or returns the timeout attribute.",
            synopsis: &["ADAPT::timeout (ADAPT_CTX)? (ADAPT_SIDE)? (TIMEOUT_VALUE)?"],
            snippet: "The ADAPT::timeout command sets or returns the timeout attribute\nof the ADAPT filter on the current or specified side of the\nvirtual server connection for which the iRule is being executed.\nThe timeout (in milliseconds) is how long ADAPT will wait for\na result from the internal virtual server before deciding the\nservice is down.",
            source: "https://clouddocs.f5.com/api/irules/ADAPT__timeout.html",
            examples: "when HTTP_RESPONSE {\n     if { [HTTP::header \"Content-Type\"] contains \"image\" } {\n        ADAPT::select ivs-icap-image\n        ADAPT::timeout 500\n     }\n     if { [HTTP::header \"Content-Type\"] contains \"video\" } {\n        ADAPT::select ivs-icap-video\n        ADAPT::timeout 2000\n     }\n }",
            return_value: "Returns the current or modified timeout in milliseconds.",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["HTTP", "REQUESTADAPT", "RESPONSEADAPT"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "ADAPT::timeout (ADAPT_CTX)? (ADAPT_SIDE)? (TIMEOUT_VALUE)?",
            dialects: None,
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::IcapState,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::Both,
            dialects: None,
        }],
        ..CommandSpec::DEFAULT
    }
}
