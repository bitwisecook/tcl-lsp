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

fn original(source: &str) -> SourceCommandBindings {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let profile = registry.profile().unwrap();
    SourceCommandBindings::analyse_with_options(
        source,
        tcl_lexer::LexerConfig::from_grammar(profile.grammar),
        registry,
        SourceAnalysisOptions {
            invocation_dialect: Some(tcl_native_model::InvocationDialect::of_profile(profile)),
            native_compilation: crate::environment_ingress::authoring_native_compilation(),
            ..Default::default()
        },
    )
}

#[test]
fn actual_same_owner_join_keeps_shared_storage_and_complete_semantics() {
    // naming.command.shared-noop-source-joins
    // docs/design/analysis/name-resolution-proofs/shared-noop-source-joins.md
    // Original model snapshot storage only; no external interpreter or
    // Native currency/handler/read/write authority is asserted here.
    let bindings = original("namespace eval N {variable value 1}; proc helper {} {}");
    let actual = bindings.final_state.as_ref();
    let mut branch = actual.clone();
    let before = branch.clone();
    assert!(Arc::ptr_eq(&branch.baseline, &actual.baseline));
    assert!(Arc::ptr_eq(
        branch.current_source_origin.as_ref().unwrap(),
        actual.current_source_origin.as_ref().unwrap()
    ));
    assert!(!branch.join(actual));
    assert_eq!(branch, before);
    assert!(Arc::ptr_eq(
        &branch.object_instances,
        &actual.object_instances
    ));
    assert!(Arc::ptr_eq(
        &branch.class_definitions,
        &actual.class_definitions
    ));
    assert!(Arc::ptr_eq(
        &branch.bounded_constructions,
        &actual.bounded_constructions
    ));
    assert!(Arc::ptr_eq(
        &branch.command_observers,
        &actual.command_observers
    ));
    assert!(Arc::ptr_eq(
        &branch.tainted_object_dispatch,
        &actual.tainted_object_dispatch
    ));
    assert!(Arc::ptr_eq(
        &branch.namespace_exports,
        &actual.namespace_exports
    ));
    assert!(Arc::ptr_eq(
        &branch.namespace_paths,
        &actual.namespace_paths
    ));
    assert!(Arc::ptr_eq(
        &branch.unknown_lookup_namespaces,
        &actual.unknown_lookup_namespaces
    ));
    assert!(Arc::ptr_eq(
        &branch.unknown_namespace_paths,
        &actual.unknown_namespace_paths
    ));
    assert!(Arc::ptr_eq(
        &branch.namespace_unknown_handlers,
        &actual.namespace_unknown_handlers
    ));
    assert!(Arc::ptr_eq(
        &branch.unknown_export_namespaces,
        &actual.unknown_export_namespaces
    ));
    assert!(Arc::ptr_eq(
        &branch.revoked_loaders,
        &actual.revoked_loaders
    ));
    assert!(Arc::ptr_eq(
        &branch.tainted_provider_state,
        &actual.tainted_provider_state
    ));
}

#[test]
fn genuine_different_observer_and_namespace_owners_still_join_without_mutating_snapshots() {
    // naming.command.shared-noop-source-joins
    // docs/design/analysis/name-resolution-proofs/shared-noop-source-joins.md
    let source = "proc helper {} {}; trace add execution helper enter {list}";
    let observed = original(source);
    let site = u32::try_from(source.find("trace add").unwrap()).unwrap();
    let previous = &observed.dispatch_points_at(site).next().unwrap().state;
    let actual = observed.final_state.as_ref();
    assert!(Arc::ptr_eq(&previous.baseline, &actual.baseline));
    assert_ne!(previous.command_observers, actual.command_observers);
    assert!(!Arc::ptr_eq(
        &previous.command_observers,
        &actual.command_observers
    ));
    let old = actual.clone();
    let mut joined = actual.clone();
    assert!(joined.join(previous));
    assert_ne!(joined.command_observers, actual.command_observers);
    assert_ne!(joined.command_observers, previous.command_observers);
    assert_eq!(actual, &old);
    let stable = joined.clone();
    assert!(!joined.join(previous));
    assert_eq!(joined, stable);

    let source = "namespace eval N {}; namespace delete N";
    let removed = original(source);
    let site = u32::try_from(source.find("namespace delete").unwrap()).unwrap();
    let previous = &removed.dispatch_points_at(site).next().unwrap().state;
    let actual = removed.final_state.as_ref();
    let removed_namespaces = previous
        .namespaces
        .iter()
        .filter(|namespace| !actual.namespaces.contains(*namespace))
        .cloned()
        .collect::<Vec<_>>();
    assert_eq!(
        removed_namespaces.len(),
        1,
        "the actual namespace transition removes one typed owner"
    );
    let namespace = &removed_namespaces[0];
    assert!(previous.namespaces.contains(namespace));
    assert!(!actual.namespaces.contains(namespace));
    let old = previous.clone();
    let mut joined = previous.clone();
    assert!(joined.join(actual));
    assert!(!joined.namespaces.contains(namespace));
    assert_eq!(previous, &old);
    assert!(previous.namespaces.contains(namespace));
}
