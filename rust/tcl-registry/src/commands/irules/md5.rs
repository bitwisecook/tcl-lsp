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

//! `md5` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "md5",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns the RSA MD5 Message Digest Algorithm message digest of the specified string.",
            synopsis: &["md5 ANY_CHARS"],
            snippet: "Returns the RSA Data Security, Inc. MD5 Message Digest Algorithm (md5) message digest of the specified string, or if an error occurs, an empty string. Used to ensure data integrity.",
            source: "https://clouddocs.f5.com/api/irules/md5.html",
            examples: "when HTTP_REQUEST {\n    binary scan [md5 [HTTP::host]] w1 key\n\n    set key [expr {$key & 1}]\n    switch $key {\n        0 { pool my_pool member 1.2.3.4:80 }\n        1 { pool my_pool member 5.6.7.8:80 }\n    }\n}",
            return_value: "md5 <string> Returns the RSA Data Security, Inc. MD5 Message Digest Algorithm (md5) message digest of the specified string, or if an error occurs, an empty string.",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "md5 ANY_CHARS",
            dialects: None,
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::Unknown,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Global,
            dialects: None,
        }],
        ..CommandSpec::DEFAULT
    }
}
