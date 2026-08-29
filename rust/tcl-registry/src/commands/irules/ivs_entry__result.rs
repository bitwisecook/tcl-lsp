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

//! `IVS_ENTRY::result` iRules command.
use crate::prelude::*;
use tcl_dialect::model::{SpecSurface};
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "IVS_ENTRY::result",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Sends a result code to the IVS client.",
            synopsis: &["IVS_ENTRY::result (noop | modified | response)"],
            snippet: "Send a result code to the IVS (Internal Virtual Server) client\n(usually ADAPT). The intent is to allow an IVS to be used in a\nuser-defined way without a specific IVS profile like \"icap\". If an\n\"icap\" profile is present, IVS_ENTRY::result should not be used as\nit would cause a second result to be sent to the IVS client\n(usually ADAPT), with undefined effect.",
            source: "https://clouddocs.f5.com/api/irules/IVS_ENTRY__result.html",
            examples: "when IVS_ENTRY_REQUEST {\n                # Tell primary virtual the IVS will not handle this request\n                IVS_ENTRY::result noop\n            }",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["ICAP", "IVS_ENTRY"],
            also_in: &[],
            flow: false,
        }),
        forms: &[FormSpec {
            synopsis: "IVS_ENTRY::result (noop | modified | response)",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            reads: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}
