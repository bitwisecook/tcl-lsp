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

//! `HTTP::class` iRules command.
use crate::prelude::*;

/// The command's iRules subcommands.
const SUBCOMMANDS: &[SubCommand] = &[SubCommand {
    name: "select",
    arity: Arity::exact(1),
    detail: "Select an HTTP class.",
    synopsis: "HTTP::class select <name>",
    mutator: true,
    ..SubCommand::DEFAULT
}];

pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "HTTP::class",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::new(0, 2),
        hover: Some(HoverSnippet {
            summary: "Returns or sets the HTTP class selected by the HTTP selector.",
            synopsis: &[
                "HTTP::class",
                "HTTP::class [enable | disable]",
                "HTTP::class [asm | wa]",
                "HTTP::class select <name>",
            ],
            snippet: "Deprecated in v11.4 — replaced by POLICY commands. See sol14381 for details.",
            source: "https://clouddocs.f5.com/api/irules/HTTP__class.html",
            examples: "",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: Some("tcp"),
            profiles: &["HTTP"],
            also_in: &["HTTP_CLASS_FAILED", "HTTP_CLASS_SELECTED"],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[
            FormSpec {
                kind: FormKind::Getter,
                synopsis: "HTTP::class",
                ..FormSpec::DEFAULT
            },
            FormSpec {
                kind: FormKind::Setter,
                synopsis: "HTTP::class <enable | disable>",
                ..FormSpec::DEFAULT
            },
            FormSpec {
                kind: FormKind::Getter,
                synopsis: "HTTP::class <asm | wa>",
                ..FormSpec::DEFAULT
            },
        ],
        subcommands: SUBCOMMANDS,
        side_effects: &[SideEffect {
            target: SideEffectTarget::ClassificationState,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        deprecated_replacement: Some("CLASSIFY::application"),
        ..CommandSpec::DEFAULT
    }
}
