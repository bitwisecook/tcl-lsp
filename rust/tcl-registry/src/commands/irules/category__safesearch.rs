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

//! `CATEGORY::safesearch` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "CATEGORY::safesearch",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Get safe search key and value pairs.",
            synopsis: &["CATEGORY::safesearch URL ('-ip' IP)?"],
            snippet: "Checks for safe search parameters for the given URL, returns them in list form with the first entry being the key, and the second being the value. Repeated in list for multiple results. (requires SWG license)",
            source: "https://clouddocs.f5.com/api/irules/CATEGORY__safesearch.html",
            examples: "when HTTP_REQUEST {\n    set this_uri http://[HTTP::host][HTTP::uri]\n    set reply [CATEGORY::safesearch $this_uri]\n    set len [llength $reply]\n    if { $len equals 2 } {\n        log local0. \"uri $this_uri returns safesearch key=[lindex $reply 0] and value=[lindex $reply 1]\"\n        if { not([HTTP::uri] contains \"&[lindex $reply 0]=[lindex $reply 1]\") } {\n            HTTP::uri [HTTP::uri]&[lindex $reply 0]=[lindex $reply 1]\n        }\n    }\n}",
            return_value: "Returns a list of alternating key and value pairs. E.g.: [key1, value1, key2, value2]",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["CATEGORY", "FASTHTTP"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[FormSpec {
            synopsis: "CATEGORY::safesearch URL ('-ip' IP)?",
            ..FormSpec::DEFAULT
        }],
        options: const {
            &[OptionSpec {
                name: "-ip",
                value: OptionValue::value(""),
                detail: "Option -ip.",
                dialects: None,
                aliases: &[],
                lifecycle: Lifecycle::UNSPECIFIED,
                min_abbrev: None,
            }]
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
