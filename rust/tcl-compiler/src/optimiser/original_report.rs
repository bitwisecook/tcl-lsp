// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Complete original optimiser collections, distinct from editable report vectors.

use super::Optimisation;
use crate::body_analysis_capsule::BodyAnalysisCapsule;
use crate::compilation_unit::OriginalOptimiserPreparedUnit;
use crate::compilation_unit::{CompilationUnit, OriginalOptimiserSourceOrigin};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tcl_registry::CommandRegistry;

type Dialect = Option<&'static tcl_dialect::DialectProfile>;

/// An entire ordered raw worker result. The reporting payload remains available
/// when the producer cannot issue an original-source collection receipt.
#[derive(Debug, Clone)]
pub struct OriginalRawOptimiserReport {
    members: Vec<Optimisation>,
    owner: Option<OriginalOptimiserPreparedUnit>,
}
impl OriginalRawOptimiserReport {
    pub(super) fn from_worker(
        members: Vec<Optimisation>,
        owner: Option<OriginalOptimiserPreparedUnit>,
    ) -> Self {
        Self { members, owner }
    }
    /// Actual report output, without a permission to reconstruct or slice a batch.
    #[must_use]
    pub fn members(&self) -> &[Optimisation] {
        &self.members
    }
    pub(super) fn current_owner(
        &self,
        cu: &CompilationUnit,
        registry: &CommandRegistry,
        dialect: Dialect,
    ) -> Option<&OriginalOptimiserPreparedUnit> {
        self.owner
            .as_ref()
            .filter(|owner| owner.matches_worker(cu, registry, dialect))
    }
}
impl PartialEq for OriginalRawOptimiserReport {
    fn eq(&self, other: &Self) -> bool {
        self.members == other.members
            && match (&self.owner, &other.owner) {
                (Some(left), Some(right)) => {
                    left.unit() == right.unit() && left.origin() == right.origin()
                }
                (None, None) => true,
                _ => false,
            }
    }
}

/// One complete body-local projection, retaining the actual capsule dependency
/// and effective worker dialect separately from its relative reporting members.
#[derive(Debug, Clone)]
pub struct OriginalBodyOptimiserReport {
    members: Vec<Optimisation>,
    capsule: BodyAnalysisCapsule,
    dialect: Dialect,
    issued: bool,
    analysis_context: Option<crate::value_transfer::AnalysisContextKey>,
}
impl OriginalBodyOptimiserReport {
    pub(crate) fn from_body_worker(
        capsule: &BodyAnalysisCapsule,
        raw: OriginalRawOptimiserReport,
        registry: &CommandRegistry,
        dialect: Dialect,
    ) -> Self {
        let analysis_context = raw
            .current_owner(capsule.unit(), registry, dialect)
            .map(|owner| owner.origin().analysis_context().clone());
        let issued = analysis_context.is_some();
        let span = capsule.procedure().span;
        let outside: HashSet<_> = raw
            .members
            .iter()
            .filter(|edit| edit.span.start() < span.start() || span.end() < edit.span.end())
            .filter_map(|edit| edit.group)
            .collect();
        let members = raw
            .members
            .into_iter()
            .filter(|edit| {
                span.start() <= edit.span.start()
                    && edit.span.end() <= span.end()
                    && edit.group.is_none_or(|group| !outside.contains(&group))
            })
            .map(|mut edit| {
                edit.span = tcl_lexer::Span::new(
                    edit.span.start() - span.start(),
                    edit.span.end() - span.start(),
                );
                edit.issued_rewrite = None;
                edit
            })
            .collect();
        Self {
            members,
            capsule: capsule.clone(),
            dialect,
            issued,
            analysis_context,
        }
    }
    /// Complete relative report output. Empty is distinct from unavailable issuance.
    #[must_use]
    pub fn members(&self) -> &[Optimisation] {
        &self.members
    }
}
impl PartialEq for OriginalBodyOptimiserReport {
    fn eq(&self, other: &Self) -> bool {
        // The existing capsule's neutral correspondence, not a full old root
        // source image or allocation ID, owns permissible body memo reuse.
        self.members == other.members
            && self.capsule == other.capsule
            && self.dialect == other.dialect
            && self.issued == other.issued
            && self.analysis_context == other.analysis_context
    }
}

