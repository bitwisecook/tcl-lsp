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

//! `ASM::uncaptcha` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "ASM::uncaptcha",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Overrides the CAPTCHA action.",
            synopsis: &["ASM::uncaptcha"],
            snippet: "Overrides the CAPTCHA action for a request mitigated during a Brute-Force attack. \n            Consequently, the request will be forwarded to the origin server. \n            If the present request was not supposed to be mitigated by CAPTCHA then the command has no effect.",
            source: "https://clouddocs.f5.com/api/irules/ASM__uncaptcha.html",
            examples: "when ASM_REQUEST_DONE {\n                set i 0\n                foreach {viol} [ASM::violation names] {\n                    if {$viol eq VIOLATION_ILLEGAL_PARAMETER} {\n                        set details [lindex [ASM::violation details] $i]\n                        set param_name [b64decode [llookup $details \"param_data.param_name\"]]\n                        #remove the bad parameter from the QS - does not work right in all cases, just for illustration!",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["ASM"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "ASM::uncaptcha",
            dialects: None,
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::AsmState,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::Client,
            dialects: None,
        }],
        ..CommandSpec::DEFAULT
    }
}
