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

//! `X509::subject_public_key_RSA_bits` iRules command.
use crate::prelude::*;
use tcl_dialect::model::SpecSurface;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "X509::subject_public_key_RSA_bits",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns the size of the subjectXs public RSA key of an X509 certificate.",
            synopsis: &["X509::subject_public_key_RSA_bits CERTIFICATE"],
            snippet: "Returns the size, in bits, of the subject’s public RSA key of the\nspecified X509 certificate. This command is only applicable when the\npublic key type is RSA. Otherwise, the command generates an error.",
            source: "https://clouddocs.f5.com/api/irules/X509__subject_public_key_RSA_bits.html",
            examples: "when HTTP_REQUEST {\n  if { [info exist error_code] } {\n    if { $error_code > 0 } {\n      HTTP::redirect \"https://some_other_site/\"\n    }\n  }\n}",
            return_value: "Returns the size of the subject’s public RSA key of an X509 certificate.",
        }),
        forms: &[FormSpec {
            synopsis: "X509::subject_public_key_RSA_bits CERTIFICATE",
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
