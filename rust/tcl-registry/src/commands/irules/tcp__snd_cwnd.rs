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

//! `TCP::snd_cwnd` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "TCP::snd_cwnd",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns the TCP congestion window (cwnd).",
            synopsis: &["TCP::snd_cwnd"],
            snippet: "Returns the TCP congestion window (cwnd), the maximum\nunacknowledged data the connection can send due to the congestion\ncontrol algorithm.\n\nThe actual amount of outstanding data may be lower, due to lack of\napplication data to send, the remote host's advertised receive\nwindow, or the size of the BIG-IP send buffer.",
            source: "https://clouddocs.f5.com/api/irules/TCP__snd_cwnd.html",
            examples: "when CLIENT_CLOSED {\n    # Get BIGIP's last congestion window.\n    log local0. \"BIGIP's cwnd: [TCP::snd_cwnd]\"\n}",
            return_value: "The cwnd in bytes.",
        }),
        forms: &[FormSpec {
            synopsis: "TCP::snd_cwnd",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::TcpState,
            reads: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}
