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

//! `ASM::raise` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "ASM::raise",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Issues a user-defined violation on the request.",
            synopsis: &["ASM::raise VIOLATION_NAME (VIOLATION_DETAILS)?"],
            snippet: "Issues a user-defined violation on the request. The violation\nis added to other possible violations, either raised by the ASM or by\nprevious invocations of this command. The consequent action is\ndetermined by the blocking setting per the raised violation, e.g. if\nthe violation was set to block, then the request will be blocked.",
            source: "https://clouddocs.f5.com/api/irules/ASM__raise.html",
            examples: "when ASM_REQUEST_DONE {\n   if {[ASM::violation count] > 3 and [ASM::severity] eq \"Error\"} {\n      ASM::raise VIOLATION_TOO_MANY_VIOLATIONS\n   }\n}",
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
            synopsis: "ASM::raise VIOLATION_NAME (VIOLATION_DETAILS)?",
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
