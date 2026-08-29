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

//! `STREAM::max_matchsize` iRules command.
use crate::prelude::*;
use tcl_dialect::model::{SpecSurface};
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "STREAM::max_matchsize",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Sets a maximum number of bytes that the system can buffer during partial matches.",
            synopsis: &["STREAM::max_matchsize SIZE"],
            snippet: "Sets the maximum size, in bytes, that the system can buffer during\npartial matches. The default value is 4096.\nThe STREAM profile will buffer data for partial matches; if more than\nmax_matchsize would be buffered, the connection will be torn down. This\nway a regex like foobarbaz+ won't keep matching until the box runs\nout of memory. The default is 4K, and STREAM::max_matchsize can be\nuse to set it to something else.",
            source: "https://clouddocs.f5.com/api/irules/STREAM__max_matchsize.html",
            examples: "when HTTP_RESPONSE {\n    STREAM::max_matchsize 2048\n}",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["FASTHTTP"],
            also_in: &[],
            flow: false,
        }),
        forms: &[FormSpec {
            synopsis: "STREAM::max_matchsize SIZE",
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
