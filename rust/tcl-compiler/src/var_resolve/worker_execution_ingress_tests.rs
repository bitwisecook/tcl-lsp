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

fn retain_worker_namespace_geometry(context: &mut ResolveContext) {
    use crate::command_binding::SourceNamespaceKey;
    use tcl_core_types::ByteNamespacePath;
    use tcl_runtime_api::native_compilation::{NativeInterpreterIdentity, NativeNamespaceContext};
    let key = |token, segments: &[&str]| {
        SourceNamespaceKey::Native(NativeNamespaceContext {
            interpreter: NativeInterpreterIdentity {
                owner: 1,
                interpreter: 2,
            },
            token,
            path: ByteNamespacePath::from_segments(segments.iter().copied()),
        })
    };
    let root = key(1, &[]);
    context.invocation_dialect = Some(tcl_native_model::InvocationDialect::for_version(
        tcl_dialect::TclVersion::V8_6,
    ));
    context.hosted_execution_context = Some(tcl_registry::f5::BigIpExecutionContext::TmmIRule);
    context.retain_namespace_world(
        root.clone(),
        [root, key(2, &["static"]), key(3, &["app", "static"])],
        Some(tcl_syntax::naming::NativeNameProtocol::C(
            tcl_dialect::TclVersion::V8_6,
        )),
    );
    context.namespace_inventory = NamespaceInventoryClosure::Closed;
}

#[test]
fn execution_context_join_cannot_retain_equal_values_from_different_cells() {
    // Implementation contract: naming.variable.event-cell-execution-isolation
    // docs/design/analysis/name-resolution-proofs/variable-event-cell-execution-isolation.md
    use tcl_registry::f5::WorkerExecution;
    let registry = tcl_registry::model::ingress::static_context_for("f5-irules").commands();
    let execution = WorkerExecution {
        worker: Some(1),
        initialisation_epoch: 2,
        connection: Some(3),
    };
    let make = |execution| {
        let mut context = ResolveContext::for_function("::event");
        retain_worker_namespace_geometry(&mut context);
        context.execution = execution;
        context.define_literal("::static::counter", "7", registry);
        assert_eq!(
            context.literal_value("::static::counter", registry),
            Some("7")
        );
        context
    };
    let first = make(Some(execution));
    let mut same = first.clone();
    same.join(&first);
    assert_eq!(same.literal_value("::static::counter", registry), Some("7"));
    assert!(!same.dynamic_bindings);
    for other in [
        Some(WorkerExecution {
            worker: Some(4),
            ..execution
        }),
        Some(WorkerExecution {
            initialisation_epoch: 5,
            ..execution
        }),
        Some(WorkerExecution {
            connection: Some(6),
            ..execution
        }),
        None,
    ] {
        let mut joined = first.clone();
        joined.join(&make(other));
        assert_eq!(joined.literal_value("::static::counter", registry), None);
        assert!(joined.constant_values.is_empty());
        assert!(joined.value_representations.is_empty());
        assert!(joined.dynamic_bindings);
        assert!(joined.dynamic_traces);
        assert_ne!(
            joined.original_contents_epoch(),
            first.original_contents_epoch()
        );
        assert_ne!(joined.representation_epoch, first.representation_epoch);
    }
}

#[test]
fn reached_helper_and_alias_retain_the_callers_worker_namespace() {
    use tcl_registry::f5::{VariableStorageDomain, WorkerExecution};
    let registry = tcl_registry::model::ingress::static_context_for("f5-irules").commands();
    let mut caller = ResolveContext::for_function("::event");
    retain_worker_namespace_geometry(&mut caller);
    caller.execution = Some(WorkerExecution {
        initialisation_epoch: 7,
        worker: Some(2),
        connection: Some(31),
    });
    caller.define_literal("::static::counter", "OLD", registry);
    let target = resolve_literal_place("static::counter", &caller, false, registry);
    let mut helper = caller.enter_called_frame(&VariableExecutionFrame::Procedure {
        namespace: "::".to_owned(),
        identity: "helper-call:17".to_owned(),
    });
    assert_eq!(helper.execution, caller.execution);
    helper
        .alias_bindings
        .insert("alias".to_owned(), target.clone());
    let alias = resolve_literal_place("alias", &helper, false, registry);
    assert_eq!(alias.cell, target.cell);
    let cell = alias
        .cell
        .as_ref()
        .expect("selected physical namespace cell");
    assert_eq!(
        cell.storage_domain,
        Some(VariableStorageDomain::WorkerNamespace)
    );
    assert_eq!(cell.execution.unwrap().connection, None);
    assert_eq!(cell.execution.unwrap().worker, Some(2));
    helper.define_literal("alias", "NEW", registry);
    let restored = restore_execution_frame(&caller, &helper);
    assert_eq!(
        restored.literal_value("::static::counter", registry),
        Some("NEW")
    );
    let ordinary = resolve_literal_place("::global", &helper, false, registry);
    assert_eq!(ordinary.cell.as_ref().unwrap().execution, cell.execution);
    let local = resolve_literal_place("local", &helper, false, registry);
    assert_eq!(local.cell.as_ref().unwrap().execution, caller.execution);
    let nested = resolve_literal_place("::app::static::counter", &helper, false, registry);
    assert_ne!(nested.cell, target.cell);
    assert_ne!(
        nested.cell.as_ref().unwrap().storage_domain,
        cell.storage_domain
    );
}

