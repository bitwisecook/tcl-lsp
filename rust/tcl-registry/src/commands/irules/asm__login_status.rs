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

//! `ASM::login_status` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "ASM::login_status",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Request status of the login session tracked by one of the login pages defined in the policy.",
            synopsis: &["ASM::login_status"],
            snippet: "Returns status of the login session tracked by one of the login pages defined in the policy. Following are the possible values:\n\n                not_logged_in: The request is not within a login session.\n                logging_in: The request is to a login URL.\n                logged_in: The request is within a login session, indicates a successful login in the ASM_RESPONSE_LOGIN event.\n                failed: The login attempt is failed, triggered only in the ASM_RESPONSE_LOGIN event.",
            source: "https://clouddocs.f5.com/api/irules/ASM__login_status.html",
            examples: "when ASM_RESPONSE_LOGIN {\n                if {[ASM::login_status] eq \"logged_in\"} {\n                    log local0. \"User [ASM::username] logged in succesfully.\"\n                }\n                else {\n                    log local0. \"Login attempt to [ASM::username] failed.\"\n                }\n            }",
            return_value: "Returns status of the login session.;",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["ASM"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "ASM::login_status",
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
