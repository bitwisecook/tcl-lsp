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

//! `link_qos` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "link_qos",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns the QoS level.",
            synopsis: &["link_qos (QOS_LEVEL)?"],
            snippet: "Returns the QoS level. The Quality of Service (QoS) standard is a means\nby which network equipment can identify and treat traffic differently\nbased on an identifier. As traffic enters the site, the BIG-IP system\ncan apply an iRule that sends the traffic to different pools of servers\nbased on the QoS level within a packet.\nThis is a BIG-IP version 4.X variable, provided for\nbackward-compatibility. You can use the equivalent 9.X command\nLINK::qos instead.",
            source: "https://clouddocs.f5.com/api/irules/link_qos.html",
            examples: "",
            return_value: "",
        }),
        forms: &[FormSpec {
            synopsis: "link_qos (QOS_LEVEL)?",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::TcpState,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        deprecated_replacement: Some("LINK::qos"),
        deprecated_replacement_drop_in: true,
        ..CommandSpec::DEFAULT
    }
}
