// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Before-store normal source writes. Name bytes, a normal read and the value
//! observed after a store cannot manufacture this operation's completion.

use super::{ContentsOrigin, ContentsPresence, ResolveContext, RootContentsKind};
use crate::place::{CellOwner, IndexKind, Place, PlaceKind};
use tcl_registry::{CommandRegistry, InvocationDialect, TraceOperation};

/// Independently closed before-store address, container and callback obligations.
/// The handler's argument completion and original worker remain separate.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct OriginalNormalValueWrite {
    receiver: Place,
    origin: ContentsOrigin,
    root_origin: ContentsOrigin,
    dialect: InvocationDialect,
    contents_epoch: Option<u64>,
    representation_epoch: Option<u64>,
}

impl OriginalNormalValueWrite {
    pub(crate) fn is_current(&self, context: &ResolveContext, registry: &CommandRegistry) -> bool {
        let root = self.receiver.base();
        context.invocation_dialect == Some(self.dialect)
            && context.contents_origin(&self.receiver) == self.origin
            && context.contents_origin(&root) == self.root_origin
            && context.original_contents_epoch() == self.contents_epoch
            && context.representation_epoch == self.representation_epoch
            && context.original_write_is_normal(&self.receiver, registry)
    }
}

impl ResolveContext {
    pub(crate) fn original_normal_value_write(
        &self,
        receiver: &Place,
        registry: &CommandRegistry,
    ) -> Option<OriginalNormalValueWrite> {
        self.original_write_is_normal(receiver, registry)
            .then(|| OriginalNormalValueWrite {
                receiver: receiver.clone(),
                origin: self.contents_origin(receiver),
                root_origin: self.contents_origin(&receiver.base()),
                dialect: self
                    .invocation_dialect
                    .expect("write kernel selected dialect"),
                contents_epoch: self.original_contents_epoch(),
                representation_epoch: self.representation_epoch,
            })
    }

    fn original_write_is_normal(&self, receiver: &Place, registry: &CommandRegistry) -> bool {
        use tcl_dialect::VariableContainerModel;
        if self.binding_identity != super::BindingIdentity::Bound
            || self
                .invocation_dialect
                .is_none_or(|dialect| dialect.variable_lookup_policy.is_none())
            || receiver.dynamic
            || receiver.observed
            || !receiver.keys.is_empty()
            || !matches!(receiver.kind, PlaceKind::Scalar | PlaceKind::ArrayElem)
            || !self.current_contents_generation(receiver)
            || self.store_would_error(receiver)
        {
            return false;
        }
        if !self.owns_original_write_receiver(receiver) {
            return false;
        }
        let observers = self.variable_observers_at(receiver, TraceOperation::Write, registry);
        if observers.unknown_residual
            || !observers.callbacks.is_empty()
            || !observers.possible_callbacks.is_empty()
        {
            return false;
        }
        let mut root = receiver.base();
        root.kind = PlaceKind::Scalar;
        let root_presence = self.contents_presence(&root);
        let Some(model) = self
            .invocation_dialect
            .and_then(|dialect| dialect.variable_container_model)
        else {
            return false;
        };
        let replaced = if receiver.kind == PlaceKind::ArrayElem {
            if receiver
                .index
                .as_ref()
                .is_none_or(|index| index.kind != IndexKind::Literal)
            {
                return false;
            }
            match model {
                VariableContainerModel::DistinctArray => {
                    if root_presence != ContentsPresence::Undefined
                        && !(root_presence == ContentsPresence::Defined
                            && self.root_contents_kind(&root) == Some(RootContentsKind::Array))
                    {
                        return false;
                    }
                    receiver
                }
                VariableContainerModel::DictionaryValue => {
                    if root_presence == ContentsPresence::Defined {
                        let valid = self
                            .original_name_read_result(&root, registry)
                            .map(|read| {
                                self.invocation_dialect
                                    .unwrap()
                                    .dictionary_variable_root_valid(read.value().bytes())
                            })
                            .or_else(|| {
                                self.literal_contents_at(&root, registry).map(|bytes| {
                                    self.invocation_dialect
                                        .unwrap()
                                        .dictionary_variable_root_valid(bytes.as_bytes())
                                })
                            });
                        if valid != Some(Some(true)) {
                            return false;
                        }
                    } else if root_presence != ContentsPresence::Undefined {
                        return false;
                    }
                    &root
                }
            }
        } else {
            if receiver.index.is_some()
                || root_presence == ContentsPresence::Unknown
                || root_presence == ContentsPresence::DefinedOrUndefined
            {
                return false;
            }
            if model == VariableContainerModel::DistinctArray
                && root_presence == ContentsPresence::Defined
                && self.root_contents_kind(&root) != Some(RootContentsKind::Scalar)
            {
                return false;
            }
            receiver
        };
        match self.contents_presence(replaced) {
            ContentsPresence::Undefined => true,
            ContentsPresence::Defined => {
                self.original_contents_release_is_closed(replaced, registry)
            }
            ContentsPresence::Unknown | ContentsPresence::DefinedOrUndefined => false,
        }
    }

