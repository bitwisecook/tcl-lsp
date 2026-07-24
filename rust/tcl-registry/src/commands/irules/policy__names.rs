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

//! `POLICY::names` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "POLICY::names",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns details about the policy names for the virtual server the iRule is enabled on.",
            synopsis: &["POLICY::names (active | matched | unmatched)"],
            snippet: "iRule command which returns details about the policy names for the\nvirtual server the iRule is enabled on.",
            source: "https://clouddocs.f5.com/api/irules/policy__names.html",
            examples: "# Log the policy names for this virtual server\nwhen HTTP_REQUEST {\n        log local0. \"Enabled on this VS: \\[POLICY::names active\\]: [POLICY::names active]\"\n        log local0. \"Matched: \\[POLICY::names matched\\]: [POLICY::names matched]\"\n        log local0. \"Not matched: \\[POLICY::names unmatched\\]: [POLICY::names unmatched]\"\n}",
            return_value: "",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "POLICY::names (active | matched | unmatched)",
            dialects: None,
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::BigipConfig,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Global,
            dialects: None,
        }],
        ..CommandSpec::DEFAULT
    }
}
