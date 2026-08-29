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

//! `ASM::payload` iRules command.
use crate::prelude::*;
use tcl_dialect::model::{SpecSurface};
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "ASM::payload",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        data_collection: Some(ASM_PAYLOAD),
        hover: Some(HoverSnippet {
            summary: "Retrieves or replaces the payload collected by ASM.",
            synopsis: &[
                "ASM::payload (LENGTH | (OFFSET LENGTH))?",
                "ASM::payload length",
                "ASM::payload replace OFFSET LENGTH ASM_PAYLOAD",
            ],
            snippet: "This command retrieves or replaces the payload collected by ASM.",
            source: "https://clouddocs.f5.com/api/irules/ASM__payload.html",
            examples: "when ASM_REQUEST_VIOLATION\n{\n  set x [ASM::violation_data]\n  if {([lindex $x 0] contains \"VIOLATION_EVASION_DETECTED\")}\n   {\n      ASM::payload replace 0 0 \"1234567890\"\n   }\n}",
            return_value: "",
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
            synopsis: "ASM::payload (LENGTH | (OFFSET LENGTH))?",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::AsmState,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::Client,
            ..SideEffect::DEFAULT
        }],
        taint_source: Some(TaintColour::TAINTED),
        ..CommandSpec::DEFAULT
    }
}
