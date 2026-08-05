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

//! `CATEGORY::lookup` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "CATEGORY::lookup",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Get category of URL.",
            synopsis: &[
                "CATEGORY::lookup URL ('-display' | '-id')? ('custom' | 'request_default' | 'request_default_and_custom')? ('-ip' IP)? ('-custom_cat_match' ANY_CHARS)?",
            ],
            snippet: "This command returns the category of the supplied URL. (requires SWG license)\nThe URL needs to be of the form:\nscheme://domain:port/path?query_string#fragment_id\n\nThe query_string and fragment_id are optional. The entire list of categories supported is available in the UI under \"Secure Web Gateway\" in the APM section. Examples of categories include Sports, Shopping, etc. The response is a list of category names in a TCL array. Most input URLs result in a single category but some will return more than one.",
            source: "https://clouddocs.f5.com/api/irules/CATEGORY__lookup.html",
            examples: "when HTTP_REQUEST {\n        set this_uri http://[HTTP::host][HTTP::uri]\n        set reply [CATEGORY::lookup $this_uri]\n        log local0. \"Category lookup for $this_uri give $reply\"\n    }",
            return_value: "Returns a list of categories returned by the categorization engine depending on the category type specified (custom, request_default, or request_default_and_custom). If no type is specified, it will return request_default.",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "CATEGORY::lookup URL ('-display' | '-id')? ('custom' | 'request_default' | 'request_default_and_custom')? ('-ip' IP)? ('-custom_cat_match' ANY_CHARS)?",
            dialects: None,
        }],
        options: const {
            &[
                OptionSpec {
                    name: "-display",
                    value: OptionValue::flag(),
                    detail: "Return display name of categories.",
                    dialects: None,
                    aliases: &[],
                    lifecycle: Lifecycle::UNSPECIFIED,
                },
                OptionSpec {
                    name: "-id",
                    value: OptionValue::flag(),
                    detail: "Return category IDs.",
                    dialects: None,
                    aliases: &[],
                    lifecycle: Lifecycle::UNSPECIFIED,
                },
                OptionSpec {
                    name: "-ip",
                    value: OptionValue::value("IP"),
                    detail: "IP address to categorize.",
                    dialects: None,
                    aliases: &[],
                    lifecycle: Lifecycle::UNSPECIFIED,
                },
                OptionSpec {
                    name: "-custom_cat_match",
                    value: OptionValue::value("CUSTOM_CAT"),
                    detail: "Match against a specified custom category.",
                    dialects: None,
                    aliases: &[],
                    lifecycle: Lifecycle::UNSPECIFIED,
                },
            ]
        },
        side_effects: &[SideEffect {
            target: SideEffectTarget::ClassificationState,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Both,
            dialects: None,
        }],
        ..CommandSpec::DEFAULT
    }
}
