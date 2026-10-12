// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Namespace holders of exact retained procedure implementations.
//!
//! Each entry retains the original immutable command-table Arc. Its address is
//! only a lookup bucket: Arc::ptr_eq authenticates reuse. Existing table changes
//! must continue through COW mutation or replacement, so a cached table cannot
//! change in place and each changed table gets its own index. Equal copied table
//! contents do not authenticate another retained table's derived index.
//!
//! A collector clone starts with an empty cache; warmth affects neither semantic
//! equality nor Debug output. Entries retain no collector and introduce no Arc
//! cycle. Index construction runs outside the cache mutex through OnceLock.
//! Only namespace holders are indexed. Full metadata, frame, actual incoming and
//! completion-outcome states remain with their existing owners and consumers.

use super::{
    BindingKind, DeferredImplementationId, MayBinding, SourceCommandTable, SourceNamespaceKey,
};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::{Arc, Mutex, OnceLock};

type BindingTable = SourceCommandTable<BTreeSet<MayBinding>>;

pub(super) struct DeferredNamespaceIndex {
    namespaces: BTreeMap<DeferredImplementationId, BTreeSet<SourceNamespaceKey>>,
}

impl DeferredNamespaceIndex {
    fn from_bindings(bindings: &BindingTable, mut count_row: impl FnMut(usize)) -> Self {
        let mut namespaces: BTreeMap<DeferredImplementationId, BTreeSet<SourceNamespaceKey>> =
            BTreeMap::new();
        for (slot, alternatives) in bindings.iter() {
            count_row(alternatives.len());
            for binding in alternatives {
                let MayBinding::Target(target) = binding else {
                    continue;
                };
                if target.kind != BindingKind::Proc {
                    continue;
                }
                let Some(token) = &target.token else { continue };
                let implementation = DeferredImplementationId {
                    command: token.origin.clone(),
                    generation: target.implementation_generation,
                    allocation: target.implementation_allocation.clone(),
                };
                namespaces
                    .entry(implementation)
                    .or_default()
                    .insert(slot.holder().into_owned());
            }
        }
        Self { namespaces }
    }
    pub(super) fn for_implementation(
        &self,
        implementation: &DeferredImplementationId,
    ) -> Option<&BTreeSet<SourceNamespaceKey>> {
        self.namespaces.get(implementation)
    }
}

struct TableIndexEntry {
    // Retaining the exact table prevents allocator reuse and forces existing
    // COW mutations to detach. Bucket lookup independently checks Arc identity.
    bindings: Arc<BindingTable>,
    index: OnceLock<Arc<DeferredNamespaceIndex>>,
}

