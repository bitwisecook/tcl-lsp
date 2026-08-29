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

//! `SSL::secure_renegotiation` iRules command.
use crate::prelude::*;
use tcl_dialect::model::SpecSurface;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "SSL::secure_renegotiation",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Controls the SSL Secure Renegotiation mode.",
            synopsis: &["SSL::secure_renegotiation (request | require | require-strict)?"],
            snippet: "Controls the SSL Secure Renegotiation mode.",
            source: "https://clouddocs.f5.com/api/irules/SSL__secure_renegotiation.html",
            examples: "when CLIENTSSL_CLIENTHELLO {\n                if { [SSL::secure_renegotiation] != 2 } {\n                    SSL::secure_renegotiation require-strict\n                }\n            }",
            return_value: "The getter returns the flow's current Secure Renegotiation mode: zero for request, one for require, or two for require-strict. The request, require, and require-strict arguments set the mode for subsequent SSL handshakes on the flow.",
        }),
        forms: &[FormSpec {
            synopsis: "SSL::secure_renegotiation (request | require | require-strict)?",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::SslState,
            reads: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}
