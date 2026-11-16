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

use super::*;

#[test]
fn rooted_declaration_queries_preserve_one_root_and_exact_descriptor_bytes() {
    // naming.registry.constructed-command-declaration-keys
    // docs/design/analysis/name-resolution-proofs/registry-constructed-command-declaration-keys.md
    // Pure catalogue keys; these declarations do not install commands.
    let mut registry = CommandRegistry::build_default();
    assert!(registry.declares_command_at_rooted_key("::set"));
    assert!(!registry.declares_command_at_rooted_key("::::set"));
    assert!(!registry.declares_command_at_rooted_key("set"));
    registry.insert(CommandSpec {
        name: "::literal",
        ..CommandSpec::DEFAULT
    });
    assert!(registry.declares_command_at_rooted_key("::::literal"));
    assert!(!registry.declares_command_at_rooted_key("::literal"));
    assert!(!registry.declares_command_at_rooted_key("::::::literal"));
    for label in [":counted", ":::counted", "N:::member", "N::", "名字"] {
        registry.insert(CommandSpec {
            name: label,
            ..CommandSpec::DEFAULT
        });
        let key = tcl_syntax::naming::root_unrooted_key(label);
        assert!(registry.declares_command_at_rooted_key(&key), "{label}");
        assert_eq!(registry.get_exact(label).unwrap().name, label);
    }
}

#[test]
fn catalogue_declarations_keep_operator_context_and_package_metadata_separate() {
    // naming.registry.constructed-command-declaration-keys
    // docs/design/analysis/name-resolution-proofs/registry-constructed-command-declaration-keys.md
    // Existing authored declaration conventions; no loaded-table claim.
    let context = crate::model::ingress::static_context_for("tcl9.0");
    let registry = context.commands();
    for key in ["::set", "::tcl::mathop::+", "::oo::Helpers::self"] {
        assert!(registry.declares_command_at_rooted_key(key), "{key}");
    }
    for key in ["::+", "::my", "::self", "::unknownDeclaration"] {
        assert!(!registry.declares_command_at_rooted_key(key), "{key}");
    }
    let mut registry = CommandRegistry::build_default();
    registry.insert(CommandSpec {
        name: "Catalogue::provided",
        required_package: Some("Catalogue"),
        ..CommandSpec::DEFAULT
    });
    assert!(registry.declares_command_at_rooted_key("::Catalogue::provided"));
    assert_eq!(
        registry
            .get_exact("Catalogue::provided")
            .unwrap()
            .required_package,
        Some("Catalogue")
    );
    assert!(!registry.declares_command_at_rooted_key("::provided"));
}
