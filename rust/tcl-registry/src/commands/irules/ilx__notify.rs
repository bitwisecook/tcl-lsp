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

//! `ILX::notify` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "ILX::notify",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Calls an ILX method asynchronously.",
            synopsis: &["ILX::notify HANDLE METHOD (ARGS)*"],
            snippet: "Make a call to the plugin extension defined by the handle but do not wait for a response before continuing to process the remainder of the iRule. The delivery of the call to the plugin extension is \"best effort\" and is not guaranteed.",
            source: "https://clouddocs.f5.com/api/irules/ILX__notify.html",
            examples: "when CLIENT_ACCEPTED {\n    # Get a handle to the running extension instance to call into.\n    set RPC_HANDLE [ILX::init my_plugin my_extension]\n    # Make the asynchronous call\n    ILX::notify $RPC_HANDLE my_js_function arg1 arg2\n}",
            return_value: "None",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "ILX::notify HANDLE METHOD (ARGS)*",
        }],
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
