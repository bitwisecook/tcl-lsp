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

//! `TCP::rcv_size` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "TCP::rcv_size",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns the maximum allowed advertised window size by BIG-IP.",
            synopsis: &["TCP::rcv_size"],
            snippet: "TCP configuration limits the advertised received window to control the memory impact of any single connection.",
            source: "https://clouddocs.f5.com/api/irules/TCP__rcv_size.html",
            examples: "when CLIENT_CLOSED {\n    # Get BIGIP's receive window size.\n    log local0. \"BIGIP's rcv wnd size: [TCP::rcv_size]\"\n}",
            return_value: "The maximum receive window in bytes.",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "TCP::rcv_size",
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
