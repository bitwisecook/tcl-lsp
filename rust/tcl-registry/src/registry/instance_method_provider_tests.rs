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
use crate::spec::ObjectClassSpec;

fn registry() -> CommandRegistry {
    static CHILD_METHODS: [SubCommand; 1] = [SubCommand {
        name: "own",
        detail: "child",
        ..SubCommand::DEFAULT
    }];
    static PARENT_METHODS: [SubCommand; 2] = [
        SubCommand {
            name: "inherited",
            lifecycle: Lifecycle::introduced_in("2.0"),
            ..SubCommand::DEFAULT
        },
        SubCommand {
            name: "future",
            lifecycle: Lifecycle::introduced_in("3.0"),
            ..SubCommand::DEFAULT
        },
    ];
    static CHILD: ObjectClassSpec = ObjectClassSpec {
        class_name: "ProviderChild",
        instance_methods: &CHILD_METHODS,
        superclasses: &["ProviderParent"],
        allow_unknown_methods: false,
        method_prefix_matching: PrefixMatching::Strict,
    };
    static PARENT: ObjectClassSpec = ObjectClassSpec {
        class_name: "ProviderParent",
        instance_methods: &PARENT_METHODS,
        superclasses: &["ProviderChild"],
        allow_unknown_methods: false,
        method_prefix_matching: PrefixMatching::Strict,
    };
    let mut registry = CommandRegistry::build_default();
    for (class, package) in [(&CHILD, "ChildPackage"), (&PARENT, "ParentPackage")] {
        registry.insert(CommandSpec {
            name: class.class_name,
            object_class: Some(class),
            required_package: Some(package),
            ..CommandSpec::DEFAULT
        });
    }
    registry
}

#[test]
fn inherited_method_provider_keeps_exact_descriptor_and_own_package_floor() {
    let registry = registry();
    let selected = registry.get("ProviderChild").unwrap();
    let rows = registry.instance_method_providers_for_descriptor_at(selected, None, |owner| {
        Some(match owner.owning_package() {
            Some("ChildPackage") => Some("1.0"),
            Some("ParentPackage") => Some("2.0"),
            _ => panic!("unexpected provider"),
        })
    });
    assert_eq!(
        rows.iter()
            .map(|(_, method)| method.name)
            .collect::<Vec<_>>(),
        ["own", "inherited"]
    );
    assert!(std::ptr::eq(
        rows[1].0,
        registry.get("ProviderParent").unwrap()
    ));
    assert_eq!(rows[1].0.owning_package(), Some("ParentPackage"));
    assert_eq!(
        registry
            .instance_methods_for_descriptor_at(selected, Some("1.0"), None)
            .iter()
            .map(|method| method.name)
            .collect::<Vec<_>>(),
        ["own"],
        "the old uniform-floor API keeps its existing scope"
    );
    let foreign = selected.clone();
    assert!(
        registry
            .instance_method_providers_for_descriptor_at(&foreign, None, |_| Some(None))
            .is_empty()
    );
    let rows = registry.instance_method_providers_for_descriptor_at(selected, None, |owner| {
        (owner.name != "ProviderParent").then_some(None)
    });
    assert_eq!(
        rows.iter()
            .map(|(_, method)| method.name)
            .collect::<Vec<_>>(),
        ["own"]
    );
}
