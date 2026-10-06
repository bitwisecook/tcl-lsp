// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Independently captured namespace table allocation facts, without values.

use super::SourceNamespaceKey;
use crate::var_resolve::{ResolveContext, VariableCellKey};
use tcl_runtime_api::NativeCompilationEntry;

pub(super) fn install(variables: &mut ResolveContext, entry: &NativeCompilationEntry) {
    let Some(protocol) = entry.command_name_policy() else {
        return;
    };
    if protocol.authority() != tcl_syntax::naming::NamePolicyAuthority::Native
        || protocol.recipe().is_jim084()
        || entry
            .execution_point
            .and_then(tcl_syntax::naming::NamePolicyProtocol::for_native_point)
            != Some(protocol)
    {
        return;
    }
    let Some(tables) = &entry.namespace_variable_tables else {
        return;
    };
    for table in tables {
        // Reject conflicting, foreign, retired or unrepresented incarnations.
        // Neither a reporting path nor command-table closure supplies a table.
        if table.namespace.interpreter != entry.interpreter
            || tables
                .iter()
                .filter(|other| other.namespace.token == table.namespace.token)
                .count()
                != 1
            || !entry
                .namespaces
                .iter()
                .any(|row| row.token == table.namespace.token && row.visible)
            || entry
                .retained_namespace_context(table.namespace.token)
                .ok()
                .as_ref()
                != Some(&table.namespace)
        {
            continue;
        }
        // A complete analytical table needs every original key representable.
        // Omitting an opaque row would turn a partial view into false absence.
        let Some(roots) = table
            .roots
            .iter()
            .map(|name| name.try_utf8().ok().map(str::to_owned))
            .collect::<Option<Vec<_>>>()
        else {
            continue;
        };
        let identity = SourceNamespaceKey::Native(table.namespace.clone());
        variables
            .namespace_cells
            .present
            .extend(roots.into_iter().map(|simple| VariableCellKey::Namespace {
                identity: identity.clone(),
                simple,
            }));
        variables.namespace_cells.closed_namespaces.insert(identity);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::place::{self, CellOwner, Index};
    use crate::var_resolve::{ContentsPresence, ContentsWorld, VariableFrameKind};
    use tcl_runtime_api::native_compilation::NativeNamespaceVariableTable;

    fn fixture() -> (NativeCompilationEntry, ResolveContext, place::Place) {
        let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
        let entry = crate::environment_ingress::captured_native_entry(profile);
        assert!(
            entry.namespace_variable_tables.is_some(),
            "actual table issuer"
        );
        let native = entry
            .retained_namespace_context(entry.current_namespace)
            .unwrap();
        let mut variables = ResolveContext {
            frame_kind: VariableFrameKind::Global,
            invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
            ..Default::default()
        };
        variables.widen();
        let mut root = place::scalar("fresh_array", "::", false);
        root.cell = Some(place::CellIdentity {
            owner: CellOwner::NamespaceIdentity(Box::new(SourceNamespaceKey::Native(native))),
            name: root.name.clone(),
            generation: place::CellGeneration::default(),
            interpreter: None,
            storage_domain: None,
            execution: None,
        });
        (entry, variables, root)
    }

    #[test]
    fn captured_namespace_table_proves_only_absence_before_literal_array_birth() {
        let (entry, mut variables, root) = fixture();
        install(&mut variables, &entry);
        assert_eq!(variables.contents_world, ContentsWorld::Unknown);
        assert_eq!(
            variables.contents_presence(&root),
            ContentsPresence::Undefined
        );
        variables.dynamic_bindings = false;
        let mut member = root.clone();
        member.kind = place::PlaceKind::ArrayElem;
        member.index = Some(Index::literal("x"));
        variables.publish_captured_store(&member, Some("X"), 10);
        let mut other = member.clone();
        other.index.as_mut().unwrap().value = "absent".into();
        assert_eq!(
            variables.contents_presence(&other),
            ContentsPresence::Undefined
        );
        assert_eq!(
            variables.contents_presence(&member),
            ContentsPresence::Defined
        );
        variables.widen(); // Unknown callback may mutate any table or member.
        assert_eq!(
            variables.contents_presence(&other),
            ContentsPresence::Unknown
        );
        assert!(variables.closed_array_roots.is_empty());
    }

    #[test]
    fn missing_or_foreign_table_cannot_close_an_incoming_variable_world() {
        let (mut entry, mut variables, root) = fixture();
        entry.namespace_variable_tables = None;
        install(&mut variables, &entry);
        assert_eq!(
            variables.contents_presence(&root),
            ContentsPresence::Unknown
        );
        let mut namespace = entry
            .retained_namespace_context(entry.current_namespace)
            .unwrap();
        namespace.interpreter.owner = namespace.interpreter.owner.checked_add(1).unwrap();
        entry.namespace_variable_tables = Some(vec![NativeNamespaceVariableTable {
            namespace,
            roots: Vec::new(),
        }]);
        install(&mut variables, &entry);
        assert_eq!(
            variables.contents_presence(&root),
            ContentsPresence::Unknown
        );
    }

    #[test]
    fn allocated_undefined_or_opaque_root_never_becomes_a_fresh_array_proof() {
        let (mut entry, mut variables, root) = fixture();
        let namespace = entry
            .retained_namespace_context(entry.current_namespace)
            .unwrap();
        entry.namespace_variable_tables = Some(vec![NativeNamespaceVariableTable {
            namespace: namespace.clone(),
            roots: vec!["fresh_array".into()],
        }]);
        install(&mut variables, &entry);
        assert_eq!(
            variables.contents_presence(&root),
            ContentsPresence::Unknown
        );
        variables.namespace_cells.widen();
        entry.namespace_variable_tables = Some(vec![NativeNamespaceVariableTable {
            namespace,
            roots: vec![tcl_core_types::NameBytes::from(&b"\xff"[..])],
        }]);
        install(&mut variables, &entry);
        let mut different = root;
        different.name = "different".into();
        assert_eq!(
            variables.contents_presence(&different),
            ContentsPresence::Unknown
        );
    }

    #[test]
    fn captured_table_absence_does_not_cross_colliding_namespace_reports() {
        let (mut entry, mut variables, mut root) = fixture();
        let mut first = entry.namespaces[0].clone();
        first.token = 91;
        first.path = tcl_core_types::ByteNamespacePath::from_segments(["a:", "b"]);
        let mut second = first.clone();
        second.token = 92;
        second.path = tcl_core_types::ByteNamespacePath::from_segments(["a", ":b"]);
        entry.namespaces.extend([first, second]);
        let first = SourceNamespaceKey::Native(entry.retained_namespace_context(91).unwrap());
        let second = SourceNamespaceKey::Native(entry.retained_namespace_context(92).unwrap());
        assert_eq!(first.display(), second.display());
        entry.namespace_variable_tables = Some(vec![NativeNamespaceVariableTable {
            namespace: entry.retained_namespace_context(91).unwrap(),
            roots: Vec::new(),
        }]);
        install(&mut variables, &entry);
        root.cell.as_mut().unwrap().owner = CellOwner::NamespaceIdentity(Box::new(first));
        assert_eq!(
            variables.contents_presence(&root),
            ContentsPresence::Undefined
        );
        root.cell.as_mut().unwrap().owner = CellOwner::NamespaceIdentity(Box::new(second));
        assert_eq!(
            variables.contents_presence(&root),
            ContentsPresence::Unknown
        );
    }

    #[test]
    fn fresh_array_inventory_survives_only_agreeing_predecessors() {
        let (entry, mut variables, root) = fixture();
        install(&mut variables, &entry);
        variables.dynamic_bindings = false;
        let mut member = root;
        member.kind = place::PlaceKind::ArrayElem;
        member.index = Some(Index::literal("x"));
        variables.publish_captured_store(&member, Some("X"), 10);
        let mut same = variables.clone();
        same.join(&variables);
        assert_eq!(same.closed_array_roots, variables.closed_array_roots);
        let mut mutated = variables.clone();
        mutated.widen();
        same.join(&mutated);
        assert!(same.closed_array_roots.is_empty());
        assert!(same.namespace_cells.closed_namespaces.is_empty());
    }
}
