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

//! `HTTP::release` iRules command.
use crate::prelude::*;
use tcl_dialect::model::{SpecSurface};
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "HTTP::release",
        traits: Traits::DIAGRAM_ACTION,
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        data_collection: Some(HTTP_RELEASE),
        hover: Some(HoverSnippet {
            summary: "Releases the data collected via HTTP::collect.",
            synopsis: &["HTTP::release"],
            snippet: "Releases the data collected via HTTP::collect. Unless a subsequent\nHTTP::collect command was issued, there is no need to use the\nHTTP::release command inside of the HTTP_REQUEST_DATA and\nHTTP_RESPONSE_DATA events, since (in these cases) the data is\nimplicitly released.\nIt is important to note that these semantics are different than those\nof the TCP::collect and TCP::release commands.",
            source: "https://clouddocs.f5.com/api/irules/HTTP__release.html",
            examples: "when CLIENT_ACCEPTED {\n    set tmm_auth_ldap_sid [AUTH::start pam default_ldap]\n}",
            return_value: "",
        }),
        forms: &[FormSpec {
            synopsis: "HTTP::release",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::HttpBody,
            writes: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}
