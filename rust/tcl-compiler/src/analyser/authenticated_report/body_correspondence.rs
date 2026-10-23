// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Complete report-only body owners from the actual diagnostic source unit.

use std::sync::Arc;

use crate::command_binding::{
    CommandAllocationSite, ExecutedScriptSource, SourceNamespaceKey, SourceOriginId,
    SourceOriginKind,
};
use crate::compilation_unit::OriginalOptimiserPreparedUnit;
use crate::ir::{MethodKind, Procedure, ProcedureBodyKind, ProcedureId};
use tcl_core_types::ProcedureModuleId;
use tcl_lexer::{NativeWord, Span};
use tcl_registry::CommandRegistry;

#[derive(Clone, Copy, PartialEq, Eq)]
enum ReportBodyRole {
    Named(ProcedureBodyKind),
    Unit(ProcedureBodyKind),
    Carried(ProcedureBodyKind),
    Method(MethodKind),
    Definition(tcl_registry::definer::DefinerFamily),
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ReportDeclarationPurpose {
    AllocatedSource,
    NativeSource,
    LogicalSource,
    AuthoredSource,
    HostedSource,
}

struct ReportBodyOwner {
    id: ProcedureId,
    role: ReportBodyRole,
    declaration_purpose: Option<ReportDeclarationPurpose>,
    declaration: Span,
    site: Option<CommandAllocationSite>,
    body_offset: Option<u32>,
    source: Option<Arc<ExecutedScriptSource>>,
    namespace: Option<Box<SourceNamespaceKey>>,
    original_word: Option<NativeWord>,
    definition_parent: Option<ProcedureId>,
}

/// The prepared owner keeps the private complete production snapshot alive.
/// Coordinates select candidates only after that actual issuer is authenticated.
pub(super) struct ReportBodyInventory {
    prepared: OriginalOptimiserPreparedUnit,
    entry: crate::command_binding::SourceAnalysisEntry,
    hosted_entry: Option<crate::command_binding::SourceAnalysisEntry>,
    root: Arc<SourceOriginId>,
    module: ProcedureModuleId,
    bodies: Vec<ReportBodyOwner>,
}

impl ReportBodyInventory {
    pub(super) fn capture(
        prepared: OriginalOptimiserPreparedUnit,
        registry: &CommandRegistry,
    ) -> Option<Self> {
        if !prepared.is_whole_source_unit() || !prepared.origin().is_current_analysis_context() {
            return None;
        }
        let module = &prepared.unit().ir_module;
        let retained = super::trace_capture_option(
            module.retained_source_bindings.as_ref(),
            "bodies.retained",
        )?;
        let bindings = super::trace_capture_option(
            retained.positioned_for_module(module, registry),
            "bodies.positioned",
        )?;
        let root = Arc::clone(super::trace_capture_option(
            bindings.source_origin(),
            "bodies.source-origin",
        )?);
        let input =
            super::trace_capture_option(module.source_metadata_input.as_ref(), "bodies.input")?;
        // Hosted source is a separate actual-entry domain, never inferred from
        // a dialect label and never promoted to Native execution eligibility.
        #[cfg(test)]
        if std::env::var_os("TCL_LSP_TRACE_AUTHENTICATED_REPORT").is_some() {
            eprintln!(
                "REPORT_BODY_DOMAIN hosted={} vendor_input_matches={} native_entry={}",
                input.has_hosted_source_name_context(),
                module.source_entry.vendor_source_input.as_ref() == Some(input),
                module.source_entry.native_entry.is_some()
            );
        }
        let hosted_entry = if input.has_hosted_source_name_context() {
            if module.source_entry.vendor_source_input.as_ref() != Some(input)
                || module.source_entry.native_entry.is_some()
            {
                return None;
            }
            Some(module.source_entry.clone())
        } else {
            if module.source_entry.vendor_source_input.is_some() {
                return None;
            }
            None
        };
        for (id, card) in &module.original_hosted_declaration_body_units {
            let body = module.body_units.get(id)?;
            if hosted_entry.is_none() || !card.matches_procedure(module, body, registry) {
                return None;
            }
        }
        #[cfg(test)]
        if std::env::var_os("TCL_LSP_TRACE_AUTHENTICATED_REPORT").is_some() {
            eprintln!(
                "REPORT_BODY_INVENTORY named={} units={} hosted_cards={} events={} definitions={} hosted_input={}",
                module.procedures.len(),
                module.body_units.len(),
                module.original_hosted_declaration_body_units.len(),
                module.irules_event_bodies.len(),
                module.original_definition_scripts.len(),
                input.has_hosted_source_name_context()
            );
        }
        let mut bodies = Vec::new();
        for (id, body) in &module.procedures {
            Self::push(
                &mut bodies,
                module,
                registry,
                *id,
                Self::procedure(module, *id, body, ReportBodyRole::Named(body.body_kind))?,
            )?;
        }
        for (id, body) in module.body_lowerings() {
            let role = if module.body_units.contains_key(id) {
                ReportBodyRole::Unit(body.body_kind)
            } else {
                ReportBodyRole::Carried(body.body_kind)
            };
            Self::push(
                &mut bodies,
                module,
                registry,
                *id,
                Self::procedure(module, *id, body, role)?,
            )?;
        }
        for (id, body) in &module.methods {
            Self::push(
                &mut bodies,
                module,
                registry,
                *id,
                ReportBodyOwner {
                    id: *id,
                    role: ReportBodyRole::Method(body.kind),
                    declaration_purpose: None,
                    declaration: body.span?,
                    site: None,
                    body_offset: None,
                    source: body.body.executed_source.clone(),
                    namespace: body.body.namespace_context.clone(),
                    original_word: None,
                    definition_parent: body.definition_scope,
                },
            )?;
        }
        for id in module.original_definition_scripts.keys() {
            let body = module.original_definition_script_source(*id, registry)?;
            Self::push(
                &mut bodies,
                module,
                registry,
                *id,
                ReportBodyOwner {
                    id: *id,
                    role: ReportBodyRole::Definition(body.family()),
                    declaration_purpose: None,
                    declaration: body.declaration_span(),
                    site: None,
                    body_offset: Some(body.content_span().start()),
                    source: None,
                    namespace: None,
                    original_word: Some(body.body_word().clone()),
                    definition_parent: None,
                },
            )?;
        }
        for (purpose, ids) in [
            (
                ReportDeclarationPurpose::AllocatedSource,
                module
                    .original_declaration_body_units
                    .keys()
                    .copied()
                    .collect::<Vec<_>>(),
            ),
            (
                ReportDeclarationPurpose::NativeSource,
                module
                    .original_native_declaration_body_units
                    .keys()
                    .copied()
                    .collect(),
            ),
            (
                ReportDeclarationPurpose::LogicalSource,
                module
                    .original_logical_declaration_body_units
                    .keys()
                    .copied()
                    .collect(),
            ),
            (
                ReportDeclarationPurpose::AuthoredSource,
                module
                    .original_authored_declaration_body_units
                    .keys()
                    .copied()
                    .collect(),
            ),
            (
                ReportDeclarationPurpose::HostedSource,
                module
                    .original_hosted_declaration_body_units
                    .keys()
                    .copied()
                    .collect(),
            ),
        ] {
            if ids.iter().any(|id| {
                !bodies
                    .iter()
                    .any(|body| body.id == *id && body.declaration_purpose == Some(purpose))
            }) {
                return None;
            }
        }
        // Every additional event card must join an actual retained body, and
        // each original event word belongs to this complete source generation.
        for (id, card) in &module.irules_event_bodies {
            let body = super::trace_capture_option(
                module
                    .procedures
                    .get(id)
                    .or_else(|| module.body_lowering(id)),
                "bodies.event-procedure",
            )?;
            if super::trace_capture_gate(
                !card.owns_procedure(body, &module.source, module.native_lexer_config(), registry),
                "bodies.event-procedure-match",
            ) {
                return None;
            }
            let row = super::trace_capture_option(
                bodies.iter_mut().find(|row| row.id == *id),
                "bodies.event-row",
            )?;
            if super::trace_capture_gate(row.original_word.is_some(), "bodies.event-duplicate-word")
            {
                return None;
            }
            row.original_word = Some(
                super::trace_capture_option(
                    card.event_word(&module.source, module.native_lexer_config(), registry),
                    "bodies.event-word",
                )?
                .clone(),
            );
        }
        // A reference into a definition scope must join this same complete
        // inventory; a foreign ID or omitted parent never becomes a free label.
        if bodies.iter().any(|body| {
            body.definition_parent.is_some_and(|parent| {
                !bodies.iter().any(|candidate| {
                    candidate.id == parent
                        && matches!(candidate.role, ReportBodyRole::Definition(_))
                })
            })
        }) {
            return None;
        }
        let module_id = module.procedure_ids.module_id();
        let entry = module.source_entry.clone();
        Some(Self {
            prepared,
            entry,
            hosted_entry,
            root,
            module: module_id,
            bodies,
        })
    }

