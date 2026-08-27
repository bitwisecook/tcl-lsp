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

//! `CACHE::priority` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "CACHE::priority",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Adds a priority to cached documents.",
            synopsis: &["CACHE::priority CACHE_PRIORITY"],
            snippet: "Assigns a priority to cached documents. The priority value can be\nbetween 1 and 10 inclusive. This command allows users to designate\ndocuments that are costly to produce as being more important than\nothers to cache. This is particularly useful when you have a document\nthat is not requested often, but is expensive to produce (such as a\nserver-compressed document.) By increasing the priority, you are\nincreasing its likelihood of being served from the cache\n\nThe default priority value for entries in the cache is zero (0 = cache\npriority disabled).\n\nCACHE::priority <1 ..",
            source: "https://clouddocs.f5.com/api/irules/CACHE__priority.html",
            examples: "when HTTP_REQUEST {\n  switch -glob [HTTP::uri] {\n    \"*.zip\" -\n    \"*.tar\" -\n    \"*.gz\" {\n      # set the priority to 8 for this document if it's a compressed archive\n      CACHE::priority 8\n    }\n    \"*.gif\" -\n    \"*.jpg\" -\n    \"*.png\" {\n      # set the priority to 5 for this document if it's an image\n      CACHE::priority 5\n    }\n    \"/mustcache*\" {\n      # Any document matching /mustcache will be set to the highest priority.\n      CACHE::priority 10\n    }\n  }\n}",
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
            synopsis: "CACHE::priority CACHE_PRIORITY",
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
