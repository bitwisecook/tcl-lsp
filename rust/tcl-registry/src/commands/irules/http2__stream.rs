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

//! `HTTP2::stream` iRules command.
use crate::prelude::*;
use tcl_dialect::model::{SpecSurface};

/// The command's subcommands.
const SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "id",
        arity: Arity::exact(0),
        detail: "Returns the stream id. Returns 0 if HTTP/2 is not active.",
        synopsis: "HTTP2::stream id",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "priority",
        arity: Arity::new(0, 1),
        detail: "Get or set the priority of the current stream.",
        synopsis: "HTTP2::stream priority ?<priority>?",
        subcommand_forms: &[
            SubCommandForm {
                name: "getter",
                ..SubCommandForm::DEFAULT
            },
            SubCommandForm {
                name: "setter",
                ..SubCommandForm::DEFAULT
            },
        ],
        ..SubCommand::DEFAULT
    },
];

pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "HTTP2::stream",
        traits: Traits::PURE,
        surface: Some(SpecSurface::IRULES),
        arity: Arity::new(0, 2),
        hover: Some(HoverSnippet {
            summary: "Gets or sets the stream attributes including id and priority.",
            synopsis: &["HTTP2::stream (id | (priority (PRIORITY)?))?"],
            snippet: "This command can be used to determine the stream attributes including id and priority. This command can also be used to set the priority for a current active stream.\n\nHTTP2::stream\n    Returns the stream id. Returns 0 if HTTP/2 is not active.\n\nHTTP2::stream id\n    Returns the stream id. Returns 0 if HTTP/2 is not active.\n\nHTTP2::stream priority\n    Returns the priority of the current stream.\n\nHTTP2::stream priority <priority>\n    Sets the priority of the current active stream. The return is 0 is the priority was set. Return is an error if the priority is out of bounds (exceeding 8 bits).",
            source: "https://clouddocs.f5.com/api/irules/HTTP2__stream.html",
            examples: "when HTTP_REQUEST {\n    if {[HTTP2::version] != 0} {\n        set new_pri [URI::query [HTTP::uri] pri]\n        if { $new_pri != \"\" } {\n             HTTP2::stream priority $new_pri\n        }\n        HTTP::header insert \"X-HTTP2-Values stream/pritority \" \"[HTTP2::stream]/[HTTP2::stream priority]\"\n    }\n}",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: Some("tcp"),
            profiles: &["HTTP"],
            also_in: &[],
            flow: false,
        }),
        forms: &[FormSpec {
            kind: FormKind::Getter,
            synopsis: "HTTP2::stream",
            ..FormSpec::DEFAULT
        }],
        subcommands: SUBCOMMANDS,
        side_effects: &[SideEffect {
            target: SideEffectTarget::Http2State,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}
