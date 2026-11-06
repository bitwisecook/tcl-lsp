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

//! Exact native argument-role descriptor IDs for authored spec packs.

use crate::{
    CommandRegistry,
    spec::{ArgRoleCountResolver, ArgRoleLayoutResolver},
};

/// A selected native metadata hook, without invocation or binding authority.
#[derive(Clone, Copy, Debug)]
pub enum NativeArgumentRoleHook {
    /// Hook over evaluated argument cardinality.
    Count(ArgRoleCountResolver),
    /// Hook over retained structured operands.
    Layout(ArgRoleLayoutResolver),
}

impl CommandRegistry {
    /// Resolve an exact command/member hook ID in this registry's catalogue.
    /// An exact qualified command takes precedence over a member interpretation,
    /// including when that command has no hook of the requested kind.
    #[must_use]
    pub fn native_argument_role_hook(&self, id: &str) -> Option<NativeArgumentRoleHook> {
        let (owner, field) = id.rsplit_once("::")?;
        if let Some(command) = self.get(owner) {
            return selected_hook(
                field,
                command.arg_role_count_resolver,
                command.arg_role_layout_resolver,
            );
        }
        let (command, member) = owner.rsplit_once("::")?;
        let member = self
            .get(command)?
            .subcommands
            .iter()
            .find(|row| row.name == member)?;
        selected_hook(
            field,
            member.arg_role_count_resolver,
            member.arg_role_layout_resolver,
        )
    }
}

fn selected_hook(
    field: &str,
    count: Option<ArgRoleCountResolver>,
    layout: Option<ArgRoleLayoutResolver>,
) -> Option<NativeArgumentRoleHook> {
    match field {
        "arg_role_count_resolver" => count.map(NativeArgumentRoleHook::Count),
        "arg_role_layout_resolver" => layout.map(NativeArgumentRoleHook::Layout),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{CommandRegistry, NativeArgumentRoleHook};

    #[test]
    fn native_role_ids_keep_qualified_command_precedence_over_members() {
        let mut registry = CommandRegistry::build_default();
        assert!(matches!(
            registry.native_argument_role_hook("trace::variable::arg_role_count_resolver"),
            Some(NativeArgumentRoleHook::Count(_))
        ));
        assert!(matches!(
            registry.native_argument_role_hook("unset::arg_role_layout_resolver"),
            Some(NativeArgumentRoleHook::Layout(_))
        ));
        for id in [
            "trace::variable::",
            "trace::variable::unknown",
            "trace::unknown::arg_role_count_resolver",
            "::arg_role_count_resolver",
        ] {
            assert!(registry.native_argument_role_hook(id).is_none(), "{id}");
        }
        let mut exact = registry.get("list").unwrap().clone();
        exact.name = "trace::variable";
        exact.arg_role_count_resolver = None;
        registry.insert(exact);
        assert!(
            registry
                .native_argument_role_hook("trace::variable::arg_role_count_resolver")
                .is_none()
        );
    }
}
