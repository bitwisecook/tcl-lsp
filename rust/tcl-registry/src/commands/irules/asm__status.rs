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

//! `ASM::status` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "ASM::status",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns the current status of the request or response.",
            synopsis: &["ASM::status"],
            snippet: "Returns the current status of the request or response\nReturns one of the following values:\n  + Alarm - there are violations and alarm has been raised, but\n    request or response is not blocked. This does not apply to\n    violations that are in staging mode. This value will also be\n    returned if the request had violations but was unblocked using\n    a previously called ASM::unblock command.\n  + Blocked - violations caused the request/response to be\n    blocked. This does not apply to violations that are in staging\n    mode.\n  + Clear - no violations found",
            source: "https://clouddocs.f5.com/api/irules/ASM__status.html",
            examples: "when ASM_REQUEST_DONE {\n    #log local0.debug \"\\[ASM::status\\] = [ASM::status]\"\n    if { [ASM::status] equals \"alarmed\" } {\n        set x [ASM::violation_data]\n        HTTP::header insert X-ASM \"violation=[lindex $x 0] supportid=[lindex $x 1]\"\n        #log local0.debug \"DEBUG02: violation=[lindex $x 0] supportid=[lindex $x 1]\"\n    }\n}",
            return_value: "* Alarm * Blocked * Clear",
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
            synopsis: "ASM::status",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::AsmState,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Client,
            dialects: None,
        }],
        ..CommandSpec::DEFAULT
    }
}
