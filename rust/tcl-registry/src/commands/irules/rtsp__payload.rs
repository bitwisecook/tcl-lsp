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

//! `RTSP::payload` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "RTSP::payload",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Queries for or replaces content information.",
            synopsis: &[
                "RTSP::payload (LENGTH | length)?",
                "RTSP::payload replace OFFSET LENGTH RTSP_PAYLOAD",
            ],
            snippet: "Queries for or replaces content information. With this command, you can\nretrieve content, query for content size, or replace a certain amount\nof content.",
            source: "https://clouddocs.f5.com/api/irules/RTSP__payload.html",
            examples: "when RTSP_REQUEST {\n        RTSP::collect\n    }",
            return_value: "",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "RTSP::payload (LENGTH | length)?",
            dialects: None,
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::NetworkIo,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Both,
            dialects: None,
        }],
        taint_source: Some(TaintColour::TAINTED),
        data_collection: Some(RTSP_PAYLOAD),
        ..CommandSpec::DEFAULT
    }
}
