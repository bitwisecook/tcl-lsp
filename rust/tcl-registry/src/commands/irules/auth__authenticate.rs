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

//! `AUTH::authenticate` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "AUTH::authenticate",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Performs a new authentication operation.",
            synopsis: &["AUTH::authenticate AUTH_ID"],
            snippet: "Performs a new authentication operation. This command returns an error\nif attempted for a standby system or while an authentication operation\nis already in progress for this authentication session.\n\nAUTH::authenticate <authid>\n\n     * Performs a new authentication operation. This command returns an\n       error if attempted for a standby system or while an authentication\n       operation is already in progress for this authentication session.",
            source: "https://clouddocs.f5.com/api/irules/AUTH__authenticate.html",
            examples: "when HTTP_REQUEST {\n  AUTH::username_credential $auth_id [HTTP::username]\n  AUTH::password_credential $auth_id [HTTP::password]\n  AUTH::authenticate $auth_id\n  HTTP::collect\n}",
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
            synopsis: "AUTH::authenticate AUTH_ID",
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