    fn procedure(
        module: &crate::ir::Module,
        id: ProcedureId,
        body: &Procedure,
        role: ReportBodyRole,
    ) -> Option<ReportBodyOwner> {
        #[cfg(test)]
        if std::env::var_os("TCL_LSP_TRACE_AUTHENTICATED_REPORT").is_some() {
            let role = match role {
                ReportBodyRole::Named(_) => "named",
                ReportBodyRole::Unit(_) => "unit",
                ReportBodyRole::Carried(_) => "carried",
                ReportBodyRole::Method(_) => "method",
                ReportBodyRole::Definition(_) => "definition",
            };
            eprintln!(
                "REPORT_BODY_CANDIDATE id={id:?} role={role} kind={:?} declaration={:?} site={:?} allocated={} native={} logical={} authored={} hosted={}",
                body.body_kind,
                body.span,
                body.declaration_site,
                module.original_declaration_body_units.contains_key(&id),
                module
                    .original_native_declaration_body_units
                    .contains_key(&id),
                module
                    .original_logical_declaration_body_units
                    .contains_key(&id),
                module
                    .original_authored_declaration_body_units
                    .contains_key(&id),
                module
                    .original_hosted_declaration_body_units
                    .contains_key(&id)
            );
        }
        if body.body_kind == ProcedureBodyKind::Event
            && !module.irules_event_bodies.contains_key(&id)
        {
            return None;
        }
        let purposes = [
            (
                ReportDeclarationPurpose::AllocatedSource,
                module.original_declaration_body_units.contains_key(&id),
            ),
            (
                ReportDeclarationPurpose::NativeSource,
                module
                    .original_native_declaration_body_units
                    .contains_key(&id),
            ),
            (
                ReportDeclarationPurpose::LogicalSource,
                module
                    .original_logical_declaration_body_units
                    .contains_key(&id),
            ),
            (
                ReportDeclarationPurpose::AuthoredSource,
                module
                    .original_authored_declaration_body_units
                    .contains_key(&id),
            ),
            (
                ReportDeclarationPurpose::HostedSource,
                module
                    .original_hosted_declaration_body_units
                    .contains_key(&id),
            ),
        ];
        let mut present = purposes
            .into_iter()
            .filter_map(|(purpose, present)| present.then_some(purpose));
        let declaration_purpose = if body.body_kind == ProcedureBodyKind::OriginalDeclaration {
            let purpose = present.next()?;
            if present.next().is_some()
                || !module
                    .body_units
                    .get(&id)
                    .is_some_and(|held| std::ptr::eq(held, body))
                || !module
                    .original_source_procedure_bodies()
                    .any(|(candidate, held)| *candidate == id && std::ptr::eq(held, body))
                || (module
                    .source_metadata_input
                    .as_ref()?
                    .has_hosted_source_name_context()
                    && purpose != ReportDeclarationPurpose::HostedSource)
            {
                return None;
            }
            Some(purpose)
        } else {
            if present.next().is_some() {
                return None;
            }
            None
        };
        Some(ReportBodyOwner {
            id,
            role,
            declaration_purpose,
            declaration: body.span,
            site: body.declaration_site.clone(),
            body_offset: Some(body.body_offset),
            source: body.body.executed_source.clone(),
            namespace: body.body.namespace_context.clone(),
            original_word: None,
            definition_parent: None,
        })
    }