    fn owns_original_write_receiver(&self, receiver: &Place) -> bool {
        let Some(cell) = &receiver.cell else {
            return false;
        };
        match &cell.owner {
            CellOwner::Activation(identity) => self.activation.as_ref() == Some(identity),
            CellOwner::NamespaceIdentity(identity) => {
                self.namespace_identities.contains(identity.as_ref())
            }
            CellOwner::Namespace(namespace) => self.known_namespaces.contains(namespace),
            // These owners need their independently captured operation/liveness
            // receipt, rather than borrowing a current activation's closure.
            _ => false,
        }
    }

    fn original_contents_release_is_closed(
        &self,
        receiver: &Place,
        registry: &CommandRegistry,
    ) -> bool {
        use crate::native_numeric::StoredNativeRepresentation;
        if !self.current_contents_generation(receiver) {
            return false;
        }
        let Some(key) = super::canonical_binding_value_key(receiver) else {
            return false;
        };
        // A representation category, readonly native byte graph or known text
        // can coexist with a custom intrep. Only sealed ordinary class producers
        // close release; list/dict child release is a separate obligation.
        match self.value_representations.get(&key) {
            Some(StoredNativeRepresentation::StockLiteralObject(_)) => true,
            Some(
                StoredNativeRepresentation::Numeric(_)
                | StoredNativeRepresentation::NumericShape(_),
            ) => self.contents_already_native_numeric_at(receiver, registry),
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_syntax::naming::{ExecutionNamePolicy, NativeVariableInputForm};

    fn selected_context(name: &str) -> ResolveContext {
        let dialect = InvocationDialect::of_point(
            tcl_dialect::model::DialectPoint::of_dialect_name(Some(name)).unwrap(),
        );
        let mut context = ResolveContext::for_function("::p");
        context.invocation_dialect = Some(dialect);
        context.execution_name_policy = Some(ExecutionNamePolicy::NativeRecipe(
            dialect.authored_name_policy().unwrap(),
        ));
        context
    }

    #[test]
    // Implementation contract: naming.variable.byte-cell-correspondence
    // docs/design/analysis/name-resolution-proofs/variable.byte-cell-correspondence.md
    fn before_store_completion_requires_kind_observers_and_old_class_independently() {
        for name in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            let registry = tcl_registry::model::ingress::static_context_for(name).commands();
            let mut context = selected_context(name);
            let receiver = super::super::resolve_evaluated_variable_input(
                NativeVariableInputForm::Combined(b"v"),
                &context,
                false,
                registry,
                TraceOperation::Write,
            );
            let receipt = context
                .original_normal_value_write(&receiver, registry)
                .unwrap();
            assert!(
                receipt.is_current(&context, registry),
                "{name}: absent fresh local"
            );
            context.define_literal("v", "KNOWN", registry);
            assert!(
                !receipt.is_current(&context, registry),
                "{name}: before-store receipt retired by write"
            );
            assert!(
                context
                    .original_normal_value_write(&receiver, registry)
                    .is_none(),
                "{name}: known text cannot close release of an unknown old object class"
            );
            context.trace_registrations.insert(
                super::super::cell_key(&receiver),
                vec![(vec![TraceOperation::Write], "watch".into())],
            );
            assert!(
                context
                    .original_normal_value_write(&receiver, registry)
                    .is_none()
            );
            let mut fresh = selected_context(name);
            fresh.dynamic_traces = true;
            assert!(
                fresh
                    .original_normal_value_write(&receiver, registry)
                    .is_none()
            );
            let mut wrong_kind = selected_context(name);
            wrong_kind.define_literal("a(k)", "VALUE", registry);
            let array_root = super::super::resolve_evaluated_variable_input(
                NativeVariableInputForm::Combined(b"a"),
                &wrong_kind,
                false,
                registry,
                TraceOperation::Write,
            );
            if wrong_kind
                .invocation_dialect
                .unwrap()
                .variable_container_model
                == Some(tcl_dialect::VariableContainerModel::DistinctArray)
            {
                assert!(
                    wrong_kind
                        .original_normal_value_write(&array_root, registry)
                        .is_none(),
                    "{name}: scalar overwrite of array"
                );
            }
        }
    }
}
