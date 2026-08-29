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

//! `ILX::init` iRules command.
use crate::prelude::*;
use tcl_dialect::model::{SpecSurface};
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "ILX::init",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Creates a handle to a running ILX plugin extension.",
            synopsis: &["ILX::init (EXTENSION | (PLUGIN EXTENSION))"],
            snippet: "Creates a handle for future use by ILX::call and ILX::notify.  This handle is a reference to a running ILX plugin extension.  The lifetime of this variable affects the behavior of the ILX target if controlled by BIG-IP.  Instances of the plugin extension will be held in draining mode as long as there are open references to the ILX handle in any event.",
            source: "https://clouddocs.f5.com/api/irules/ILX__init.html",
            examples: "when CLIENT_ACCEPTED {\n    # Get a handle to the running extension instance to call into.\n    set RPC_HANDLE [ILX::init my_plugin my_extension]\n    # Make the call and store the response in $rpc_response\n    set rpc_response [ILX::call $RPC_HANDLE my_js_function arg1 arg2]\n}",
            return_value: "Returns a handle to the running extension to call into.",
        }),
        forms: &[FormSpec {
            synopsis: "ILX::init (EXTENSION | (PLUGIN EXTENSION))",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::NetworkIo,
            writes: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}
