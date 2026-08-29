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

//! `ADAPT::context_static` iRules command.
use crate::prelude::*;
use tcl_dialect::model::{SpecSurface};
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "ADAPT::context_static",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Gets the static context.",
            synopsis: &["ADAPT::context_static (ADAPT_SIDE)?"],
            snippet: "Obtains a handle for the static context on the current\nor specified side. The static context is the profile-based\ncontext that applies when there are no dynamic contexts on that\nside. Returns a null string if the connection flow has not\nyet been initialized (for example, if the command was issued\nfrom a request-adapt (client side) event and the server side\nconnection has not yet been established).\n\nSyntax:\n\nADAPT::context_static\n\n    * Gets the static context on the current side.\n\nADAPT::context_static request\n\n    * Gets the static context on the request-adapt side.",
            source: "https://clouddocs.f5.com/api/irules/ADAPT__context_static.html",
            examples: "when ADAPT_REQUEST_RESULT {\n    set static_ctx [ADAPT::context_static]\n    set ctx [ADAPT::context_current]\n    if {$ctx == $static_ctx} {\n        log local0. \"No dynamic contexts have been created.\"\n    }\n}",
            return_value: "Returns the handle of the static context, or a null string.",
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
            synopsis: "ADAPT::context_static (ADAPT_SIDE)?",
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