pub(super) struct DeferredNamespaceIndexCache {
    entries: Mutex<HashMap<usize, Arc<TableIndexEntry>>>,
    #[cfg(any(test, debug_assertions))]
    trace: bool,
    #[cfg(any(test, debug_assertions))]
    requests: std::sync::atomic::AtomicUsize,
    #[cfg(any(test, debug_assertions))]
    builds: std::sync::atomic::AtomicUsize,
    #[cfg(any(test, debug_assertions))]
    rows: std::sync::atomic::AtomicUsize,
    #[cfg(any(test, debug_assertions))]
    alternatives: std::sync::atomic::AtomicUsize,
}
impl Default for DeferredNamespaceIndexCache {
    fn default() -> Self {
        Self {
            entries: Mutex::default(),
            #[cfg(any(test, debug_assertions))]
            trace: std::env::var_os("TCL_LSP_TRACE_SOURCE_PHASES").is_some(),
            #[cfg(any(test, debug_assertions))]
            requests: std::sync::atomic::AtomicUsize::default(),
            #[cfg(any(test, debug_assertions))]
            builds: std::sync::atomic::AtomicUsize::default(),
            #[cfg(any(test, debug_assertions))]
            rows: std::sync::atomic::AtomicUsize::default(),
            #[cfg(any(test, debug_assertions))]
            alternatives: std::sync::atomic::AtomicUsize::default(),
        }
    }
}
// A forked collector starts with an empty derived cache. Warmth contributes
// neither to retained semantic equality nor to diagnostic Debug output.
impl Clone for DeferredNamespaceIndexCache {
    fn clone(&self) -> Self {
        Self::default()
    }
}
impl std::fmt::Debug for DeferredNamespaceIndexCache {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("DeferredNamespaceIndexCache")
    }
}
impl DeferredNamespaceIndexCache {
    pub(super) fn for_bindings(&self, bindings: &Arc<BindingTable>) -> Arc<DeferredNamespaceIndex> {
        #[cfg(any(test, debug_assertions))]
        if self.trace {
            self.requests
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }
        // The address is an indexing bucket only. The actual strong table Arc
        // below proves reuse, independently of hashes or displayed names.
        let address = Arc::as_ptr(bindings) as usize;
        let entry = {
            let mut entries = self
                .entries
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if let Some(entry) = entries
                .get(&address)
                .filter(|entry| Arc::ptr_eq(&entry.bindings, bindings))
            {
                Arc::clone(entry)
            } else {
                let entry = Arc::new(TableIndexEntry {
                    bindings: Arc::clone(bindings),
                    index: OnceLock::new(),
                });
                entries.insert(address, Arc::clone(&entry));
                entry
            }
        };
        // No table traversal or future callback runs under the memo mutex.
        Arc::clone(entry.index.get_or_init(|| {
            #[cfg(any(test, debug_assertions))]
            if cfg!(test) || self.trace {
                self.builds
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            }
            #[cfg(any(test, debug_assertions))]
            let (mut rows, mut alternatives) = (0, 0);
            let index = DeferredNamespaceIndex::from_bindings(&entry.bindings, |count| {
                #[cfg(any(test, debug_assertions))]
                if self.trace {
                    rows += 1;
                    alternatives += count;
                }
                #[cfg(not(any(test, debug_assertions)))]
                let _ = count;
            });
            #[cfg(any(test, debug_assertions))]
            if self.trace {
                self.rows
                    .fetch_add(rows, std::sync::atomic::Ordering::Relaxed);
                self.alternatives
                    .fetch_add(alternatives, std::sync::atomic::Ordering::Relaxed);
            }
            Arc::new(index)
        }))
    }
    #[cfg(any(test, debug_assertions))]
    pub(super) fn trace_counts(&self) -> (usize, usize, usize, usize) {
        use std::sync::atomic::Ordering::Relaxed;
        (
            self.requests.load(Relaxed),
            self.builds.load(Relaxed),
            self.rows.load(Relaxed),
            self.alternatives.load(Relaxed),
        )
    }
    #[cfg(test)]
    fn build_count(&self) -> usize {
        self.builds.load(std::sync::atomic::Ordering::Relaxed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command_binding::{
        ModuleCommandBindings, SourceAnalysisOptions, SourceCommandBindings,
    };
    fn analyse(source: &str) -> SourceCommandBindings {
        let context =
            tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
        let input = crate::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            Arc::clone(&context),
            config,
        );
        SourceCommandBindings::analyse_image_in_frame_with_options(
            &tcl_lexer::SourceImage::document(source),
            &crate::var_resolve::VariableExecutionFrame::Unknown,
            config,
            context.commands(),
            SourceAnalysisOptions::for_logical_source(&input).unwrap(),
        )
        .unwrap()
    }
    fn original_target(state: &ModuleCommandBindings) -> super::super::ResolvedCommandTarget {
        state
            .bindings
            .get("::p")
            .unwrap()
            .iter()
            .find_map(|binding| match binding {
                MayBinding::Target(target) if target.kind == BindingKind::Proc => {
                    Some(target.clone())
                }
                _ => None,
            })
            .expect("genuine original procedure target")
    }
    fn key(target: &super::super::ResolvedCommandTarget) -> DeferredImplementationId {
        DeferredImplementationId {
            command: target.token.as_ref().unwrap().origin.clone(),
            generation: target.implementation_generation,
            allocation: target.implementation_allocation.clone(),
        }
    }
    fn legacy_namespaces(
        state: &ModuleCommandBindings,
        implementation: &DeferredImplementationId,
    ) -> BTreeSet<SourceNamespaceKey> {
        state
            .bindings
            .iter()
            .flat_map(|(slot, alternatives)| {
                alternatives.iter().map(move |binding| (slot, binding))
            })
            .filter_map(|(slot, binding)| match binding {
                MayBinding::Target(target)
                    if target.kind == BindingKind::Proc
                        && target.implementation_generation == implementation.generation
                        && target.implementation_allocation == implementation.allocation
                        && target
                            .token
                            .as_ref()
                            .is_some_and(|token| token.origin == implementation.command) =>
                {
                    Some(slot.holder().into_owned())
                }
                _ => None,
            })
            .collect()
    }
    #[test]
    fn deferred_namespace_index_preserves_exact_implementation_and_all_holder_alternatives() {
        // naming.source.original-point-operand-projection
        // docs/design/analysis/name-resolution-proofs/original-point-operand-projection.md
        let analysis = analyse("proc p {} {return VALUE}");
        let mut state = (*analysis.final_state).clone();
        let target = original_target(&state);
        assert!(target.implementation_allocation.is_some());
        let implementation = key(&target);
        state.replace(
            "::one::p",
            BTreeSet::from([
                MayBinding::Target(target.clone()),
                MayBinding::Missing,
                MayBinding::Unknown,
            ]),
        );
        state.replace(
            "::one::same",
            BTreeSet::from([MayBinding::Target(target.clone())]),
        );
        state.replace(
            "::two::p",
            BTreeSet::from([MayBinding::Target(target.clone())]),
        );
        let mut wrong_kind = target.clone();
        wrong_kind.kind = BindingKind::Alias;
        let mut no_token = target.clone();
        no_token.token = None;
        let mut generation = target.clone();
        generation.implementation_generation += 1;
        let mut allocation = target.clone();
        allocation.implementation_allocation = None;
        state.replace(
            "::excluded::kind",
            BTreeSet::from([MayBinding::Target(wrong_kind)]),
        );
        state.replace(
            "::excluded::token",
            BTreeSet::from([MayBinding::Target(no_token)]),
        );
        state.replace(
            "::excluded::generation",
            BTreeSet::from([MayBinding::Target(generation)]),
        );
        state.replace(
            "::excluded::allocation",
            BTreeSet::from([MayBinding::Target(allocation)]),
        );
        state.replace(
            "::excluded::import",
            BTreeSet::from([MayBinding::Imported(target.token.unwrap().into())]),
        );
        let index = DeferredNamespaceIndexCache::default().for_bindings(&state.bindings);
        let expected = BTreeSet::from([
            SourceNamespaceKey::from("::"),
            SourceNamespaceKey::from("::one"),
            SourceNamespaceKey::from("::two"),
        ]);
        assert_eq!(
            index.for_implementation(&implementation).unwrap(),
            &expected
        );
        assert_eq!(legacy_namespaces(&state, &implementation), expected);
        for other in [
            DeferredImplementationId {
                generation: implementation.generation + 1,
                ..implementation.clone()
            },
            DeferredImplementationId {
                allocation: None,
                ..implementation.clone()
            },
        ] {
            assert_eq!(
                index
                    .for_implementation(&other)
                    .cloned()
                    .unwrap_or_default(),
                legacy_namespaces(&state, &other)
            );
        }
    }
    #[test]
    fn deferred_namespace_index_reuses_only_the_retained_table_and_detaches_on_mutation() {
        // naming.source.original-point-operand-projection
        // docs/design/analysis/name-resolution-proofs/original-point-operand-projection.md
        let analysis = analyse("proc p {} {return VALUE}");
        let original = &analysis.final_state;
        let target = original_target(original);
        let key = key(&target);
        let cache = DeferredNamespaceIndexCache::default();
        let first = cache.for_bindings(&original.bindings);
        let mut child = (**original).clone();
        assert!(Arc::ptr_eq(&original.bindings, &child.bindings));
        assert!(Arc::ptr_eq(&first, &cache.for_bindings(&child.bindings)));
        assert_eq!(cache.build_count(), 1);
        child.replace(
            "::new_holder::p",
            BTreeSet::from([MayBinding::Target(target)]),
        );
        assert!(!Arc::ptr_eq(&original.bindings, &child.bindings));
        let changed = cache.for_bindings(&child.bindings);
        assert!(!Arc::ptr_eq(&first, &changed));
        assert_eq!(cache.build_count(), 2);
        assert!(
            !first
                .for_implementation(&key)
                .unwrap()
                .contains(&SourceNamespaceKey::from("::new_holder"))
        );
        assert!(
            changed
                .for_implementation(&key)
                .unwrap()
                .contains(&SourceNamespaceKey::from("::new_holder"))
        );
        child.replace("::new_holder::p", BTreeSet::from([MayBinding::Unknown]));
        let opaque = cache.for_bindings(&child.bindings);
        assert_eq!(cache.build_count(), 3);
        assert!(
            !opaque
                .for_implementation(&key)
                .unwrap()
                .contains(&SourceNamespaceKey::from("::new_holder"))
        );
        assert!(Arc::ptr_eq(&first, &cache.for_bindings(&original.bindings)));
        let copied_table = Arc::new((*original.bindings).clone());
        let independent = cache.for_bindings(&copied_table);
        assert!(!Arc::ptr_eq(&first, &independent));
        assert_eq!(first.namespaces, independent.namespaces);
        assert_eq!(cache.build_count(), 4);
    }
    #[test]
    fn deferred_namespace_index_keeps_real_renamed_source_entries_and_cache_independent_equality() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        let analysis = analyse(
            "namespace eval original {proc p {} {return VALUE}}; namespace eval moved {}; rename ::original::p ::moved::p",
        );
        assert_eq!(analysis.deferred.len(), 1, "genuine retained declaration");
        let before = analysis.clone();
        let debug = format!("{:?}", analysis.deferred_namespace_index);
        let roots = analysis.deferred_entry_bodies(&analysis.final_state);
        assert_eq!(roots.len(), 1);
        assert_eq!(roots[0].namespace_key, SourceNamespaceKey::from("::moved"));
        assert_eq!(roots[0].namespace, "::moved");
        let entries = analysis.deferred_entry_states(&analysis.final_state);
        assert_eq!(entries.len(), 1);
        assert!(entries[0].1.same_state(&analysis.final_state));
        assert_eq!(
            entries[0].0.implementation_id(),
            roots[0].implementation_id()
        );
        assert_eq!(analysis, before);
        assert_eq!(before.deferred_namespace_index.build_count(), 0);
        assert_eq!(analysis.deferred_namespace_index.build_count(), 1);
        assert_eq!(debug, format!("{:?}", analysis.deferred_namespace_index));
        assert_eq!(roots, analysis.deferred_entry_bodies(&analysis.final_state));
    }
    #[test]
    fn deferred_namespace_index_keeps_special_roots_and_original_outcome_states_separate() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        let analysis = analyse("proc p {} {return VALUE}");
        let original = analysis.deferred.values().next().unwrap().clone();
        for variant in 0..3 {
            let mut case = analysis.clone();
            let mut special = original.clone();
            // These are selection-rule controls over a retained declaration;
            // they issue no reached event, method or callback frame.
            special.identity.push_str("_not_in_the_command_table");
            match variant {
                0 => special.event = Some("TEST_EVENT".into()),
                1 => special.receiver_method = true,
                _ => {
                    special.future_frame = Some(crate::var_resolve::VariableExecutionFrame::Unknown)
                }
            }
            case.deferred.clear();
            case.deferred
                .insert(special.implementation_id(), special.clone());
            assert_eq!(case.deferred_entry_bodies(&case.final_state), vec![special]);
            assert_eq!(case.deferred_namespace_index.build_count(), 0);
        }

