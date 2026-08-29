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

//! `IPFIX::template` iRules command.
use crate::prelude::*;
use tcl_dialect::model::{SpecSurface};
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "IPFIX::template",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "IPFIX::template Provides the ability to create and delete IPFIX message templates that may be used to generate IPFIX messages based on processing in the iRule.",
            synopsis: &["IPFIX::template ( (create TEMPLATE_STRING) |"],
            snippet: "This command provides the ability to create and delete user defined IPFIX\nmessage templates that may be used to send IPFIX messages to a specified\ndestination.",
            source: "https://clouddocs.f5.com/api/irules/IPFIX__template.html",
            examples: "when RULE_INIT {\n    set static::http_track_dest \"\"\n    set static::http_track_tmplt \"\"\n}",
            return_value: "IPFIX::template create TEMPLATE_STRING returns an IPFIX template object that is used by the IPFIX::msg create command and IPFIX::template delete command.",
        }),
        forms: &[FormSpec {
            synopsis: "IPFIX::template ( (create TEMPLATE_STRING) |",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::NetworkIo,
            reads: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}
