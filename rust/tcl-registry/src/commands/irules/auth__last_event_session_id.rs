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

//! `AUTH::last_event_session_id` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "AUTH::last_event_session_id",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns the session ID of the last auth event.",
            synopsis: &["AUTH::last_event_session_id"],
            snippet: "This command returns the session ID of the last auth event, which can\nthen be used to relate to the user behind each session.\n\nAUTH::last_event_session_id\n\n     * Returns the session ID of the last auth event",
            source: "https://clouddocs.f5.com/api/irules/AUTH__last_event_session_id.html",
            examples: "when AUTH_SUCCESS {\n  if {$auth_id eq [AUTH::last_event_session_id]} {\n    log local0. \"auth success event\"\n    set authorized 1\n  }\n}",
            return_value: "",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "AUTH::last_event_session_id",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::ApmState,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Both,
            dialects: None,
        }],
        ..CommandSpec::DEFAULT
    }
}
