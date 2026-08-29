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

//! `STREAM::replace` iRules command.
use crate::prelude::*;
use tcl_dialect::model::{SpecSurface};
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "STREAM::replace",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Changes a replacement string in the Stream profile.",
            synopsis: &["STREAM::replace (TARGET_STRING)?"],
            snippet: "Changes the specified target replacement string in the Stream profile.\nThis command is not sticky and is applied only once during the current\nmatch. If the target expression is missing, the replacement is skipped.",
            source: "https://clouddocs.f5.com/api/irules/STREAM__replace.html",
            examples: "when STREAM_MATCHED {\n    set server [string tolower [STREAM::match]]\n    if {$server contains \"mail\"} {\n        STREAM::replace \"webmail.yourdomain.com/$mailhost\"\n    }\n}",
            return_value: "",
        }),
        forms: &[FormSpec {
            synopsis: "STREAM::replace (TARGET_STRING)?",
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
