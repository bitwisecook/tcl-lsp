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

//! `RTSP::header` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "RTSP::header",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Manages headers in RTSP requests and responses.",
            synopsis: &[
                "RTSP::header (exists | remove | value) HEADER_NAME",
                "RTSP::header replace HEADER_NAME HEADER_VALUE",
                "RTSP::header insert (<(HEADER_NAME HEADER_VALUE)+> |",
            ],
            snippet: "Manages headers in RTSP requests and responses.",
            source: "https://clouddocs.f5.com/api/irules/RTSP__header.html",
            examples: "when RTSP_REQUEST {\n        puts [RTSP::header value \"x-header\"]\n    }",
            return_value: "",
        }),
        forms: &[FormSpec {
            synopsis: "RTSP::header (exists | remove | value) HEADER_NAME",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::NetworkIo,
            reads: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        taint_source: Some(TaintColour::TAINTED),
        ..CommandSpec::DEFAULT
    }
}
