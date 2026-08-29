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

//! `ADAPT::result` iRules command.
use crate::prelude::*;
use tcl_dialect::model::{SpecSurface};
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "ADAPT::result",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Sets or returns the adaptation result code.",
            synopsis: &["ADAPT::result (ADAPT_CTX)? (ADAPT_SIDE)? ('bypass' | 'close' | 'abort')?"],
            snippet: "The ADAPT::result command sets or returns the adaptation result\ncode of the ADAPT filter on the current or specified side of the\nvirtual server connection for which the iRule is being executed.\n\nPossible result codes are:\n    * unknown - The internal virtual server has not returned a\n      result yet. It is not possible to change the result code\n      to this value.\n    * bypass - The internal virtual server does not need to modify\n      the request or response.",
            source: "https://clouddocs.f5.com/api/irules/ADAPT__result.html",
            examples: "when ADAPT_REQUEST_RESULT {\n     if {[ADAPT::result] == \"respond\"} {\n        # Force ADAPT to ignore any direct response from IVS\n        # (contrived example, probably not useful as-is).\n        ADAPT::result bypass\n     }\n}",
            return_value: "Returns the current or modified result code.",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["REQUESTADAPT", "RESPONSEADAPT"],
            also_in: &[],
            flow: false,
        }),
        forms: &[FormSpec {
            synopsis: "ADAPT::result (ADAPT_CTX)? (ADAPT_SIDE)? ('bypass' | 'close' | 'abort')?",
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
