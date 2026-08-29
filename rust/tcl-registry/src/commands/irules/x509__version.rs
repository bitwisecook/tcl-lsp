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

//! `X509::version` iRules command.
use crate::prelude::*;
use tcl_dialect::model::{SpecSurface};
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "X509::version",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns the version number of an X509 certificate.",
            synopsis: &["X509::version CERTIFICATE"],
            snippet: "Returns the version number of the specified X509 certificate (an\ninteger).",
            source: "https://clouddocs.f5.com/api/irules/X509__version.html",
            examples: "when HTTP_REQUEST {\n  log local0. \"Cert version - [X509::version ssl_cert]\"\n  if { [X509::version ssl_cert] eq 3 } {\n    pool v3_pool\n  } else {\n    pool default_pool\n  }\n}",
            return_value: "Returns the version number of an X509 certificate.",
        }),
        forms: &[FormSpec {
            synopsis: "X509::version CERTIFICATE",
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