    fn push(
        bodies: &mut Vec<ReportBodyOwner>,
        module: &crate::ir::Module,
        registry: &CommandRegistry,
        id: ProcedureId,
        body: ReportBodyOwner,
    ) -> Option<()> {
        #[cfg(test)]
        if std::env::var_os("TCL_LSP_TRACE_AUTHENTICATED_REPORT").is_some() {
            eprintln!(
                "REPORT_BODY_PUSH id={id:?} owns={} duplicate={} declaration_nonempty={}",
                module.procedure_ids.owns(id),
                bodies.iter().any(|held| held.id == id),
                module
                    .source
                    .bytes()
                    .get(body.declaration.as_range())
                    .is_some_and(|source| !source.is_empty())
            );
        }
        if !module.procedure_ids.owns(id)
            || bodies.iter().any(|held| held.id == id)
            || module
                .source
                .bytes()
                .get(body.declaration.as_range())
                .is_none_or(|source| {
                    source.is_empty()
                        && Self::empty_installed_body(module, registry, id, &body).is_none()
                })
        {
            return None;
        }
        bodies.push(body);
        Some(())
    }

    // An installed body retains its evaluated value extent. Admit an empty
    // extent only through the same current source card used by the Lowerer.
    fn empty_installed_body(
        module: &crate::ir::Module,
        registry: &CommandRegistry,
        id: ProcedureId,
        body: &ReportBodyOwner,
    ) -> Option<()> {
        (matches!(
            body.role,
            ReportBodyRole::Unit(ProcedureBodyKind::Installed)
        ) && body.id == id
            && body.declaration_purpose.is_none()
            && body.declaration.is_empty())
        .then_some(())?;
        let positioned = module
            .retained_source_bindings
            .as_ref()?
            .positioned_for_module(module, registry)?;
        let root = positioned.source_origin()?;
        let allocation = module.installed_procedure_body_units.get(&id)?;
        let held = module.body_units.get(&id)?;
        let source = body.source.as_ref()?;
        let namespace = body.namespace.as_deref()?;
        (module.procedure_ids.owns(id)
            && held.body_kind == ProcedureBodyKind::Installed
            && held.span == body.declaration
            && held.declaration_site == body.site
            && Some(held.body_offset) == body.body_offset
            && held.body_offset == body.declaration.start()
            && held
                .body
                .executed_source
                .as_ref()
                .is_some_and(|actual| Arc::ptr_eq(actual, source))
            && held.body.namespace_context.as_deref() == Some(namespace)
            && body.site.as_ref() == Some(&allocation.site)
            && allocation.matches_source_declaration_image(&module.source, allocation.site.offset)
            && Arc::ptr_eq(&allocation.site.source, root)
            && Arc::ptr_eq(&source.origin, root)
            && source.text.bytes().is_empty()
            && matches!(
                &source.mapping,
                crate::command_binding::ExecutedScriptMapping::Contiguous { base }
                    if *base == body.declaration.start()
            ))
        .then_some(())?;
        positioned
            .installed_procedure_body_units()
            .into_iter()
            .any(|card| {
                card.allocation == *allocation
                    && card.source == **source
                    && Arc::ptr_eq(&card.source.origin, root)
                    && card.namespace_key == *namespace
            })
            .then_some(())
    }

