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

//! Report correspondence keeps independent body issuers out of runtime equality.

use super::{ObjectFactOwner, ObjectHandleFacts, ObjectScopeReceiver, OwnerSpan};
use crate::analyser::{AnalysisResult, ResolvedAnalysisInput};
use crate::compilation_unit::CompilationUnit;
use crate::ir::{MethodKind, ProcedureBodyKind, ProcedureId};
use std::collections::{HashMap, HashSet};
use tcl_core_types::{BodyFrameId, Knowledge, ProcedureModuleId};
use tcl_lexer::{SourceImage, Span};
use tcl_registry::{CommandRegistry, RegistrySemanticKey};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum BodyKind {
    Procedure(ProcedureBodyKind),
    Method(MethodKind),
    Definition(u8),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct BodyCoordinate {
    kind: BodyKind,
    declaration: Option<u32>,
    span: Span,
    body_offset: Option<u32>,
    original: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct ObjectReportSource {
    image: SourceImage,
    input: ResolvedAnalysisInput,
    registry: RegistrySemanticKey,
    module: ProcedureModuleId,
    bodies: HashMap<ProcedureId, BodyCoordinate>,
    scopes: Vec<OwnerSpan>,
    classes: HashMap<(ObjectFactOwner, String), HashSet<String>>,
    returns: HashMap<ProcedureId, String>,
}

#[derive(Debug, PartialEq, Eq, Hash)]
enum ReportFrame<'a> {
    Top,
    Body(&'a BodyCoordinate),
}

#[derive(Debug, PartialEq, Eq, Hash)]
enum ReportOwner<'a> {
    Frame(ReportFrame<'a>),
    Definition {
        body: &'a BodyCoordinate,
        receiver: ObjectScopeReceiver,
    },
    /// A definition script of this report's own module with no retained
    /// source coordinate, compared by its deterministic ordinal.
    UnretainedDefinition {
        ordinal: u32,
        receiver: ObjectScopeReceiver,
    },
    Original(
        &'a tcl_names::command::CommandAllocation,
        ObjectScopeReceiver,
    ),
}

#[derive(PartialEq, Eq)]
struct ReportCandidates<'a> {
    instance_variables: HashMap<ReportOwner<'a>, &'a HashSet<String>>,
    frames: HashSet<ReportFrame<'a>>,
    original_classes: HashSet<ReportOwner<'a>>,
    definition_classes: HashSet<ReportOwner<'a>>,
}

type ReportedScope<'a> = (
    Span,
    &'a str,
    ReportFrame<'a>,
    Option<ReportOwner<'a>>,
    Option<&'a str>,
);

