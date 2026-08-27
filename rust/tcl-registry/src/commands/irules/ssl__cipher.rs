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

//! `SSL::cipher` iRules command.
use crate::prelude::*;

/// The command's subcommands.
const SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "bits",
        arity: Arity::exact(0),
        detail: "Get number of secret bits used.",
        synopsis: "SSL::cipher bits",
        pure: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::SslState,
            reads: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "name",
        arity: Arity::exact(0),
        detail: "Get cipher name.",
        synopsis: "SSL::cipher name",
        pure: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::SslState,
            reads: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "version",
        arity: Arity::exact(0),
        detail: "Get cipher version.",
        synopsis: "SSL::cipher version",
        pure: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::SslState,
            reads: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "clientlist",
        arity: Arity::exact(0),
        detail: "Get client cipher list.",
        synopsis: "SSL::cipher clientlist",
        pure: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::SslState,
            reads: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..SubCommand::DEFAULT
    },
];

pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "SSL::cipher",
        traits: Traits::PURE.union(Traits::CSE_CANDIDATE),
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns SSL cipher information.",
            synopsis: &[
                "SSL::cipher bits",
                "SSL::cipher name",
                "SSL::cipher version",
                "SSL::cipher clientlist",
            ],
            snippet: "Returns an SSL cipher name, its version, and the number of secret bits used.",
            source: "https://clouddocs.f5.com/api/irules/SSL__cipher.html",
            examples: "when HTTP_REQUEST {\n    # Check encryption strength\n    if { [SSL::cipher bits] >= 128 } {\n        pool web_servers\n    } else {\n        # Client is using a weak cipher\n        # Use one of the destination commands\n\n        # Either specify a pool\n        pool sorry_servers\n\n        # or to a specific node\n        node 10.10.10.10\n\n        # or send a 302 response to redirect to a specific URL\n        # Set cache control headers to prevent proxies from caching the response.",
            return_value: "SSL::cipher name Returns the current SSL cipher name using the format of the L<OpenSSL SSL_CIPHER_get_name() function|https://www.openssl.org/docs/ssl/SSL_CIPHER_get_name.html> (e.g. \"EDH-RSA-DES-CBC3-SHA\" or \"RC4-MD5\").",
        }),
        // Measured on the appliance
        // (`docs/design/bigip-irule-parser-measurements.md` §8): the rule
        // compiler accepts `SSL::cipher` in `HTTP_REQUEST`,
        // `LB_SELECTED` and `HTTP_RESPONSE` and refuses it in
        // `CLIENT_ACCEPTED`, `CLIENT_DATA`, `SERVER_CONNECTED` and
        // `CLIENT_CLOSED` — the four events either side of a completed
        // handshake, where no cipher has been negotiated on the
        // connection yet (or the connection is already gone). No
        // `EventProps` predicate separates that set — `LB_SELECTED`
        // implies no profile at all and `HTTP_REQUEST` implies an HTTP
        // one, not an SSL one — so the refused cells are carried as the
        // measured closed list they are.
        excluded_events: &[
            "CLIENT_ACCEPTED",
            "CLIENT_CLOSED",
            "CLIENT_DATA",
            "SERVER_CONNECTED",
        ],
        // …and `RULE_INIT` is refused for the ordinary reason: there is
        // no connection to read a cipher from (§8).
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &[],
            also_in: &[],
            flow: true,
        }),
        forms: &[FormSpec {
            synopsis: "SSL::cipher <subcommand>",
            ..FormSpec::DEFAULT
        }],
        subcommands: SUBCOMMANDS,
        side_effects: &[SideEffect {
            target: SideEffectTarget::SslState,
            reads: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}
