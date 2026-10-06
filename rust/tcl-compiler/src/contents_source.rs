// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Full source-instance attestation beside represented contents-store offsets.
//!
//! Attestations follow exact typed cell slots. Native namespace incarnations,
//! retained allocations and array members never use reporting text as keys.

use crate::{
    command_binding::{SourceOriginId, SourceOriginKind},
    place::{Place, PlaceKind},
    var_resolve::{
        ResolveContext, VariableCellKey, VariableCellTable, VariableProofRelocation,
        canonical_binding_value_key,
    },
};
use std::sync::Arc;

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub(crate) struct ContentsSourceProofs {
    active: Option<Arc<SourceOriginId>>,
    sources: VariableCellTable<Arc<SourceOriginId>>,
}

impl ContentsSourceProofs {
    pub(crate) fn relocated(&self, relocation: &VariableProofRelocation) -> Self {
        Self {
            active: self
                .active
                .as_ref()
                .map(|source| relocation.source_origin(source)),
            sources: self
                .sources
                .iter()
                .map(|(key, source)| (relocation.cell_key(key), relocation.source_origin(source)))
                .collect(),
        }
    }

    pub(crate) fn restore(&mut self, child: &Self, selected: &impl Fn(&VariableCellKey) -> bool) {
        self.sources.retain(|key, _| !selected(key));
        self.sources.extend(
            child
                .sources
                .iter()
                .filter(|(key, _)| selected(key))
                .map(|(key, source)| (key.clone(), Arc::clone(source))),
        );
    }

    pub(crate) fn copy_slot(&mut self, original: &VariableCellKey, retained: &VariableCellKey) {
        if let Some(source) = self.sources.get(original).cloned() {
            self.sources.insert(retained.clone(), source);
        } else {
            self.sources.remove(retained);
        }
    }

    pub(crate) fn joined(&mut self, other: &Self) {
        self.sources
            .retain(|key, source| other.sources.get(key) == Some(source));
        if self.active != other.active {
            self.active = None;
        }
    }
}

impl ResolveContext {
    pub(crate) fn contents_source(&self, place: &Place) -> Option<&Arc<SourceOriginId>> {
        canonical_binding_value_key(place)
            .and_then(|key| self.contents_source_proofs.sources.get(&key))
    }

    pub(crate) fn contents_write_source(&self) -> Option<&Arc<SourceOriginId>> {
        self.contents_source_proofs.active.as_ref()
    }

    /// Select the source instance for subsequent reached stores. Changing this
    /// selection preserves the provenance of every untouched physical value.
    pub(crate) fn set_contents_write_source(&mut self, source: Option<Arc<SourceOriginId>>) {
        self.contents_source_proofs.active = source;
    }

    /// Whether every reaching represented store belongs to these exact authored
    /// bytes. This adds no value, lifetime, observer or execution proof.
    pub(crate) fn contents_have_authored_source(&self, place: &Place, source: &str) -> bool {
        canonical_binding_value_key(place).and_then(|key| self.contents_source_proofs.sources.get(&key))
            .is_some_and(|origin| matches!(origin.kind(), SourceOriginKind::Authored(text) if text.bytes() == source.as_bytes()))
    }

    /// Exact source identity of the surviving store, independent of presentation.
    pub(crate) fn contents_have_source(&self, place: &Place, source: &Arc<SourceOriginId>) -> bool {
        canonical_binding_value_key(place)
            .and_then(|key| self.contents_source_proofs.sources.get(&key))
            == Some(source)
    }

