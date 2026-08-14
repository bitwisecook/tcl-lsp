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

//! `PEM::subscriber` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "PEM::subscriber",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "This command allows you to create, delete or retreive information of a PEM subscriber session with subscriber-id and subscriber-id-type in the PEM Session DB.",
            synopsis: &[
                "PEM::subscriber config policy ( (get SUBS_ID (PEM_SUBS_TYPE2))",
                "PEM::subscriber create SUBS_ID ( (subscriber-id-type | subscriber-type) (PEM_SUBS_TYPE2)) (ip-address IP_ADDR)+",
            ],
            snippet: "Each PEM session carries the following standard attributes: imsi, imeisv, tower-id, rat-type, user-name, subscriber-type, subscriber-id, state, aaa-reporting-interval, provision.\n\nDetails (Syntax):\nPEM::subscriber create <subscriber-id> subscriber-type <e164 | imsi | nai | private> ip-address <IP_ADDR> [imsi <sring>] [user-name <string>] [tower-id <string>] [imeisv <string>] [provision <yes | no>] [<custom attr> <custom value>] [policy <string1> ...",
            source: "https://clouddocs.f5.com/api/irules/PEM__subscriber.html",
            examples: "when HTTP_REQUEST {\n    PEM::subscriber create 4086007577 subscriber-type e164 ip-address 10.10.10.10\n    PEM::subscriber config policy referential set 4086007577 e164 policy1\n    PEM::subscriber ip 4086007577 e164 10.0.0.92 2001::101\n    PEM::subscriber info 4086007577 e164 imsi 1223456\n    PEM::subscriber info 4086007577 e164 imeisv 2534535\n    PEM::subscriber info 4086007577 e164 tower-id 8248249\n    PEM::subscriber info 4086007577 e164 user-name 2139449",
            return_value: "",
        }),
        forms: &[FormSpec {
            synopsis: "PEM::subscriber config policy ( (get SUBS_ID (PEM_SUBS_TYPE2))",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::ConnectionControl,
            reads: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}
