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

//! `URI::escape` iRules command.
use crate::prelude::*;
use tcl_dialect::model::SpecSurface;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "URI::escape",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Percent-encodes a URI string (alias for URI::encode).",
            synopsis: &["URI::escape URI_STRING"],
            snippet: "Percent-encodes *URI_STRING* according to RFC 3986.\nThis is an alias for ``URI::encode``.",
            source: "https://clouddocs.f5.com/api/irules/URI__encode.html",
            examples: "",
            return_value: "Returns a percent-encoded URI string.",
        }),
        // URL-encodes its input (and strips CR/LF);
        // re-encoding a URL-encoded value double-encodes (T106).
        taint_transform: Some(TaintColour::URL_ENCODED.union(TaintColour::CRLF_FREE)),
        taint_double_encode_colour: Some(TaintColour::URL_ENCODED),
        forms: &[FormSpec {
            synopsis: "URI::escape URI_STRING",
            ..FormSpec::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}
