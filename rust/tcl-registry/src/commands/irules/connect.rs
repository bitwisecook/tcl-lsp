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

//! `connect` iRules command.
use crate::prelude::*;
/// The command's option table, hoisted out of the spec literal so the
/// builder stays inside the line budget.
const OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-protocol",
        value: OptionValue::value("PROTO"),
        detail: "IP protocol (default TCP).",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-myaddr",
        value: OptionValue::value("IP_ADDR"),
        detail: "Source address for the connection.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-myport",
        value: OptionValue::value("PORT"),
        detail: "Source port for the connection.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-timeout",
        value: OptionValue::value("MSEC"),
        detail: "Time in ms to wait for connection.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-idle",
        value: OptionValue::value("SEC"),
        detail: "Idle timeout in seconds (default 300).",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-tos",
        value: OptionValue::value("TOS"),
        detail: "IP TOS value.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-status",
        value: OptionValue::value("VARIABLE"),
        detail: "Save connection status into variable.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
];

pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "connect",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Establishes a sideband connection.",
            synopsis: &["connect info (", "connect (("],
            snippet: "This command establishes a sideband connection. It is one of several commands that make up the ability to use sideband connections from iRules.",
            source: "https://clouddocs.f5.com/api/irules/connect.html",
            examples: "# Open a sideband connection with a connection timeout of 100 ms and an idle timeout of 30 seconds\n#   to a local virtual server name sideband_virtual_server\nset conn_id [connect -timeout 100 -idle 30 -status conn_status sideband_virtual_server]\n\n# Same as above, but use an external host IP:port instead of a virtual server name\nset conn_id [connect -timeout 100 -idle 30 -status conn_status 10.0.0.10:80]\n\n\nExample with more complete error handling:",
            return_value: "This command opens a sideband connection to the specified destination.",
        }),
        // Sideband `connect` is a network sink (SSRF, T104);
        // the address-bearing arg positions are not pinned
        // (`taint_network_sink_args=()`).
        taint_network_sink_args: Some(&[]),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &[],
            also_in: &[],
            init_only: false,
            flow: true,
            capability: None,
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "connect ?options? destination",
            dialects: None,
        }],
        options: OPTIONS,
        side_effects: &[SideEffect {
            target: SideEffectTarget::NetworkIo,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::Both,
            dialects: None,
        }],
        ..CommandSpec::DEFAULT
    }
}
