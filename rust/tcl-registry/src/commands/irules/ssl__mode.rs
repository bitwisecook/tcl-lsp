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

//! `SSL::mode` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "SSL::mode",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Gets the enabled/disabled state of SSL.",
            synopsis: &["SSL::mode"],
            snippet: "Gets the enabled/disabled state of SSL",
            source: "https://clouddocs.f5.com/api/irules/SSL__mode.html",
            examples: "when CLIENT_ACCEPTED {\n    if { [TCP::local_port] != 443 } {\n        SSL::disable\n    }\n}",
            return_value: "SSL::mode Gets the enabled/disabled state of SSL. Returns 1 if it is enabled, and 0 if it is disabled.",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "SSL::mode",
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
