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

//! `CATEGORY::result` iRules command.
use crate::prelude::*;
use tcl_dialect::model::{SpecSurface};
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "CATEGORY::result",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns the category or safesearch results retrieved during normal traffic flow.",
            synopsis: &[
                "CATEGORY::result (('category' ('-display' | '-id')? ('custom' | 'request_default' | 'request_default_and_custom')?) | 'safesearch')",
            ],
            snippet: "This iRule command is useful for when it is necessary to know the category or safesearch parameters returned during the categorization in the Category Lookup Agent in the per-request policy. As opposed to CATEGORY::lookup and CATEGORY::safesearch, which each require an additional query to the categorization engine, CATEGORY::result will give back what was found and stored, eliminating the need for additional lookups.\n\nChoose which should be returned (either \"category\" or \"safesearch\"). If \"category\", additional specifications may apply: \"-display\" will return categories in display name format.",
            source: "https://clouddocs.f5.com/api/irules/CATEGORY__result.html",
            examples: "when CATEGORY_MATCHED {\n    set cat [CATEGORY::result category -display request_default_and_custom]\n    log local0. \"Category result retrieved: [lindex $cat 0]\"\n    set ss [CATEGORY::result safesearch]\n    log local0. \"Safe Search result retrieved: [lindex $ss 0], [lindex $ss 1]\"\n}",
            return_value: "Returns a list of categories or safe search parameters. Return format is the same as CATEGORY::lookup and CATEGORY::safesearch.",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["CATEGORY"],
            also_in: &[],
            flow: false,
        }),
        forms: &[FormSpec {
            synopsis: "CATEGORY::result (('category' ('-display' | '-id')? ('custom' | 'request_default' | 'request_default_and_custom')?) | 'safesearch')",
            ..FormSpec::DEFAULT
        }],
        options: const {
            &[
                OptionSpec {
                    name: "-display",
                    value: OptionValue::flag(),
                    detail: "Return categories in display name format.",
                    surface: None,
                    aliases: &[],
                    lifecycle: Lifecycle::UNSPECIFIED,
                    min_abbrev: None,
                },
                OptionSpec {
                    name: "-id",
                    value: OptionValue::flag(),
                    detail: "Return categories in ID format.",
                    surface: None,
                    aliases: &[],
                    lifecycle: Lifecycle::UNSPECIFIED,
                    min_abbrev: None,
                },
            ]
        },
        side_effects: &[SideEffect {
            target: SideEffectTarget::ClassificationState,
            reads: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}
