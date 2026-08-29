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

//! `ADAPT::context_current` iRules command.
use crate::prelude::*;
use tcl_dialect::model::{SpecSurface};
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "ADAPT::context_current",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Gets the current context.",
            synopsis: &["ADAPT::context_current"],
            snippet: "Obtains a handle for the current context. The current context\nis usually that in which the event occurred from which this\ncommand was issued.\n\nSyntax:\n\nADAPT::context_current",
            source: "https://clouddocs.f5.com/api/irules/ADAPT__context_current.html",
            examples: "when ADAPT_REQUEST_RESULT {\n    set ctx [ADAPT::context_current]\n    if {$ctx == $req_ctx2 && $need_another_ctx} {\n        set req_ctx3 [ADAPT::context_create my_req_ctx3]\n        ADAPT::select $req_ctx3 ivs-icap-req3\n    }\n}",
            return_value: "Returns the handle of the current context.",
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
            synopsis: "ADAPT::context_current",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::IcapState,
            reads: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}
