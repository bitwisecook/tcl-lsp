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

//! `ONECONNECT::select` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "ONECONNECT::select",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Instruct the proxy to use persistence data as a OneConnect keying label when connecting to a server.",
            synopsis: &["ONECONNECT::select (persist | none)"],
            snippet: "The 'select persist' command instructs the proxy to use persistence data as the\nOneConnect keying label when connecting to the server. NTLM connection pooling\nleverages these commands internally, and it is not necessary for the user to\nuse them directly.  Persistance data should be established via the 'persist'\ncommand.",
            source: "https://clouddocs.f5.com/api/irules/ONECONNECT__select.html",
            examples: "when HTTP_REQUEST_SEND {\n     if { $keymatch == \"/myuri\"} {\n     ONECONNECT::label update $keymatch\n     }\n   }",
            return_value: "",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "ONECONNECT::select (persist | none)",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::ConnectionControl,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::Server,
            dialects: None,
        }],
        ..CommandSpec::DEFAULT
    }
}
