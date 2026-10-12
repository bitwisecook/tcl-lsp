// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original body intervals derived from one exact retained declaration inventory.

use super::{DeferredImplementationId, DeferredSourceBody, SourceOriginId};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, Weak};

type Inventory = BTreeMap<DeferredImplementationId, DeferredSourceBody>;

struct BodyRange {
    implementation: DeferredImplementationId,
    start: u32,
    end: u64,
    prefix_end: u64,
    length: usize,
}

pub(super) struct DeferredBodyIndex {
    inventory: Weak<Inventory>,
    origins: BTreeMap<Arc<SourceOriginId>, Vec<BodyRange>>,
}

impl DeferredBodyIndex {
    fn build(inventory: &Arc<Inventory>) -> Self {
        let mut origins: BTreeMap<_, Vec<_>> = BTreeMap::new();
        for (implementation, body) in inventory.iter() {
            let Some(origin) = &body.source_origin else {
                continue;
            };
            origins
                .entry(Arc::clone(origin))
                .or_default()
                .push(BodyRange {
                    implementation: implementation.clone(),
                    start: body.offset,
                    end: u64::from(body.offset) + body.source.len() as u64,
                    prefix_end: 0,
                    length: body.source.len(),
                });
        }
        for ranges in origins.values_mut() {
            ranges.sort_unstable_by_key(|range| range.start);
            let mut end = 0;
            for range in ranges {
                end = end.max(range.end);
                range.prefix_end = end;
            }
        }
        Self {
            inventory: Arc::downgrade(inventory),
            origins,
        }
    }

    /// Preserve all innermost candidates, including conflicting equal spans.
    /// This index supplies no frame, namespace, formal, compiler or Native role.
    pub(super) fn innermost<'a>(
        &self,
        inventory: &'a Arc<Inventory>,
        origin: &Arc<SourceOriginId>,
        offset: u32,
    ) -> Vec<&'a DeferredSourceBody> {
        if !self
            .inventory
            .upgrade()
            .is_some_and(|original| Arc::ptr_eq(&original, inventory))
        {
            return Vec::new();
        }
        let Some(ranges) = self.origins.get(origin) else {
            return Vec::new();
        };
        let mut cursor = ranges.partition_point(|range| range.start <= offset);
        let mut shortest = usize::MAX;
        let mut selected = Vec::new();
        while cursor != 0 {
            cursor -= 1;
            let range = &ranges[cursor];
            if range.prefix_end <= u64::from(offset) {
                break;
            }
            if u64::from(offset) >= range.end || range.length > shortest {
                continue;
            }
            if range.length < shortest {
                shortest = range.length;
                selected.clear();
            }
            selected.push(&range.implementation);
        }
        // The original worker visits the exact inventory's ordered entries.
        selected.sort_unstable();
        selected
            .into_iter()
            .filter_map(|id| inventory.get(id))
            .collect()
    }
}

struct RetainedIndex {
    inventory: Weak<Inventory>,
    index: Arc<DeferredBodyIndex>,
}

/// One derived index; inventory detachment replaces it rather than retaining
/// every earlier flow world. The Weak owner prevents allocator reuse and does
/// not force cloning the inventory's body payloads on an exclusive mutation.
#[derive(Default)]
pub(super) struct DeferredBodyIndexCache(Mutex<Option<RetainedIndex>>);

