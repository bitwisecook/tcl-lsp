// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Namespace projections of one completed, immutable source inventory.
//!
//! Each owner retains the complete source interpretation, including original
//! input, store, configuration, origin and namespace receipts. Only the existing
//! structural namespace selection is memoized. Body ranges, expression stages,
//! current metadata and native admission remain independent queries. A selected
//! child owns a separate memo over its filtered inventory; it cannot recover
//! observations discarded by its parent selection.
//! Sharing retains those observations, not their present execution validity:
//! existing point, module and native-generation guards still decide each query.

use super::{ExecutedScriptSource, SourceCommandBindings, SourceNamespaceKey, SourceOriginId};
use std::collections::HashMap;
use std::ops::Deref;
use std::sync::{Arc, Mutex, OnceLock};

// Complete semantic identity selects an entry; hashes only select buckets.
type ProjectionKey = (Arc<SourceOriginId>, SourceNamespaceKey);
type ProjectionEntry = Arc<OnceLock<Option<RetainedSourceBodyInventory>>>;

#[derive(Default)]
struct BodyProjectionMemo {
    entries: Mutex<HashMap<ProjectionKey, ProjectionEntry>>,

    #[cfg(test)]
    requests: std::sync::atomic::AtomicUsize,
    #[cfg(test)]
    captures: std::sync::atomic::AtomicUsize,
}

/// Exact-offset original invocation projections of one completed inventory
/// (`naming.source.original-invocation-projection-memo`). Lowering stamps every
/// command's descendant points again for each enclosing token vector, so a
/// nested body would otherwise re-derive each inner point once per enclosing
/// level. The complete derived result is shared, never another interpretation.
#[derive(Default)]
struct InvocationProjectionMemo {
    entries: Mutex<HashMap<u32, Arc<super::SourceInvocationBinding>>>,
}

/// Bound on retained point projections per completed body inventory.
const MAX_RETAINED_INVOCATIONS: usize = 4096;

impl InvocationProjectionMemo {
    fn get_or_prepare(
        &self,
        offset: u32,
        prepare: impl FnOnce() -> super::SourceInvocationBinding,
    ) -> Arc<super::SourceInvocationBinding> {
        if let Some(retained) = self
            .entries
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(&offset)
        {
            return Arc::clone(retained);
        }
        let prepared = Arc::new(prepare());
        let mut entries = self
            .entries
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(retained) = entries.get(&offset) {
            return Arc::clone(retained);
        }
        if entries.len() < MAX_RETAINED_INVOCATIONS {
            entries.insert(offset, Arc::clone(&prepared));
        }
        prepared
    }
}

/// A completed inventory used read-only by lowering. There is no mutable
/// projection or borrowed builder escape: every changed source interpretation
/// must construct a new owner, even when its text or namespace label is equal.
#[derive(Clone)]
pub(crate) struct RetainedSourceBodyInventory {
    bindings: Arc<SourceCommandBindings>,
    projections: Arc<BodyProjectionMemo>,
    logical_realm: Arc<OnceLock<Option<crate::realm::CommandBindingRealm>>>,
    invocations: Arc<InvocationProjectionMemo>,
}

impl std::fmt::Debug for RetainedSourceBodyInventory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RetainedSourceBodyInventory")
            .field("bindings", &self.bindings)
            .finish_non_exhaustive()
    }
}

impl From<SourceCommandBindings> for RetainedSourceBodyInventory {
    fn from(bindings: SourceCommandBindings) -> Self {
        Self {
            bindings: Arc::new(bindings),
            projections: Arc::default(),
            logical_realm: Arc::default(),
            invocations: Arc::default(),
        }
    }
}

impl Deref for RetainedSourceBodyInventory {
    type Target = SourceCommandBindings;

    fn deref(&self) -> &Self::Target {
        &self.bindings
    }
}

