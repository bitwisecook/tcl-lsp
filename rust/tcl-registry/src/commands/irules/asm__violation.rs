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

//! `ASM::violation` iRules command.
use crate::prelude::*;
use tcl_dialect::model::SpecSurface;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "ASM::violation",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns the list of violations found in the request or response together with details on each one.",
            synopsis: &["ASM::violation (count | names | attack_types | details | rating)"],
            snippet: "Returns the list of violations found in the request or response together with details on each one.",
            source: "https://clouddocs.f5.com/api/irules/ASM__violation.html",
            examples: "when ASM_REQUEST_DONE {\n  set i 0\n  foreach {viol} [ASM::violation names]{\n    if {$viol eq \"VIOLATION_ILLEGAL_PARAMETER\"} {\n      set details [lindex [ASM::violation details] $i]\n      set param_name [b64_decode [llookup $details \"param_data.param_name\"]]\n      #remove the bad parameter from the QS - does not work right in all cases,just for illustration!\n      regsub -all \"\\?.*($param_name=[^\\&]*)\" [HTTP::uri] \"?\" $new_uri\n      HTTP::uri $new_uri\n      ASM::unblock\n    }",
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
            synopsis: "ASM::violation (count | names | attack_types | details | rating)",
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
