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

//! `RTSP::version` iRules command.
use crate::prelude::*;
use tcl_dialect::model::SpecSurface;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "RTSP::version",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns the version in the current RTSP request/response.",
            synopsis: &["RTSP::version"],
            snippet: "Returns the version (for example, RTSP/1.0) in the current RTSP\nrequest/response. You can use this command to determine if RTSP is\nbeing tunneled over HTTP on the RTSP port (the version would be an HTTP\nversion). The command is valid in the RTSP_REQUEST and RTSP_RESPONSE\nevents.",
            source: "https://clouddocs.f5.com/api/irules/RTSP__version.html",
            examples: "when RTSP_REQUEST {\n        puts [RTSP::version]\n    }",
            return_value: "Returns the version in the current RTSP request/response.",
        }),
        forms: &[FormSpec {
            synopsis: "RTSP::version",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::NetworkIo,
            reads: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}
