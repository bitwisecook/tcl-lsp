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

//! `AAA::auth_send` iRules command.
use crate::prelude::*;
use tcl_dialect::model::SpecSurface;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "AAA::auth_send",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "This command is used to send user authentication information to IVS(internal virtual server).",
            synopsis: &["AAA::auth_send VIRTUAL_SERVER USERNAME (PASSWORD)?"],
            snippet: "This command is used to send user authentication information to IVS(internal virtual server).",
            source: "https://clouddocs.f5.com/api/irules/AAA__auth_send.html",
            examples: "when HTTP_REQUEST_DATA {\n    set request_id [AAA::auth_send $internal_radius_aaa_vip $username $password]\n\n    set aaa_result [AAA::auth_result $request_id]\n    if { $aaa_result == \"OK\" } {\n        # request was successfull\n    } else {\n        # handle errors\n    }\n}",
            return_value: "request_id - the id of the current connection that can be used to check the status later with AAA::auth_result command",
        }),
        forms: &[FormSpec {
            synopsis: "AAA::auth_send VIRTUAL_SERVER USERNAME (PASSWORD)?",
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
