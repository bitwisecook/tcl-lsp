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

//! `BOTDEFENSE::cookie_status` iRules command.
use crate::prelude::*;
use tcl_dialect::model::{SpecSurface};
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "BOTDEFENSE::cookie_status",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns the status of the Bot Defense cookie.",
            synopsis: &["BOTDEFENSE::cookie_status"],
            snippet: "Returns the status of the Bot Defense cookie that is received on the request. The returned value is one of the following strings:\n    * not_received - the cookie did not appear in the request\n    * valid - the cookie is valid and not expired\n    * invalid - the cookie cannot be parsed; this could mean that it was modified by an attacker, or that it is older than two days, or due to a configuration change\n    * expired - the cookie is valid, but is expired\n    * valid_redirect_challenge - the cookie of the redirect was validated\n    * renewal - browser challenge answer is about to expire",
            source: "https://clouddocs.f5.com/api/irules/BOTDEFENSE__cookie_status.html",
            examples: "# EXAMPLE: In case of an invalid cookie, send a message to High Speed Logging\nwhen BOTDEFENSE_REQUEST {\n    if {[BOTDEFENSE::cookie_status] eq \"invalid\"} {\n        HSL::send $hsl \"invalid botdefense cookie from IP [IP::client_addr]\"\n    }\n}",
            return_value: "A string signifying the status of the Bot Defense cookie.",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["BOTDEFENSE"],
            also_in: &[],
            flow: false,
        }),
        forms: &[FormSpec {
            synopsis: "BOTDEFENSE::cookie_status",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::AsmState,
            reads: true,
            connection_side: ConnectionSide::Client,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}
