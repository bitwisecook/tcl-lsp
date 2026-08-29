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

//! `SSL::collect` iRules command.
use crate::prelude::*;
use tcl_dialect::model::{SpecSurface};
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "SSL::collect",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        data_collection: Some(SSL_COLLECT),
        hover: Some(HoverSnippet {
            summary: "Collect plaintext data after SSL offloading.",
            synopsis: &["SSL::collect (LENGTH)?"],
            snippet: "Starts the collection of plaintext data either indefinitely or for the specified amount of data. On successful collection, the corresponding data event is triggered. For clientside collection, the CLIENTSSL_DATA event is triggered. For serverside collection, the SERVERSSL_DATA event is triggered.",
            source: "https://clouddocs.f5.com/api/irules/SSL__collect.html",
            examples: "when SERVERSSL_HANDSHAKE {\n    SSL::collect\n}",
            return_value: "SSL::collect [<length>] Starts the collection of plaintext data either indefinitely or for the specified amount of data. When is specified, the data event will not be triggered until that length has been collected.",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: Some("tcp"),
            profiles: &["CLIENTSSL", "SERVERSSL"],
            also_in: &[],
            flow: false,
        }),
        forms: &[FormSpec {
            synopsis: "SSL::collect (LENGTH)?",
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
