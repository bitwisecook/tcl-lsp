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

//! The lifecycle containment pass: every child lifecycle of a finished command or subcommand stays inside its parent's window.

use super::super::Log;
use super::super::value_parsers::check_contained;
use tcl_registry::spec::{CommandSpec, SubCommand};

/// Report every lifecycle in a finished command that reaches outside the
/// window of the declaration enclosing it.
///
/// Run once the whole body is read, because a `command` may state its own
/// releases *after* the rows they gate — which is also why every notice
/// carries the `command` statement's line rather than the row's.
pub(super) fn check_command_containment(spec: &CommandSpec, line: u32, log: &mut Log) {
    let parent = spec.lifecycle;
    for window in spec.arity_windows {
        check_contained(
            &format!("arity window {:?}", window.arity),
            window.lifecycle,
            parent,
            line,
            log,
        );
    }
    for option in spec.options {
        check_contained(
            &format!("option `{}`", option.name),
            option.lifecycle,
            parent,
            line,
            log,
        );
    }
    for constraint in spec.option_relations {
        check_contained(
            &constraint.describe(),
            constraint.lifecycle,
            parent,
            line,
            log,
        );
    }
    for form in spec.forms {
        check_contained(
            &format!("form `{}`", form.synopsis),
            form.lifecycle,
            parent,
            line,
            log,
        );
    }
    for effect in spec.side_effects.lower_bound() {
        check_contained(
            &format!("side_effect `{:?}`", effect.target),
            effect.lifecycle,
            parent,
            line,
            log,
        );
    }
    for (index, values) in spec.arg_values {
        for value in *values {
            check_contained(
                &format!("value `{}` of arg {index}", value.value),
                value.lifecycle,
                parent,
                line,
                log,
            );
        }
    }
    for gate in spec.versioned_arg_values {
        check_contained(
            &format!("versioned_arg_value `{}`", gate.value),
            gate.lifecycle,
            parent,
            line,
            log,
        );
    }
    for sub in spec.subcommands {
        check_contained(
            &format!("subcommand `{}`", sub.name),
            sub.lifecycle,
            parent,
            line,
            log,
        );
        check_subcommand_containment(sub, line, log);
    }
}

/// The same pass for one subcommand's own children.
fn check_subcommand_containment(sub: &SubCommand, line: u32, log: &mut Log) {
    let parent = sub.lifecycle;
    let path = sub.name;
    for window in sub.arity_windows {
        check_contained(
            &format!("subcommand `{path}` arity window {:?}", window.arity),
            window.lifecycle,
            parent,
            line,
            log,
        );
    }
    for option in sub.options {
        check_contained(
            &format!("subcommand `{path}` option `{}`", option.name),
            option.lifecycle,
            parent,
            line,
            log,
        );
    }
    for constraint in sub.option_relations {
        check_contained(
            &format!("subcommand `{path}` `{}`", constraint.describe()),
            constraint.lifecycle,
            parent,
            line,
            log,
        );
    }
    for effect in sub.side_effects.lower_bound() {
        check_contained(
            &format!("subcommand `{path}` side_effect `{:?}`", effect.target),
            effect.lifecycle,
            parent,
            line,
            log,
        );
    }
    for row in sub.sub_subcommands {
        check_contained(
            &format!("sub_subcommand `{path} {}`", row.name),
            row.lifecycle,
            parent,
            line,
            log,
        );
    }
    for (index, values) in sub.arg_values {
        for value in *values {
            check_contained(
                &format!("subcommand `{path}` value `{}` of arg {index}", value.value),
                value.lifecycle,
                parent,
                line,
                log,
            );
        }
    }
    for gate in sub.versioned_arg_values {
        check_contained(
            &format!("subcommand `{path}` versioned_arg_value `{}`", gate.value),
            gate.lifecycle,
            parent,
            line,
            log,
        );
    }
}
