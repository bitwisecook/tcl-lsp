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

//! `priority` iRules command.
use crate::prelude::*;
use tcl_dialect::model::SpecSurface;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "priority",
        traits: Traits::IRULES_TOP_LEVEL_ONLY.union(Traits::SETS_EVENT_PRIORITY),
        surface: Some(SpecSurface::IRULES),
        arity: Arity::exact(1),
        irules_top_level_effect: Some(IrulesTopLevelEffect::Priority),
        hover: Some(HoverSnippet {
            summary: "Sets the order of execution for iRule events.",
            synopsis: &["priority EVENT_PRIORITY"],
            snippet: "The priority command is used as an attribute associated with any iRule\nevent. When the iRules are loaded into the internal iRules engine for a\ngiven virtual server, they are stored in a table with the event name\nand a priority (with a default of 500).\nLower numbered priority events are evaluated before higher numbered\npriority events: When an event is triggered an event, the irules engine\npasses control to each of the code blocks for that given event in the\norder of lowest to highest priority.",
            source: "https://clouddocs.f5.com/api/irules/priority.html",
            examples: "when CLIENT_ACCEPTED {\n       log \"Client [IP::remote_addr] connected\"\n    }",
            return_value: "",
        }),
        forms: &[FormSpec {
            synopsis: "priority EVENT_PRIORITY",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::ConnectionControl,
            writes: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}
