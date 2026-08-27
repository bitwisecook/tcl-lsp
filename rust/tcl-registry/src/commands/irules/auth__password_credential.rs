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

//! `AUTH::password_credential` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "AUTH::password_credential",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Sets the password credential to the specified string for a future AUTH::authenticate call.",
            synopsis: &["AUTH::password_credential AUTH_ID PASSWORD_CREDENTIAL"],
            snippet: "Sets the password credential to the specified string for a future\nAUTH::authenticate call. This command returns an error if\nattempted for a standby system.\n\nAUTH::password_credential authid <string>\n\n     * Sets the password credential to the specified string for a future\n       AUTH::authenticate call.",
            source: "https://clouddocs.f5.com/api/irules/AUTH__password_credential.html",
            examples: "when HTTP_REQUEST {\n  AUTH::password_credential $auth_id [HTTP::password]\n}",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["HTTP"],
            also_in: &[],
            flow: false,
        }),
        forms: &[FormSpec {
            synopsis: "AUTH::password_credential AUTH_ID PASSWORD_CREDENTIAL",
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
