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

//! `BOTDEFENSE::cs_possible` iRules command.
use crate::prelude::*;
use tcl_dialect::model::{SpecSurface};
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "BOTDEFENSE::cs_possible",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns whether it is possible for Bot Defense to take a client-side action.",
            synopsis: &["BOTDEFENSE::cs_possible"],
            snippet: "Returns \"true\" or \"false\" based on whether it is possible to take one of the client-side actions that initiate a response (browser challenge, or CAPTCHA challenge, or device id collection) or send browser challenge in response. Certain characteristics of a request make it impossible to respond with a browser verification or CAPTCHA challenge or device id, in which case \"false\" is returned.\n\nSetting to a client-side action with BOTDEFENSE::action, while the value of BOTDEFENSE::cs_possible is \"false\", will fail.",
            source: "https://clouddocs.f5.com/api/irules/BOTDEFENSE__cs_possible.html",
            examples: "# EXAMPLE: Prevent blocking of requests that cannot be responded with a\n# client-side challenge.\nwhen BOTDEFENSE_ACTION {\n    if {    ([BOTDEFENSE::action] eq \"tcp_rst\") &&\n            (not [BOTDEFENSE::cs_possible])} {\n        BOTDEFENSE::action allow\n    }\n}",
            return_value: "Returns a boolean value (0 or 1), whether taking a client-side action is possible.",
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
            synopsis: "BOTDEFENSE::cs_possible",
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