impl ObjectReportSource {
    fn frame(&self, frame: BodyFrameId) -> Option<ReportFrame<'_>> {
        match frame {
            BodyFrameId::Top(module) => (module == self.module).then_some(ReportFrame::Top),
            BodyFrameId::Body(body) => self.bodies.get(&body).map(ReportFrame::Body),
        }
    }

    fn owner<'a>(&'a self, owner: &'a ObjectFactOwner) -> Option<ReportOwner<'a>> {
        match owner {
            ObjectFactOwner::Frame(frame) => self.frame(*frame).map(ReportOwner::Frame),
            ObjectFactOwner::DefinitionScript {
                definition,
                receiver,
            } => Some(match self.bodies.get(definition) {
                Some(body) => ReportOwner::Definition {
                    body,
                    receiver: *receiver,
                },
                None if definition.module() == self.module => ReportOwner::UnretainedDefinition {
                    ordinal: definition.ordinal(),
                    receiver: *receiver,
                },
                None => return None,
            }),
            ObjectFactOwner::OriginalClass {
                allocation,
                receiver,
            } => Some(ReportOwner::Original(allocation, *receiver)),
        }
    }

    fn matches_facts(&self, facts: &ObjectHandleFacts) -> bool {
        facts.owner_spans == self.scopes
            && facts.by_scope == self.classes
            && facts.returns_object == self.returns
            && self.bodies.values().collect::<HashSet<_>>().len() == self.bodies.len()
            && self.reported_classes().is_some()
            && self.reported_scopes().is_some()
            && self.reported_returns().is_some()
            && self.reported_candidates(facts).is_some()
    }

    fn reported_classes(&self) -> Option<HashMap<(ReportOwner<'_>, &str), &HashSet<String>>> {
        self.classes
            .iter()
            .map(|((owner, name), classes)| Some(((self.owner(owner)?, name.as_str()), classes)))
            .collect()
    }

    fn reported_scopes(&self) -> Option<Vec<ReportedScope<'_>>> {
        self.scopes
            .iter()
            .map(|scope| {
                Some((
                    scope.span,
                    scope.unit.as_str(),
                    self.frame(scope.frame)?,
                    match scope.class_owner.as_ref() {
                        Some(owner) => Some(self.owner(owner)?),
                        None => None,
                    },
                    scope.class.as_deref(),
                ))
            })
            .collect()
    }

    fn reported_candidates<'a>(
        &'a self,
        facts: &'a ObjectHandleFacts,
    ) -> Option<Knowledge<ReportCandidates<'a>>> {
        let Some(owner) = facts.logical_source_candidates.as_ref() else {
            return Some(Knowledge::Absent);
        };
        let parts = owner.report_parts(facts, &self.image, &self.input, &self.registry)?;
        Some(Knowledge::Proven(ReportCandidates {
            instance_variables: parts
                .instance_variables
                .iter()
                .map(|(owner, variables)| Some((self.owner(owner)?, variables)))
                .collect::<Option<_>>()?,
            frames: parts
                .frames
                .iter()
                .map(|frame| self.frame(*frame))
                .collect::<Option<_>>()?,
            original_classes: parts
                .original_classes
                .iter()
                .map(|owner| self.owner(owner))
                .collect::<Option<_>>()?,
            definition_classes: parts
                .definition_classes
                .iter()
                .map(|owner| self.owner(owner))
                .collect::<Option<_>>()?,
        }))
    }

    fn reported_returns(&self) -> Option<HashMap<&BodyCoordinate, &str>> {
        self.returns
            .iter()
            .map(|(id, class)| Some((self.bodies.get(id)?, class.as_str())))
            .collect()
    }
}

#[cfg(test)]
fn trace_declaration_cardinality(
    module: &crate::ir::Module,
    id: ProcedureId,
    procedure: &crate::ir::Procedure,
    total: usize,
) {
    if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_REPORT_DECLARATIONS").is_none() {
        return;
    }
    let Some(site) = procedure.declaration_site.as_ref() else {
        return;
    };
    let named = module.procedures.at_declaration(site).count();
    let lowered = module
        .body_lowerings()
        .filter(|(_, body)| body.declaration_site.as_ref() == Some(site))
        .count();
    eprintln!(
        "REPORT_DECLARATION_CARDINALITY offset={} total={total} named={named} lowerings={lowered} body_kind={:?} current_id_owned={} current_named={} current_lowering={} classification={}",
        site.offset,
        procedure.body_kind,
        module.procedure_ids.owns(id),
        module.procedures.contains_key(&id),
        module
            .body_lowerings()
            .any(|(candidate, _)| *candidate == id),
        if total == 0 {
            "missing-joined-site"
        } else {
            "multiple-joined-bodies"
        },
    );
    for (ordinal, (candidate, body)) in module
        .procedures
        .at_declaration(site)
        .chain(
            module
                .body_lowerings()
                .filter(|(_, body)| body.declaration_site.as_ref() == Some(site)),
        )
        .take(8)
        .enumerate()
    {
        eprintln!(
            "REPORT_DECLARATION_BODY ordinal={ordinal} same_id={} owned={} kind={:?} span_start={} span_end={} body_offset={} same_original_span={}",
            *candidate == id,
            module.procedure_ids.owns(*candidate),
            body.body_kind,
            body.span.start(),
            body.span.end(),
            body.body_offset,
            body.span == procedure.span,
        );
    }
}

