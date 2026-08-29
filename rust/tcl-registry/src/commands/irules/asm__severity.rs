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

//! `ASM::severity` iRules command.
use crate::prelude::*;
use tcl_dialect::model::{SpecSurface};
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "ASM::severity",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns the overall severity of the violations found in the transaction (both request and response).",
            synopsis: &["ASM::severity"],
            snippet: "Returns the overall severity of the violations found in the transaction\n(both request and response). This equals to the maximum severity of all\nthese violations",
            source: "https://clouddocs.f5.com/api/irules/ASM__severity.html",
            examples: "when ASM_REQUEST_DONE {\n   if {[ASM::violation count] > 3 and [ASM::severity] eq \"Error\"} {\n      ASM::raise VIOLATION_TOO_MANY_VIOLATIONS\n   }\n}",
            return_value: "+ Null string (in case there was no violation until the time the command is invoked) + Emergency + Alert + Critical + Error + Warning + Notice + Informational",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["ASM"],
            also_in: &[],
            flow: false,
        }),
        forms: &[FormSpec {
            synopsis: "ASM::severity",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::AsmState,
            reads: true,
            connection_side: ConnectionSide::Client,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}
