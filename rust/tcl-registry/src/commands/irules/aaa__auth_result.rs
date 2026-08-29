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

//! `AAA::auth_result` iRules command.
use crate::prelude::*;
use tcl_dialect::model::SpecSurface;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "AAA::auth_result",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "This command is used to check the result of an authentication request.",
            synopsis: &["AAA::auth_result AAA_REQUEST_ID"],
            snippet: "This command is used to check the result of an authentication request. It can be used to determine whether the user was successfully authenticated, or if the authentication failed or if the system encountered an error.",
            source: "https://clouddocs.f5.com/api/irules/AAA__auth_result.html",
            examples: "when HTTP_REQUEST_DATA {\n    set aaa_result [AAA::auth_result $request_id]\n    if { $aaa_result == \"INPROGRESS\" } {\n        after 200\n        continue\n    }\n\n    if { $aaa_result == \"OK\" } {\n        # request was successfull\n    } else {\n        # handle errors\n    }\n}",
            return_value: "There are 4 possible return values for this command (All STRING type): \"OK\" - User was successfully authenticated \"FAIL\" - Authentication failed \"INPROGRESS\" - the request is still in progress (asyncronous). \"ERROR\" - there was an error during the request.",
        }),
        forms: &[FormSpec {
            synopsis: "AAA::auth_result AAA_REQUEST_ID",
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