fn retain_body_coordinates(
    module: &crate::ir::Module,
    image: &SourceImage,
    registry: &CommandRegistry,
    coordinates: &mut HashSet<BodyCoordinate>,
    bodies: &mut HashMap<ProcedureId, BodyCoordinate>,
) -> Option<()> {
    for (id, procedure) in module.original_source_procedure_bodies() {
        let original = image.bytes().get(procedure.span.as_range())?;
        if !module.procedure_ids.owns(*id) || original.is_empty() {
            return None;
        }
        if let Some(site) = procedure.declaration_site.as_ref() {
            let count = module
                .procedures
                .at_declaration(site)
                .chain(
                    module
                        .body_lowerings()
                        .filter(|(_, body)| body.declaration_site.as_ref() == Some(site)),
                )
                .count();
            if count != 1 {
                #[cfg(test)]
                trace_declaration_cardinality(module, *id, procedure, count);
                return None;
            }
        }
        let coordinate = BodyCoordinate {
            kind: BodyKind::Procedure(procedure.body_kind),
            declaration: procedure.declaration_site.as_ref().map(|site| site.offset),
            span: procedure.span,
            body_offset: Some(procedure.body_offset),
            original: original.to_vec(),
        };
        if !coordinates.insert(coordinate.clone()) || bodies.insert(*id, coordinate).is_some() {
            return None;
        }
    }
    for (id, method) in &module.methods {
        let span = method.span?;
        let original = image.bytes().get(span.as_range())?;
        if !module.procedure_ids.owns(*id) || original.is_empty() {
            return None;
        }
        let coordinate = BodyCoordinate {
            kind: BodyKind::Method(method.kind),
            declaration: Some(span.start()),
            span,
            body_offset: None,
            original: original.to_vec(),
        };
        if !coordinates.insert(coordinate.clone()) || bodies.insert(*id, coordinate).is_some() {
            return None;
        }
    }
    for id in module.original_definition_scripts.keys() {
        let source = module.original_definition_script_source(*id, registry)?;
        let span = source.declaration_span();
        let original = image.bytes().get(span.as_range())?;
        let coordinate = BodyCoordinate {
            kind: BodyKind::Definition(source.family() as u8),
            declaration: Some(span.start()),
            span,
            body_offset: Some(source.content_span().start()),
            original: original.to_vec(),
        };
        if !coordinates.insert(coordinate.clone()) || bodies.insert(*id, coordinate).is_some() {
            return None;
        }
    }
    Some(())
}

/// A body coordinate relative to the module that owns the facts.
#[derive(PartialEq, Eq, Hash)]
enum RelativeFrame {
    Top,
    Body(u32),
}

#[derive(PartialEq, Eq, Hash)]
enum RelativeOwner<'a> {
    Frame(RelativeFrame),
    Definition(u32, ObjectScopeReceiver),
    Original(
        &'a tcl_names::command::CommandAllocation,
        ObjectScopeReceiver,
    ),
}

fn relative_frame(frame: BodyFrameId, module: ProcedureModuleId) -> Option<RelativeFrame> {
    match frame {
        BodyFrameId::Top(owner) => (owner == module).then_some(RelativeFrame::Top),
        BodyFrameId::Body(id) => (id.module() == module).then(|| RelativeFrame::Body(id.ordinal())),
    }
}

fn relative_owner(owner: &ObjectFactOwner, module: ProcedureModuleId) -> Option<RelativeOwner<'_>> {
    Some(match owner {
        ObjectFactOwner::Frame(frame) => RelativeOwner::Frame(relative_frame(*frame, module)?),
        ObjectFactOwner::DefinitionScript {
            definition,
            receiver,
        } => {
            (definition.module() == module).then_some(())?;
            RelativeOwner::Definition(definition.ordinal(), *receiver)
        }
        ObjectFactOwner::OriginalClass {
            allocation,
            receiver,
        } => RelativeOwner::Original(allocation, *receiver),
    })
}

type RelativeScope<'a> = (
    Span,
    &'a str,
    RelativeFrame,
    Option<RelativeOwner<'a>>,
    Option<&'a str>,
);

impl ObjectHandleFacts {
    /// The module whose top level owns these facts.
    fn owning_module(&self) -> Option<ProcedureModuleId> {
        self.owner_spans.iter().find_map(|scope| match scope.frame {
            BodyFrameId::Top(module) => Some(module),
            BodyFrameId::Body(_) => None,
        })
    }

