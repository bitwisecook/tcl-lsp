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

//! `SSL::release` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "SSL::release",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        data_collection: Some(SSL_RELEASE),
        hover: Some(HoverSnippet {
            summary: "Releases the collected plaintext data.",
            synopsis: &["SSL::release (LENGTH)?"],
            snippet: "Releases the collected plaintext data to the next layer/filter up.",
            source: "https://clouddocs.f5.com/api/irules/SSL__release.html",
            examples: "when SERVERSSL_DATA {\n    # Do something with the decrypted data\n    set payload [SSL::payload]\n\n    # Release the payload\n    SSL::release\n}",
            return_value: "SSL::release [<length>] Releases the collected plaintext data to the next layer/filter up.",
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
            synopsis: "SSL::release (LENGTH)?",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::SslState,
            writes: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}