    #[cfg(test)]
    pub(super) fn hosted_domain_for_test(&self) -> bool {
        self.hosted_entry.is_some()
    }

    pub(super) fn same_request(
        &self,
        origin: &crate::compilation_unit::OriginalOptimiserSourceOrigin,
    ) -> bool {
        self.prepared.origin().same_request_allocation(origin)
            && self.prepared.origin().is_current_analysis_context()
            && self.entry == self.prepared.unit().ir_module.source_entry
    }
}

/// A total bijection between actual issued bodies, never a replacement ID or
/// runtime equality. Every role at a shared declaration remains represented.
pub(crate) struct ReportBodyCorrespondence<'a> {
    left: &'a ReportBodyInventory,
    right: &'a ReportBodyInventory,
    pairs: Vec<(ProcedureId, ProcedureId)>,
}

impl<'a> ReportBodyCorrespondence<'a> {
    pub(super) fn capture(
        left: &'a ReportBodyInventory,
        right: &'a ReportBodyInventory,
    ) -> Option<Self> {
        if left.bodies.len() != right.bodies.len()
            || left.root.kind() != right.root.kind()
            || left.entry != right.entry
            || left.hosted_entry != right.hosted_entry
        {
            return None;
        }
        let mut pairs = Vec::new();
        for body in &left.bodies {
            let mut matching = right
                .bodies
                .iter()
                .filter(|candidate| Self::same_source(left, body, right, candidate));
            let matched = matching.next()?;
            if matching.next().is_some() || pairs.iter().any(|(_, held)| *held == matched.id) {
                return None;
            }
            pairs.push((body.id, matched.id));
        }
        if pairs.len() != left.bodies.len() || pairs.len() != right.bodies.len() {
            return None;
        }
        for (a, b) in &pairs {
            let a = left.bodies.iter().find(|body| body.id == *a)?;
            let b = right.bodies.iter().find(|body| body.id == *b)?;
            match (a.definition_parent, b.definition_parent) {
                (None, None) => {}
                (Some(a), Some(b)) if pairs.contains(&(a, b)) => {}
                _ => return None,
            }
        }
        Some(Self { left, right, pairs })
    }