    fn relative_scopes(&self, module: ProcedureModuleId) -> Option<Vec<RelativeScope<'_>>> {
        self.owner_spans
            .iter()
            .map(|scope| {
                Some((
                    scope.span,
                    scope.unit.as_str(),
                    relative_frame(scope.frame, module)?,
                    match &scope.class_owner {
                        Some(owner) => Some(relative_owner(owner, module)?),
                        None => None,
                    },
                    scope.class.as_deref(),
                ))
            })
            .collect()
    }

    fn relative_classes(
        &self,
        module: ProcedureModuleId,
    ) -> Option<HashMap<(RelativeOwner<'_>, &str), &HashSet<String>>> {
        self.by_scope
            .iter()
            .map(|((owner, name), classes)| {
                Some(((relative_owner(owner, module)?, name.as_str()), classes))
            })
            .collect()
    }

    fn relative_returns(&self, module: ProcedureModuleId) -> Option<HashMap<u32, &String>> {
        self.returns_object
            .iter()
            .map(|(id, class)| (id.module() == module).then(|| (id.ordinal(), class)))
            .collect()
    }

    fn relative_type_reads(
        &self,
        module: ProcedureModuleId,
    ) -> Option<Vec<(RelativeFrame, &super::PhysicalObjectTypeReadEvidence)>> {
        self.physical_type_reads
            .iter()
            .map(|read| Some((relative_frame(read.frame, module)?, read)))
            .collect()
    }

    /// Equality of two report-less fact sets with each body coordinate taken
    /// relative to its own module, since module identities are issued per
    /// process. A coordinate from any other module refuses.
    fn module_relative_eq(&self, other: &Self) -> bool {
        let (Some(a), Some(b)) = (self.owning_module(), other.owning_module()) else {
            return self == other;
        };
        let reads_match = match (self.relative_type_reads(a), other.relative_type_reads(b)) {
            (Some(left), Some(right)) => {
                left.len() == right.len()
                    && left.iter().zip(&right).all(|((lf, l), (rf, r))| {
                        lf == rf
                            && l.source == r.source
                            && l.evaluation == r.evaluation
                            && l.unit == r.unit
                            && l.variable == r.variable
                            && l.place == r.place
                            && l.value == r.value
                            && l.class == r.class
                    })
            }
            _ => false,
        };
        reads_match
            && self.report_source == other.report_source
            && self.logical_source_candidates == other.logical_source_candidates
            && self.any_scope == other.any_scope
            && self.proven_reads == other.proven_reads
            && self.collections == other.collections
            && self.normal_procedure_results == other.normal_procedure_results
            && self.global_object_cells == other.global_object_cells
            && self
                .relative_scopes(a)
                .is_some_and(|left| other.relative_scopes(b) == Some(left))
            && self
                .relative_classes(a)
                .is_some_and(|left| other.relative_classes(b) == Some(left))
            && self
                .relative_returns(a)
                .is_some_and(|left| other.relative_returns(b) == Some(left))
    }
}

impl ObjectHandleFacts {
    pub(crate) fn retain_report_source(
        &mut self,
        analysis: &AnalysisResult,
        unit: &CompilationUnit,
        registry: &CommandRegistry,
    ) {
        self.report_source = None;
        let Some(input) = analysis.resolved_input.as_ref() else {
            return;
        };
        let Some(realm) = analysis.retained_command_realm_arc() else {
            return;
        };
        let Some(image) = realm.original_source_image() else {
            return;
        };
        let module = &unit.ir_module;
        let logical = input.has_logical_source_name_context();
        // Unreached versioned source has original coordinates independently
        // of Native dispatch. This record is used only for report comparison.
        // The unit's own point versions its source; `tk` analyses that unit
        // under the anonymous lenient profile, every other input under itself.
        let versioned_source = !input.has_hosted_source_name_context()
            && tcl_native_model::InvocationDialect::of_profile(input.unit_profile())
                .native_name_protocol()
                .is_some();
        if (!logical && !versioned_source)
            || (logical && !analysis.allows_retained_logical_declaration_advice())
            || !realm.matches_resolved_analysis_input(input)
            || image != &module.source
            || image.try_text().ok() != Some(unit.source.as_str())
            || module.source_metadata_input.as_ref() != Some(input)
            || module.native_lexer_config() != input.lexer_config()
            || module.source_entry.native_entry.is_some()
            || (module.source_entry.unknown_entry && !versioned_source)
            || module.source_entry.metadata_context.source_analysis_input() != Some(input)
            || module
                .retained_source_bindings
                .as_ref()
                .is_none_or(|held| !held.matches_module(module, registry))
            || registry.snapshot().semantic_key()
                != input
                    .borrowed_context_registry()
                    .commands()
                    .snapshot()
                    .semantic_key()
        {
            return;
        }
        let mut bodies = HashMap::new();
        let mut coordinates = HashSet::new();
        if retain_body_coordinates(module, image, registry, &mut coordinates, &mut bodies).is_none()
        {
            return;
        }
        self.report_source = Some(std::sync::Arc::new(ObjectReportSource {
            image: image.clone(),
            input: input.clone(),
            registry: registry.snapshot().semantic_key(),
            module: module.procedure_ids.module_id(),
            bodies,
            scopes: self.owner_spans.clone(),
            classes: self.by_scope.clone(),
            returns: self.returns_object.clone(),
        }));
    }

