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

//! `SSL::allow_nonssl` iRules command.
use crate::prelude::*;
use tcl_dialect::model::{SpecSurface};
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "SSL::allow_nonssl",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Get or set Allow Non-SSL connections.",
            synopsis: &["SSL::allow_nonssl (ZERO_ONE)?"],
            snippet: "SSL::allow_nonssl\n  Returns the currently set value for Allow Non-SSL connections\nSSL::allow_nonssl ( 0 | 1 )\n  0 disables Non-SSL Connections, 1 enables it.\n  Allow Non-ssl connections, sets SSL to passthrough mode.",
            source: "https://clouddocs.f5.com/api/irules/SSL__allow_nonssl.html",
            examples: "when CLIENT_ACCEPTED {\n    SSL::allow_nonssl 1\n}",
            return_value: "SSL::allow_nonssl Returns the currently set Allow Non-SSL connections value. SSL::allow_nonssl [0|1] There is no return value.",
        }),
        forms: &[FormSpec {
            synopsis: "SSL::allow_nonssl (ZERO_ONE)?",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::SslState,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}
