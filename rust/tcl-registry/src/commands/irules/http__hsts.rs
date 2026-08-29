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

//! `HTTP::hsts` iRules command.
use crate::prelude::*;
use tcl_dialect::model::{SpecSurface};

/// The command's subcommands.
const SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "mode",
        arity: Arity::exact(1),
        detail: "Enable or disable HSTS on a per-flow basis.",
        synopsis: "HTTP::hsts mode <enable | disable>",
        mutator: true,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "maximum-age",
        arity: Arity::exact(1),
        detail: "Set HSTS maximum-age on a per-flow basis.",
        synopsis: "HTTP::hsts maximum-age <seconds>",
        mutator: true,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "include-subdomains",
        arity: Arity::exact(1),
        detail: "Enable or disable HSTS include-subdomains on a per-flow basis.",
        synopsis: "HTTP::hsts include-subdomains <enable | disable>",
        mutator: true,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "preload",
        arity: Arity::exact(1),
        detail: "Enable or disable HSTS preload on a per-flow basis (v13+).",
        synopsis: "HTTP::hsts preload <enable | disable>",
        mutator: true,
        ..SubCommand::DEFAULT
    },
];

pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "HTTP::hsts",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::new(0, 2),
        hover: Some(HoverSnippet {
            summary: "Controls HTTP Strict Transport Security.",
            synopsis: &[
                "HTTP::hsts",
                "HTTP::hsts mode <enable | disable>",
                "HTTP::hsts maximum-age <seconds>",
                "HTTP::hsts include-subdomains <enable | disable>",
                "HTTP::hsts preload <enable | disable>",
            ],
            snippet: "Controls HSTS options on a per-flow basis, overriding the configured values in the HTTP profile.",
            source: "https://clouddocs.f5.com/api/irules/HTTP__hsts.html",
            examples: "when HTTP_REQUEST {\n    if { [HTTP::uri] contains \"secure\"} {\n        HTTP::hsts mode enable\n        HTTP::hsts maximum-age 8600\n        HTTP::hsts include-subdomains disable\n        HTTP::hsts preload enable\n    }\n}",
            return_value: "With no arguments, returns the currently configured HSTS header value for this connection.",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: Some("tcp"),
            profiles: &["FASTHTTP", "HTTP"],
            also_in: &[],
            flow: false,
        }),
        forms: &[FormSpec {
            kind: FormKind::Getter,
            synopsis: "HTTP::hsts",
            ..FormSpec::DEFAULT
        }],
        subcommands: SUBCOMMANDS,
        side_effects: &[SideEffect {
            target: SideEffectTarget::HttpHeader,
            writes: true,
            connection_side: ConnectionSide::Client,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}
