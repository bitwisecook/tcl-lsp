// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Pure catalogue geometry, independent of source walks and Native overlays.

use super::{
    AdviceCell, AdviceNamingPolicy, Arc, ByteCommandSlot, ByteNamespacePath, ContextRegistry,
    HashMap, InvocationDialect, NameBytes, NamePolicyProtocol, NativeNameProtocol, root_context,
};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Mutex, Weak};

pub(super) struct AdviceCatalogueSeed {
    cells: crate::command_binding::SharedSourceInventory<HashMap<ByteCommandSlot, AdviceCell>>,
    namespaces: BTreeSet<ByteNamespacePath>,
    namespace_exports: BTreeMap<ByteNamespacePath, Vec<NameBytes>>,
}

impl AdviceCatalogueSeed {
    pub(super) fn cells(
        &self,
    ) -> crate::command_binding::SharedSourceInventory<HashMap<ByteCommandSlot, AdviceCell>> {
        self.cells.clone()
    }
    pub(super) fn namespaces(&self) -> BTreeSet<ByteNamespacePath> {
        self.namespaces.clone()
    }
    pub(super) fn namespace_exports(&self) -> BTreeMap<ByteNamespacePath, Vec<NameBytes>> {
        self.namespace_exports.clone()
    }
    pub(super) fn build(context: &ContextRegistry, policy: &AdviceNamingPolicy) -> Option<Self> {
        let mut seed = Self {
            cells: HashMap::new().into(),
            namespaces: BTreeSet::from([ByteNamespacePath::root()]),
            namespace_exports: BTreeMap::new(),
        };
        for command in context.commands().command_names() {
            if context
                .context()
                .resolve_spec(context.commands(), command)
                .is_none()
            {
                continue;
            }
            let key = match policy.native().map(NamePolicyProtocol::recipe) {
                Some(NativeNameProtocol::C(_)) => {
                    let native = policy.native()?;
                    native
                        .recipe()
                        .command_c_api_publication_slot(root_context(native), command.as_bytes())
                        .ok()?
                }
                Some(NativeNameProtocol::Jim084) => ByteCommandSlot::new(
                    ByteNamespacePath::root(),
                    NameBytes::from(command.as_bytes()),
                ),
                None => AdviceNamingPolicy::logical_slot(command.as_bytes())?,
            };
            seed.namespaces.insert(key.namespace.clone());
            if context
                .commands()
                .get(command)
                .is_some_and(|spec| spec.is_namespace_exported)
            {
                seed.namespace_exports
                    .entry(key.namespace.clone())
                    .or_default()
                    .push(key.simple.clone());
            }
            seed.cells
                .insert(key, AdviceCell::Descriptor(command.to_owned(), Vec::new()));
        }
        Some(seed)
    }
}

struct SeedEntry {
    commands: Weak<tcl_registry::CommandRegistry>,
    generation: tcl_registry::RegistrySemanticKey,
    context: tcl_registry::model::ResolvedContext,
    policy: AdviceNamingPolicy,
    dialect: InvocationDialect,
    seed: Option<Arc<AdviceCatalogueSeed>>,
}

impl SeedEntry {
    fn matches(
        &self,
        context: &ContextRegistry,
        policy: &AdviceNamingPolicy,
        dialect: InvocationDialect,
        generation: &tcl_registry::RegistrySemanticKey,
    ) -> bool {
        let policy_matches = match (&self.policy, policy) {
            (AdviceNamingPolicy::Native(old), AdviceNamingPolicy::Native(new)) => old == new,
            (AdviceNamingPolicy::Logical(old), AdviceNamingPolicy::Logical(new)) => old == new,
            _ => false,
        };
        self.commands.ptr_eq(&Arc::downgrade(context.commands()))
            && self.generation == *generation
            && self.context == *context.context()
            && policy_matches
            && self.dialect == dialect
    }
}

/// A bounded derived memo only; original baseline equality remains unchanged.
#[derive(Default)]
pub(in crate::command_binding) struct AdviceCatalogueSeedMemo {
    entries: Mutex<Vec<SeedEntry>>,
}

impl std::fmt::Debug for AdviceCatalogueSeedMemo {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("AdviceCatalogueSeedMemo")
            .finish_non_exhaustive()
    }
}

