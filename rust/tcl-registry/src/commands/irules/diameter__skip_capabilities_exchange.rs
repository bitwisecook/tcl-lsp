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

//! `DIAMETER::skip_capabilities_exchange` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "DIAMETER::skip_capabilities_exchange",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Instructs DIAMETER protocol to skip capabilities exchange when establishing a peering relationship.",
            synopsis: &["DIAMETER::skip_capabilities_exchange ( HOSTNAME )?"],
            snippet: "Once called, the current connection will skip DIAMETER capabilities exchange message communication with the peer device and will immediately be able to receive DIAMETER messaegs.\n\nIf the HOSTNAME parameter is provided, the provided name will be used as the peer device's origin-host attribute for logging.",
            source: "https://clouddocs.f5.com/api/irules/DIAMETER__skip_capabilities_exchange.html",
            examples: "when CLIENT_ACCEPTED {\n                if { ([IP::address] starts_with \"192.168.\") } {\n                    DIAMETER::skip_capabilities_exchange [IP::address].somesp.com\n                }\n            }",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &[],
            also_in: &["CLIENT_ACCEPTED"],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "DIAMETER::skip_capabilities_exchange ( HOSTNAME )?",
            dialects: None,
        }],
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
