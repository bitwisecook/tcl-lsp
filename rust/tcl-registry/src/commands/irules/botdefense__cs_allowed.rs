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

//! `BOTDEFENSE::cs_allowed` iRules command.
use crate::prelude::*;
use tcl_dialect::model::{SpecSurface};
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "BOTDEFENSE::cs_allowed",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns or sets whether it is allowed for Bot Defense to take a client-side action.",
            synopsis: &["BOTDEFENSE::cs_allowed (BOOLEAN)?"],
            snippet: "Returns or sets if the client-side actions (browser challenge or CAPTCHA challenge or allow_and_browser_challenge_in_response or Device-ID) are allowed on this request. By default, only requests to URLs which are detected as HTML URLs are cs_allowed (qualified), but this command can override this detection.\n\nUsing this command during BOTDEFENSE_ACTION event to modify the value will have no effect and will be ignored.\n\nUsing this command during BOTDEFENSE_REQUEST or HTTP_REQUEST event to modify the value will override the detection.",
            source: "https://clouddocs.f5.com/api/irules/BOTDEFENSE__cs_allowed.html",
            examples: "# EXAMPLE: Always allow client-side actions to be taken on URLs with the .html extension.\nwhen BOTDEFENSE_REQUEST {\n    if {[HTTP::uri] ends_with \".html\"} {\n        BOTDEFENSE::cs_allowed true\n    }\n}",
            return_value: "* When called without any arguments: Returns whether a client-side action is allowed to be taken by Bot Defense. If the value was overridden, the returned value is the overridden one. * When called with an argument for overriding the value of cs_allowed, no value is returned.",
        }),
        forms: &[FormSpec {
            synopsis: "BOTDEFENSE::cs_allowed (BOOLEAN)?",
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
