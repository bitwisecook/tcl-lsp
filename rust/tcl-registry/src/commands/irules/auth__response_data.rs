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

//! `AUTH::response_data` iRules command.
use crate::prelude::*;
use tcl_dialect::model::{SpecSurface};
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "AUTH::response_data",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns pairwise auth query results.",
            synopsis: &["AUTH::response_data (AUTH_ID)?"],
            snippet: "AUTH::response_data returns the a set of name/value query results from\nthe most recent query. This command would normally be called from the\nAUTH_RESULT event. The format of the data returned is suitable for\nsetting as the value of a TCL array.\nAUTH::subscribe must first be called to register interest in query\nresults prior to calling AUTH::authenticate. As a convenience when\nusing the builtin system auth rules, these rules will call\nAUTH::subscribe if the variable tmm_auth_subscription is set.",
            source: "https://clouddocs.f5.com/api/irules/AUTH__response_data.html",
            examples: "when CLIENT_ACCEPTED {\n        set tmm_auth_subscription \"*\"\n    }",
            return_value: "",
        }),
        forms: &[FormSpec {
            synopsis: "AUTH::response_data (AUTH_ID)?",
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
