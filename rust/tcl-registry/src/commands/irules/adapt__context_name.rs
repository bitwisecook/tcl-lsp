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

//! `ADAPT::context_name` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "ADAPT::context_name",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Gets the name of a dynamic adaptation context.",
            synopsis: &["ADAPT::context_name ADAPT_CTX"],
            snippet: "Obtains the name of an adaptation context. The name of a\ndynamic context was specified when it was created. The name\nof a static (profile) context is that of the ADAPT profile\non the side of the virtual server where the context resides.\n\nSyntax:\n\nADAPT::context_name <context>",
            source: "https://clouddocs.f5.com/api/irules/ADAPT__context_name.html",
            examples: "when ADAPT_RESPONSE_RESULT {\n   set ctx [ADAPT::context_current]\n   set ctx_name [ADAPT::context_name $ctx]\n   log local0. \"ADAPT_RESPONSE_RESULT in context $ctx_name\"\n}",
            return_value: "Returns the context name.",
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
            synopsis: "ADAPT::context_name ADAPT_CTX",
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
