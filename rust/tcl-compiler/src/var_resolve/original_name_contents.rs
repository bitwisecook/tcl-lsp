// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Original produced bytes at exact live variable cells. Numeric cache state,
//! immutable historical values and current read correspondence are separate.

use super::{
    ContentsOrigin, ContentsPresence, ResolveContext, VariableProofRelocation,
    canonical_binding_value_key,
};
use crate::{
    command_binding::original_name_value::OriginalProducedNameValue,
    place::{Place, PlaceKind},
};
use tcl_registry::{CommandRegistry, TraceOperation};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct StoredOriginalNameValue {
    receiver: Place,
    origin: ContentsOrigin,
    value: OriginalProducedNameValue,
}

impl StoredOriginalNameValue {
    pub(super) fn relocated(&self, relocation: &VariableProofRelocation) -> Self {
        Self {
            receiver: relocation.place(&self.receiver),
            origin: relocation.contents_origin(&self.origin),
            value: self.value.clone(),
        }
    }

    pub(super) fn joined(&self, other: &Self) -> Option<Self> {
        if !same_receiver(&self.receiver, &other.receiver) {
            return None;
        }
        Some(Self {
            receiver: self.receiver.clone(),
            origin: self.origin.joined(&other.origin),
            value: self.value.joined(&other.value)?,
        })
    }
}

fn same_receiver(left: &Place, right: &Place) -> bool {
    left.cell == right.cell
        && left.index == right.index
        && left.keys == right.keys
        && left.kind == right.kind
}

/// A value-free normal source read. This does not supply contents, representation,
/// an original word, a native header or an incoming argument object root.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct OriginalNormalValueRead {
    receiver: Place,
    origin: ContentsOrigin,
    dialect: tcl_registry::InvocationDialect,
}

impl OriginalNormalValueRead {
    pub(crate) fn is_current(&self, context: &ResolveContext, registry: &CommandRegistry) -> bool {
        context.invocation_dialect == Some(self.dialect)
            && context.contents_origin(&self.receiver) == self.origin
            && context.read_produces_value(&self.receiver, registry)
    }
}

/// Current read-only byte correspondence at a quiet normal read. The retained
/// value remains independently historical; the receipt also rechecks the cell.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct OriginalNameValueRead {
    normal: OriginalNormalValueRead,
    stored: StoredOriginalNameValue,
    dictionary_root: Option<Box<OriginalNameValueRead>>,
}

impl OriginalNameValueRead {
    pub(crate) fn value(&self) -> &OriginalProducedNameValue {
        &self.stored.value
    }

    pub(crate) fn is_current(&self, context: &ResolveContext, registry: &CommandRegistry) -> bool {
        self.normal.is_current(context, registry)
            && self.stored.value.is_current(context)
            && match &self.dictionary_root {
                Some(root) => root.is_current(context, registry),
                None => {
                    canonical_binding_value_key(&self.normal.receiver)
                        .and_then(|key| context.original_name_values.get(&key))
                        == Some(&self.stored)
                }
            }
    }
}

impl ResolveContext {
    pub(crate) fn original_normal_value_read(
        &self,
        receiver: &Place,
        registry: &CommandRegistry,
    ) -> Option<OriginalNormalValueRead> {
        self.read_produces_value(receiver, registry)
            .then(|| OriginalNormalValueRead {
                receiver: receiver.clone(),
                origin: self.contents_origin(receiver),
                dialect: self
                    .invocation_dialect
                    .expect("read kernel selects its dialect"),
            })
    }

    /// Install a separately authenticated original value after a proved exact
    /// store. This checks the write receiver and current defined contents; it
    /// does not infer that execution succeeded from known bytes alone.
    pub(crate) fn retain_original_name_value(
        &mut self,
        receiver: &Place,
        value: &OriginalProducedNameValue,
        registry: &CommandRegistry,
    ) -> bool {
        let Some(key) = canonical_binding_value_key(receiver) else {
            return false;
        };
        self.original_name_values.remove(&key);
        if receiver.observed
            || receiver.dynamic
            || !matches!(receiver.kind, PlaceKind::Scalar | PlaceKind::ArrayElem)
            || self.contents_presence(receiver) != ContentsPresence::Defined
            || !self.current_contents_generation(receiver)
            || self.store_would_error(receiver)
            || !value.is_current(self)
        {
            return false;
        }
        let observers = self.variable_observers_at(receiver, TraceOperation::Write, registry);
        if observers.unknown_residual
            || !observers.callbacks.is_empty()
            || !observers.possible_callbacks.is_empty()
        {
            return false;
        }
        self.original_name_values.insert(
            key,
            StoredOriginalNameValue {
                receiver: receiver.clone(),
                origin: self.contents_origin(receiver),
                value: value.clone(),
            },
        );
        true
    }

