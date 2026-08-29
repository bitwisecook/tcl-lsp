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

//! `MESSAGE::field` iRules command.
use crate::prelude::*;
use tcl_dialect::model::{SpecSurface};
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "MESSAGE::field",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Various operations for a message's fields.",
            synopsis: &["MESSAGE::field ( ('names') |"],
            snippet: "This command is used for below mentioned operations for a message's field.\nThis is valid for messages of the following protocols:\n\n    SIP",
            source: "https://clouddocs.f5.com/api/irules/MESSAGE__field.html",
            examples: "when MR_INGRESS {\n    switch ( [MESSAGE::proto] ) {\n        \"SIP\" {\n           if { [MESSAGE::type] eq \"request\" } {\n              set uri [MESSAGE::field value ':uri']\n              log local0. \"Message's URI is : $uri\"\n           }\n        }\n    }\n}",
            return_value: "Returns value depends on the subcommands. See description for more details.",
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
            synopsis: "MESSAGE::field ( ('names') |",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::MessageState,
            reads: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}
