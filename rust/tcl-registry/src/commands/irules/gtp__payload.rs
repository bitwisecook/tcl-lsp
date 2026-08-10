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

//! `GTP::payload` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "GTP::payload",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns the entire payload for G-PDU message.",
            synopsis: &[
                "GTP::payload",
                "GTP::payload COUNT",
                "GTP::payload OFFSET COUNT",
                "GTP::payload 'replace' ('-message' MESSAGE)? OFFSET COUNT NEW_VALUE",
            ],
            snippet: "Returns the payload, either complete or partial, for G-PDU message.\nThis command returns an empty value, in case of non-G-PDU messages.",
            source: "https://clouddocs.f5.com/api/irules/GTP__payload.html",
            examples: "when CLIENT_ACCEPTED {\n    set payload [UDP::payload]\n    set t2 [GTP::parse $payload]\n    log local0. \"GTP version [GTP::header version -message $t2]\"\n    log local0. \"GTP type [GTP::header type -message $t2]\"\n    log local0. \"GTP teid [GTP::header teid -message $t2]\"\n}",
            return_value: "",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "GTP::payload",
            dialects: None,
        }],
        options: const {
            &[OptionSpec {
                name: "-message",
                value: OptionValue::value(""),
                detail: "Option -message.",
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
        taint_source: Some(TaintColour::TAINTED),
        byte_array_payload: Some(BytePayloadSpec {
            message_flag_shift: true,
            ..BytePayloadSpec::DEFAULT
        }),
        data_collection: Some(GTP_PAYLOAD),
        ..CommandSpec::DEFAULT
    }
}
