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

//! `URI::encode_component` iRules command.
use crate::prelude::*;
use tcl_dialect::model::{SpecSurface};
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "URI::encode_component",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Percent-encodes a single URI component.",
            synopsis: &["URI::encode_component STRING"],
            snippet: "Percent-encodes a single URI component (path segment, query\nparameter name or value, fragment, etc.) according to RFC 3986\nsection 2.1.  Unlike ``URI::encode`` this encodes every\nreserved delimiter (``/``, ``?``, ``&``, ``=``, …) so the\nresult is safe to embed inside a larger URI without altering\nits structure.",
            source: "https://clouddocs.f5.com/api/irules/URI__encode.html",
            examples: "when HTTP_REQUEST {\n  set value \"key=value&other\"\n  HTTP::uri \"/search?q=[URI::encode_component $value]\"\n}",
            return_value: "Returns a percent-encoded string.",
        }),
        // URL-encodes its input (and strips CR/LF);
        // re-encoding a URL-encoded value double-encodes (T106).
        taint_transform: Some(TaintColour::URL_ENCODED.union(TaintColour::CRLF_FREE)),
        taint_double_encode_colour: Some(TaintColour::URL_ENCODED),
        forms: &[FormSpec {
            synopsis: "URI::encode_component STRING",
            ..FormSpec::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}