impl AdviceCatalogueSeedMemo {
    pub(super) fn get(
        &self,
        context: &ContextRegistry,
        policy: &AdviceNamingPolicy,
        dialect: InvocationDialect,
    ) -> Option<Arc<AdviceCatalogueSeed>> {
        const MAX_SEEDS: usize = 8;
        let generation = context.commands().snapshot().semantic_key();
        {
            let Ok(mut entries) = self.entries.lock() else {
                return AdviceCatalogueSeed::build(context, policy).map(Arc::new);
            };
            entries.retain(|entry| entry.commands.strong_count() != 0);
            if let Some(entry) = entries
                .iter()
                .find(|entry| entry.matches(context, policy, dialect, &generation))
            {
                return entry.seed.clone();
            }
        }
        // Availability and coordinate work never runs under the memo lock.
        let seed = AdviceCatalogueSeed::build(context, policy).map(Arc::new);
        if generation != context.commands().snapshot().semantic_key() {
            return AdviceCatalogueSeed::build(context, policy).map(Arc::new);
        }
        let Ok(mut entries) = self.entries.lock() else {
            return seed;
        };
        entries.retain(|entry| entry.commands.strong_count() != 0);
        if let Some(entry) = entries
            .iter()
            .find(|entry| entry.matches(context, policy, dialect, &generation))
        {
            return entry.seed.clone();
        }
        if entries.len() < MAX_SEEDS {
            entries.push(SeedEntry {
                commands: Arc::downgrade(context.commands()),
                generation,
                context: context.context().clone(),
                policy: policy.clone(),
                dialect,
                seed: seed.clone(),
            });
        }
        seed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command_binding::source_transition_advice::AdviceGraph;
    use std::sync::atomic::Ordering;

    fn fixture(environment: &str) -> (Arc<ContextRegistry>, AdviceNamingPolicy, InvocationDialect) {
        let context =
            crate::environment_ingress::resolve_environment(environment).default_context_registry();
        let profile = tcl_dialect::DialectProfile::find(environment).unwrap();
        let dialect = InvocationDialect::of_profile(profile);
        let policy = AdviceNamingPolicy::Native(dialect.authored_name_policy().unwrap());
        (context, policy, dialect)
    }

    #[test]
    fn cached_catalogue_keeps_fresh_budgets_and_independent_mutations() {
        let (context, policy, dialect) = fixture("tcl8.6");
        let memo = AdviceCatalogueSeedMemo::default();
        let seed = memo.get(&context, &policy, dialect).unwrap();
        assert!(Arc::ptr_eq(
            &seed,
            &memo.get(&context, &policy, dialect).unwrap()
        ));
        assert!(
            seed.cells.values().all(
                |cell| matches!(cell, AdviceCell::Descriptor(_, lineage) if lineage.is_empty())
            )
        );
        let mut first = AdviceGraph::from_catalogue_seed(&policy, dialect, &seed);
        let second = AdviceGraph::from_catalogue_seed(&policy, dialect, &seed);
        let original = AdviceGraph::new(&context, &policy, dialect).unwrap();
        assert_eq!(first.cells, original.cells);
        assert_eq!(first.namespaces, original.namespaces);
        assert_eq!(first.namespace_exports, original.namespace_exports);
        assert!(std::ptr::eq(&*first.cells, &*second.cells));
        assert!(!Arc::ptr_eq(
            &first.procedure_steps,
            &second.procedure_steps
        ));
        assert!(!Arc::ptr_eq(
            &first.decoded_phase_steps,
            &second.decoded_phase_steps
        ));
        first.procedure_steps.fetch_add(3, Ordering::Relaxed);
        first.decoded_phase_steps.fetch_add(5, Ordering::Relaxed);
        assert_eq!(second.procedure_steps.load(Ordering::Relaxed), 0);
        assert_eq!(second.decoded_phase_steps.load(Ordering::Relaxed), 0);
        let key = first.cells.keys().next().unwrap().clone();
        first.cells.insert(key.clone(), AdviceCell::Deleted);
        first.namespaces.clear();
        first.namespace_exports.clear();
        assert!(matches!(
            second.cells.get(&key),
            Some(AdviceCell::Descriptor(_, _))
        ));
        assert_eq!(second.cells, seed.cells);
        assert_eq!(second.namespaces, seed.namespaces);
        assert_eq!(second.namespace_exports, seed.namespace_exports);
    }

    #[test]
    fn changed_context_store_policy_and_dialect_cannot_reuse_catalogue() {
        let (context, policy, dialect) = fixture("tcl8.6");
        let memo = AdviceCatalogueSeedMemo::default();
        let seed = memo.get(&context, &policy, dialect).unwrap();
        let (other_context, other_policy, other_dialect) = fixture("tcl9.0");
        let changed_context = memo.get(&other_context, &policy, dialect).unwrap();
        let same_store = other_context.with_command_store(Arc::clone(context.commands()));
        assert!(Arc::ptr_eq(context.commands(), same_store.commands()));
        assert!(context.context() != same_store.context());
        let context_only = memo.get(&same_store, &policy, dialect).unwrap();
        assert!(!Arc::ptr_eq(&seed, &context_only));
        let uncached = AdviceCatalogueSeed::build(&same_store, &policy).unwrap();
        assert_eq!(context_only.cells, uncached.cells);
        assert_eq!(context_only.namespaces, uncached.namespaces);
        assert_eq!(context_only.namespace_exports, uncached.namespace_exports);
        let changed_policy = memo.get(&context, &other_policy, dialect).unwrap();
        let changed_dialect = memo.get(&context, &policy, other_dialect).unwrap();
        for changed in [changed_context, changed_policy, changed_dialect] {
            assert!(!Arc::ptr_eq(&seed, &changed));
        }
        let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
        let new_store = Arc::new(context.commands().project_for_profile(profile));
        let generation = context.with_command_store(new_store);
        assert!(!Arc::ptr_eq(context.commands(), generation.commands()));
        assert!(!Arc::ptr_eq(
            &seed,
            &memo.get(&generation, &policy, dialect).unwrap()
        ));
        let input = crate::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            Arc::clone(&context),
            tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
        );
        let logical = AdviceNamingPolicy::Logical(input);
        assert!(!Arc::ptr_eq(
            &seed,
            &memo.get(&context, &logical, dialect).unwrap()
        ));
        let bounded = AdviceCatalogueSeedMemo::default();
        for depth in 0..9 {
            let input = crate::analyser::ResolvedAnalysisInput::new(
                profile,
                profile,
                Arc::clone(&context),
                tcl_lexer::LexerConfig::for_file_grammar(profile.grammar).at_depth(depth),
            );
            let policy = AdviceNamingPolicy::Logical(input);
            let first = bounded.get(&context, &policy, dialect).unwrap();
            if depth == 8 {
                let second = bounded.get(&context, &policy, dialect).unwrap();
                assert!(!Arc::ptr_eq(&first, &second));
                assert_eq!(first.cells, second.cells);
            }
        }
        assert_eq!(bounded.entries.lock().unwrap().len(), 8);
    }

