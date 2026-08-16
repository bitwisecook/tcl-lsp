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

//! `call` iRules command.
use crate::prelude::*;

fn call_arg_roles(args: &[&str]) -> Vec<(u8, ArgRole)> {
    let index = u8::from(args.first() == Some(&"-debug"));
    (usize::from(index) < args.len())
        .then_some((index, ArgRole::Name))
        .into_iter()
        .collect()
}

pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "call",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(1),
        traits: Traits::INVOKES_USER_PROC,
        hover: Some(HoverSnippet {
            summary: "Calls an iRule procedure.",
            synopsis: &["call ?-debug? <proc_name> ?arg ...?"],
            snippet: "iRule procedures:\n    - Are similar to procedures, functions, subroutines from other languages\n    - Allow for reuse of common code\n        - Reference the same code from multiple locations, but only define it in one place\n        - Simplifies code maintenance\n    - Allow you to augment the predefined iRule commands\n\nProcedures are defined with the proc statement. This must be done\noutside of any event. Procedures can be defined within an iRule\nassigned to a virtual server or in a separate iRule not assigned to\nany virtual server.\n\nCall a local proc (defined in the same iRule) without a namespace prefix:\n    call my_proc $args\n\nTo reference a proc in another iRule in the same partition, prefix with\nthe iRule name:\n    call other_rule::my_proc $args\n\nTo reference a proc in another partition:\n    call /other_partition/other_rule::procname args",
            source: "https://clouddocs.f5.com/api/irules/call.html",
            examples: "when RULE_INIT {\n    # Call a proc which returns no values\n    call proc_rule::printArguments one two three\n\n    # Save the return value of a proc\n    set return_values [call proc_rule::returnArguments one two three]\n}",
            return_value: "Returns the value(s) that return (if any).",
        }),
        forms: &[FormSpec {
            synopsis: "call ?-debug? <proc_name> ?arg ...?",
            ..FormSpec::DEFAULT
        }],
        options: const {
            &[OptionSpec {
                name: "-debug",
                value: OptionValue::flag(),
                detail: "Enable debug mode.",
                dialects: None,
                aliases: &[],
                lifecycle: Lifecycle::UNSPECIFIED,
                min_abbrev: None,
            }]
        },
        side_effects: &[SideEffect {
            target: SideEffectTarget::ProcDefinition,
            reads: true,
            connection_side: ConnectionSide::Global,
            ..SideEffect::DEFAULT
        }],
        arg_role_resolver: Some(call_arg_roles),
        ..CommandSpec::DEFAULT
    }
}
