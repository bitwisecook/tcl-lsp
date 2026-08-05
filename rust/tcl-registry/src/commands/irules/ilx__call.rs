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

//! `ILX::call` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "ILX::call",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Calls an ILX method.",
            synopsis: &["ILX::call HANDLE ?-timeout ms? ?--? METHOD ?args ...?"],
            snippet: "Make a call to a method defined within the plugin extension referenced by the handle.  Provide the method with the arguments listed in ARGS, do not continue processing the iRule until a response is received.",
            source: "https://clouddocs.f5.com/api/irules/ILX__call.html",
            examples: "when CLIENT_ACCEPTED {\n    # Get a handle to the running extension instance to call into.\n    set RPC_HANDLE [ILX::init my_plugin my_extension]\n    # Make the call and store the response in $rpc_response\n    set rpc_response [ILX::call $RPC_HANDLE my_js_function arg1 arg2]\n}",
            return_value: "The return value is the argument passed to response.reply() call on the extension side (eg. an array, a string, etc).",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "ILX::call HANDLE ?-timeout ms? ?--? METHOD ?args ...?",
            dialects: None,
        }],
        options: const {
            &[
                OptionSpec {
                    name: "-timeout",
                    value: OptionValue::value("MSEC"),
                    detail: "Timeout in milliseconds.",
                    dialects: None,
                    aliases: &[],
                    lifecycle: Lifecycle::UNSPECIFIED,
                    min_abbrev: None,
                },
                OptionSpec {
                    name: "--",
                    value: OptionValue::flag(),
                    detail: "",
                    dialects: None,
                    aliases: &[],
                    lifecycle: Lifecycle::UNSPECIFIED,
                    min_abbrev: None,
                },
            ]
        },
        side_effects: &[SideEffect {
            target: SideEffectTarget::NetworkIo,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::Both,
            dialects: None,
        }],
        ..CommandSpec::DEFAULT
    }
}
