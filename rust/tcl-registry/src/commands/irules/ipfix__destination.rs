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

//! `IPFIX::destination` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "IPFIX::destination",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "IPFIX::destination Provides the ability to manage IPFIX logging destinations and send IPFIX messages based on processing in the iRule.",
            synopsis: &["IPFIX::destination ((open (-publisher LOG_PUBLISHER)) |"],
            snippet: "Provides the ability to open and close IPFIX logging destinations in\nthe context of an iRule, as well as the ability to send IPFIX messages\nto the IPFIX logging destinations.",
            source: "https://clouddocs.f5.com/api/irules/IPFIX__destination.html",
            examples: "when RULE_INIT {\n    set static::http_track_dest \"\"\n    set static::http_track_tmplt \"\"\n}",
            return_value: "IPFIX::destination open returns an IPFIX_DESTINATION object that is used by the IPFIX::destination close or send command.",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "IPFIX::destination ((open (-publisher LOG_PUBLISHER)) |",
            dialects: None,
        }],
        options: const {
            &[OptionSpec {
                name: "-publisher",
                value: OptionValue::value(""),
                detail: "Option -publisher.",
                dialects: None,
                aliases: &[],
                lifecycle: Lifecycle::UNSPECIFIED,
                min_abbrev: None,
            }]
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