impl Clone for DeferredBodyIndexCache {
    fn clone(&self) -> Self {
        Self::default()
    }
}
impl std::fmt::Debug for DeferredBodyIndexCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("DeferredBodyIndexCache")
    }
}
impl DeferredBodyIndexCache {
    pub(super) fn for_inventory(&self, inventory: &Arc<Inventory>) -> Arc<DeferredBodyIndex> {
        {
            let held = self
                .0
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if let Some(held) = held.as_ref()
                && held
                    .inventory
                    .upgrade()
                    .is_some_and(|original| Arc::ptr_eq(&original, inventory))
            {
                return Arc::clone(&held.index);
            }
        }
        // Construction and semantic source comparisons run outside the mutex.
        let index = Arc::new(DeferredBodyIndex::build(inventory));
        *self
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(RetainedIndex {
            inventory: Arc::downgrade(inventory),
            index: Arc::clone(&index),
        });
        index
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command_binding::SourceCommandBindings;

    fn original(source: &str) -> SourceCommandBindings {
        SourceCommandBindings::analyse_with_options(
            source,
            tcl_lexer::LexerConfig::default(),
            &tcl_registry::CommandRegistry::build_default(),
            crate::command_binding::SourceAnalysisOptions {
                native_compilation: crate::environment_ingress::authoring_native_compilation(),
                invocation_dialect: Some(tcl_registry::InvocationDialect::for_version(
                    tcl_dialect::TclVersion::V8_6,
                )),
                ..Default::default()
            },
        )
    }

    #[test]
    fn original_body_index_preserves_innermost_source_and_boundary_ownership() {
        let source = "proc outer {} {proc inner {} {return INNER}; return OUTER}; proc sibling {} {return SIBLING}";
        let bindings = original(source);
        let origin = bindings.source_origin().unwrap();
        let inner = u32::try_from(source.find("return INNER").unwrap()).unwrap();
        let outer = u32::try_from(source.find("return OUTER").unwrap()).unwrap();
        let sibling = u32::try_from(source.find("return SIBLING").unwrap()).unwrap();
        let inner_entry = bindings.conditional_body_entry_at(origin, inner).unwrap();
        let outer_entry = bindings.conditional_body_entry_at(origin, outer).unwrap();
        let sibling_entry = bindings.conditional_body_entry_at(origin, sibling).unwrap();
        assert_eq!(inner_entry.source().try_text().unwrap(), "return INNER");
        assert!(
            outer_entry
                .source()
                .try_text()
                .unwrap()
                .contains("return OUTER")
        );
        assert_eq!(sibling_entry.source().try_text().unwrap(), "return SIBLING");
        assert_ne!(inner_entry.frame(), outer_entry.frame());
        assert_ne!(sibling_entry.frame(), outer_entry.frame());
        let foreign = Arc::new(SourceOriginId::authored(&Arc::from(
            source.replace("INNER", "OTHER"),
        )));
        assert!(
            bindings
                .conditional_body_entry_at(&foreign, inner)
                .is_none()
        );
        assert!(
            bindings
                .conditional_body_entry_at(origin, u32::try_from(source.len()).unwrap())
                .is_none()
        );
        assert!(
            bindings
                .conditional_body_entry_at(origin, inner_entry.source().base() - 1)
                .is_some_and(|entry| entry.frame() == outer_entry.frame())
        );
    }

    #[test]
    fn original_body_index_invalidates_real_inventory_without_cloning_body_buffers() {
        let source = "proc p {x} {return $x}";
        let mut bindings = original(source);
        let origin = bindings.source_origin().unwrap().clone();
        let offset = u32::try_from(source.find("return").unwrap()).unwrap();
        let first = bindings.deferred.keys().next().unwrap().clone();
        // Detach this genuine retained analysis from the bounded source cache
        // before observing the exclusive mutation's body allocation.
        let _ = bindings.deferred.get_mut(&first).unwrap();
        let inventory = bindings.deferred.shared();
        let index = bindings.deferred_body_index.for_inventory(&inventory);
        let again = bindings.deferred_body_index.for_inventory(&inventory);
        assert!(Arc::ptr_eq(&index, &again));
        let weak = Arc::downgrade(&inventory);
        let (id, pointer) = inventory
            .iter()
            .map(|(id, body)| (id.clone(), body.source.as_ptr()))
            .next()
            .unwrap();
        assert_eq!(Arc::strong_count(&inventory), 2);
        drop(inventory);
        bindings.deferred.get_mut(&id).unwrap().event = Some("EVENT".into());
        assert!(weak.upgrade().is_none());
        assert_eq!(bindings.deferred.get(&id).unwrap().source.as_ptr(), pointer);
        assert!(
            bindings
                .conditional_body_entry_at(&origin, offset)
                .is_none()
        );
        let fresh = bindings.deferred.shared();
        let changed = bindings.deferred_body_index.for_inventory(&fresh);
        assert!(!Arc::ptr_eq(&index, &changed));
        let fork = bindings.clone();
        assert!(fork.deferred_body_index.0.lock().unwrap().is_none());
        assert_eq!(bindings, fork);
    }

    #[test]
    fn original_body_index_retains_equal_span_conflicts_and_layout_config() {
        let source = "proc p {x} {return $x}";
        let mut bindings = original(source);
        let origin = bindings.source_origin().unwrap().clone();
        let offset = u32::try_from(source.find("return").unwrap()).unwrap();
        assert!(
            bindings
                .conditional_body_entry_at(&origin, offset)
                .is_some()
        );
        let mut conflicting = bindings.deferred.values().next().unwrap().clone();
        conflicting.namespace_key = crate::command_binding::SourceNamespaceKey::authored("::other");
        conflicting.implementation_generation += 1;
        bindings
            .deferred
            .insert(conflicting.implementation_id(), conflicting);
        assert!(
            bindings
                .conditional_body_entry_at(&origin, offset)
                .is_none()
        );
        let observations = bindings
            .declaration_layouts
            .values()
            .find(|rows| {
                crate::command_binding::declaration_layout::original_declaration_layouts(rows)
                    .is_some()
            })
            .unwrap();
        let mut changed = observations[0].clone();
        changed.config.strict_quoting = !changed.config.strict_quoting;
        assert_ne!(changed, observations[0]);
        assert!(
            crate::command_binding::declaration_layout::original_declaration_layouts(&[
                observations[0].clone(),
                changed
            ])
            .is_none()
        );
    }
}
