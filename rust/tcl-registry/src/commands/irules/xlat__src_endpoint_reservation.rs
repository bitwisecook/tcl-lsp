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

//! `XLAT::src_endpoint_reservation` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "XLAT::src_endpoint_reservation",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "XLAT:src_endpoint_reservation",
            synopsis: &[
                "XLAT::src_endpoint_reservation create",
                "XLAT::src_endpoint_reservation update_lifetime TRANS_ADDR TRANS_PORT LSN_POOL XLAT_PROTO XLAT_LIFETIME",
            ],
            snippet: "Create, update, or get reserved entry values.\n\nSyntax:\nXLAT::src_endpoint_reservation create [-no-persist] [-dslite  <local> <remote>] [-pool <source translation object/pool name>] [-translation-loose|-translation-strict <ip> <port>] <client ip> <client port> <protocol> <lifetime>;\n\nCreates a reservation in the reservation table which can be viewed using the command \"lsndb list endpoint-reservation\" for the lifetime specified by the user. The command has the following characteristics:\n    1) The returned endpoint cannot be reserved for another client IP:port as long as it is active.",
            source: "https://clouddocs.f5.com/api/irules/XLAT__src_endpoint_reservation.html",
            examples: "",
            return_value: "create returns the translation endpoint used for the reservation.",
        }),
        excluded_events: &["RULE_INIT"],
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "XLAT::src_endpoint_reservation create ?options? <client_ip> <client_port> <protocol> <lifetime>",
            dialects: None,
        }],
        options: const {
            &[
                OptionSpec {
                    name: "-no-persist",
                    value: OptionValue::flag(),
                    detail: "Skip creation of persist entry.",
                    dialects: None,
                    aliases: &[],
                    min_version: None,
                },
                OptionSpec {
                    name: "-dslite",
                    value: OptionValue::value("LOCAL_ADDR REMOTE_ADDR"),
                    detail: "DS-Lite local and remote endpoint.",
                    dialects: None,
                    aliases: &[],
                    min_version: None,
                },
                OptionSpec {
                    name: "-pool",
                    value: OptionValue::value("POOL_NAME"),
                    detail: "Specify pool for endpoint reservation.",
                    dialects: None,
                    aliases: &[],
                    min_version: None,
                },
                OptionSpec {
                    name: "-translation-loose",
                    value: OptionValue::value("IP PORT"),
                    detail: "Hint data; command won't fail if hints can't be used.",
                    dialects: None,
                    aliases: &[],
                    min_version: None,
                },
                OptionSpec {
                    name: "-translation-strict",
                    value: OptionValue::value("IP PORT"),
                    detail: "Hint data; command fails if hints can't be used.",
                    dialects: None,
                    aliases: &[],
                    min_version: None,
                },
            ]
        },
        side_effects: &[SideEffect {
            target: SideEffectTarget::LsnState,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Both,
            dialects: None,
        }],
        ..CommandSpec::DEFAULT
    }
}
