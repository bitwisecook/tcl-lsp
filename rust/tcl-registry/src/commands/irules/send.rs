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

//! `send` iRules command.
use crate::prelude::*;
use tcl_dialect::model::{SpecSurface};
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "send",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Sends data on an existing sideband connection.",
            synopsis: &["send (("],
            snippet: "This command sends data on an existing sideband connection (established with connect). It is one of several commands that make up the ability to create sideband connections from iRules.\n\nArguments\n\n    <connection> is the connection identifier returned from connect\n\n    <data> is the data to send\n\n    -timeout ms specifies the amount of time to wait for the data to be sent. The default is an immediate timeout.\n\n    -status varname will save the result of the send command into varname. The possible status values are:\n        1. sent - the data was sent successfully\n        2.",
            source: "https://clouddocs.f5.com/api/irules/send.html",
            examples: "when LB_SELECTED {\n    # Save some data to send\n    set dest \"10.0.16.1:8888\"\n    set data \"GET /mypage/myindex2.html HTTP/1.0\\r\\n\\r\\n\"\n\n    # Open a new TCP connection to $dest\n    set conn_id [connect -protocol TCP -timeout 30000 -idle 30 $dest]\n\n    # Send the data with a 1000ms timeout on the connection identifier received from the connect command\n    set send_bytes [send -timeout 1000 -status send_status $conn_id $data]\n\n    # Log the number of bytes sent and the send status",
            return_value: "Sends data on a specified sideband connection, and returns an integer representing the amount of data that was sent.",
        }),
        forms: &[FormSpec {
            synopsis: "send ?options? ?--? connection data",
            ..FormSpec::DEFAULT
        }],
        options: const {
            &[
                OptionSpec {
                    name: "-timeout",
                    value: OptionValue::value("MSEC"),
                    detail: "Time in ms to wait for data to be sent.",
                    surface: None,
                    aliases: &[],
                    lifecycle: Lifecycle::UNSPECIFIED,
                    min_abbrev: None,
                },
                OptionSpec {
                    name: "-status",
                    value: OptionValue::value("VARIABLE"),
                    detail: "Save send status into variable.",
                    surface: None,
                    aliases: &[],
                    lifecycle: Lifecycle::UNSPECIFIED,
                    min_abbrev: None,
                },
                OptionSpec {
                    name: "--",
                    value: OptionValue::flag(),
                    detail: "",
                    surface: None,
                    aliases: &[],
                    lifecycle: Lifecycle::UNSPECIFIED,
                    min_abbrev: None,
                },
            ]
        },
        side_effects: &[SideEffect {
            target: SideEffectTarget::NetworkIo,
            writes: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}
