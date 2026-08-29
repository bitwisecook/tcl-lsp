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

//! `CACHE::userkey` iRules command.
use crate::prelude::*;
use tcl_dialect::model::SpecSurface;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "CACHE::userkey",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Allows users to add user-defined values to the key used by the cache to reference the cached content.",
            synopsis: &["CACHE::userkey KEY"],
            snippet: "By default, cached content is stored with a unique key referring to both\nthe URI of the resource to be cached and the User-Agent for which it\nwas formatted. If multiple variations of the same content must be\ncached under specific conditions (different client), you can use this\ncommand to create a unique key, thus creating cached content specific\nto that condition. This can be used to prevent one user or group's\ncached data from being served to different users/groups.",
            source: "https://clouddocs.f5.com/api/irules/CACHE__userkey.html",
            examples: "when HTTP_REQUEST {\n  if {[matchclass [IP::client_addr] equals $::InternalIPs]} {\n    CACHE::userkey \"Internal\"\n  } else {\n    CACHE::userkey \"External\"\n  }\n}",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["HTTP"],
            also_in: &[],
            flow: false,
        }),
        forms: &[FormSpec {
            synopsis: "CACHE::userkey KEY",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::StreamProfile,
            writes: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}
