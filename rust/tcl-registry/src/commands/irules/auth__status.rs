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

//! `AUTH::status` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "AUTH::status",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns authentication status.",
            synopsis: &["AUTH::status (AUTH_ID)?"],
            snippet: "Returns authentication status. The returned status is a value of 0, 1,\n-1, or 2, corresponding to success, failure, error, or not-authed,\nbased on the result of the most recent authorization that the system\nperformed for the specified authorization session .\nIn the case of a not-authed result, the authentication process desires\na credential not yet provided. Specifics of the requested credential\ncan be determined using the AUTH::wantcredential_ commands. The\nauthentication process could be continued using\nAUTH::authenticate_continue*.",
            source: "https://clouddocs.f5.com/api/irules/AUTH__status.html",
            examples: "when HTTP_RESPONSE {\n  set authStatus [AUTH::status $authSessionId]\n}",
            return_value: "",
        }),
        forms: &[FormSpec {
            synopsis: "AUTH::status (AUTH_ID)?",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::ApmState,
            reads: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}
