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

//! `FTP::enforce_tls_session_reuse` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "FTP::enforce_tls_session_reuse",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Get or set the state of enforcing TLS session reuse.",
            synopsis: &["FTP::enforce_tls_session_reuse (enable | disable)?"],
            snippet: "Enable or disable enforcing TLS session reuse, when enabled, Bigip rejects the data connection if it fails to reuse existed TLS session. Returns the current status if no option is specified.",
            source: "https://clouddocs.f5.com/api/irules/FTP__enforce_tls_session_reuse.html",
            examples: "when CLIENT_ACCEPTED {\n                FTP::enforce_tls_session_reuse enable\n            }",
            return_value: "",
        }),
        forms: &[FormSpec {
            synopsis: "FTP::enforce_tls_session_reuse (enable | disable)?",
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
