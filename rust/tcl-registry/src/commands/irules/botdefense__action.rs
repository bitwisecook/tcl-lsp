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

//! `BOTDEFENSE::action` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "BOTDEFENSE::action",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns or overrides the action to be taken by Bot Defense.",
            synopsis: &["BOTDEFENSE::action (allow |"],
            snippet: "Returns or overrides the action to be taken by Bot Defense.\n\nOverriding the action may fail on certain cases. For example, overriding to the \"browser_challenge\" action, may only be done on requests to which the value of BOTDEFENSE::cs_possible is \"true\". When overriding the action, the command returns \"ok\" if the action was successfully set. Otherwise, the action is not changed, and the reason for failure is returned.\n\nAfter a successful action override (resulting in the \"ok\" string), the action cannot be overridden again.",
            source: "https://clouddocs.f5.com/api/irules/BOTDEFENSE__action.html",
            examples: "when HTTP_RESPONSE_RELEASE {\n    if {[info exists botdefense_responded]} {\n        HTTP::header insert \"myheader\" \"blocked request\"\n    }\n}",
            return_value: "* When called without any arguments: Returns a string signifying the action to be taken by Bot Defense. If the action was overridden, the returned action is the overridden one. * When called with an argument for overriding the action, the return value is a status string.",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["BOTDEFENSE"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "BOTDEFENSE::action (allow |",
            dialects: None,
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
