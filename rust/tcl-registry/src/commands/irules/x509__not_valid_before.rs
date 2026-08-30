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

//! `X509::not_valid_before` iRules command.
use crate::prelude::*;
use tcl_dialect::model::SpecSurface;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "X509::not_valid_before",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns the not-valid-before date of an X509 certificate.",
            synopsis: &["X509::not_valid_before CERTIFICATE"],
            snippet: "Returns the not-valid-before date of the specified X509 certificate.",
            source: "https://clouddocs.f5.com/api/irules/X509__not_valid_before.html",
            examples: "when SERVERSSL_HANDSHAKE {\n  set server_cert [SSL::cert 0]\n  log local0. \"Server Certificate Valid Date -\n   [X509::not_valid_before $server_cert] -\n   [X509::not_valid_after $server_cert]\"\n}",
            return_value: "Returns the not-valid-before date of an X509 certificate.",
        }),
        forms: &[FormSpec {
            synopsis: "X509::not_valid_before CERTIFICATE",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::SslState,
            reads: true,
            connection_side: ConnectionSide::Global,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}
