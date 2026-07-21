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

//! `TCP::enhanced_loss_recovery` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "TCP::enhanced_loss_recovery",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Toggles enhanced loss recovery.",
            synopsis: &["TCP::enhanced_loss_recovery BOOL_VALUE"],
            snippet: "Enables or disables enhanced loss recovery which recovers from random packet loss more effectively.",
            source: "https://clouddocs.f5.com/api/irules/TCP__enhanced_loss_recovery.html",
            examples: "when CLIENT_ACCEPTED {\n    TCP::enhanced_loss_recovery enable\n}",
            return_value: "None.",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "TCP::enhanced_loss_recovery BOOL_VALUE",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::TcpState,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::Both,
            dialects: None,
        }],
        ..CommandSpec::DEFAULT
    }
}
