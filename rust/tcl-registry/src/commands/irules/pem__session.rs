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

//! `PEM::session` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "PEM::session",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "This command allows you to create, delete or retreive information of a PEM session using session IP address in the PEM Session DB.",
            synopsis: &[
                "PEM::session config policy ((get IP_ADDR) |",
                "PEM::session delete IP_ADDR",
            ],
            snippet: "This command allows you to create, delete or retreive information of a PEM Session in the PEM Session DB.\nEach PEM session carries the following standard attributes: imsi, imeisv, tower-id, rat-type, user-name, state, aaa-reporting-interval, provision.\n\nDetails (Syntax):\nPEM::session create <framed ip> [subscriber-id <string> subscriber-type <e164 | imsi | nai | private>] [imsi <sring>] [user-name <string>] [tower-id <string>] [imeisv <string>] [provision <yes | no>] [<custom attr> <custom value>] [policy <string1> ...",
            source: "https://clouddocs.f5.com/api/irules/PEM__session.html",
            examples: "when HTTP_REQUEST {\n    PEM::session create 10.10.10.10 subscriber-id 12345 subscriber-type e164 policy pem-policy1 pem-policy2\n\n    set polisy_var [PEM::session config policy get 10.10.10.10]\n    set ip_var [PEM::session ip 12345 e164]\n    set id_var [PEM::session info 10.10.10.10 subscriber-id]\n\n    PEM::session delete 10.10.10.10\n}",
            return_value: "",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "PEM::session config policy ((get IP_ADDR) |",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::ConnectionControl,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Both,
            dialects: None,
        }],
        ..CommandSpec::DEFAULT
    }
}