        let mut absent = (*analysis.final_state).clone();
        absent.replace("::p", BTreeSet::from([MayBinding::Missing]));
        let mut case = analysis.clone();
        case.deferred_outcomes.clear();
        assert!(case.deferred_entry_states(&absent).is_empty());
        // Modelled completion alternatives retain their actual table owner.
        // The index neither issues a completion nor folds it into continuation.
        let outcome = super::super::SourceDeferredOutcome {
            implementation: original.implementation_id(),
            namespace_key: original.namespace_key.clone(),
            frame: crate::var_resolve::VariableExecutionFrame::Unknown,
            realm: original.realm,
            route: tcl_registry::completion_route::InvocationCompletionRoute::ProcessExit,
            state: Arc::clone(&analysis.final_state),
        };
        case.deferred_outcomes.push(outcome.clone());
        assert!(case.deferred_entry_states(&absent).is_empty());
        let mut abrupt = outcome;
        abrupt.route = tcl_registry::completion_route::InvocationCompletionRoute::Tcl(
            tcl_core_types::Code::Error,
        );
        case.deferred_outcomes.push(abrupt.clone());
        case.deferred_outcomes.push(abrupt);
        let entries = case.deferred_entry_states(&absent);
        assert_eq!(entries.len(), 1, "duplicate actual outcome worlds coalesce");
        assert_eq!(
            entries[0].0.implementation_id(),
            original.implementation_id()
        );
        assert!(Arc::ptr_eq(&entries[0].1, &analysis.final_state));
        assert!(!entries[0].1.same_state(&absent));
        assert!(case.deferred_entry_bodies(&absent).is_empty());
    }
}
