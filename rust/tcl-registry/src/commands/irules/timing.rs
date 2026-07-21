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

//! `timing` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "timing",
        traits: Traits::IRULES_TOP_LEVEL_ONLY,
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Enables or disables iRule timing statistics.",
            synopsis: &["timing TIMING"],
            snippet: "The timing command can be used to enable iRule timing statistics. This\nwill then collect timing information as specified each time the rule is\nevaluated. Statistics may be viewed with \"b rule show all\" or in the\nStatistics tab of the iRules Editor.\n\nNote: In 11.5.0, timing was enabled by default for all iRules in\nBZ375905. The performance impact is negligible. As a result, you no\nlonger need to use this command to view timing statistics.",
            source: "https://clouddocs.f5.com/api/irules/timing.html",
            examples: "when HTTP_REQUEST {\n    ...\n  }",
            return_value: "",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "timing TIMING",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::Unknown,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Global,
            dialects: None,
        }],
        ..CommandSpec::DEFAULT
    }
}