    fn same_source(
        left: &ReportBodyInventory,
        a: &ReportBodyOwner,
        right: &ReportBodyInventory,
        b: &ReportBodyOwner,
    ) -> bool {
        if a.role != b.role
            || a.declaration_purpose != b.declaration_purpose
            || a.declaration != b.declaration
            || a.body_offset != b.body_offset
            || a.namespace != b.namespace
            || a.original_word != b.original_word
        {
            return false;
        }
        match (&a.site, &b.site) {
            (None, None) => {}
            (Some(a), Some(b))
                if a.offset == b.offset
                    && Self::origins(&a.source, &left.root, &b.source, &right.root) => {}
            _ => return false,
        }
        match (&a.source, &b.source) {
            (None, None) => true,
            (Some(a), Some(b)) => {
                a.text == b.text
                    && a.mapping == b.mapping
                    && Self::origins(&a.origin, &left.root, &b.origin, &right.root)
            }
            _ => false,
        }
    }

    fn origins(
        mut a: &Arc<SourceOriginId>,
        left: &Arc<SourceOriginId>,
        mut b: &Arc<SourceOriginId>,
        right: &Arc<SourceOriginId>,
    ) -> bool {
        loop {
            match (Arc::ptr_eq(a, left), Arc::ptr_eq(b, right)) {
                (true, true) => return true,
                (true, false) | (false, true) => return false,
                (false, false) => {}
            }
            match (a.kind(), b.kind()) {
                (
                    SourceOriginKind::Derived {
                        parent: x,
                        arguments: xa,
                        source: xs,
                        kind: xk,
                    },
                    SourceOriginKind::Derived {
                        parent: y,
                        arguments: ya,
                        source: ys,
                        kind: yk,
                    },
                ) if x.offset == y.offset && xa == ya && xs == ys && xk == yk => {
                    a = &x.source;
                    b = &y.source;
                }
                (
                    SourceOriginKind::ConditionalField {
                        parent: x,
                        argument: xa,
                        field: xf,
                        source: xs,
                    },
                    SourceOriginKind::ConditionalField {
                        parent: y,
                        argument: ya,
                        field: yf,
                        source: ys,
                    },
                ) if x.offset == y.offset && xa == ya && xf == yf && xs == ys => {
                    a = &x.source;
                    b = &y.source;
                }
                _ => return false,
            }
        }
    }

    pub(crate) fn contains_definition(&self, id: ProcedureId, left: bool) -> bool {
        let inventory = if left { self.left } else { self.right };
        inventory
            .bodies
            .iter()
            .any(|body| body.id == id && matches!(body.role, ReportBodyRole::Definition(_)))
    }
    pub(crate) fn contains_body(&self, id: ProcedureId, left: bool) -> bool {
        let inventory = if left { self.left } else { self.right };
        inventory.bodies.iter().any(|body| body.id == id)
    }
    pub(crate) fn contains_module(&self, id: ProcedureModuleId, left: bool) -> bool {
        (if left { self.left } else { self.right }).module == id
    }
    pub(crate) fn bodies(&self, left: ProcedureId, right: ProcedureId) -> bool {
        self.pairs.contains(&(left, right))
    }
    pub(crate) fn modules(&self, left: ProcedureModuleId, right: ProcedureModuleId) -> bool {
        self.left.module == left && self.right.module == right
    }
}

#[cfg(test)]
mod owner_controls {
    use super::*;
    use crate::analyser::Analyser;

    fn actual() -> super::super::AuthenticatedAnalysisReport {
        let report = Analyser::new().analyse_for_report(
            "namespace eval ::owned { proc retained {} { return VALUE } }",
            "tcl8.6",
        );
        assert!(report.current_capture().is_ok());
        assert!(!report.current_capture().unwrap().bodies.bodies.is_empty());
        report
    }

