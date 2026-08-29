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

//! `TMM::cmp_group` iRules command.
use crate::prelude::*;
use tcl_dialect::model::{SpecSurface};
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "TMM::cmp_group",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns the number (0-x) of the group of the CPU executing the rule.",
            synopsis: &["TMM::cmp_group"],
            snippet: "This command returns the number (0-x) of the group of the CPU currently\nexecuting the rule. Typically, a group refers to the blade number on a\nchassis system, and is always 0 on other platforms. New meanings may be\nadded for future platform architectures.\nThis is helpful if you believe one CPU is doing something it shouldn't\nand you want to isolate the issue rather than see an aggregate of all\nCPUs.\nTo determine the total number of TMM instances running, see the\nTMM::cmp_count page. To determine the CPU ID an iRule is current\nexecuting on within a blade, see the TMM::cmp_unit page.",
            source: "https://clouddocs.f5.com/api/irules/TMM__cmp_group.html",
            examples: "# Note this example won't work in 10.1.0 - 10.2.2 and 11.0.x\n# as the iRule parser doesn't allow these commands in RULE_INIT\nwhen RULE_INIT {\n\n   # Check if we're running on the first CPU right now\nif { [TMM::cmp_unit] == 0 && [TMM::cmp_group] == 0 } {\n      # This execution is happening on the first TMM instance\n      # Conduct any initialization functionality just once here\n      log local0. \"some code\"\n   }\n}",
            return_value: "Returns the number (0-x) of the group of the CPU executing the rule.",
        }),
        forms: &[FormSpec {
            synopsis: "TMM::cmp_group",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::BigipConfig,
            reads: true,
            connection_side: ConnectionSide::Global,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}
