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

//! `GTP::ie` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "GTP::ie",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "This set of commands allows for the parsing and interpretation of GTP IE elements.",
            synopsis: &[
                "GTP::ie 'exists' ('-message' MESSAGE)? (IE_PATH)?",
                "GTP::ie 'count' ('-message' MESSAGE)? ('-type' TYPE)? ('-instance' INSTANCE)? (IE_PATH)?",
                "GTP::ie 'get' ('instance' | 'length' | 'encode-type' | 'value') ('-message' MESSAGE)? IE_PATH",
                "GTP::ie 'get' 'list' ('-message' MESSAGE)? ('-type' TYPE)? ('-instance' INSTANCE)? (IE_PATH)?",
            ],
            snippet: "This set of commands allows for the parsing and interpretation of GTP\nIE elements.",
            source: "https://clouddocs.f5.com/api/irules/GTP__ie.html",
            examples: "when GTP_SIGNALLING_INGRESS {\n    if { [GTP::ie exists imsi:0] } {\n        log local0. \"GTP imsi [GTP::ie get value imsi:0]\"\n    }\n    log local0. \"Total number of top level IEs [GTP::ie count]\"\n    set ie_list [ GTP::ie get list]\n    foreach ie $ie_list {\n        log local0. \"IE $ie\"\n    }\n}",
            return_value: "",
        }),
        forms: &[FormSpec {
            synopsis: "GTP::ie 'exists' ('-message' MESSAGE)? (IE_PATH)?",
            ..FormSpec::DEFAULT
        }],
        options: const {
            &[
                OptionSpec {
                    name: "-message",
                    value: OptionValue::value("MESSAGE"),
                    detail: "Operate on a specific GTP message object.",
                    dialects: None,
                    aliases: &[],
                    lifecycle: Lifecycle::UNSPECIFIED,
                    min_abbrev: None,
                },
                OptionSpec {
                    name: "-type",
                    value: OptionValue::value("TYPE"),
                    detail: "Filter by IE type value.",
                    dialects: None,
                    aliases: &[],
                    lifecycle: Lifecycle::UNSPECIFIED,
                    min_abbrev: None,
                },
                OptionSpec {
                    name: "-instance",
                    value: OptionValue::value("INSTANCE"),
                    detail: "Filter by IE instance.",
                    dialects: None,
                    aliases: &[],
                    lifecycle: Lifecycle::UNSPECIFIED,
                    min_abbrev: None,
                },
            ]
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
