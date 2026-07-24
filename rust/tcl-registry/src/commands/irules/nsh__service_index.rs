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

//! `NSH::service_index` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "NSH::service_index",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Sets/Get the Service Index for NSH.",
            synopsis: &["NSH::service_index DIRECTION (NSH_SERVICE_IDX)?"],
            snippet: "Set: Service index for NSH.\n            Get(DIRECTION as the only parameter): Service index from NSH.",
            source: "https://clouddocs.f5.com/api/irules/NSH__service_index.html",
            examples: "rvice index for NSH.\n            when CLIENT_ACCEPTED {\n                NSH::service_index serverside_egress 20\n                set myservice_index [NSH::service_index serverside_egress]\n            }",
            return_value: "None.",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "NSH::service_index DIRECTION (NSH_SERVICE_IDX)?",
            dialects: None,
        }],
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
