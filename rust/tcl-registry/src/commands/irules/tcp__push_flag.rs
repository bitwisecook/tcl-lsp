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

//! `TCP::push_flag` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "TCP::push_flag",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "This command can be used to set/get the PUSH flag mode of a TCP connection.",
            synopsis: &["TCP::push_flag ('default' | 'none' | 'one' | 'auto')?"],
            snippet: "TCP::push_flag returns the PUSH flag mode of a TCP connection.\nTCP::push_flag mode sets the PUSH flag mode to specified mode.",
            source: "https://clouddocs.f5.com/api/irules/TCP__push_flag.html",
            examples: "# get/set the PUSH flag mode of the TCP flow.\nwhen CLIENT_ACCEPTED {\n    log local0. \"TCP set PUSH flag mode: [TCP::push_flag auto]\"\n    log local0. \"TCP get PUSH flag more: [TCP::push_flag]\"\n}",
            return_value: "TCP::push_flag returns the PUSH flag mode.",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "TCP::push_flag ('default' | 'none' | 'one' | 'auto')?",
            dialects: None,
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::TcpState,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Both,
            dialects: None,
        }],
        ..CommandSpec::DEFAULT
    }
}
