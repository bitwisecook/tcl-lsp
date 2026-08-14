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

//! `SSL::authenticate` iRules command.
use crate::prelude::*;

/// The command's subcommands.
const SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "once",
        arity: Arity::exact(0),
        detail: "Authenticate once per session.",
        synopsis: "SSL::authenticate once",
        mutator: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::SslState,
            writes: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "always",
        arity: Arity::exact(0),
        detail: "Authenticate on every connection.",
        synopsis: "SSL::authenticate always",
        mutator: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::SslState,
            writes: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "depth",
        arity: Arity::exact(1),
        detail: "Set max certificate chain traversal depth.",
        synopsis: "SSL::authenticate depth <number>",
        mutator: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::SslState,
            writes: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..SubCommand::DEFAULT
    },
];

pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "SSL::authenticate",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Overrides the current setting for authentication frequency or for the maximum depth of certificate chain traversal.",
            synopsis: &["SSL::authenticate (once | always | (depth DEPTH))"],
            snippet: "Overrides the current setting for authentication frequency or for the maximum depth of certificate chain traversal.\n\nSSL::authenticate <\"once\" | \"always\">\n    Valid in a client-side context only, this command overrides the client-side SSL connection's current setting regarding authentication frequency.\n\nSSL::authenticate depth <number>\n    In a client-side context, this overrides the client-side SSL connection's maximum certificate-chain traversal depth. In a server-side context, it overrides the server-side SSL connection's maximum certificate-chain traversal depth.",
            source: "https://clouddocs.f5.com/api/irules/SSL__authenticate.html",
            examples: "when CLIENT_ACCEPTED {\n    set session_flag 0\n}",
            return_value: "",
        }),
        forms: &[FormSpec {
            synopsis: "SSL::authenticate <once | always | depth <number>>",
            ..FormSpec::DEFAULT
        }],
        subcommands: SUBCOMMANDS,
        side_effects: &[SideEffect {
            target: SideEffectTarget::SslState,
            writes: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}
