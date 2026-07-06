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

//! `trace` — monitor variable accesses, command usages and executions.
use crate::prelude::*;

const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::InterpState,
    reads: false,
    writes: true,
    connection_side: ConnectionSide::None,
}];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "trace option ?arg arg ...?",
}];

/// Arg-role resolver for `trace add`.
///
/// `trace add variable name ops commandPrefix` writes to `name` —
/// the trace handler can rewrite the variable at runtime, so SSA
/// must see `name` as a definition site.
///
/// The resolver only fires for the `variable` form so
/// `trace add execution` and `trace add command` (which take a
/// command name, not a variable) don't appear as SSA defs.
fn trace_add_arg_roles(args: &[&str]) -> Vec<(u8, ArgRole)> {
    if args.first() == Some(&"variable") && args.len() >= 2 {
        return vec![(1, ArgRole::VarWrite)];
    }
    Vec::new()
}

/// Same arg-role pattern for `trace remove variable` — keeps
/// registry consistency with `trace add variable` so consumers can
/// query both spellings via the same `ArgRole::VarWrite` lookup.
fn trace_remove_arg_roles(args: &[&str]) -> Vec<(u8, ArgRole)> {
    if args.first() == Some(&"variable") && args.len() >= 2 {
        return vec![(1, ArgRole::VarWrite)];
    }
    Vec::new()
}

static SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "add",
        traits: Traits::TARGETS_VARIABLE_BY_NAME,
        arity: Arity::exact(4),
        detail: "Arrange for a command to be executed on the specified operation.",
        synopsis: "trace add type name ops commandPrefix",
        arg_role_resolver: Some(trace_add_arg_roles),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "info",
        traits: Traits::TARGETS_VARIABLE_BY_NAME,
        arity: Arity::exact(2),
        detail: "Return trace info for the given name.",
        synopsis: "trace info type name",
        pure: true,
        return_type: Some(TclType::List),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "remove",
        traits: Traits::TARGETS_VARIABLE_BY_NAME,
        arity: Arity::exact(4),
        detail: "Remove a trace.",
        synopsis: "trace remove type name ops commandPrefix",
        arg_role_resolver: Some(trace_remove_arg_roles),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "variable",
        traits: Traits::TARGETS_VARIABLE_BY_NAME,
        arity: Arity::exact(3),
        detail: "Arrange for command to be executed whenever variable name is accessed. Deprecated in favour of trace add variable.",
        synopsis: "trace variable name ops command",
        // Deprecated legacy form; removed in Tcl 9.0 (8.4-8.6 only).
        dialects: Some(DialectSet::TCL8X),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "vdelete",
        traits: Traits::TARGETS_VARIABLE_BY_NAME,
        arity: Arity::exact(3),
        detail: "Delete a variable trace. Deprecated in favour of trace remove variable.",
        synopsis: "trace vdelete name ops command",
        // Deprecated legacy form; removed in Tcl 9.0 (8.4-8.6 only).
        dialects: Some(DialectSet::TCL8X),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "vinfo",
        traits: Traits::TARGETS_VARIABLE_BY_NAME,
        arity: Arity::exact(1),
        detail: "Return trace information for the given variable. Deprecated in favour of trace info variable.",
        synopsis: "trace vinfo name",
        // Deprecated legacy form; removed in Tcl 9.0 (8.4-8.6 only).
        dialects: Some(DialectSet::TCL8X),
        ..SubCommand::DEFAULT
    },
];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "trace",
        traits: Traits::CREATES_BARRIER | Traits::CREATES_DYNAMIC_BARRIER,
        arity: Arity::at_least(1),
        subcommands: SUBCOMMANDS,
        hover: Some(HoverSnippet {
            summary: "Monitor variable accesses, command usages and command executions",
            synopsis: &["trace option ?arg arg ...?"],
            snippet: "Arranges for commands to be executed whenever certain operations are invoked.",
            source: "Tcl man page trace.n",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        ..CommandSpec::DEFAULT
    }
}