    #[test]
    fn native_overlays_and_poison_preserve_uncached_catalogue_boundaries() {
        use tcl_runtime_api::native_compilation::NativeCommandImplementation;
        let (context, _, dialect) = fixture("tcl8.6");
        let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
        let (_owner, entry) = crate::environment_ingress::captured_native_entry_with_owner(profile);
        let policy = AdviceNamingPolicy::Native(entry.command_name_policy().unwrap());
        let memo = AdviceCatalogueSeedMemo::default();
        let seed = memo.get(&context, &policy, dialect).unwrap();
        let key = entry
            .commands
            .iter()
            .find(|row| row.slot.simple.as_bytes() == b"set")
            .unwrap()
            .slot
            .clone();
        let mut deleted_entry = entry.clone();
        deleted_entry.commands.retain(|row| row.slot != key);
        let mut deleted = AdviceGraph::from_catalogue_seed(&policy, dialect, &seed);
        deleted
            .retain_native_baseline(&context, &deleted_entry)
            .unwrap();
        assert_eq!(deleted.cells.get(&key), Some(&AdviceCell::Deleted));
        let mut shadow_entry = entry;
        shadow_entry
            .commands
            .iter_mut()
            .find(|row| row.slot == key)
            .unwrap()
            .implementation = NativeCommandImplementation::Opaque;
        let mut shadowed = AdviceGraph::from_catalogue_seed(&policy, dialect, &seed);
        shadowed
            .retain_native_baseline(&context, &shadow_entry)
            .unwrap();
        assert_eq!(shadowed.cells.get(&key), Some(&AdviceCell::Shadowed));
        assert!(
            matches!(seed.cells.get(&key), Some(AdviceCell::Descriptor(_, lineage)) if lineage.is_empty())
        );
        assert!(
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let _guard = memo.entries.lock().unwrap();
                panic!("poisoned memo uses the original uncached builder");
            }))
            .is_err()
        );
        let uncached = memo.get(&context, &policy, dialect).unwrap();
        assert!(!Arc::ptr_eq(&seed, &uncached));
        assert_eq!(uncached.cells, seed.cells);
        assert_eq!(uncached.namespaces, seed.namespaces);
        assert_eq!(uncached.namespace_exports, seed.namespace_exports);
    }
}
