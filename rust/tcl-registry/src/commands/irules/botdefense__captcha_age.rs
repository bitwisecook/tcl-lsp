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

//! `BOTDEFENSE::captcha_age` iRules command.
use crate::prelude::*;
use tcl_dialect::model::SpecSurface;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "BOTDEFENSE::captcha_age",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns the age of the CAPTCHA challenge in seconds.",
            synopsis: &["BOTDEFENSE::captcha_age"],
            snippet: "Returns the age of the CAPTCHA challenge in seconds. This is only relevant if the value of BOTDEFENSE::captcha_status is \"correct\", \"renewal\" or \"expired\"; otherwise, -1 is returned.",
            source: "https://clouddocs.f5.com/api/irules/BOTDEFENSE__captcha_age.html",
            examples: "# EXAMPLE: Send CAPTCHA challenge and validate the response, but to avoid\n# blocking requests to which CAPTCHA challenge cannot be sent (non-HTML pages),\n# send the CAPTCHA challenge on HTML pages after 30 seconds of aging, which is\n# before the expiration of the answer.\nwhen BOTDEFENSE_ACTION {\n    if {[BOTDEFENSE::action] eq \"allow\"} {\n        if {[BOTDEFENSE::captcha_status] eq \"correct\"} {\n            if {    ([BOTDEFENSE::cs_allowed]) &&",
            return_value: "Returns the age of the CAPTCHA challenge in seconds, or -1 if not applicable.",
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
            synopsis: "BOTDEFENSE::captcha_age",
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
