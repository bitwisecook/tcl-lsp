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

//! `REWRITE::enable` iRules command.
use crate::prelude::*;
use tcl_dialect::model::{SpecSurface};
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "REWRITE::enable",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Changes the REWRITE plugin from passthrough to full patching mode.",
            synopsis: &["REWRITE::enable"],
            snippet: "Changes the REWRITE plugin from passthrough to full patching mode. A\nplace where this might be helpful would be a POST request where REWRITE\nwould modify the post body unnecessarily, so we disable it. However, we\nwant REWRITE to modify the response, so we would enable it later in the\nHTTP_RESPONSE. Use of this command can be extremely tricky to get\nexactly right; its use is not recommended in the majority of cases",
            source: "https://clouddocs.f5.com/api/irules/REWRITE__enable.html",
            examples: "",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["ACCESS", "FASTHTTP", "REWRITE"],
            also_in: &[],
            flow: false,
        }),
        forms: &[FormSpec {
            synopsis: "REWRITE::enable",
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
