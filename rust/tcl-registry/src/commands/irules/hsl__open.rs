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

//! `HSL::open` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "HSL::open",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        return_type: Some(TclType::Channel),
        hover: Some(HoverSnippet {
            summary: "Opens a handle for High Speed Logging communication.",
            synopsis: &[
                "HSL::open ('-publisher' | '-pub') PUBLISHER",
                "HSL::open '-proto' ('UDP' | 'TCP') '-pool' POOL_OBJ",
            ],
            snippet: "Open a handle for High Speed Logging communication. After creating the\nconnection, send data on the connection using HSL::send.",
            source: "https://clouddocs.f5.com/api/irules/HSL__open.html",
            examples: "#2\nwhen CLIENT_ACCEPTED {\n    set hsl [HSL::open -publisher /Common/lpAll]\n}",
            return_value: "",
        }),
        forms: &[FormSpec {
            synopsis: "HSL::open ('-publisher' | '-pub') PUBLISHER",
            ..FormSpec::DEFAULT
        }],
        options: const {
            &[
                OptionSpec {
                    name: "-publisher",
                    value: OptionValue::value(""),
                    detail: "Option -publisher.",
                    dialects: None,
                    aliases: &[],
                    lifecycle: Lifecycle::UNSPECIFIED,
                    min_abbrev: None,
                },
                OptionSpec {
                    name: "-pub",
                    value: OptionValue::value(""),
                    detail: "Option -pub.",
                    dialects: None,
                    aliases: &[],
                    lifecycle: Lifecycle::UNSPECIFIED,
                    min_abbrev: None,
                },
                OptionSpec {
                    name: "-proto",
                    value: OptionValue::value(""),
                    detail: "Option -proto.",
                    dialects: None,
                    aliases: &[],
                    lifecycle: Lifecycle::UNSPECIFIED,
                    min_abbrev: None,
                },
                OptionSpec {
                    name: "-pool",
                    value: OptionValue::value(""),
                    detail: "Option -pool.",
                    dialects: None,
                    aliases: &[],
                    lifecycle: Lifecycle::UNSPECIFIED,
                    min_abbrev: None,
                },
            ]
        },
        side_effects: &[SideEffect {
            target: SideEffectTarget::LogIo,
            writes: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}
