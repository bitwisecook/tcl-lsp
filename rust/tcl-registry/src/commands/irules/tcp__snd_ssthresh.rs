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

//! `TCP::snd_ssthresh` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "TCP::snd_ssthresh",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns the TCP slow start threshold (ssthresh).",
            synopsis: &["TCP::snd_ssthresh"],
            snippet: "The slow start threshold (ssthresh) is the point at which the\ncongestion window (cwnd) grows less aggressively. When the cwnd is\nless than ssthresh, it roughly doubles for every cwnd worth of\nacknowledged data. When cwnd is greater than ssthresh, it increases\nby approximately one MSS for each cwnd worth of acknowledged data.\n\nssthresh starts at 1,073,725,440 bytes unless there is a cmetrics\ncache entry. When TCP detects packet loss it usually sets ssthresh\nto a value between 1/2 cwnd and cwnd, depending on  connection\nconditions and the congestion control algorithm.",
            source: "https://clouddocs.f5.com/api/irules/TCP__snd_ssthresh.html",
            examples: "when CLIENT_CLOSED {\n    # Get BIGIP's last slow-start threshold.\n    log local0. \"BIGIP's ssthresh: [TCP::snd_ssthresh]\"\n}",
            return_value: "The connection slow start threshold in bytes.",
        }),
        forms: &[FormSpec {
            synopsis: "TCP::snd_ssthresh",
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
