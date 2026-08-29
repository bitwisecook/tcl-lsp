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

//! `BWC::priority` iRules command.
use crate::prelude::*;
use tcl_dialect::model::SpecSurface;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "BWC::priority",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "This command is used to create a priority class for a bwc policy.",
            synopsis: &["BWC::priority (PRIORITY_CLASS WEIGHT)"],
            snippet: "A BWC policy instance or category can be mapped to a priority class of a priority group. This is part of the configuration and can be done via tmsh or GUI. Once a BWC instance has these mappings we can use the iRule defined below to change those. The syntax for this iRule is like below,\n\nBWC::priority <name1> <value1> [<name2> <value2>] [<name3> <value3>] [<name4> <value4>]\n\n    nameX - name of a priority class. valueX - weight of the priority class in percentage.\n\nIn the above iRule you can specify one or more traffic classes and specify the desired weight-percentage for that priority-class.",
            source: "https://clouddocs.f5.com/api/irules/BWC__priority.html",
            examples: "when SERVER_CONNECTED {\n\n    set my_policy bwc_policy set my_cat none set my_cookie [IP::remote_addr] switch -glob [TCP::remote_port] {\n        \"101\" {\n            set my_cat c1\n        }\n        \"102\" {\n            set my_cat c2\n        }\n    }\n    BWC::policy attach $my_policy $my_cookie if { $my_cat != \"none\" } {\n        log local0. \"BWC::color set $my_policy $my_cat\" BWC::color set $my_policy $my_cat BWC::priority \"tc1\" 60 \"tc2\" 40\n    }\n}",
            return_value: "",
        }),
        forms: &[FormSpec {
            synopsis: "BWC::priority (PRIORITY_CLASS WEIGHT)",
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
