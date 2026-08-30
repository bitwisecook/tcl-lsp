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

//! `ASM::captcha_status` iRules command.
use crate::prelude::*;
use tcl_dialect::model::SpecSurface;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "ASM::captcha_status",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns the status of the user's answer to the CAPTCHA challenge.",
            synopsis: &["ASM::captcha_status"],
            snippet: "Returns the status of the user's answer to the CAPTCHA challenge. The returned value is one of the following strings:\n            not_received - the answer to the CAPTCHA challenge did not appear in the request; this is the normal result, before the CAPTCHA challenge is sent to the client\n            correct - the answer is correct\n            incorrect - the answer is incorrect\n            empty - an empty answer was given, or if the user clicked on the CAPTCHA Refresh button",
            source: "https://clouddocs.f5.com/api/irules/ASM__captcha_status.html",
            examples: "# EXAMPLE: Send a CAPTCHA challenge on the login page, and only allow the\n            # login if the user passed the CAPTCHA challenge\n            when ASM_REQUEST_DONE {\n                if {[ASM::captcha_status] ne \"correct\"} {\n                    if {[HTTP::uri] eq \"/t/login.php\"} {\n                        set res [ASM::captcha]\n                        if {$res ne \"ok\"} {\n                            log local0. \"cannot send captcha_challenge: \\\"$res\\\"\"\n                        }",
            return_value: "Returns a string signifying the status of the CAPTCHA challenge.",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["ASM"],
            also_in: &[],
            flow: false,
        }),
        forms: &[FormSpec {
            synopsis: "ASM::captcha_status",
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