    #[test]
    fn empty_installed_body_requires_the_current_complete_source_card() {
        let source = "proc retained {} {}\nretained EXTRA";
        let left = Analyser::new().analyse_for_report(source, "tcl8.6");
        let right = Analyser::new().analyse_for_report(source, "tcl8.6");
        let capture = left
            .current_capture()
            .expect("genuine empty installed body");
        assert_eq!(
            left.compare(&right),
            super::super::AnalysisReportComparison::Equal
        );
        let module = &capture.bodies.prepared.unit().ir_module;
        let registry = left.analysis.resolved_registry().unwrap();
        let (&id, _) = module.installed_procedure_body_units.iter().next().unwrap();
        let make = |module: &crate::ir::Module| {
            ReportBodyInventory::procedure(
                module,
                id,
                module.body_units.get(&id).unwrap(),
                ReportBodyRole::Unit(ProcedureBodyKind::Installed),
            )
            .unwrap()
        };
        let body = make(module);
        assert_eq!(body.declaration, Span::empty(18));
        assert!(body.declaration_purpose.is_none());
        assert!(ReportBodyInventory::empty_installed_body(module, registry, id, &body).is_some());
        let mut missing = module.clone();
        missing.installed_procedure_body_units.remove(&id);
        assert!(ReportBodyInventory::empty_installed_body(&missing, registry, id, &body).is_none());
        let mut changed = module.clone();
        changed.body_units.get_mut(&id).unwrap().body_offset += 1;
        assert!(
            ReportBodyInventory::empty_installed_body(&changed, registry, id, &make(&changed))
                .is_none()
        );
        let mut no_owner = module.clone();
        no_owner.retained_source_bindings = None;
        assert!(
            ReportBodyInventory::empty_installed_body(&no_owner, registry, id, &body).is_none()
        );
        let foreign = &right
            .current_capture()
            .unwrap()
            .bodies
            .prepared
            .unit()
            .ir_module;
        assert!(ReportBodyInventory::empty_installed_body(foreign, registry, id, &body).is_none());
        let mut wrong_role = make(module);
        wrong_role.role = ReportBodyRole::Unit(ProcedureBodyKind::OriginalDeclaration);
        assert!(
            ReportBodyInventory::empty_installed_body(module, registry, id, &wrong_role).is_none()
        );
        let mut wrong_purpose = make(module);
        wrong_purpose.declaration_purpose = Some(ReportDeclarationPurpose::AllocatedSource);
        assert!(
            ReportBodyInventory::empty_installed_body(module, registry, id, &wrong_purpose)
                .is_none()
        );
        let mut wrong_span = make(module);
        wrong_span.declaration = Span::empty(0);
        assert!(
            ReportBodyInventory::empty_installed_body(module, registry, id, &wrong_span).is_none()
        );
        let mut out_of_range = make(module);
        out_of_range.declaration = Span::empty(u32::try_from(source.len() + 1).unwrap());
        assert!(
            ReportBodyInventory::push(&mut Vec::new(), module, registry, id, out_of_range)
                .is_none()
        );
        let mut bodies = Vec::new();
        assert!(
            ReportBodyInventory::push(&mut bodies, module, registry, id, make(module)).is_some()
        );
        assert!(
            ReportBodyInventory::push(&mut bodies, module, registry, id, make(module)).is_none()
        );
        assert_eq!(bodies.len(), 1);
    }

    #[test]
    fn actual_complete_body_inventory_is_total_and_role_sensitive() {
        let left = actual();
        let right = actual();
        let a = &left.current_capture().unwrap().bodies;
        let b = &right.current_capture().unwrap().bodies;
        let joined = ReportBodyCorrespondence::capture(a, b).expect("actual complete owners");
        assert_eq!(joined.pairs.len(), a.bodies.len());
        assert_eq!(joined.pairs.len(), b.bodies.len());
        assert!(joined.modules(a.module, b.module));
        for (a, b) in &joined.pairs {
            assert!(joined.bodies(*a, *b));
        }
    }