    pub(crate) fn original_name_read_result(
        &self,
        receiver: &Place,
        registry: &CommandRegistry,
    ) -> Option<OriginalNameValueRead> {
        let normal = self.original_normal_value_read(receiver, registry)?;
        if receiver.kind == PlaceKind::ArrayElem
            && self.invocation_dialect?.variable_container_model
                == Some(tcl_dialect::VariableContainerModel::DictionaryValue)
        {
            let mut root = receiver.base();
            root.kind = PlaceKind::Scalar;
            if let Some(root_read) = self.original_name_read_result(&root, registry) {
                let index = receiver.index.as_ref()?;
                let ordinal = self.invocation_dialect?.dictionary_variable_value_ordinal(
                    root_read.value().bytes(),
                    index.value.as_bytes(),
                )?;
                let input = crate::signature_scan::name_value::SignatureSourceNameValue::from_dictionary_variable_element(
                    &root_read, receiver, ordinal,
                )?;
                let value = OriginalProducedNameValue::from_source_input(
                    &crate::signature_scan::scope::SignatureSourceNameInput::OriginalValue(input),
                    self,
                )?;
                return Some(OriginalNameValueRead {
                    stored: StoredOriginalNameValue {
                        receiver: receiver.clone(),
                        origin: normal.origin.clone(),
                        value,
                    },
                    normal,
                    dictionary_root: Some(Box::new(root_read)),
                });
            }
        }
        let stored = self
            .original_name_values
            .get(&canonical_binding_value_key(receiver)?)?;
        if !same_receiver(receiver, &stored.receiver)
            || stored.origin != normal.origin
            || !stored.value.is_current(self)
        {
            return None;
        }
        Some(OriginalNameValueRead {
            normal,
            stored: stored.clone(),
            dictionary_root: None,
        })
    }

    pub(super) fn join_original_name_values(&mut self, other: &Self) {
        self.original_name_values = self
            .original_name_values
            .iter()
            .filter_map(|(key, left)| {
                Some((
                    key.clone(),
                    left.joined(other.original_name_values.get(key)?)?,
                ))
            })
            .collect();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::signature_scan::scope::{SignatureSourceNameInput, SignatureSourceNameKey};
    use tcl_lexer::{LexerConfig, SourceImage, Span};
    use tcl_syntax::{
        naming::{ExecutionNamePolicy, NativeVariableInputForm},
        word_rules::WordValueRules,
    };

    fn context(name: &str) -> ResolveContext {
        let dialect = tcl_registry::InvocationDialect::of_point(
            tcl_dialect::model::DialectPoint::of_dialect_name(Some(name)).unwrap(),
        );
        let policy = dialect.authored_name_policy().unwrap();
        let mut context = ResolveContext::for_function("::p");
        context.invocation_dialect = Some(dialect);
        context.execution_name_policy = Some(ExecutionNamePolicy::NativeRecipe(policy));
        context
    }

    fn value(source: &[u8], context: &ResolveContext) -> OriginalProducedNameValue {
        let policy = context
            .execution_name_policy
            .unwrap()
            .native_recipe()
            .unwrap();
        let config = LexerConfig::from_grammar(context.invocation_dialect.unwrap().lexer_grammar);
        let parsed = tcl_lexer::native_script_words_in(
            SourceImage::native(source),
            Span::new(0, u32::try_from(source.len()).unwrap()),
            config,
        )
        .unwrap();
        let key = SignatureSourceNameKey::from_original_native_word(
            &parsed.commands[0].words[0],
            WordValueRules::from_config(&config),
            policy,
        )
        .unwrap();
        OriginalProducedNameValue::from_source_input(
            &SignatureSourceNameInput::OriginalWord(key),
            context,
        )
        .unwrap()
    }

    fn root(context: &ResolveContext, registry: &CommandRegistry) -> Place {
        super::super::resolve_evaluated_variable_input(
            NativeVariableInputForm::Combined(b"a"),
            context,
            false,
            registry,
            TraceOperation::Read,
        )
    }

    fn element(context: &ResolveContext, registry: &CommandRegistry, index: &[u8]) -> Place {
        super::super::resolve_evaluated_variable_input(
            NativeVariableInputForm::Separate {
                root: b"a",
                element: Some(index),
            },
            context,
            false,
            registry,
            TraceOperation::Read,
        )
    }

    #[test]
    fn original_value_read_separates_cache_current_cell_and_historical_bytes() {
        for name in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            let registry = tcl_registry::model::ingress::static_context_for(name).commands();
            let mut context = context(name);
            context.define_literal("a", "SAME", registry);
            let receiver = root(&context, registry);
            let first = value(b"{SAME}", &context);
            assert!(context.retain_original_name_value(&receiver, &first, registry));
            let read = context
                .original_name_read_result(&receiver, registry)
                .unwrap();
            context.invalidate_shared_representations();
            assert!(read.is_current(&context, registry));
            assert!(first.is_current(&context));
            context.define_literal("a", "SAME", registry);
            assert!(!read.is_current(&context, registry));
            assert!(first.is_current(&context));
            let second = value(b"\"SAME\"", &context);
            assert_eq!(first.bytes(), second.bytes());
            assert_ne!(first.original_origins(), second.original_origins());
            assert!(context.retain_original_name_value(&receiver, &second, registry));
            assert!(!read.is_current(&context, registry));
            let current = context
                .original_name_read_result(&receiver, registry)
                .unwrap();
            context.invalidate_original_contents();
            assert!(!current.is_current(&context, registry));
            assert!(!first.is_current(&context));
        }
    }

