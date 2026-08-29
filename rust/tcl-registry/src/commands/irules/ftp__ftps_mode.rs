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

//! `FTP::ftps_mode` iRules command.
use crate::prelude::*;
use tcl_dialect::model::SpecSurface;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "FTP::ftps_mode",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Get or set the activation mode for FTPS.",
            synopsis: &["FTP::ftps_mode (disallow | allow | require)?"],
            snippet: "Sets the FTPS activation mode to disallow (FTP commands \"AUTH SSL/TLS\" will be filtered out, and implicit FTPS connection will be dropped), allow (FTP will optionally activate TLS if client or server support \"AUTH SSL/TLS\"), or require (FTP will require that the client and server complete \"AUTH SSL/TLS\" before data transfers).",
            source: "https://clouddocs.f5.com/api/irules/FTP__ftps_mode.html",
            examples: "when CLIENT_ACCEPTED {\n                if { ([IP::addr [IP::client_addr] equals 10.0.0.0/8]) } {\n                    FTP::ftps_mode require\n                }\n\n                if { ([IP::addr [IP::client_addr] equals 10.0.0.0/8]) } {\n                    set mode [FTP::ftps_mode]\n                }\n            }",
            return_value: "Returns the current activation mode.",
        }),
        forms: &[FormSpec {
            synopsis: "FTP::ftps_mode (disallow | allow | require)?",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::FtpState,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}
