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

//! `ROUTE::expiration` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "ROUTE::expiration",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns the remaining time for a route or congestion metrics cache entry.",
            synopsis: &["ROUTE::expiration DESTINATION_IP_ADDRESS (GATEWAY_IP_ADDRESS)?"],
            snippet: "Returns the remaining time in seconds. The lifetime of an entry may\nhave been set by the route.metrics.timeout sys db variable, the\ncmetrics-cache-timeout TCP profile attribute, or a\nTCP::rt_metrics_timeout iRule.\n\nThe return value only applies to the TMM executing the command. It\ndoes not consider cache entries on other TMMs.",
            source: "https://clouddocs.f5.com/api/irules/ROUTE__expiration.html",
            examples: "when CLIENT_CLOSED {\n    # If the entry almost timed out, keep it a little longer next time.\n    set time_remaining [ROUTE::expiration [IP::remote_addr]]\n    if { $time_remaining > 0 && $time_remaining < 100 } {\n         # Default value is 600\n         TCP::rt_metrics_timeout 700\n    }\n}",
            return_value: "",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "ROUTE::expiration DESTINATION_IP_ADDRESS (GATEWAY_IP_ADDRESS)?",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::TcpState,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Both,
            dialects: None,
        }],
        ..CommandSpec::DEFAULT
    }
}
