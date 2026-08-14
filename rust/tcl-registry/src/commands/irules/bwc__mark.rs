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

//! `BWC::mark` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "BWC::mark",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "This command allows you to set or unset marking for traffic flows in bwc when configured rate limit is exceeded.",
            synopsis: &[
                "BWC::mark SESSION_ID ('qos'|'tos') (BWC_VALUE | 'passthrough')",
                "BWC::mark SESSION_ID APP_NAME ('qos'|'tos') (BWC_VALUE | 'passthrough')",
            ],
            snippet: "This command allows you to set or unset marking for traffic flows in bwc when configured rate limit is exceeded. Marking can be on DSCP (ToS - L3) and/or QoS (L2). The ToS/QoS value needs to be in valid range and can be passthrough.",
            source: "https://clouddocs.f5.com/api/irules/BWC__mark.html",
            examples: "when CLIENT_ACCEPTED {\n    set mycookie [IP::remote_addr]:[TCP::remote_port]\n    BWC::policy attach gold_user $mycookie\n    BWC::color set gold_user p2p\n}",
            return_value: "",
        }),
        forms: &[FormSpec {
            synopsis: "BWC::mark SESSION_ID ('qos'|'tos') (BWC_VALUE | 'passthrough')",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::ConnectionControl,
            writes: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}
