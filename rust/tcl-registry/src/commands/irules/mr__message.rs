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

//! `MR::message` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "MR::message",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns or sets details in the message routing table.",
            synopsis: &[
                "MR::message clone (CLONE_ID)+",
                "MR::message clone -count CLONE_COUNT",
            ],
            snippet: "Clones the message a number of times (one for each CLONE_ID) and dispatches each cloned\nmessage as ingress. After the original message has completed the event in which this command\nis executed, each cloned message executes the MR_INGRESS iRule event for itself.\n(CLONE_ID)+ can be one or more strings separated by space.\nProtection against infinite loops should be considered!\nReturns the clone_count, see below, (allowed only at MR_INGRESS).\n            \nClones the message CLONE_COUNT number of times and dispatches each cloned\nmessage as ingress.",
            source: "https://clouddocs.f5.com/api/irules/MR__message.html",
            examples: "# Example 1\nwhen MR_INGRESS {\n    if {[GENERICMESSAGE::message is_request] != 0} {\n        set host [MR::message pick_host peer /Common/mypeer]\n        MR::message route config tcp_tc host $host\n    }\n}",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["MR"],
            also_in: &[],
            flow: false,
        }),
        forms: &[FormSpec {
            synopsis: "MR::message clone (CLONE_ID)+",
            ..FormSpec::DEFAULT
        }],
        options: const {
            &[OptionSpec {
                name: "-count",
                value: OptionValue::value(""),
                detail: "Option -count.",
                dialects: None,
                aliases: &[],
                lifecycle: Lifecycle::UNSPECIFIED,
                min_abbrev: None,
            }]
        },
        side_effects: &[SideEffect {
            target: SideEffectTarget::MessageState,
            writes: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}
