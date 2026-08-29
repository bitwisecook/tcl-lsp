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

//! `SSL::sessionid` iRules command.
use crate::prelude::*;
use tcl_dialect::model::{SpecSurface};
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "SSL::sessionid",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Gets the SSL session ID.",
            synopsis: &["SSL::sessionid (desired)?"],
            snippet: "Gets the SSL session ID.",
            source: "https://clouddocs.f5.com/api/irules/SSL__sessionid.html",
            examples: "when CLIENTSSL_CLIENTCERT {\n    set cert [SSL::cert 0]\n    set sid [SSL::sessionid]\n    if { $sid ne \"\" } {\n        # If this SSL session will be cached, then it may be\n        # resumed later on a new connection. Cache the cert\n        # in the session table in case that happens. Because ID's\n        # are not globally unique, the session id needs to be combined\n        # with something from client address to avoid mismatch.\n        set key [concat [IP::remote_addr]@$sid]",
            return_value: "SSL::sessionid Returns the current connection's SSL session ID if it exists in the session cache. In version 10.x and higher, if the session ID does not exist in the cache, returns a null string. In version 9.x, if the session ID does not exist in the cache, returns a string of 64 zeroes.",
        }),
        forms: &[FormSpec {
            synopsis: "SSL::sessionid (desired)?",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::SslState,
            reads: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}
