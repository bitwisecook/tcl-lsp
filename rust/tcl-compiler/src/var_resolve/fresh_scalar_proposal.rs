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

//! Proposed scalar slots under independently closed actual activation ownership.

use super::{
    BindingIdentity, ContentsPresence, ContentsWorld, ResolveContext, VariableCellKey,
    VariableFrameKind,
};
use crate::place::{CellGeneration, CellOwner, PlaceKind};
use tcl_registry::{CommandRegistry, TraceOperation};
use tcl_syntax::naming::{
    NamePolicyProtocol, NativeNameContext, NativeVariableInputForm, NativeVariableRootGeometry,
};

#[derive(Debug, Clone)]
pub(crate) struct FreshScalarVariableSlot {
    name: tcl_core_types::NameBytes,
    cell: VariableCellKey,
    policy: NamePolicyProtocol,
}

impl FreshScalarVariableSlot {
    pub(crate) fn name(&self) -> &[u8] {
        self.name.as_bytes()
    }
    pub(crate) fn cell(&self) -> &VariableCellKey {
        &self.cell
    }
    pub(crate) fn policy(&self) -> NamePolicyProtocol {
        self.policy
    }
}

impl ResolveContext {
    /// New local absence requires the actual selected fresh activation and
    /// complete current local/observer inventory. Namespace and event storage
    /// require separate inventories and cannot borrow this receipt.
    pub(crate) fn fresh_scalar_variable_slot(
        &self,
        name: &[u8],
        registry: &CommandRegistry,
    ) -> Option<FreshScalarVariableSlot> {
        if self.binding_identity != BindingIdentity::Bound
            || self.frame_kind != VariableFrameKind::Local
            || self.selected_frame.is_some()
            || self.activation_contents_world != Some(ContentsWorld::Tracked)
            || !self.activation_observers_closed()
            || self.dynamic_bindings
            || self.execution.is_some()
            || self
                .hosted_execution_context
                .is_some_and(|context| context == tcl_registry::f5::BigIpExecutionContext::TmmIRule)
            || name.is_empty()
            || name.contains(&0)
        {
            return None;
        }
        let policy = self.execution_name_policy?.native_recipe()?;
        let protocol = policy.recipe();
        let selected = protocol.combined_variable_input(name);
        if selected.element().is_some()
            || selected.root().selected() != name
            || !matches!(protocol.variable_root_geometry(NativeNameContext::root(), name), NativeVariableRootGeometry::Local(ref simple) if simple.as_bytes() == name)
        {
            return None;
        }
        let place = super::resolve_evaluated_variable_input(
            NativeVariableInputForm::Combined(name),
            self,
            false,
            registry,
            TraceOperation::Write,
        );
        let cell = place.cell.as_ref()?;
        if place.kind != PlaceKind::Scalar
            || place.dynamic
            || place.observed
            || place.index.is_some()
            || !place.keys.is_empty()
            || cell.name.as_bytes() != name
            || cell.generation != CellGeneration::Incoming
            || !matches!(&cell.owner, CellOwner::Activation(identity) if self.activation.as_ref() == Some(identity))
            || self.contents_presence(&place) != ContentsPresence::Undefined
            || self.contents_kinds.contains_key(&super::cell_key(&place))
            || self.generations.contains_key(&super::cell_key(&place))
        {
            return None;
        }
        let key = super::canonical_binding_value_key(&place)?;
        if !matches!(&key, VariableCellKey::Activation { identity, simple } if self.activation.as_ref() == Some(identity) && simple.as_bytes() == name)
            || self.alias_bindings.contains_key(&key)
            || self.name_alias_bindings.contains_key(&key)
            || self.unknown_bindings.contains(&key)
            || self.constant_values.contains_key(&key)
            || self.original_name_values.contains_key(&key)
        {
            return None;
        }
        for operation in [
            TraceOperation::Read,
            TraceOperation::Write,
            TraceOperation::Unset,
        ] {
            let observers = self.variable_observers_at(&place, operation, registry);
            if observers.unknown_residual
                || !observers.callbacks.is_empty()
                || !observers.possible_callbacks.is_empty()
            {
                return None;
            }
        }
        Some(FreshScalarVariableSlot {
            name: name.into(),
            cell: key,
            policy,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entered(profile: &str) -> ResolveContext {
        let dialect = tcl_registry::InvocationDialect::of_point(
            tcl_dialect::model::DialectPoint::of_dialect_name(Some(profile)).unwrap(),
        );
        let mut caller = ResolveContext::for_namespace("::");
        caller.invocation_dialect = Some(dialect);
        caller.execution_name_policy = Some(tcl_syntax::naming::ExecutionNamePolicy::NativeRecipe(
            dialect.authored_name_policy().unwrap(),
        ));
        let root = crate::command_binding::SourceNamespaceKey::authored("::");
        caller.retain_namespace_world(
            root.clone(),
            [root],
            Some(dialect.authored_name_policy().unwrap().recipe()),
        );
        caller.enter_called_frame(&super::super::VariableExecutionFrame::Procedure {
            namespace: "::".into(),
            identity: "actual entered procedure".into(),
        })
    }

    #[test]
    // Implementation contract: naming.variable.fresh-scalar-proposal
    // docs/design/analysis/name-resolution-proofs/fresh-scalar-proposal.md
    fn original_fresh_scalar_proposal_requires_unoccupied_actual_frame_and_quiet_observers() {
        for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jimtcl"] {
            let registry = tcl_registry::model::ingress::static_context_for(profile).commands();
            let context = entered(profile);
            let proposal = context
                .fresh_scalar_variable_slot(b"fresh", registry)
                .expect("independently closed entered activation");
            assert_eq!(proposal.name(), b"fresh");
            assert!(matches!(
                proposal.cell(),
                VariableCellKey::Activation { .. }
            ));
            for name in [b"".as_slice(), b"::fresh", b"a(k)", b"fresh\0tail"] {
                assert!(context.fresh_scalar_variable_slot(name, registry).is_none());
            }
            let mut occupied = context.clone();
            occupied.define_unknown_contents("fresh", registry);
            assert!(
                occupied
                    .fresh_scalar_variable_slot(b"fresh", registry)
                    .is_none()
            );
            let mut unknown = context.clone();
            unknown.mark_unenumerated_variable_observers();
            assert!(
                unknown
                    .fresh_scalar_variable_slot(b"fresh", registry)
                    .is_none()
            );
            let mut alias = context.clone();
            alias.unknown_bindings.insert(proposal.cell().clone());
            assert!(
                alias
                    .fresh_scalar_variable_slot(b"fresh", registry)
                    .is_none()
            );
            let namespace = ResolveContext::for_namespace("::");
            assert!(
                namespace
                    .fresh_scalar_variable_slot(b"fresh", registry)
                    .is_none()
            );
            let mut incoming = context.clone();
            incoming.activation_contents_world = None;
            assert!(
                incoming
                    .fresh_scalar_variable_slot(b"fresh", registry)
                    .is_none()
            );
        }
    }
}
