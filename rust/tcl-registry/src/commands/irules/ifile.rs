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

//! `ifile` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "ifile",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns content and attributes from external files on the BIG-IP system.",
            synopsis: &["ifile 'listall'", "ifile ("],
            snippet: "This iRules command returns content and attributes from external files\non the BIG-IP system",
            source: "https://clouddocs.f5.com/api/irules/ifile.html",
            examples: "when HTTP_REQUEST {\n   # Retrieve the file contents, send it in an HTTP 200 response and clear the temporary variable\n   set ifileContent [ifile get \"/Common/iFile-index.html\"]\n   HTTP::respond 200 content $ifileContent\n   unset ifileContent\n}",
            return_value: "",
        }),
        forms: &[FormSpec {
            synopsis: "ifile 'listall'",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::FileIo,
            reads: true,
            connection_side: ConnectionSide::Global,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}