    fn assert_alias_body_field_changes_are_unavailable(
        left: &super::super::AuthenticatedAnalysisReport,
        mut right: super::super::AuthenticatedAnalysisReport,
        purpose: ReportDeclarationPurpose,
    ) {
        let index = right
            .capture
            .as_ref()
            .unwrap()
            .bodies
            .bodies
            .iter()
            .position(|body| body.declaration_purpose == Some(purpose))
            .expect("the genuine original declaration purpose remains present");
        let original_role = right.capture.as_ref().unwrap().bodies.bodies[index].role;
        right.capture.as_mut().unwrap().bodies.bodies[index].role =
            ReportBodyRole::Named(ProcedureBodyKind::Named);
        assert!(matches!(
            left.compare(&right),
            super::super::AnalysisReportComparison::Unavailable(_)
        ));
        right.capture.as_mut().unwrap().bodies.bodies[index].role = original_role;
        let changed_purpose = match purpose {
            ReportDeclarationPurpose::NativeSource => ReportDeclarationPurpose::LogicalSource,
            ReportDeclarationPurpose::LogicalSource => ReportDeclarationPurpose::NativeSource,
            _ => panic!("this control requires its actual Native or Logical source purpose"),
        };
        right.capture.as_mut().unwrap().bodies.bodies[index].declaration_purpose =
            Some(changed_purpose);
        assert!(matches!(
            left.compare(&right),
            super::super::AnalysisReportComparison::Unavailable(_)
        ));
        right.capture.as_mut().unwrap().bodies.bodies[index].declaration_purpose = Some(purpose);
        let original_extent = right.capture.as_ref().unwrap().bodies.bodies[index].declaration;
        assert!(original_extent.start() < original_extent.end());
        right.capture.as_mut().unwrap().bodies.bodies[index].declaration =
            Span::empty(original_extent.start());
        assert!(matches!(
            left.compare(&right),
            super::super::AnalysisReportComparison::Unavailable(_)
        ));
        right.capture.as_mut().unwrap().bodies.bodies[index].declaration = original_extent;
        assert_eq!(
            left.compare(&right),
            super::super::AnalysisReportComparison::Equal
        );
    }

    #[test]
    fn original_aliases_keep_both_actual_roles_at_the_shared_declaration() {
        let source = include_str!("../../../tests/data/name-consumers/aliases.tcl");
        let left = Analyser::new().analyse_for_report(source, "tcl8.6");
        let right = Analyser::new().analyse_for_report(source, "tcl8.6");
        let a = &left
            .current_capture()
            .expect("actual original alias diagnostics CU")
            .bodies;
        let b = &right
            .current_capture()
            .expect("independent original alias diagnostics CU")
            .bodies;
        let declaration = a
            .bodies
            .iter()
            .find(|body| {
                matches!(
                    body.role,
                    ReportBodyRole::Unit(ProcedureBodyKind::OriginalDeclaration)
                ) && body.declaration_purpose == Some(ReportDeclarationPurpose::NativeSource)
                    && a.bodies.iter().any(|other| {
                        matches!(
                            other.role,
                            ReportBodyRole::Unit(ProcedureBodyKind::Installed)
                        ) && other.declaration_purpose.is_none()
                            && body.site.is_some()
                            && body.site == other.site
                    })
            })
            .expect("actual NativeSource declaration and Installed owners at one original site");
        assert_eq!(a.bodies.len(), 3);
        assert_eq!(b.bodies.len(), 3);
        assert_eq!(declaration.declaration, Span::new(227, 256));
        let other = a
            .bodies
            .iter()
            .find(|body| {
                body.site == declaration.site
                    && matches!(
                        body.role,
                        ReportBodyRole::Unit(ProcedureBodyKind::Installed)
                    )
            })
            .unwrap();
        assert_eq!(other.declaration, Span::new(241, 256));
        assert_ne!(declaration.declaration, other.declaration);
        let joined = a
            .bodies
            .iter()
            .filter(|body| body.site == declaration.site)
            .collect::<Vec<_>>();
        assert_eq!(
            joined.len(),
            2,
            "both original actual roles remain present, never deduplicated"
        );
        assert_ne!(joined[0].id, joined[1].id);
        let correspondence = ReportBodyCorrespondence::capture(a, b)
            .expect("complete unique actual owner correspondence");
        for row in joined {
            assert_eq!(
                correspondence
                    .pairs
                    .iter()
                    .filter(|(left, _)| *left == row.id)
                    .count(),
                1
            );
        }
        assert_eq!(
            left.compare(&right),
            super::super::AnalysisReportComparison::Equal
        );
        assert_alias_body_field_changes_are_unavailable(
            &left,
            right,
            ReportDeclarationPurpose::NativeSource,
        );
    }

