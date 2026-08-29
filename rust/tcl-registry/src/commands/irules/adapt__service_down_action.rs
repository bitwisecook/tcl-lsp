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

//! `ADAPT::service_down_action` iRules command.
use crate::prelude::*;
use tcl_dialect::model::{SpecSurface};
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "ADAPT::service_down_action",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Sets or returns the service-down-action attribute.",
            synopsis: &[
                "ADAPT::service_down_action (ADAPT_CTX)? (ADAPT_SIDE)? ('ignore' | 'reset' | 'drop')?",
            ],
            snippet: "The ADAPT::service_down_action command sets or returns the\nservice-down-action attribute of the ADAPT filter on the\ncurrent or specified side of the virtual server connection\nfor which the iRule is being executed.\n\nPossible service-down-actions aare:\n    * ignore - Do not send the HTTP request or response to the\n      internal virtual server (bypass). Pass it through unchanged.\n    * reset - Reset (RST) the connection.\n    * drop - Drop (FIN) the connection.",
            source: "https://clouddocs.f5.com/api/irules/ADAPT__service_down_action.html",
            examples: "when ADAPT_REQUEST_HEADERS {\n     # Cause connection to be dropped if ICAP server handling\n     # response is down for requests with a custom HTTP header\n     # (which might have been resulted from request adaptation).\n     if {[HTTP::header exists \"X-Drop-if-down\"]} {\n        ADAPT::service_down_action response drop\n     }\n}",
            return_value: "Returns the current or modified service-down-action.",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["HTTP", "REQUESTADAPT", "RESPONSEADAPT"],
            also_in: &[],
            flow: false,
        }),
        forms: &[FormSpec {
            synopsis: "ADAPT::service_down_action (ADAPT_CTX)? (ADAPT_SIDE)? ('ignore' | 'reset' | 'drop')?",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::IcapState,
            writes: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}
