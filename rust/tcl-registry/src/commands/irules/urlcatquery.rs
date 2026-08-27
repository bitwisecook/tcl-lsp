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

//! `urlcatquery` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "urlcatquery",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Query the URL for URL categorization.",
            synopsis: &["urlcatquery URL_STRING"],
            snippet: "This command is similar in functionality to whereis command of geoip.\nThis will be available from HTTP_REQUEST irule event. It takes the URL\nas the input. The input could be a URL string or an IPV4 address. IPV6\naddresses are not currently supported. iRule returns the URL categories\nreturned by the urlcat library.",
            source: "https://clouddocs.f5.com/api/irules/urlcatquery.html",
            examples: "when HTTP_REQUEST {\n    set input_url [HTTP::host][HTTP::uri]\n    set urlcat [urlcatquery  $input_url]\n    log local0. \"INPUT-URL: $input_url\"\n    log local0. \"Category - $urlcat\"\n    CLASSIFY::urlcat add $urlcat\n}",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["FASTHTTP"],
            also_in: &[],
            flow: false,
        }),
        forms: &[FormSpec {
            synopsis: "urlcatquery URL_STRING",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::DataGroup,
            reads: true,
            connection_side: ConnectionSide::Global,
            ..SideEffect::DEFAULT
        }],
        deprecated_replacement: Some("CATEGORY::lookup"),
        ..CommandSpec::DEFAULT
    }
}