impl RetainedSourceBodyInventory {
    /// Reuse the inventory's complete Logical owner for conditional word advice.
    /// Other purposes retain their original point-only stamping and refusals.
    pub(crate) fn stamp_original_tokens(&self, tokens: &mut crate::ir::CommandTokens) {
        match self.logical_realm.get_or_init(|| {
            crate::realm::original_logical_realm_from_source_bindings(&self.bindings)
        }) {
            Some(realm) => realm.stamp_original_tokens(tokens),
            None => self
                .bindings
                .stamp_original_tokens_with_bindings(tokens, |offset| {
                    self.invocations
                        .get_or_prepare(offset, || self.bindings.invocation_at_source("", offset))
                        .as_ref()
                        .clone()
                }),
        }
    }

    /// Share only the exact structural projection of this immutable owner.
    /// The existing selector is namespace-wide: the script's range and mapping
    /// deliberately stay with later per-body queries. No reached operation,
    /// source parsing, registry reconstruction or callback runs under the mutex.
    pub(crate) fn selected_source_in_context(
        &self,
        script: &ExecutedScriptSource,
        namespace: &SourceNamespaceKey,
    ) -> Option<Self> {
        #[cfg(test)]
        {
            self.projections
                .requests
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }
        let entry = {
            let mut entries = self
                .projections
                .entries
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            Arc::clone(
                entries
                    .entry((Arc::clone(&script.origin), namespace.clone()))
                    .or_default(),
            )
        };
        entry
            .get_or_init(|| {
                #[cfg(test)]
                {
                    self.projections
                        .captures
                        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                }
                self.bindings
                    .selected_source_in_context(script, namespace)
                    .map(Self::from)
            })
            .clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyser::ResolvedAnalysisInput;

    fn script(bindings: &SourceCommandBindings, source: &str) -> ExecutedScriptSource {
        ExecutedScriptSource::contiguous(Arc::clone(bindings.source_origin().unwrap()), source, 0)
            .unwrap()
    }

    #[test]
    fn original_body_projection_shares_exact_namespace_inventory_and_keeps_body_ranges() {
        let source = "namespace eval a {set x VALUE; set y $x; expr {1 + 2}; expr {3 + 4}}; namespace eval b {expr {5 + 6}}";
        let registry = tcl_registry::CommandRegistry::build_default();
        let bindings =
            SourceCommandBindings::analyse(source, tcl_lexer::LexerConfig::default(), &registry);
        let whole = script(&bindings, source);
        let preparations = bindings.expression_preparations_for_script(&whole);
        assert_eq!(preparations.len(), 3, "{preparations:?}");
        let namespace = preparations[0].namespace_key.clone();
        assert_eq!(preparations[1].namespace_key, namespace);
        assert_ne!(preparations[2].namespace_key, namespace);
        let expected = bindings
            .selected_source_in_context(&whole, &namespace)
            .unwrap();
        assert!(!expected.variable_accesses.is_empty());
        let owner = RetainedSourceBodyInventory::from(bindings);
        let first_text = "expr {1 + 2}";
        let second_text = "expr {3 + 4}";
        let first_body = ExecutedScriptSource::contiguous(
            Arc::clone(&whole.origin),
            first_text,
            u32::try_from(source.find(first_text).unwrap()).unwrap(),
        )
        .unwrap();
        let second_body = ExecutedScriptSource::contiguous(
            Arc::clone(&whole.origin),
            second_text,
            u32::try_from(source.find(second_text).unwrap()).unwrap(),
        )
        .unwrap();
        let first = owner
            .selected_source_in_context(&first_body, &namespace)
            .unwrap();
        let second = owner
            .clone()
            .selected_source_in_context(&second_body, &namespace)
            .unwrap();
        assert!(Arc::ptr_eq(&first.bindings, &second.bindings));
        assert_eq!(first.bindings.as_ref(), &expected);
        assert!(first.points.shares_storage(&second.points));
        assert!(
            first
                .variable_accesses
                .shares_storage(&second.variable_accesses)
        );
        assert_eq!(
            first.expression_preparations_for_script(&first_body),
            vec![preparations[0].clone()]
        );
        assert_eq!(
            second.expression_preparations_for_script(&second_body),
            vec![preparations[1].clone()]
        );
        // The full namespace inventory intentionally includes other body ranges.
        assert_eq!(
            first.expression_preparations_for_script(&whole),
            preparations[..2]
        );
        assert_eq!(
            owner
                .projections
                .captures
                .load(std::sync::atomic::Ordering::Relaxed),
            1
        );
        assert_eq!(
            owner
                .projections
                .requests
                .load(std::sync::atomic::Ordering::Relaxed),
            2
        );
    }

    #[test]
    fn original_body_projection_children_cannot_restore_filtered_or_foreign_origins() {
        let source = "namespace eval a {expr {1 + 2}}; namespace eval b {expr {3 + 4}}";
        let bindings = SourceCommandBindings::analyse(
            source,
            tcl_lexer::LexerConfig::default(),
            &tcl_registry::CommandRegistry::build_default(),
        );
        let whole = script(&bindings, source);
        let preparations = bindings.expression_preparations_for_script(&whole);
        assert_eq!(preparations.len(), 2);
        let owner = RetainedSourceBodyInventory::from(bindings);
        let a = owner
            .selected_source_in_context(&whole, &preparations[0].namespace_key)
            .unwrap();
        let b = owner
            .selected_source_in_context(&whole, &preparations[1].namespace_key)
            .unwrap();
        assert!(!Arc::ptr_eq(&a.bindings, &b.bindings));
        assert_eq!(
            b.expression_preparations_for_script(&whole),
            vec![preparations[1].clone()]
        );
        let child = a
            .selected_source_in_context(&whole, &preparations[1].namespace_key)
            .unwrap();
        assert!(!Arc::ptr_eq(&child.bindings, &b.bindings));
        assert!(child.points.is_empty());
        assert!(child.expression_preparations_for_script(&whole).is_empty());
        let foreign_text = "expr {9 + 10}";
        let foreign_origin = Arc::new(SourceOriginId::authored(&Arc::from(foreign_text)));
        let foreign = ExecutedScriptSource::contiguous(foreign_origin, foreign_text, 0).unwrap();
        let absent = owner
            .selected_source_in_context(&foreign, &preparations[0].namespace_key)
            .unwrap();
        assert!(absent.points.is_empty());
        assert!(
            absent
                .expression_preparations_for_script(&foreign)
                .is_empty()
        );
        assert_eq!(
            a.expression_preparations_for_script(&whole),
            vec![preparations[0].clone()]
        );
    }

    #[test]
    fn original_body_projection_keeps_full_logical_input_and_withdrawn_owner_separate() {
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        let context = tcl_registry::model::ingress::resolve_known_environment("tcl8.6")
            .unwrap()
            .default_context_registry();
        let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
        let input = ResolvedAnalysisInput::new(profile, profile, Arc::clone(&context), config);
        let foreign_commands = Arc::new(context.commands().project_for_profile(profile));
        let foreign_context = Arc::new(context.with_command_store(foreign_commands));
        let foreign = ResolvedAnalysisInput::new(profile, profile, foreign_context, config);
        let nested =
            ResolvedAnalysisInput::new(profile, profile, Arc::clone(&context), config.nested());
        assert_ne!(input, foreign);
        assert_ne!(input, nested);
        let source = "set x VALUE";
        let build = |input: &ResolvedAnalysisInput| {
            SourceCommandBindings::analyse_with_options(
                source,
                input.lexer_config(),
                input.borrowed_context_registry().commands(),
                super::super::SourceAnalysisOptions::for_logical_source(input).unwrap(),
            )
        };
        let mut owners = Vec::new();
        for actual in [&input, &foreign, &nested] {
            let bindings = build(actual);
            let whole = script(&bindings, source);
            let namespace = bindings.points.first().unwrap().namespace_key.clone();
            let frozen = RetainedSourceBodyInventory::from(bindings);
            let selected = frozen
                .selected_source_in_context(&whole, &namespace)
                .unwrap();
            let current = selected.invocation_at_source("presentation", 0);
            assert!(current.proved_target().is_some());
            assert_eq!(current.logical_source_name_advice_input(), Some(actual));
            assert!(current.proved_execution_target().is_none());
            assert!(current.proved_handler_target().is_none());
            assert!(current.native_compiler_admission.is_none());
            assert!(Arc::ptr_eq(
                &selected.bindings,
                &frozen
                    .selected_source_in_context(&whole, &namespace)
                    .unwrap()
                    .bindings
            ));
            owners.push(selected);
        }
        assert!(!Arc::ptr_eq(&owners[0].bindings, &owners[1].bindings));
        assert!(!Arc::ptr_eq(&owners[0].bindings, &owners[2].bindings));
        let mut options = super::super::SourceAnalysisOptions::for_logical_source(&input).unwrap();
        options.logical_source_input = None;
        options.metadata_context =
            crate::registry_invocation::InvocationMetadataInput::SuppliedSource(None);
        let bindings = SourceCommandBindings::analyse_with_options(
            source,
            config,
            context.commands(),
            options,
        );
        let whole = script(&bindings, source);
        let namespace = owners[0].points.first().unwrap().namespace_key.clone();
        let withdrawn = RetainedSourceBodyInventory::from(bindings)
            .selected_source_in_context(&whole, &namespace)
            .unwrap();
        assert!(!Arc::ptr_eq(&owners[0].bindings, &withdrawn.bindings));
        let current = withdrawn.invocation_at_source("presentation", 0);
        assert!(current.logical_source_name_advice_input().is_none());
        assert!(current.proved_execution_target().is_none());
        assert!(current.native_compiler_admission.is_none());
        let mut changed = build(&input);
        let retained = RetainedSourceBodyInventory::from(changed.clone());
        changed.points.clear();
        assert!(!retained.points.is_empty());
    }

    #[test]
    fn original_body_projection_uses_actual_native_namespace_incarnation() {
        let context = tcl_registry::model::ingress::static_context_for("tcl8.6");
        let profile = context.commands().profile().unwrap();
        let (_first_runtime, first_entry) =
            crate::environment_ingress::captured_native_entry_with_owner(profile);
        let (_second_runtime, second_entry) =
            crate::environment_ingress::captured_native_entry_with_owner(profile);
        let first_namespace = SourceNamespaceKey::from_native_entry(&first_entry).unwrap();
        let second_namespace = SourceNamespaceKey::from_native_entry(&second_entry).unwrap();
        assert_eq!(first_namespace.display(), second_namespace.display());
        assert_ne!(first_namespace, second_namespace);
        let source = "set x VALUE; expr {1 + 2}";
        let bindings = SourceCommandBindings::analyse_with_options(
            source,
            tcl_lexer::LexerConfig::from_grammar(profile.grammar),
            context.commands(),
            super::super::SourceAnalysisOptions {
                native_entry: Some(&first_entry),
                invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
                ..Default::default()
            },
        );
        let whole = script(&bindings, source);
        assert!(!bindings.points.is_empty());
        let expected = bindings
            .selected_source_in_context(&whole, &first_namespace)
            .unwrap();
        assert!(!expected.points.is_empty());
        let frozen = RetainedSourceBodyInventory::from(bindings);
        let current = frozen
            .selected_source_in_context(&whole, &first_namespace)
            .unwrap();
        assert_eq!(current.bindings.as_ref(), &expected);
        let foreign = frozen
            .selected_source_in_context(&whole, &second_namespace)
            .unwrap();
        assert!(!Arc::ptr_eq(&current.bindings, &foreign.bindings));
        assert!(foreign.points.is_empty());
        assert!(
            foreign
                .expression_preparations_for_script(&whole)
                .is_empty()
        );
        assert!(Arc::ptr_eq(
            &current.bindings,
            &frozen
                .selected_source_in_context(&whole, &first_namespace)
                .unwrap()
                .bindings
        ));
    }
}
