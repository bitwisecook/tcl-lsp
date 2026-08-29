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

//! `CACHE::accept_encoding` iRules command.
use crate::prelude::*;
use tcl_dialect::model::{SpecSurface};
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "CACHE::accept_encoding",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Overrides the accept_encoding value used by the cache to store the cached content.",
            synopsis: &["CACHE::accept_encoding ENCODING_STRING"],
            snippet: "Overrides the accept_encoding value used by the cache to store the\ncached content. You can use this command to group various user encoding\nvalues into a single group, to minimize duplicated cached content.\n\nCACHE::accept_encoding <string>\n\n     * Overrides the accept_encoding value used by the cache to store the\n       cached content, according to the specified string.",
            source: "https://clouddocs.f5.com/api/irules/CACHE__accept_encoding.html",
            examples: "",
            return_value: "",
        }),
        forms: &[FormSpec {
            synopsis: "CACHE::accept_encoding ENCODING_STRING",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::StreamProfile,
            reads: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}