    pub(crate) fn analysis_report_eq(&self, other: &Self) -> bool {
        let a = self
            .report_source
            .as_ref()
            .filter(|source| source.matches_facts(self));
        let b = other
            .report_source
            .as_ref()
            .filter(|source| source.matches_facts(other));
        let (a, b) = match (a, b) {
            (Some(a), Some(b)) => (a, b),
            (None, None) => return self.module_relative_eq(other),
            _ => return false,
        };
        if a.image != b.image
            || a.input != b.input
            || a.registry != b.registry
            || a.bodies.values().collect::<HashSet<_>>()
                != b.bodies.values().collect::<HashSet<_>>()
            || self.any_scope != other.any_scope
            || self.collections != other.collections
            || self.global_object_cells != other.global_object_cells
            || self.proven_reads != other.proven_reads
            || self.physical_type_reads != other.physical_type_reads
            || self.normal_procedure_results != other.normal_procedure_results
            || a.reported_candidates(self) != b.reported_candidates(other)
        {
            return false;
        }
        match (
            a.reported_classes(),
            b.reported_classes(),
            a.reported_scopes(),
            b.reported_scopes(),
            a.reported_returns(),
            b.reported_returns(),
        ) {
            (Some(ac), Some(bc), Some(as_), Some(bs), Some(ar), Some(br)) => {
                ac == bc && as_ == bs && ar == br
            }
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::analyser::Analyser;

    #[test]
    fn independent_analyses_of_a_class_with_an_unretained_definition_script_compare_equal() {
        // Each analysis issues its own process-unique module identity; the
        // class body `K` is a definition script without a retained source
        // coordinate, so its owner compares by ordinal within the module.
        let source =
            "oo::class create K {\n  method m {a} { set n $a }\n}\nproc p {} { set q 1 }\n";
        let first = Analyser::new().analyse(source, "tcl8.6");
        let second = Analyser::new().analyse(source, "tcl8.6");
        assert!(
            first
                .object_handle_facts
                .analysis_report_eq(&second.object_handle_facts)
        );
        crate::analyser::types::assert_analysis_results_eq(&first, &second, "two analyses");
    }

    #[test]
    fn current_native_alias_report_capture_control() {
        let source = include_str!("../../tests/data/name-consumers/aliases.tcl");
        let first = Analyser::new().analyse(source, "tcl8.6");
        let second = Analyser::new().analyse(source, "tcl8.6");
        eprintln!(
            "REPORT_CONTROL label=aliases dialect=tcl8.6 bytes={} full_equal={} diagnostics_equal={} reports_present={}/{}",
            source.len(),
            first == second,
            first.diagnostics == second.diagnostics,
            first.object_handle_facts.report_source.is_some(),
            second.object_handle_facts.report_source.is_some(),
        );
        assert!(
            first == second,
            "unchanged complete AnalysisResult equality: aliases"
        );
    }

    #[test]
    fn independent_logical_reports_match_original_coordinates_without_equal_runtime_owners() {
        let source = "oo::class create C {variable {$literal}; method m {} {return VALUE}; method n {} {return OTHER}}";
        let a = Analyser::new().analyse(source, tcl_dialect::DialectProfile::plain_tcl().name);
        let b = Analyser::new().analyse(source, tcl_dialect::DialectProfile::plain_tcl().name);
        assert!(a.object_handle_facts.report_source.is_some());
        assert!(b.object_handle_facts.report_source.is_some());
        assert_ne!(a.object_handle_facts, b.object_handle_facts);
        crate::analyser::types::assert_analysis_results_eq(&a, &b, "report equality");
        let incremental =
            Analyser::new().analyse_per_item(source, tcl_dialect::DialectProfile::plain_tcl().name);
        crate::analyser::types::assert_analysis_results_eq(
            &incremental,
            &a,
            "incremental report equality",
        );

        let changed = Analyser::new().analyse(
            "oo::class create C {variable {$literal}; method m {} {return DIFFERENT}; method n {} {return OTHER}}", tcl_dialect::DialectProfile::plain_tcl().name);
        assert!(
            !std::cmp::PartialEq::eq(&changed, &a),
            "expected distinct analysis results"
        );
        let mut donated = b.clone();
        donated.object_handle_facts = a.object_handle_facts.clone();
        crate::analyser::types::assert_analysis_results_eq(&donated, &b, "donated report equality");
        crate::analyser::types::assert_analysis_results_eq(&donated, &a, "donated report equality");
        crate::analyser::types::assert_analysis_results_eq(
            &donated,
            &donated.clone(),
            "donated report self equality",
        );
        crate::analyser::types::assert_analysis_results_eq(&a, &a.clone(), "report self equality");
        assert!(std::cmp::PartialEq::eq(&b, &a) && std::cmp::PartialEq::eq(&a, &b));
        assert!(
            std::cmp::PartialEq::eq(&a, &b)
                && std::cmp::PartialEq::eq(&b, &donated)
                && std::cmp::PartialEq::eq(&a, &donated)
        );
    }

    #[test]
    fn report_coordinate_mapping_refuses_missing_duplicate_and_changed_body_owners() {
        let source = "oo::class create C {method m {} {return VALUE}; method n {} {return OTHER}}";
        let a = Analyser::new().analyse(source, tcl_dialect::DialectProfile::plain_tcl().name);
        let b = Analyser::new().analyse(source, tcl_dialect::DialectProfile::plain_tcl().name);
        crate::analyser::types::assert_analysis_results_eq(&a, &b, "report equality");
        let mut missing = b.clone();
        let owner =
            std::sync::Arc::make_mut(missing.object_handle_facts.report_source.as_mut().unwrap());
        let id = *owner.bodies.keys().next().unwrap();
        owner.bodies.remove(&id);
        assert!(
            !std::cmp::PartialEq::eq(&missing, &a),
            "expected distinct analysis results"
        );
        let mut changed = b.clone();
        let owner =
            std::sync::Arc::make_mut(changed.object_handle_facts.report_source.as_mut().unwrap());
        owner.bodies.get_mut(&id).unwrap().original.push(b' ');
        assert!(
            !std::cmp::PartialEq::eq(&changed, &a),
            "expected distinct analysis results"
        );
        let mut duplicate = b.clone();
        let owner = std::sync::Arc::make_mut(
            duplicate
                .object_handle_facts
                .report_source
                .as_mut()
                .unwrap(),
        );
        let value = owner.bodies.get(&id).unwrap().clone();
        let other_id = *owner.bodies.keys().find(|other| **other != id).unwrap();
        owner.bodies.insert(other_id, value);
        assert!(
            !std::cmp::PartialEq::eq(&duplicate, &a),
            "expected distinct analysis results"
        );
        let mut changed_facts = b.clone();
        changed_facts.object_handle_facts.owner_spans[0]
            .unit
            .push_str("changed");
        assert!(
            !std::cmp::PartialEq::eq(&changed_facts, &a),
            "expected distinct analysis results"
        );
        crate::analyser::types::assert_analysis_results_eq(
            &changed_facts,
            &changed_facts.clone(),
            "changed report self equality",
        );
        assert!(
            !std::cmp::PartialEq::eq(&a, &changed_facts),
            "expected distinct analysis results"
        );
    }

    #[test]
    fn versioned_source_reports_compare_coordinates_without_logical_or_native_admission() {
        let source = "oo::class create C {method m {} {return VALUE}}";
        let a = Analyser::new().analyse(source, "tcl8.6");
        let b = Analyser::new().analyse(source, "tcl8.6");
        assert!(
            !a.resolved_input
                .as_ref()
                .unwrap()
                .has_logical_source_name_context()
        );
        assert!(!a.allows_retained_logical_declaration_advice());
        assert!(a.object_handle_facts.logical_source_candidates.is_none());
        assert!(a.object_handle_facts.report_source.is_some());
        assert!(b.object_handle_facts.report_source.is_some());
        assert_ne!(a.object_handle_facts, b.object_handle_facts);
        crate::analyser::types::assert_analysis_results_eq(&a, &b, "report equality");
        crate::analyser::types::assert_analysis_results_eq(&b, &a, "report equality");
        let mut copied = b.clone();
        copied.object_handle_facts = a.object_handle_facts.clone();
        assert!(std::cmp::PartialEq::eq(&copied, &a) && std::cmp::PartialEq::eq(&copied, &b));
        crate::analyser::types::assert_analysis_results_eq(&a, &a.clone(), "report self equality");
        crate::analyser::types::assert_analysis_results_eq(
            &copied,
            &copied.clone(),
            "copied report self equality",
        );
    }
}