    #[test]
    fn supported_logical_alias_roles_keep_the_shared_declaration() {
        // Explicit Logical source analysis retains its own source-only roles.
        let source = "namespace eval ::owned {proc retained {} {return VALUE}}\ninterp alias {} ::pick {} ::owned::retained\npick";
        let left = Analyser::new().analyse_for_report(source, "tcl");
        let right = Analyser::new().analyse_for_report(source, "tcl");
        assert!(
            left.current_capture()
                .unwrap()
                .input
                .has_logical_source_name_context()
        );
        assert!(
            right
                .current_capture()
                .unwrap()
                .input
                .has_logical_source_name_context()
        );
        let a = &left
            .current_capture()
            .expect("actual original alias diagnostics CU")
            .bodies;
        let b = &right
            .current_capture()
            .expect("independent original alias diagnostics CU")
            .bodies;
        let declaration = a
            .bodies
            .iter()
            .find(|body| {
                matches!(
                    body.role,
                    ReportBodyRole::Unit(ProcedureBodyKind::OriginalDeclaration)
                ) && body.declaration_purpose == Some(ReportDeclarationPurpose::LogicalSource)
                    && a.bodies.iter().any(|other| {
                        matches!(
                            other.role,
                            ReportBodyRole::Unit(ProcedureBodyKind::NamespaceScript)
                        ) && other.declaration_purpose.is_none()
                            && body.site.is_some()
                            && body.site == other.site
                    })
            })
            .expect(
                "actual LogicalSource declaration and NamespaceScript owners at one original site",
            );
        assert_eq!(a.bodies.len(), 2);
        assert_eq!(b.bodies.len(), 2);
        assert_eq!(declaration.declaration, Span::new(42, 54));
        let other = a
            .bodies
            .iter()
            .find(|body| {
                body.site == declaration.site
                    && matches!(
                        body.role,
                        ReportBodyRole::Unit(ProcedureBodyKind::NamespaceScript)
                    )
            })
            .unwrap();
        assert_eq!(other.declaration, Span::new(24, 55));
        assert_ne!(declaration.declaration, other.declaration);
        let joined = a
            .bodies
            .iter()
            .filter(|body| body.site == declaration.site)
            .collect::<Vec<_>>();
        assert_eq!(
            joined.len(),
            2,
            "both original actual roles remain present, never deduplicated"
        );
        assert_ne!(joined[0].id, joined[1].id);
        let correspondence = ReportBodyCorrespondence::capture(a, b)
            .expect("complete unique actual owner correspondence");
        for row in joined {
            assert_eq!(
                correspondence
                    .pairs
                    .iter()
                    .filter(|(left, _)| *left == row.id)
                    .count(),
                1
            );
        }
        assert_eq!(
            left.compare(&right),
            super::super::AnalysisReportComparison::Equal
        );
        assert_alias_body_field_changes_are_unavailable(
            &left,
            right,
            ReportDeclarationPurpose::LogicalSource,
        );
    }

    #[test]
    fn actual_body_census_retains_the_complete_source_entry_domain() {
        let left = actual();
        let mut right = actual();
        let capture = right.capture.as_mut().unwrap();
        capture.bodies.entry.unknown_entry = !capture.bodies.entry.unknown_entry;
        assert!(!capture.bodies.same_request(&capture.origin));
        assert!(
            ReportBodyCorrespondence::capture(
                &left.current_capture().unwrap().bodies,
                &capture.bodies,
            )
            .is_none()
        );
    }

    #[test]
    fn omitted_or_duplicate_actual_body_role_is_unavailable() {
        let left = actual();
        let mut right = actual();
        right.capture.as_mut().unwrap().bodies.bodies.pop().unwrap();
        assert!(
            ReportBodyCorrespondence::capture(
                &left.current_capture().unwrap().bodies,
                &right.capture.as_ref().unwrap().bodies,
            )
            .is_none()
        );
        let mut right = actual();
        let bodies = &mut right.capture.as_mut().unwrap().bodies.bodies;
        assert!(
            bodies.len() >= 2,
            "the genuine namespace and procedure roles are retained"
        );
        bodies[1].role = bodies[0].role;
        bodies[1].declaration = bodies[0].declaration;
        bodies[1].body_offset = bodies[0].body_offset;
        bodies[1].site = bodies[0].site.clone();
        bodies[1].source = bodies[0].source.clone();
        bodies[1].namespace = bodies[0].namespace.clone();
        bodies[1].original_word = bodies[0].original_word.clone();
        assert!(
            ReportBodyCorrespondence::capture(
                &left.current_capture().unwrap().bodies,
                &right.capture.as_ref().unwrap().bodies,
            )
            .is_none()
        );
    }
}
