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

//! `SSL::cert_constraint` iRules command.
use crate::prelude::*;
use tcl_dialect::model::SpecSurface;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "SSL::cert_constraint",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Inserts cert constraint information to the certificate.",
            synopsis: &["SSL::cert_constraint (ARG ARG)"],
            snippet: "Inserts a certificate extension to the certificate.",
            source: "https://clouddocs.f5.com/api/irules/SSL__cert_constraint.html",
            examples: "when CLIENTSSL_HANDSHAKE {\n    log local0.info \"CLIENTSSL_HANDSHAKE\"\n    SSL::cert_constraint 1.2.3.4.5 \"This is the oid-value of 1.2.3.4.5\"\n}",
            return_value: "SSL::cert_constraint <oid oid-value> Inserts the <oid oid-value> as an extension with OID=oid and value=oid-value to the certificate.",
        }),
        event_requires: Some(EventRequires {
            client_side: true,
            server_side: false,
            transport: Some("tcp"),
            profiles: &["CLIENTSSL"],
            also_in: &[],
            flow: false,
        }),
        forms: &[FormSpec {
            synopsis: "SSL::cert_constraint (ARG ARG)",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::SslState,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}