/// Whole body-partition assembly. The current root owns the required roster;
/// callers cannot omit an entire raw batch, an ungrouped edit or an empty body
/// and still obtain final issuance. Reports remain available on refusal.
#[derive(Debug)]
pub struct OriginalOptimiserAssembly {
    unit: CompilationUnit,
    registry: tcl_registry::RegistrySnapshot,
    dialect: Dialect,
    expected: HashMap<crate::ir::ProcedureId, BodyAnalysisCapsule>,
    seen: HashSet<crate::ir::ProcedureId>,
    owner: Option<OriginalOptimiserPreparedUnit>,
    raw: Vec<Optimisation>,
    group_base: u32,
    top_seen: bool,
    complete: bool,
}
impl OriginalOptimiserAssembly {
    /// Retain the actual current whole unit and every independently captured
    /// source body capsule before consuming memo reports. No body-name inverse.
    #[must_use]
    pub fn for_body_partition(
        cu: &CompilationUnit,
        registry: &CommandRegistry,
        dialect: Dialect,
        capsules: &[BodyAnalysisCapsule],
    ) -> Self {
        let owner = cu
            .original_optimiser_prepared_unit(registry, dialect)
            .filter(|owner| owner.is_whole_source_unit());
        let mut expected = HashMap::new();
        let mut complete = cu.methods.is_empty();
        let required: HashSet<_> = cu
            .all_body_function_units()
            .filter_map(|fu| fu.body_id)
            .collect();
        for capsule in capsules {
            let id = capsule.body_id();
            complete &= required.contains(&id)
                && capsule.registry_snapshot() == &registry.snapshot()
                && capsule.unit() == cu
                && capsule.original_optimiser_analysis_context(registry, dialect)
                    == owner
                        .as_ref()
                        .map(|owner| owner.origin().analysis_context().clone())
                && cu.original_procedure_for_body(id).is_some()
                && expected.insert(id, capsule.clone()).is_none();
        }
        complete &= expected.len() == required.len()
            && cu
                .procedures
                .keys()
                .chain(cu.body_units.keys())
                .all(|id| required.contains(id));
        Self {
            unit: cu.clone(),
            registry: registry.snapshot(),
            dialect,
            expected,
            seen: HashSet::new(),
            owner,
            raw: Vec::new(),
            group_base: 0,
            top_seen: false,
            complete,
        }
    }
    /// Derive the existing top-level-only pass view from this retained root.
    /// An unavailable source witness leaves an ordinary report-only view.
    #[must_use]
    pub fn top_level_unit(&self) -> CompilationUnit {
        if let Some(owner) = &self.owner {
            return owner.top_level_view();
        }
        let mut unit = self.unit.clone();
        unit.procedures.clear();
        unit.methods.clear();
        unit.body_units.clear();
        unit.cfg_module.procedures.clear();
        unit
    }
    /// Append the whole current body's memo output, using the capsule's existing
    /// certified neutral correspondence to rebind an equivalent previous result.
    pub fn append_body(
        &mut self,
        current: &BodyAnalysisCapsule,
        report: &OriginalBodyOptimiserReport,
    ) {
        let id = current.body_id();
        self.complete &= self
            .expected
            .get(&id)
            .is_some_and(|expected| expected == current)
            && self.seen.insert(id)
            && report.capsule == *current
            && report.dialect == self.dialect
            && report.issued
            && report.analysis_context.is_some()
            && report.analysis_context
                == self
                    .owner
                    .as_ref()
                    .map(|owner| owner.origin().analysis_context().clone());
        let offset = current.procedure().span.start();
        self.append_members(report.members(), offset);
    }
    /// Append the entire genuine top-level raw collection. A public manually
    /// selected vector cannot be supplied to this operation.
    pub fn append_top_level(&mut self, report: &OriginalRawOptimiserReport) {
        self.complete &= !self.top_seen
            && self.owner.as_ref().is_some_and(|root| {
                report
                    .owner
                    .as_ref()
                    .is_some_and(|top| top.is_top_level_projection_of(root))
            });
        self.top_seen = true;
        self.append_members(report.members(), 0);
    }
    fn append_members(&mut self, members: &[Optimisation], offset: u32) {
        let max_group = members.iter().filter_map(|member| member.group).max();
        for member in members {
            let mut member = member.clone();
            let start = member.span.start().checked_add(offset);
            let end = member.span.end().checked_add(offset);
            let group = member.group.map(|group| self.group_base.checked_add(group));
            if let (Some(start), Some(end)) = (start, end) {
                member.span = tcl_lexer::Span::new(start, end);
            } else {
                self.complete = false;
            }
            if let Some(group) = group {
                member.group = group;
                self.complete &= group.is_some();
            }
            member.issued_rewrite = None;
            self.raw.push(member);
        }
        if let Some(max) = max_group {
            if let Some(base) = max
                .checked_add(1)
                .and_then(|delta| self.group_base.checked_add(delta))
            {
                self.group_base = base;
            } else {
                self.complete = false;
            }
        }
    }
    /// Run the unchanged whole-module tail exactly once. Final issuance requires
    /// every required body (including empty batches) and the original top view.
    #[must_use]
    pub fn finish_report(self) -> Vec<Optimisation> {
        let registry = self.registry.shared_registry();
        let mut finalised =
            super::finalise_optimisations(&self.raw, &self.unit, &registry, self.dialect);
        if self.complete
            && self.top_seen
            && self.seen.len() == self.expected.len()
            && let Some(owner) = &self.owner
        {
            issue_final(&mut finalised, owner, &self.unit, &registry, self.dialect);
        }
        finalised
    }
}

