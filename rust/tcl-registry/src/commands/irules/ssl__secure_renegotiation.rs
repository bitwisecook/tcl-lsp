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
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "SSL::secure_renegotiation",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Controls the SSL Secure Renegotiation mode.",
            synopsis: &["SSL::secure_renegotiation (request | require | require-strict)?"],
            snippet: "Controls the SSL Secure Renegotiation mode.",
            source: "https://clouddocs.f5.com/api/irules/SSL__secure_renegotiation.html",
            examples: "when CLIENTSSL_CLIENTHELLO {\n                if { [SSL::secure_renegotiation] != 2 } {\n                    SSL::secure_renegotiation require-strict\n                }\n            }",
            return_value: "SSL::secure_renegotiation¶ Get the current Secure Renegotiation mode for the flow. A return value of zero denotes request mode. A value of one denotes require mode. A value of two denotes require-strict mode.",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "SSL::secure_renegotiation (request | require | require-strict)?",
            dialects: None,
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::SslState,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Both,
            dialects: None,
        }],
        ..CommandSpec::DEFAULT
    }
}
