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

//! `ROUTE::age` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "ROUTE::age",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Deprecated: Returns the age of the route metrics in seconds.",
            synopsis: &["ROUTE::age DESTINATION_IP_ADDRESS (GATEWAY_IP_ADDRESS)?"],
            snippet: "The amount of time that has elapsed since the last update to the\nROUTE::rtt, ROUTE::rttvar and ROUTE::bandwidth\nstatistics for the matched route metric entry.\nROUTE::age has a value of 0 when there are no statistics\navailable.\n\nNote: As of v12.0 ROUTE::age is deprecated, as the expiration time,\nrather than the creation time, is now stored. Since deprecation,\nROUTE::age reports the age assuming that initial timeout was the\nsys db variable route.metrics.timeout. Results are incorrect if\ntimeout was changed by the TCP profile or an iRule.",
            source: "https://clouddocs.f5.com/api/irules/ROUTE__age.html",
            examples: "when CLIENT_ACCEPTED {\n    log local0. \"Cached age is: [ROUTE::age [IP::remote_addr]]\"\n}",
            return_value: "The age of the route metrics in seconds",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "ROUTE::age DESTINATION_IP_ADDRESS (GATEWAY_IP_ADDRESS)?",
            dialects: None,
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::TcpState,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Both,
            dialects: None,
        }],
        deprecated_replacement: Some("(removed)"),
        ..CommandSpec::DEFAULT
    }
}
