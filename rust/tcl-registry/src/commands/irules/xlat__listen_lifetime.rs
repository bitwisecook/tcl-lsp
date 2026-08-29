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

//! `XLAT::listen_lifetime` iRules command.
use crate::prelude::*;
use tcl_dialect::model::SpecSurface;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "XLAT::listen_lifetime",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Set/Get the listener lifetime.",
            synopsis: &["XLAT::listen_lifetime (HANDLE)+ (XLAT_LIFETIME)?"],
            snippet: "Set/Get the listener lifetime.\nValid range is between 0 and 31536000 (365 days).",
            source: "https://clouddocs.f5.com/api/irules/XLAT__listen_lifetime.html",
            examples: "when SERVER_CONNECTED {\n    set listener [XLAT::listen 30 {\n        proto [IP::protocol]\n        bind -allow [serverside {LINK::vlan_id}] -ip [serverside {IP::local_addr}]\n        server [IP::client_addr] [expr [TCP::local_port] + 1]\n        allow [LB::server addr] 0\n    }]\n    log local0. \"[XLAT::listen_lifetime $listener]\"\n}",
            return_value: "Return the listener lifetime value.",
        }),
        forms: &[FormSpec {
            synopsis: "XLAT::listen_lifetime (HANDLE)+ (XLAT_LIFETIME)?",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::LsnState,
            writes: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}