    #[test]
    fn dictionary_root_child_keeps_last_counted_key_and_live_read_lifetime() {
        let registry = tcl_registry::model::ingress::static_context_for("jim").commands();
        let mut context = context("jim");
        context.define_literal("a", "advisory value", registry);
        let receiver = root(&context, registry);
        let parent = value(
            b"{k OLD k LAST k\0tail COUNTED \xed\xa0\x80 OPAQUE}",
            &context,
        );
        assert!(context.retain_original_name_value(&receiver, &parent, registry));
        let indexed = element(&context, registry, b"k");
        assert!(context.read_produces_value(&indexed, registry));
        let read = context
            .original_name_read_result(&indexed, registry)
            .unwrap();
        assert_eq!(read.value().bytes(), b"LAST");
        assert_eq!(
            context
                .original_name_read_result(&element(&context, registry, b"k\0tail"), registry)
                .unwrap()
                .value()
                .bytes(),
            b"COUNTED"
        );
        assert_eq!(
            context
                .original_name_read_result(&element(&context, registry, b"\xed\xa0\x80"), registry)
                .unwrap()
                .value()
                .bytes(),
            b"OPAQUE"
        );
        assert!(
            context
                .original_name_read_result(&element(&context, registry, b"missing"), registry)
                .is_none()
        );
        assert!(
            context
                .original_name_read_result(&element(&context, registry, b"k\0fail"), registry)
                .is_none()
        );
        let historical = read.value().clone();
        context.define_literal("a", "replacement advice", registry);
        let replacement = value(b"{k NEW}", &context);
        assert!(context.retain_original_name_value(&receiver, &replacement, registry));
        assert!(!read.is_current(&context, registry));
        assert!(historical.is_current(&context));
        assert_eq!(
            context
                .original_name_read_result(&indexed, registry)
                .unwrap()
                .value()
                .bytes(),
            b"NEW"
        );
        for malformed in [b"{odd}".as_slice(), b"{other VALUE}"] {
            let parent = value(malformed, &context);
            assert!(context.retain_original_name_value(&receiver, &parent, registry));
            assert!(!context.read_produces_value(&indexed, registry));
            assert!(
                context
                    .original_name_read_result(&indexed, registry)
                    .is_none()
            );
        }
    }

    #[test]
    fn dictionary_contents_cannot_turn_a_c_scalar_into_an_array_read() {
        for name in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let registry = tcl_registry::model::ingress::static_context_for(name).commands();
            let mut context = context(name);
            context.define_literal("a", "k VALUE", registry);
            let receiver = root(&context, registry);
            let parent = value(b"{k VALUE}", &context);
            assert!(context.retain_original_name_value(&receiver, &parent, registry));
            let indexed = element(&context, registry, b"k");
            assert!(!context.read_produces_value(&indexed, registry));
            assert!(
                context
                    .original_name_read_result(&indexed, registry)
                    .is_none()
            );
        }
    }
}
