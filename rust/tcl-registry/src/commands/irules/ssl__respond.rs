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

//! `SSL::respond` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "SSL::respond",
        traits: Traits::DIAGRAM_ACTION,
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Return data back to the origin via SSL.",
            synopsis: &["SSL::respond DATA"],
            snippet: "Returns the specified plaintext data back to the origin over the encrypted SSL connection.",
            source: "https://clouddocs.f5.com/api/irules/SSL__respond.html",
            examples: "when CLIENTSSL_HANDSHAKE {\n              # Trigger collection of the decrypted payload once the SSL or DTLS handshake has been completed successfully\n              SSL::collect\n            }",
            return_value: "SSL::respond <data> Returns the specified plaintext data back to the origin over the encrypted SSL connection.",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: Some("tcp"),
            profiles: &["CLIENTSSL", "SERVERSSL"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "SSL::respond DATA",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::SslState,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::Both,
            dialects: None,
        }],
        ..CommandSpec::DEFAULT
    }
}