    pub(crate) fn record_contents_source(&mut self, place: &Place, conditional: bool) {
        let source = self.contents_source_proofs.active.clone();
        let broad = place.kind == PlaceKind::Unknown
            || place.kind == PlaceKind::ArrayWhole
            || place
                .index
                .as_ref()
                .is_some_and(|index| index.kind != crate::place::IndexKind::Literal);
        self.contents_source_proofs.sources.retain(|key, retained| {
            let affected = self
                .contents_presence_slots
                .get(key)
                .is_none_or(|slot| crate::place::overlap(place, slot));
            !affected
                || (!place.observed && source.as_ref() == Some(retained) && (conditional || broad))
        });
        if !broad
            && !conditional
            && !place.observed
            && let (Some(key), Some(source)) = (canonical_binding_value_key(place), source)
        {
            self.contents_source_proofs.sources.insert(key, source);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        command_binding::{CommandAllocationSite, ExecutedScriptSource},
        var_resolve::resolve_literal_place,
    };
    use std::collections::BTreeMap;

    #[test]
    fn native_namespace_contents_sources_cannot_be_replaced_by_rendered_names() {
        use crate::{
            command_binding::SourceNamespaceKey,
            place::{CellGeneration, CellIdentity, CellOwner},
        };
        use tcl_core_types::ByteNamespacePath;
        use tcl_runtime_api::native_compilation::{
            NativeInterpreterIdentity, NativeNamespaceContext,
        };

        let mut context = ResolveContext::for_function("::p");
        let source = Arc::new(SourceOriginId::authored(&"original native store".into()));
        let namespace = SourceNamespaceKey::Native(NativeNamespaceContext {
            interpreter: NativeInterpreterIdentity {
                owner: 11,
                interpreter: 0,
            },
            token: 1,
            path: ByteNamespacePath::from_segments([b"A".as_slice(), b":q".as_slice()]),
        });
        let mut native = crate::place::scalar("x", namespace.display().unwrap(), false);
        native.cell = Some(CellIdentity {
            owner: CellOwner::NamespaceIdentity(Box::new(namespace)),
            name: "x".into(),
            generation: CellGeneration::Incoming,
            interpreter: None,
            storage_domain: None,
            execution: None,
        });
        let key = canonical_binding_value_key(&native).unwrap();
        let forged = crate::place::scalar(key.compatibility_name(), crate::place::LOCAL_NS, false);
        context.set_contents_write_source(Some(Arc::clone(&source)));
        context.record_contents_write(&native, 0, false);
        assert!(context.contents_have_source(&native, &source));
        assert_eq!(context.contents_source(&forged), None);

        let foreign = Arc::new(SourceOriginId::authored(&"unrelated authored store".into()));
        context.set_contents_write_source(Some(Arc::clone(&foreign)));
        context.record_contents_write(&forged, 1, false);
        assert!(context.contents_have_source(&native, &source));
        assert!(context.contents_have_source(&forged, &foreign));
    }

    #[test]
    fn same_offset_foreign_store_withdraws_only_its_physical_contents_source() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let mut context = ResolveContext::for_function("::p");
        let authored = Arc::new(SourceOriginId::authored(&Arc::from("original document")));
        context.set_contents_write_source(Some(Arc::clone(&authored)));
        let x = resolve_literal_place("x", &context, false, registry);
        let y = resolve_literal_place("y", &context, false, registry);
        context.record_contents_write(&x, 0, false);
        context.record_contents_write(&y, 0, false);
        assert!(context.contents_have_authored_source(&x, "original document"));
        assert!(context.contents_have_source(&x, &authored));
        let before = context.clone();
        let derived = ExecutedScriptSource::materialised(
            CommandAllocationSite {
                source: Arc::clone(&authored),
                offset: 9,
            },
            vec![0],
            "set x SAME",
        )
        .origin;
        context.set_contents_write_source(Some(Arc::clone(&derived)));
        context.record_contents_write(&x, 0, false);
        assert_eq!(context.contents_origin(&x), before.contents_origin(&x));
        assert!(context.contents_have_source(&x, &derived));
        assert!(!context.contents_have_source(&x, &authored));
        assert!(context.contents_have_source(&y, &authored));
        assert!(!context.contents_have_authored_source(&x, "original document"));
        assert!(context.contents_have_authored_source(&y, "original document"));
        context.join(&before);
        assert!(!context.contents_have_source(&x, &derived));
        assert!(!context.contents_have_authored_source(&x, "original document"));
        assert!(context.contents_have_authored_source(&y, "original document"));
    }

    #[test]
    fn exact_source_attestation_relocates_and_round_trips_with_the_cell_proof() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let mut context = ResolveContext::for_function("::p");
        let actual = Arc::new(SourceOriginId::authored(&Arc::from("actual document")));
        let template = Arc::new(SourceOriginId::authored(&Arc::from("body template")));
        context.set_contents_write_source(Some(Arc::clone(&actual)));
        let place = resolve_literal_place("x", &context, false, registry);
        context.record_contents_write(&place, 10, false);
        let relocation = VariableProofRelocation {
            source_origins: BTreeMap::from([(actual, template)]),
            source_offsets: BTreeMap::from([(10, 0)]),
            ..Default::default()
        };
        let relocated = context.relocated(&relocation);
        assert!(
            relocated.contents_have_authored_source(&relocation.place(&place), "body template")
        );
        assert!(!relocated.contents_have_authored_source(&place, "actual document"));
        assert_eq!(relocated.relocated(&relocation.inverse().unwrap()), context);
    }
}
