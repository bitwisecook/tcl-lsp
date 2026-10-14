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

//! Package variable-name arguments feed the shared role and effect owners.

use tcl_registry::{ArgRole, CommandRegistry, Traits};

#[test]
fn tcllib_set_mutators_name_the_caller_variable() {
    let registry = CommandRegistry::build_default();
    for subcommand in ["include", "exclude", "add", "subtract"] {
        let args = [subcommand, "target", "value"];
        assert_eq!(
            registry.arg_indices_for_role("struct::set", &args, ArgRole::VarWrite),
            [1],
            "{subcommand}"
        );
        assert!(
            !registry
                .call_traits("struct::set", &args)
                .contains(Traits::PURE),
            "{subcommand}"
        );
    }
}

#[test]
fn tcllib_list_mutators_name_each_caller_destination() {
    let registry = CommandRegistry::build_default();
    for (args, expected) in [
        (vec!["shift", "target"], vec![1]),
        (vec!["swap", "target", "0", "1"], vec![1]),
        (vec!["delete", "target", "value"], vec![1]),
        (
            vec!["assign", "values", "first", "second", "third"],
            vec![2, 3, 4],
        ),
    ] {
        assert_eq!(
            registry.arg_indices_for_role("struct::list", &args, ArgRole::VarWrite),
            expected,
            "{args:?}"
        );
        assert!(
            !registry
                .call_traits("struct::list", &args)
                .contains(Traits::PURE),
            "{args:?}"
        );
    }
}

#[test]
fn tcllib_join_and_split_outputs_share_variable_name_roles() {
    let registry = CommandRegistry::build_default();
    for subcommand in ["dbJoin", "dbJoinKeyed"] {
        let args = [subcommand, "-keys", "output", "table", "other"];
        assert_eq!(
            registry.arg_indices_for_role("struct::list", &args, ArgRole::VarWrite),
            [2],
            "{subcommand}"
        );
        assert!(
            !registry
                .call_traits("struct::list", &args)
                .contains(Traits::PURE),
            "{subcommand}"
        );
    }
    assert_eq!(
        registry.arg_indices_for_role(
            "struct::list",
            &["split", "values", "predicate", "pass", "fail"],
            ArgRole::VarWrite
        ),
        [3, 4]
    );
    assert!(
        registry
            .arg_indices_for_role(
                "struct::list",
                &["split", "values", "predicate"],
                ArgRole::VarWrite
            )
            .is_empty()
    );
}

#[test]
fn tcllib_callback_members_do_not_inherit_ensemble_purity() {
    let registry = CommandRegistry::build_default();
    for subcommand in [
        "filter",
        "map",
        "fold",
        "split",
        "filterfor",
        "mapfor",
        "foreachperm",
    ] {
        assert!(
            !registry
                .call_traits("struct::list", &[subcommand])
                .contains(Traits::PURE),
            "{subcommand}"
        );
    }
    assert!(
        registry
            .call_traits("struct::set", &["contains", "values", "value"])
            .contains(Traits::PURE)
    );
}
