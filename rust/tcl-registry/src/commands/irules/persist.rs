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

//! `persist` iRules command.
use crate::prelude::*;
use tcl_dialect::model::SpecSurface;

const TABLE_MODES: &[ArgValue] = &[
    ArgValue {
        value: "source_addr",
        detail: "Source-address persistence.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "simple",
        detail: "Simple persistence.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "dest_addr",
        detail: "Destination-address persistence.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "host",
        detail: "Host persistence.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "sticky",
        detail: "Sticky persistence.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "ssl",
        detail: "SSL persistence.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "uie",
        detail: "Universal inspection engine persistence.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "universal",
        detail: "Universal persistence.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "hash",
        detail: "Hash persistence.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "sip",
        detail: "SIP persistence.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "mcp",
        detail: "MCP persistence (BIG-IP 21.1.0+).",
        ..ArgValue::DEFAULT
    },
];

const MCP_MODE_GATE: &[VersionedArgValue] = &[VersionedArgValue {
    index: 0,
    value: "mcp",
    lifecycle: Lifecycle::introduced_in("21.1.0"),
}];

const SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "none",
        arity: Arity::exact(0),
        detail: "Disable persistence for this connection.",
        synopsis: "persist none",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "cookie",
        detail: "Use cookie persistence.",
        synopsis: "persist cookie ?insert|rewrite|passive|hash ...?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "source_addr",
        arity: Arity::new(0, 2),
        detail: "Use source-address persistence.",
        synopsis: "persist source_addr ?mask? ?timeout?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "simple",
        arity: Arity::new(0, 2),
        detail: "Use simple persistence.",
        synopsis: "persist simple ?mask? ?timeout?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "dest_addr",
        arity: Arity::new(0, 2),
        detail: "Use destination-address persistence.",
        synopsis: "persist dest_addr ?mask? ?timeout?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "sticky",
        arity: Arity::new(0, 2),
        detail: "Use sticky persistence.",
        synopsis: "persist sticky ?mask? ?timeout?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "msrdp",
        arity: Arity::new(0, 1),
        detail: "Use Microsoft Remote Desktop persistence.",
        synopsis: "persist msrdp ?timeout?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "ssl",
        arity: Arity::new(0, 1),
        detail: "Use SSL persistence.",
        synopsis: "persist ssl ?timeout?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "uie",
        arity: Arity::new(1, 2),
        detail: "Use universal inspection engine persistence.",
        synopsis: "persist uie <string> ?timeout?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "universal",
        arity: Arity::new(1, 2),
        detail: "Use universal persistence.",
        synopsis: "persist universal <string> ?timeout?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "hash",
        arity: Arity::new(1, 2),
        detail: "Use hash persistence.",
        synopsis: "persist hash <string> ?timeout?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "carp",
        arity: Arity::new(1, 2),
        detail: "Use CARP persistence.",
        synopsis: "persist carp <string> ?timeout?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "sip",
        arity: Arity::new(1, 2),
        detail: "Use SIP persistence.",
        synopsis: "persist sip <string> ?timeout?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "host",
        arity: Arity::new(0, 1),
        detail: "Use host persistence.",
        synopsis: "persist host ?timeout?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "mcp",
        arity: Arity::exact(1),
        detail: "Use MCP persistence.",
        synopsis: "persist mcp <persistence-name>",
        hover: Some(HoverSnippet {
            summary: "Persists the connection using an MCP persistence name.",
            synopsis: &["persist mcp <persistence-name>"],
            snippet: "Uses MCP persistence for the supplied name. Introduced in BIG-IP 21.1.0.",
            source: "https://clouddocs.f5.com/api/irules/persist.html",
            examples: "persist mcp persistence_name",
            return_value: "",
        }),
        lifecycle: Lifecycle::introduced_in("21.1.0"),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "add",
        arity: Arity::new(2, 3),
        detail: "Add a persistence-table entry.",
        synopsis: "persist add <mode> <key> ?timeout?",
        arg_values: &[(0, TABLE_MODES)],
        versioned_arg_values: MCP_MODE_GATE,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "lookup",
        arity: Arity::new(2, 3),
        detail: "Look up a persistence-table entry.",
        synopsis: "persist lookup <mode> <key> ?all|node|port|pool?",
        arg_values: &[(0, TABLE_MODES)],
        versioned_arg_values: MCP_MODE_GATE,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "delete",
        arity: Arity::exact(2),
        detail: "Delete a persistence-table entry.",
        synopsis: "persist delete <mode> <key>",
        arg_values: &[(0, TABLE_MODES)],
        versioned_arg_values: MCP_MODE_GATE,
        ..SubCommand::DEFAULT
    },
];

pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "persist",
        traits: Traits::DIAGRAM_ACTION,
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Sets the connection persistence type.",
            synopsis: &[
                "persist none",
                "persist cookie (('insert' (COOKIE_NAME (EXPIRATION)?)?) | ('rewrite' (COOKIE_NAME (EXPIRATION)?)?) | ('passive' (COOKIE_NAME)?) | ('hash' COOKIE_NAME ( (<OFFSET LENGTH>)? (TIMEOUT)?)?))?",
                "persist source_addr (IPV4_MASK)? (TIMEOUT)?",
                "persist simple (IPV4_MASK)? (TIMEOUT)?",
                "persist mcp PERSIST_MCP_NAME",
                "persist add <mode> <key> ?timeout?",
                "persist lookup <mode> <key> ?all|node|port|pool?",
                "persist delete <mode> <key>",
            ],
            snippet: "Causes the system to use the named persistence type to persist the\nconnection. Also allows direct inspection and manipulation of the\npersistence table.",
            source: "https://clouddocs.f5.com/api/irules/persist.html",
            examples: "when CLIENTSSL_HANDSHAKE {\n   # Persist the client connection based on the SSL session ID\n    persist ssl\n}",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: true,
            server_side: false,
            transport: None,
            profiles: &[],
            also_in: &["PERSIST_DOWN"],
            flow: true,
        }),
        forms: &[FormSpec {
            synopsis: "persist <mode> ?args?",
            ..FormSpec::DEFAULT
        }],
        subcommands: SUBCOMMANDS,
        side_effects: &[SideEffect {
            target: SideEffectTarget::PersistenceTable,
            writes: true,
            connection_side: ConnectionSide::Client,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}
