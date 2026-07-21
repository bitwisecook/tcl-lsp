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
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "persist",
        traits: Traits::DIAGRAM_ACTION,
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Sets the connection persistence type.",
            synopsis: &[
                "persist none",
                "persist cookie (('insert' (COOKIE_NAME (EXPIRATION)?)?) | ('rewrite' (COOKIE_NAME (EXPIRATION)?)?) | ('passive' (COOKIE_NAME)?) | ('hash' COOKIE_NAME ( (<OFFSET LENGTH>)? (TIMEOUT)?)?))?",
                "persist source_addr (IPV4_MASK)? (TIMEOUT)?",
                "persist simple (IPV4_MASK)? (TIMEOUT)?",
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
            init_only: false,
            flow: true,
            capability: None,
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "persist <mode> ?args?",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::PersistenceTable,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::Client,
            dialects: None,
        }],
        ..CommandSpec::DEFAULT
    }
}
