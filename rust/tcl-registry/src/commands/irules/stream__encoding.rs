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

//! `STREAM::encoding` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "STREAM::encoding",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Specifies non-default content encoding.",
            synopsis: &["STREAM::encoding (ascii | utf-8 | unicode)"],
            snippet: "Specifies non-default content encoding. The default value is ascii.",
            source: "https://clouddocs.f5.com/api/irules/STREAM__encoding.html",
            examples: "when STREAM_MATCHED {\n    set stream_match [STREAM::match]\n    log local0. \"$stream_match\"\n    STREAM::encoding utf-8\n    # The ?/? represents unicode characters.\n    if { $stream_match contains \"hello?/?\" } {\n        STREAM::replace \"hello hey\"\n        log local0. \"stream match is [STREAM::match]\"\n    }\n}",
            return_value: "",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "STREAM::encoding (ascii | utf-8 | unicode)",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::StreamProfile,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::Both,
            dialects: None,
        }],
        ..CommandSpec::DEFAULT
    }
}