#[test]
// Implementation contract: naming.variable.hosted-storage-context
// docs/design/analysis/name-resolution-proofs/variable-hosted-storage-context.md
fn hosted_storage_requires_selected_context_and_retained_namespace_geometry() {
    use tcl_registry::f5::{BigIpExecutionContext, VariableStorageDomain};
    let irules = tcl_registry::model::ingress::static_context_for("f5-irules").commands();
    let stock = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let mut context = ResolveContext::for_function("::event");
    retain_worker_namespace_geometry(&mut context);
    let target = resolve_literal_place("::static::counter", &context, false, stock);
    assert_eq!(
        target.cell.as_ref().unwrap().storage_domain,
        Some(VariableStorageDomain::WorkerNamespace)
    );
    assert_eq!(
        target.cell,
        resolve_literal_place("::static::counter", &context, false, irules).cell
    );
    let root = resolve_literal_place("::counter", &context, false, stock);
    assert_eq!(
        root.cell.as_ref().unwrap().storage_domain,
        Some(VariableStorageDomain::CmpGlobal)
    );
    let nested = resolve_literal_place("::app::static::counter", &context, false, stock);
    assert_eq!(
        nested.cell.as_ref().unwrap().storage_domain,
        Some(VariableStorageDomain::InterpreterNamespace)
    );
    for host in BigIpExecutionContext::ALL {
        if host != BigIpExecutionContext::TmmIRule {
            let mut other = context.clone();
            other.hosted_execution_context = Some(host);
            let target = resolve_literal_place("::static::counter", &other, false, irules);
            assert_eq!(
                target.cell.as_ref().unwrap().storage_domain,
                Some(VariableStorageDomain::InterpreterNamespace)
            );
        }
    }
    context.hosted_execution_context = None;
    let target = resolve_literal_place("::static::counter", &context, false, irules);
    assert_eq!(
        target.cell.as_ref().unwrap().storage_domain,
        Some(VariableStorageDomain::InterpreterNamespace)
    );
    let mut unavailable = ResolveContext::for_function("::event");
    unavailable.hosted_execution_context = Some(BigIpExecutionContext::TmmIRule);
    unavailable.known_namespaces.insert("::static".into());
    assert_eq!(
        resolve_literal_place("::static::counter", &unavailable, false, irules)
            .cell
            .unwrap()
            .storage_domain,
        None
    );
}

#[test]
// Implementation contract: naming.variable.hosted-storage-context
// docs/design/analysis/name-resolution-proofs/variable-hosted-storage-context.md
fn different_hosted_contexts_with_equal_values_withdraw_cell_contents() {
    use tcl_registry::f5::BigIpExecutionContext;
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let mut first = ResolveContext::for_function("::event");
    retain_worker_namespace_geometry(&mut first);
    first.define_literal("::static::counter", "7", registry);
    assert_eq!(
        first.literal_value("::static::counter", registry),
        Some("7")
    );
    let entered = first.enter_called_frame(&VariableExecutionFrame::Procedure {
        namespace: "::".into(),
        identity: "genuine helper".into(),
    });
    assert_eq!(
        entered.hosted_execution_context,
        first.hosted_execution_context
    );
    assert_eq!(
        restore_execution_frame(&first, &entered).hosted_execution_context,
        first.hosted_execution_context
    );
    let mut other = first.clone();
    other.hosted_execution_context = Some(BigIpExecutionContext::IAppImplementation);
    first.join(&other);
    assert_eq!(first.hosted_execution_context, None);
    assert_eq!(first.literal_value("::static::counter", registry), None);
    assert!(first.constant_values.is_empty());
    assert!(first.dynamic_bindings);
    assert!(first.dynamic_traces);
}
