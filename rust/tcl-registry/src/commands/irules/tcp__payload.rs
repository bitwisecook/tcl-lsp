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

//! `TCP::payload` iRules command.
use crate::prelude::*;
use tcl_dialect::model::{SpecSurface};

/// The command's subcommands.
const SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "replace",
        arity: Arity::exact(3),
        detail: "Replace bytes in collected payload.",
        synopsis: "TCP::payload replace <offset> <length> <data>",
        mutator: true,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "length",
        arity: Arity::exact(0),
        detail: "Returns the amount of accumulated TCP data in bytes.",
        synopsis: "TCP::payload length",
        ..SubCommand::DEFAULT
    },
];

pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "TCP::payload",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::new(0, 4),
        data_collection: Some(TCP_PAYLOAD),
        hover: Some(HoverSnippet {
            summary: "Returns or changes the data collected by TCP::collect.",
            synopsis: &[
                "TCP::payload ?<size>?",
                "TCP::payload replace <offset> <length> <data>",
                "TCP::payload length",
            ],
            snippet: "Returns the accumulated TCP data content, or replaces collected payload with the specified data.",
            source: "https://clouddocs.f5.com/api/irules/TCP__payload.html",
            examples: "when CLIENT_ACCEPTED {\n  TCP::collect\n}",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: Some("tcp"),
            profiles: &[],
            also_in: &["SIP_REQUEST", "SIP_REQUEST_SEND", "SIP_RESPONSE"],
            flow: false,
        }),
        forms: &[FormSpec {
            kind: FormKind::Getter,
            synopsis: "TCP::payload ?<size>?",
            ..FormSpec::DEFAULT
        }],
        subcommands: SUBCOMMANDS,
        side_effects: &[SideEffect {
            target: SideEffectTarget::TcpState,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        taint_source: Some(TaintColour::TAINTED),
        byte_array_payload: Some(BytePayloadSpec::DEFAULT),
        ..CommandSpec::DEFAULT
    }
}
