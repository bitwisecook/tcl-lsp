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

//! `TMM::cmp_count` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "TMM::cmp_count",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Provides the active number of TMM instances running.",
            synopsis: &["TMM::cmp_count"],
            snippet: "This command provides the active number of TMM instances running.\nTo determine the blade the iRule is currently executing on, see the\nTMM::cmp_group page. To determine the CPU ID an iRule is currently\nexecuting on within a blade, see the TMM::cmp_unit page.",
            source: "https://clouddocs.f5.com/api/irules/TMM__cmp_count.html",
            examples: "when CLIENT_ACCEPTED {\n  if { [TMM::cmp_count] >= 2 } {\n    set cmpstatus 1\n  } else { set cmpstatus 0 }\n}",
            return_value: "Returns the active number of TMM instances running.",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "TMM::cmp_count",
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
