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

//! `AUTH::abort` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "AUTH::abort",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Cancels any outstanding auth operations in this authentication session.",
            synopsis: &["AUTH::abort AUTH_ID"],
            snippet: "Cancels any outstanding auth operations in this authentication session,\nand generates an AUTH_FAILURE event if there was an outstanding\nauthentication query in progress. This command invalidates the\nspecified authentication session ID, which should be discarded upon\ncalling this command.\n\nAUTH::abort authid\n\n     * Cancels any outstanding auth operations in this authentication\n       session, and generates an AUTH_FAILURE event if there was an\n       outstanding authentication query in progress.",
            source: "https://clouddocs.f5.com/api/irules/AUTH__abort.html",
            examples: "when CLIENT_ACCEPTED {\n    set auth_http_successes 0\n    set auth_http_sufficient_successes 2\n}",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["HTTP"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[FormSpec {
            synopsis: "AUTH::abort AUTH_ID",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::ApmState,
            writes: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}