/// Original final worker inputs and exact final rewrite members. This receipt
/// comes only from a complete sealed raw collection or complete body assembly.
#[derive(Debug, Clone, PartialEq)]
pub struct OriginalOptimiserRewriteBatch {
    image: tcl_lexer::SourceImage,
    input: crate::analyser::ResolvedAnalysisInput,
    registry: tcl_registry::RegistrySnapshot,
    members: Vec<OptimisationPayload>,
    realm: crate::realm::CommandBindingRealm,
    origin: OriginalOptimiserSourceOrigin,
    dialect: Dialect,
}
#[derive(Debug, Clone, PartialEq, Eq)]
struct OptimisationPayload {
    code: tcl_core_types::DiagCode,
    message: String,
    span: tcl_lexer::Span,
    replacement: String,
    group: Option<u32>,
    hint_only: bool,
}
impl From<&Optimisation> for OptimisationPayload {
    fn from(edit: &Optimisation) -> Self {
        Self {
            code: edit.code,
            message: edit.message.clone(),
            span: edit.span,
            replacement: edit.replacement.clone(),
            group: edit.group,
            hint_only: edit.hint_only,
        }
    }
}
impl OriginalOptimiserRewriteBatch {
    /// Exact independently current source interpretation, not a dispatch grant.
    #[must_use]
    pub fn matches_source_realm(&self, realm: &crate::realm::CommandBindingRealm) -> bool {
        &self.realm == realm
    }
    /// Original private source-pipeline inputs, independently rejoined by the adopter.
    #[must_use]
    pub fn source_origin(&self) -> &OriginalOptimiserSourceOrigin {
        &self.origin
    }
    /// Actual whole document image, including its source channel.
    #[must_use]
    pub fn image(&self) -> &tcl_lexer::SourceImage {
        &self.image
    }
    /// Complete original selected input.
    #[must_use]
    pub fn analysis_input(&self) -> &crate::analyser::ResolvedAnalysisInput {
        &self.input
    }
    /// Actual immutable worker Registry.
    #[must_use]
    pub fn registry(&self) -> &tcl_registry::RegistrySnapshot {
        &self.registry
    }
}
pub(super) fn issue_final(
    selected: &mut [Optimisation],
    owner: &OriginalOptimiserPreparedUnit,
    cu: &CompilationUnit,
    registry: &CommandRegistry,
    dialect: Dialect,
) {
    if !owner.matches_worker(cu, registry, dialect)
        || selected
            .iter()
            .any(|edit| cu.source.get(edit.span.as_range()).is_none())
    {
        return;
    }
    let Some(input) = cu.ir_module.source_metadata_input.as_ref() else {
        return;
    };
    let Some(realm) = crate::realm::document_realm_bindings_for_module(&cu.ir_module, registry)
    else {
        return;
    };
    let batch = Arc::new(OriginalOptimiserRewriteBatch {
        image: cu.ir_module.source.clone(),
        input: input.clone(),
        registry: registry.snapshot(),
        members: selected.iter().map(OptimisationPayload::from).collect(),
        realm,
        origin: owner.origin().clone(),
        dialect,
    });
    for member in selected {
        member.issued_rewrite = Some(Arc::clone(&batch));
    }
}
impl Optimisation {
    /// Original final worker receipt; manual and raw reports have none. A public
    /// payload mutation cannot borrow the earlier member's permission.
    #[must_use]
    pub fn original_rewrite_batch(&self) -> Option<&OriginalOptimiserRewriteBatch> {
        let batch = self.issued_rewrite.as_ref()?;
        batch
            .members
            .contains(&OptimisationPayload::from(self))
            .then_some(batch.as_ref())
    }
    /// Compare retained final issuer currency independently of ordinary report
    /// payload equality. Tracked aggregate containers use this to avoid keeping
    /// an old receipt merely because the visible diagnostic text stayed equal.
    #[must_use]
    pub fn same_original_rewrite_provenance(&self, other: &Self) -> bool {
        self.original_rewrite_batch() == other.original_rewrite_batch()
    }
    /// An exact entire actionable group from the same final collection. Missing,
    /// duplicated, changed and foreign members remain unavailable.
    #[must_use]
    pub fn is_original_rewrite_group(members: &[&Self]) -> bool {
        let Some(first) = members.first() else {
            return false;
        };
        let Some(batch) = first.issued_rewrite.as_ref() else {
            return false;
        };
        if members.iter().any(|member| {
            member.hint_only
                || member.group != first.group
                || member.original_rewrite_batch().is_none()
                || !member
                    .issued_rewrite
                    .as_ref()
                    .is_some_and(|other| Arc::ptr_eq(batch, other))
        }) {
            return false;
        }
        let actual: Vec<_> = members
            .iter()
            .map(|member| OptimisationPayload::from(*member))
            .collect();
        let expected: Vec<_> = match first.group {
            Some(group) => batch
                .members
                .iter()
                .filter(|member| member.group == Some(group))
                .cloned()
                .collect(),
            None => vec![OptimisationPayload::from(*first)],
        };
        actual == expected
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyser::Analyser;
    use crate::compilation_unit::UnitBuildOptions;
    fn original(
        source: &str,
    ) -> (
        CompilationUnit,
        Arc<CommandRegistry>,
        &'static tcl_dialect::DialectProfile,
    ) {
        let mut analyser = Analyser::new();
        let input = analyser.prepare_analysis_input("tcl8.6").unwrap();
        let registry = input
            .borrowed_context_registry()
            .commands()
            .snapshot()
            .shared_registry();
        let profile = input.unit_profile();
        let unit = CompilationUnit::build_standalone_with_analysis_input(
            source,
            UnitBuildOptions {
                registry: &registry,
                defer_top_level: false,
                config: input.lexer_config(),
                dialect: Some(profile),
                external_call_sites: None,
                declared_commands: None,
            },
            &input,
        )
        .unwrap()
        .with_interprocedural(&registry, Some(profile));
        assert!(
            unit.original_optimiser_prepared_unit(&registry, Some(profile))
                .is_some()
        );
        (unit, registry, profile)
    }
    #[test]
    fn complete_raw_worker_refuses_changed_analysis_dialect_and_vector_reconstruction() {
        let source = "proc p {y} {\n set x [llength $y]\n puts $x\n}\n";
        let (unit, registry, profile) = original(source);
        let raw = super::super::optimise_unit_raw_report(&unit, &registry, Some(profile));
        assert!(raw.owner.is_some());
        assert!(!raw.members().is_empty());
        let finalised =
            super::super::finalise_original_optimiser_report(&raw, &unit, &registry, Some(profile));
        assert!(!finalised.is_empty());
        assert!(
            finalised
                .iter()
                .all(|member| member.original_rewrite_batch().is_some())
        );
        let mut changed = unit.clone();
        changed.top_level.sccp.reads_module = !changed.top_level.sccp.reads_module;
        assert!(
            super::super::optimise_unit_raw_report(&changed, &registry, Some(profile))
                .owner
                .is_none()
        );
        assert!(
            super::super::finalise_original_optimiser_report(
                &raw,
                &changed,
                &registry,
                Some(profile)
            )
            .iter()
            .all(|member| member.original_rewrite_batch().is_none())
        );
        assert!(
            super::super::optimise_unit_raw_report(&unit, &registry, None)
                .owner
                .is_none()
        );
        let foreign = tcl_dialect::DialectProfile::find("tcl9.0").unwrap();
        assert!(
            super::super::optimise_unit_raw_report(&unit, &registry, Some(foreign))
                .owner
                .is_none()
        );
        assert!(
            super::super::finalise_optimisations(
                &raw.members()[..1],
                &unit,
                &registry,
                Some(profile)
            )
            .iter()
            .all(|member| member.original_rewrite_batch().is_none())
        );
        let top = unit
            .original_optimiser_top_level_view(&registry, Some(profile))
            .unwrap();
        let top_raw = super::super::optimise_unit_raw_report(&top, &registry, Some(profile));
        assert!(top_raw.owner.is_some());
        assert!(!top_raw.owner.as_ref().unwrap().is_whole_source_unit());
        let enriched_top = top.clone().with_interprocedural(&registry, Some(profile));
        if let Some(owner) = enriched_top.original_optimiser_prepared_unit(&registry, Some(profile))
        {
            assert!(!owner.is_whole_source_unit());
        }

        assert!(
            super::super::finalise_original_optimiser_report(
                &top_raw,
                &top,
                &registry,
                Some(profile)
            )
            .iter()
            .all(|member| member.original_rewrite_batch().is_none())
        );
    }
    #[test]
    fn complete_body_roster_includes_empty_batches_and_top_level_selection() {
        let source = "proc p {y} {\n set x [llength $y]\n puts $x\n}\nproc quiet {} {}\n";
        let (unit, registry, profile) = original(source);
        let retained = Arc::new(unit.clone());
        let capsules: Vec<_> = unit
            .all_body_function_units()
            .filter_map(|fu| fu.body_id)
            .map(|id| BodyAnalysisCapsule::retain(Arc::clone(&retained), id, &registry).unwrap())
            .collect();
        assert_eq!(capsules.len(), 2);
        let reports: Vec<_> = capsules
            .iter()
            .map(|capsule| capsule.original_body_optimiser_report(&registry, Some(profile)))
            .collect();
        assert!(reports.iter().all(|report| report.issued));
        assert!(reports.iter().any(|report| report.members().is_empty()));
        let mut complete = OriginalOptimiserAssembly::for_body_partition(
            &unit,
            &registry,
            Some(profile),
            &capsules,
        );
        let top = complete.top_level_unit();
        let top_report = super::super::optimise_unit_raw_report(&top, &registry, Some(profile));
        for (capsule, report) in capsules.iter().zip(&reports) {
            complete.append_body(capsule, report);
        }
        complete.append_top_level(&top_report);
        let finalised = complete.finish_report();
        assert!(!finalised.is_empty());
        assert!(
            finalised
                .iter()
                .all(|member| member.original_rewrite_batch().is_some())
        );
        let mut omitted = OriginalOptimiserAssembly::for_body_partition(
            &unit,
            &registry,
            Some(profile),
            &capsules,
        );
        let top = omitted.top_level_unit();
        omitted.append_top_level(&super::super::optimise_unit_raw_report(
            &top,
            &registry,
            Some(profile),
        ));
        for (capsule, report) in capsules
            .iter()
            .zip(&reports)
            .filter(|(_, report)| !report.members().is_empty())
        {
            omitted.append_body(capsule, report);
        }
        let incomplete = omitted.finish_report();
        assert!(!incomplete.is_empty());
        assert!(
            incomplete
                .iter()
                .all(|member| member.original_rewrite_batch().is_none())
        );
    }
    #[test]
    fn uncached_source_reborrow_preserves_actual_standalone_ingress() {
        let source = "proc p {x} {set y [llength $x]; puts $y}\n";
        let (unit, registry, profile) = original(source);
        let origin = unit.original_optimiser_source_origin().unwrap();
        let input = unit.ir_module.source_metadata_input.as_ref().unwrap();
        let rebuilt = origin
            .rebuild_for_current_analysis(source, &registry, input, None)
            .unwrap();
        assert_eq!(rebuilt.original_optimiser_source_origin(), Some(origin));
        assert!(
            rebuilt
                .original_optimiser_prepared_unit(&registry, Some(profile))
                .is_some()
        );
        let changed = format!("{source}set changed 1\n");
        assert!(
            origin
                .rebuild_for_current_analysis(&changed, &registry, input, None)
                .is_none()
        );
        let supplied = CompilationUnit::build_with_analysis_input(
            source,
            UnitBuildOptions {
                registry: &registry,
                defer_top_level: false,
                config: input.lexer_config(),
                dialect: Some(profile),
                external_call_sites: None,
                declared_commands: None,
            },
            Some(&unit.ir_module.source_entry),
            input,
        );
        assert_ne!(supplied.original_optimiser_source_origin(), Some(origin));
    }
    #[test]
    fn neutral_body_reports_cannot_rebind_across_actual_evaluator_hosts() {
        std::thread::spawn(|| {
            use tcl_registry::pack_hooks::{
                HookAnswer, HookCall, HookSlot, PackHookHost, clear_host, install_host,
            };
            struct AbstainHost;
            impl PackHookHost for AbstainHost {
                fn invoke(&self, _: HookSlot, _: &HookCall<'_>) -> HookAnswer {
                    HookAnswer::Abstain
                }
            }
            struct ClearHost;
            impl Drop for ClearHost {
                fn drop(&mut self) {
                    clear_host();
                }
            }
            let _clear = ClearHost;
            install_host(std::rc::Rc::new(AbstainHost));
            let mut analyser = Analyser::new();
            let input = analyser.prepare_analysis_input("tcl8.6").unwrap();
            let registry = input.borrowed_context_registry().commands();
            let dialect = Some(input.unit_profile());
            let build = || {
                let unit = Arc::new(
                    CompilationUnit::build_standalone_with_analysis_input(
                        "proc p {x} {set y [llength $x]; puts $y}\n",
                        UnitBuildOptions {
                            registry,
                            defer_top_level: false,
                            config: input.lexer_config(),
                            dialect,
                            external_call_sites: None,
                            declared_commands: None,
                        },
                        &input,
                    )
                    .unwrap()
                    .with_interprocedural(registry, dialect),
                );
                let body = unit
                    .all_body_function_units()
                    .find_map(|fu| fu.body_id)
                    .unwrap();
                let capsule =
                    BodyAnalysisCapsule::retain(Arc::clone(&unit), body, registry).unwrap();
                (unit, capsule)
            };
            let (_, first) = build();
            let old_body = first.original_body_optimiser_report(registry, dialect);
            let old_taint = first.original_interproc_taint_report(registry, dialect);
            assert!(old_body.issued);
            assert!(old_taint.matches_current(&first, registry, dialect));
            install_host(std::rc::Rc::new(AbstainHost));
            let (unit, current) = build();
            assert!(first == current);
            let fresh_body = current.original_body_optimiser_report(registry, dialect);
            let fresh_taint = current.original_interproc_taint_report(registry, dialect);
            assert!(fresh_body.issued);
            assert!(fresh_taint.matches_current(&current, registry, dialect));
            assert!(!old_taint.matches_current(&current, registry, dialect));
            assert_ne!(old_body.analysis_context, fresh_body.analysis_context);
            assert_eq!(old_body.members(), fresh_body.members());
            let finish = |report: &OriginalBodyOptimiserReport| {
                let mut assembly = OriginalOptimiserAssembly::for_body_partition(
                    &unit,
                    registry,
                    dialect,
                    std::slice::from_ref(&current),
                );
                assembly.append_body(&current, report);
                let top = assembly.top_level_unit();
                let top_report = super::super::optimise_unit_raw_report(&top, registry, dialect);
                assembly.append_top_level(&top_report);
                assembly.finish_report()
            };
            let fresh = finish(&fresh_body);
            assert!(!fresh.is_empty());
            assert!(
                fresh
                    .iter()
                    .all(|edit| edit.original_rewrite_batch().is_some())
            );
            let stale = finish(&old_body);
            assert_eq!(stale, fresh);
            assert!(
                stale
                    .iter()
                    .all(|edit| edit.original_rewrite_batch().is_none())
            );
        })
        .join()
        .unwrap();
    }
}
