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

//! `RTSP::respond` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "RTSP::respond",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Sends an RTSP response to the client.",
            synopsis: &["RTSP::respond STATUS_CODE STATUS_STRING (HEADERS)?"],
            snippet: "Sends an RTSP response to the client. The return value of the\nRTSP::msg_source command must be client. When an iRule responds to an\nRTSP request, the RTSP filter performs no further processing on the\nrequest and will not send the RTSP request to the server.\nA maximum of one response is allowed per RTSP request.",
            source: "https://clouddocs.f5.com/api/irules/RTSP__respond.html",
            examples: "when RTSP_REQUEST {\n        RTSP::respond 401 Unauthorized \"x-header\\r\\n\\r\\n  Hey, you need a password\"\n    }",
            return_value: "",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "RTSP::respond STATUS_CODE STATUS_STRING (HEADERS)?",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::NetworkIo,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::Both,
            dialects: None,
        }],
        ..CommandSpec::DEFAULT
    }
}
