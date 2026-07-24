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

//! `ACCESS::ephemeral-auth` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "ACCESS::ephemeral-auth",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Ephemeral auth related iRule",
            synopsis: &[
                "ACCESS::ephemeral-auth create ('-user' USER) ('-auth_cfg' AUTH_CONFIG)? ('-sid' SESSION_ID)?",
                "ACCESS::ephemeral-auth verify ('-user' USER) ('-password' PASSWORD) ('-protocol' EPHEMERAL_AUTH_PROTOCOL)",
            ],
            snippet: "Ephemeral auth related iRule\n\nThis command can be used either to create or verify a temporary password for ephemeral authentication.\n\nACCESS::ephemeral-auth create [] will create a temporary password and return its value. When auth_cfg is not given, it will use the one deduced from access-config that is associated with the virtual server.  When sid is not given, it will use the one retrieved from the current access environment.\n\nACCESS::ephemeral-auth verify [] will verify the user credentials and return the session id that was used to generate temporary password.",
            source: "https://clouddocs.f5.com/api/irules/ACCESS__ephemeral-auth.html",
            examples: "when HTTP_REQUEST {\n    if { [ HTTP::path ] starts_with \"/test1\" } {\n        call ephemeral_auth_test1\n        HTTP::respond 200 -content \"<html>test1</html>\\n\"\n    }\n}",
            return_value: "ACCESS::ephemeral-auth create [] will return the generated temporary password. ACCESS::ephemeral-auth verify [] will return the session id.",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "ACCESS::ephemeral-auth create ('-user' USER) ('-auth_cfg' AUTH_CONFIG)? ('-sid' SESSION_ID)?",
            dialects: None,
        }],
        options: const {
            &[
                OptionSpec {
                    name: "-user",
                    value: OptionValue::value(""),
                    detail: "Option -user.",
                    dialects: None,
                    aliases: &[],
                    min_version: None,
                },
                OptionSpec {
                    name: "-auth_cfg",
                    value: OptionValue::value(""),
                    detail: "Option -auth_cfg.",
                    dialects: None,
                    aliases: &[],
                    min_version: None,
                },
                OptionSpec {
                    name: "-sid",
                    value: OptionValue::value(""),
                    detail: "Option -sid.",
                    dialects: None,
                    aliases: &[],
                    min_version: None,
                },
                OptionSpec {
                    name: "-password",
                    value: OptionValue::value(""),
                    detail: "Option -password.",
                    dialects: None,
                    aliases: &[],
                    min_version: None,
                },
                OptionSpec {
                    name: "-protocol",
                    value: OptionValue::value(""),
                    detail: "Option -protocol.",
                    dialects: None,
                    aliases: &[],
                    min_version: None,
                },
            ]
        },
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
