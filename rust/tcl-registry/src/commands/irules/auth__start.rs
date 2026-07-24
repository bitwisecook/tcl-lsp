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

//! `AUTH::start` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "AUTH::start",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Initializes an authentication session.",
            synopsis: &["AUTH::start TYPE SERVICE"],
            snippet: "Initializes an authentication session. This command returns the\nauthentication session ID, which must be specified to other\nauthentication commands. Multiple simultaneous authentication sessions\n(up to 10) can be opened for a single connection, but it is the user’s\nresponsibility to keep track of their respective session IDs. This\ncommand returns an error if attempted for a standby system.\n\nAUTH::start <type> <PAM service>\n\n     * Returns the authentication session ID, which must be specified to\n       other authentication commands.",
            source: "https://clouddocs.f5.com/api/irules/AUTH__start.html",
            examples: "when CLIENT_ACCEPTED {\n  set auth_id [AUTH::start pam default_radius]\n}",
            return_value: "",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "AUTH::start TYPE SERVICE",
            dialects: None,
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::ApmState,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::Both,
            dialects: None,
        }],
        ..CommandSpec::DEFAULT
    }
}
