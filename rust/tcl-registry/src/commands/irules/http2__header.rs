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

//! `HTTP2::header` iRules command.
use crate::prelude::*;

/// The command's subcommands.
const SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "replace",
        arity: Arity::new(1, 2),
        detail: "Replace pseudo-header value.",
        synopsis: "HTTP2::header replace <name> ?<string>?",
        mutator: true,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "remove",
        arity: Arity::exact(1),
        detail: "Remove a pseudo-header.",
        synopsis: "HTTP2::header remove <name>",
        mutator: true,
        ..SubCommand::DEFAULT
    },
];

pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "HTTP2::header",
        dialects: Some(DialectSet::IRULES),
        // Versioned-library axis (§7.1): the F5 surface is the ambient
        // `f5-irules-cmds` library keyed on the BIG-IP release; this
        // command's introducing release comes from the hover prose below.
        required_package: Some("f5-irules-cmds"),
        lifecycle: Lifecycle::introduced_in("16.1.0"),
        arity: Arity::new(1, 3),
        hover: Some(HoverSnippet {
            summary: "Queries or modifies HTTP/2 pseudo-headers.",
            synopsis: &[
                "HTTP2::header <name>",
                "HTTP2::header replace <name> ?<string>?",
                "HTTP2::header remove <name>",
            ],
            snippet: "Queries or modifies HTTP/2 pseudo-headers.\nThe HTTP/2 pseudo-header names are lowercase and start with a ':' character.\nIntroduced in BIG-IP 16.1.0.",
            source: "https://clouddocs.f5.com/api/irules/HTTP2__header.html",
            examples: "when HTTP_REQUEST {\n  if { [HTTP2::header :authority] starts_with \"uat\" }  {\n    pool uat_pool\n  } else {\n    pool main_pool\n  }\n}",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: Some("tcp"),
            profiles: &["HTTP"],
            also_in: &["MR_EGRESS", "MR_INGRESS", "SERVER_CONNECTED"],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[FormSpec {
            kind: FormKind::Getter,
            synopsis: "HTTP2::header <name>",
            dialects: None,
        }],
        subcommands: SUBCOMMANDS,
        side_effects: &[SideEffect {
            target: SideEffectTarget::Http2State,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::Both,
            dialects: None,
        }],
        taint_source: Some(TaintColour::TAINTED),
        ..CommandSpec::DEFAULT
    }
}
