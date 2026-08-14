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

//! `ADAPT::preview_size` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "ADAPT::preview_size",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Sets or returns the preview-size attribute.",
            synopsis: &["ADAPT::preview_size (ADAPT_CTX)? (ADAPT_SIDE)? (SIZE)?"],
            snippet: "The ADAPT::preview_size command sets or returns the preview-size\nattribute of the ADAPT filter on the current or specified side of\nthe virtual server connection for which the iRule is being executed.",
            source: "https://clouddocs.f5.com/api/irules/ADAPT__preview_size.html",
            examples: "when HTTP_RESPONSE {\n    if { [HTTP::header \"Content-Type\"] contains \"image\" } {\n        ADAPT::select ivs-icap-image\n        ADAPT::preview_size 10000\n    }\n    if { [HTTP::header \"Content-Type\"] contains \"video\" } {\n       ADAPT::select ivs-icap-video\n       ADAPT::preview_size 30000\n    }\n}",
            return_value: "Returns the current or modified preview size (bytes).",
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
            synopsis: "ADAPT::preview_size (ADAPT_CTX)? (ADAPT_SIDE)? (SIZE)?",
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
