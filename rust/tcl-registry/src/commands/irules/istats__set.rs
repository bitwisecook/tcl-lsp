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

//! `ISTATS::set` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "ISTATS::set",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Sets the given key's value within iStats.",
            synopsis: &["ISTATS::set KEY VALUE"],
            snippet: "Set the given key's value within iStats",
            source: "https://clouddocs.f5.com/api/irules/ISTATS__set.html",
            examples: "when HTTP_REQUEST {\n  # send request to /invalidate?policy=<policy>\n  if { [HTTP::path] eq \"/invalidate\" } {\n        set wa_policy [URI::query [HTTP::uri] policy]\n        if { $wa_policy ne \"\" } {\n          ISTATS::set \"WA policy string $wa_policy\" \"invalidated\"\n        }\n        HTTP::respond 200 content \"<html><body>Cache Invalidated for Policy: $wa_policy</body></html>\"\n  }\n}",
            return_value: "",
        }),
        forms: &[FormSpec {
            synopsis: "ISTATS::set KEY VALUE",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::IStats,
            writes: true,
            connection_side: ConnectionSide::Global,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}
