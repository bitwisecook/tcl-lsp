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

//! `recv` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "recv",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Receives data from a given sideband connection.",
            synopsis: &["recv (("],
            snippet: "This command receives data from an existing sideband connection (established with connect). It is one of several commands that make up the ability to create sideband connections from iRules.",
            source: "https://clouddocs.f5.com/api/irules/recv.html",
            examples: "when HTTP_REQUEST {\n    set dest \"10.10.145.1:80\"\n    set conn [connect -protocol TCP -timeout 30000 -idle 30 $dest]\n    set data \"GET /mypage/index.html HTTP/1.0\\r\\n\\r\\n\"\n    send $conn $data\n    # recv 2000 chars get recv bytes error\n    set recv_2kbytes [recv -timeout 30000 2000 $conn]\n    log local0. \"Recv data: '$recv_2kbytes'\"\n    set length_recv [string length $recv_2kbytes]\n    log local0. \"length_recv has $length_recv bytes in it.\"\n    close $conn\n}",
            return_value: "Receives some data (up to numChars bytes) from a sideband connection. If varname is specified, the response is stored in that variable, and recv returns the amount of data received. Otherwise, recv returns the response data.",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "recv ?options? ?numChars? connection ?varname?",
        }],
        options: const {
            &[
                OptionSpec {
                    name: "-eol",
                    value: OptionValue::flag(),
                    detail: "Suspend until end-of-line received.",
                    dialects: None,
                    aliases: &[],
                    min_version: None,
                },
                OptionSpec {
                    name: "-peek",
                    value: OptionValue::flag(),
                    detail: "Return data but leave it buffered.",
                    dialects: None,
                    aliases: &[],
                    min_version: None,
                },
                OptionSpec {
                    name: "-timeout",
                    value: OptionValue::value("MSEC"),
                    detail: "Time in ms to wait for data.",
                    dialects: None,
                    aliases: &[],
                    min_version: None,
                },
                OptionSpec {
                    name: "-status",
                    value: OptionValue::value("VARIABLE"),
                    detail: "Save recv status into variable.",
                    dialects: None,
                    aliases: &[],
                    min_version: None,
                },
            ]
        },
        side_effects: &[SideEffect {
            target: SideEffectTarget::NetworkIo,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Both,
            dialects: None,
        }],
        ..CommandSpec::DEFAULT
    }
}
