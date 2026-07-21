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

//! `LSN::inbound-entry` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "LSN::inbound-entry",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "This command creates and gets the inbound mapping for a translation address, translation port and protocol.",
            synopsis: &[
                "LSN::inbound-entry (get | delete) IP_TUPLE IP_PROTOCOL",
                "LSN::inbound-entry create (-mirror)?",
            ],
            snippet: "This command creates and gets the inbound mapping for a translation address, translation port and protocol.\n\nLSN::inbound-entry get <translation_address>:<translation_port> <protocol>\nLSN::inbound-entry create [-mirror] [-override] [-dslite <dslite local address> <dslite remote address>] [-prefix <IPv6 address>] <LSN pool name> <timeout> <client IP:client port> <translation address:translation port> <protocol>\n\nv11.5+\nLSN::inbound-entry delete <translation_address>:<translation_port> <protocol>",
            source: "https://clouddocs.f5.com/api/irules/LSN__inbound-entry.html",
            examples: "",
            return_value: "LSN::inbound-entry get <translation IP>:<translation port> <protocol> - Gets inbound entry for the specified translation IP, translation port and protocol. Protocol can be set TCP or UDP. This command returns the client IP address, port and route domain ID.",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "LSN::inbound-entry (get | delete) IP_TUPLE IP_PROTOCOL",
        }],
        options: const {
            &[OptionSpec {
                name: "-mirror",
                value: OptionValue::flag(),
                detail: "Option -mirror.",
                dialects: None,
                aliases: &[],
                min_version: None,
            }]
        },
        side_effects: &[SideEffect {
            target: SideEffectTarget::LsnState,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::Both,
            dialects: None,
        }],
        ..CommandSpec::DEFAULT
    }
}
