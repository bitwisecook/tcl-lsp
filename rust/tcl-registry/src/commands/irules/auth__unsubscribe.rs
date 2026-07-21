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

//! `AUTH::unsubscribe` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "AUTH::unsubscribe",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Cancels interest in auth query results.",
            synopsis: &["AUTH::unsubscribe AUTH_ID"],
            snippet: "AUTH::unsubscribe cancels interest in auth query results.\nAUTH::response_data will not return data from query results for\nwhich a subscription has been cancelled before AUTH::authenticate\nhas been called. Also see AUTH::subscribe.\n\nAUTH::unsubscribe <authid>\n\n     * Cancels interest in auth query results.",
            source: "https://clouddocs.f5.com/api/irules/AUTH__unsubscribe.html",
            examples: "when HTTP_REQUEST {\n        if {not [info exists auth_pass]} {\n            set auth_sid [AUTH::start pam auth_method_user]\n            AUTH::subscribe $auth_sid\n            set auth_username [HTTP::username]\n            set auth_password [HTTP::password]\n            AUTH::username_credential $auth_sid $auth_username\n            AUTH::password_credential $auth_sid $auth_password\n            AUTH::authenticate $auth_sid\n            set auth_pass 1\n        }\n    }",
            return_value: "",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "AUTH::unsubscribe AUTH_ID",
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
