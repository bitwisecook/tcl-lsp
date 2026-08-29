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

//! `ADAPT::context_create` iRules command.
use crate::prelude::*;
use tcl_dialect::model::SpecSurface;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "ADAPT::context_create",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Creates a new dynamic adaptation context.",
            synopsis: &["ADAPT::context_create (ADAPT_SIDE)? NAME"],
            snippet: "Creates a new dynamic adaptation context in the ADAPT filter on\nthe current or specified side of the virtual server connection\nfor which the iRule is being executed. Maybe called mulitple\ntimes to dynamically create chains of adaptation contexts.\n\nSyntax:\n\nADAPT::context_create <name>\n\n    * Creates a dynamic context on the current side.\n      This must be called from the request-adapt side, so has\n      the same effect as ADAPT::context_create request <name>.",
            source: "https://clouddocs.f5.com/api/irules/ADAPT__context_create.html",
            examples: "when HTTP_RESPONSE {\n    # Configure a response context from the current (response) side.\n    ADAPT::select $rsp_ctx2 ivs-icap-rsp2\n    ADAPT::timeout $rsp_ctx2 2000\n}",
            return_value: "Returns the context handle.",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["HTTP", "REQUESTADAPT"],
            also_in: &[],
            flow: false,
        }),
        forms: &[FormSpec {
            synopsis: "ADAPT::context_create (ADAPT_SIDE)? NAME",
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
