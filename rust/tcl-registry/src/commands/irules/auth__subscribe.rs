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

//! `AUTH::subscribe` iRules command.
use crate::prelude::*;
use tcl_dialect::model::SpecSurface;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "AUTH::subscribe",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Registers interest in auth query results.",
            synopsis: &["AUTH::subscribe AUTH_ID"],
            snippet: "AUTH::subscribe registers interest in auth query results.\nAUTH::response_data will only return data from query results for\nwhich a subscription has been made prior to calling\nAUTH::authenticate. As a convenience when using the built-in\nsystem auth rules, these rules will call AUTH::subscribe if the\nvariable tmm_auth_subscription is set. Instead of calling\nAUTH::subscribe directly, we recommend setting tmm_auth_subscription to\n\"*\" when using the built-in system auth rules in the interest of\nforward-compatibility. Also see AUTH::unsubscribe.",
            source: "https://clouddocs.f5.com/api/irules/AUTH__subscribe.html",
            examples: "when HTTP_REQUEST {\n        if {not [info exists auth_pass]} {\n            set auth_sid [AUTH::start pam auth_method_user]\n            AUTH::subscribe $auth_sid\n            set auth_username [HTTP::username]\n            set auth_password [HTTP::password]\n            AUTH::username_credential $auth_sid $auth_username\n            AUTH::password_credential $auth_sid $auth_password\n            AUTH::authenticate $auth_sid\n            set auth_pass 1\n        }\n    }",
            return_value: "",
        }),
        forms: &[FormSpec {
            synopsis: "AUTH::subscribe AUTH_ID",
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
