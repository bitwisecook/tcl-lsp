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

//! `CACHE::trace` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "CACHE::trace",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Dump the list of cached objects for a HTTP profile where RAM Cache is enabled.",
            synopsis: &["CACHE::trace (MAX)?"],
            snippet: "Dump the list of cached objects for a HTTP profile where RAM Cache is\nenabled.\nThis event will execute only if a RAM Cache profile is enabled on the\nVirtual Server, and for objects that match the RAM Cache configuration.\nThe list will represent the size of the cache (Cache Size), number of\nobjects (Cache Count), and starting by the term Entity, it will list\nevery object:\n  * Pos (0001), list the position of the object in the cache\n  * Local Hits (00031/00007) indicate the number of Local Hits\n  * Remote Hits (00031/00007) indicate the number of Remote Hits",
            source: "https://clouddocs.f5.com/api/irules/CACHE__trace.html",
            examples: "when RULE_INIT {\n    set static::cache \"\"\n}",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["CACHE"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[FormSpec {
            synopsis: "CACHE::trace (MAX)?",
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
