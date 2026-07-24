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

//! `X509::hash` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "X509::hash",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns the MD5 hash (fingerprint) of an X509 certificate.",
            synopsis: &["X509::hash CERTIFICATE"],
            snippet: "Returns the MD5 hash (fingerprint) of the specified X509 certificate.",
            source: "https://clouddocs.f5.com/api/irules/X509__hash.html",
            examples: "when HTTP_REQUEST {\n  if { [info exist cert_hash] } {\n    if { $cert_hash equals \"XX:XX:XX:XX:XX:XX:XX:XX:XX:XX:XX:XX:XX:XX:XX:XX\"} {\n      HTTP::redirect \"https://somesite/\"\n    } else {\n      HTTP::redirect \"https://someothersite/\"\n    }\n  }\n}",
            return_value: "Returns the MD5 hash (fingerprint) of an X509 certificate.",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "X509::hash CERTIFICATE",
            dialects: None,
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::SslState,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Global,
            dialects: None,
        }],
        ..CommandSpec::DEFAULT
    }
}
