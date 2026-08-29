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

//! `IPFIX::msg` iRules command.
use crate::prelude::*;
use tcl_dialect::model::{SpecSurface};
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "IPFIX::msg",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "IPFIX::msg Provides the ability to create, delete and set values in an IPFIX message that can then be used to send IPFIX message based on processing in the iRule.",
            synopsis: &["IPFIX::msg ((create IPFIX_TEMPLATE) |"],
            snippet: "Provides the ability to create, delete and set data values in an IPFIX\nmessage based on the provided IPFIX_TEMPLATE.",
            source: "https://clouddocs.f5.com/api/irules/IPFIX__msg.html",
            examples: "when RULE_INIT {\n    set static::http_track_dest \"\"\n    set static::http_track_tmplt \"\"\n}",
            return_value: "IPFIX::msg create returns an IPFIX_MESSAGE object that is used by the IPFIX::msg set|delete and IPFIX::destination send commands.",
        }),
        forms: &[FormSpec {
            synopsis: "IPFIX::msg <subcommand> ?options? args...",
            ..FormSpec::DEFAULT
        }],
        options: const {
            &[OptionSpec {
                name: "-pos",
                value: OptionValue::value("IPFIX_POS"),
                detail: "Position index for duplicate field types.",
                surface: None,
                aliases: &[],
                lifecycle: Lifecycle::UNSPECIFIED,
                min_abbrev: None,
            }]
        },
        side_effects: &[SideEffect {
            target: SideEffectTarget::NetworkIo,
            reads: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}
