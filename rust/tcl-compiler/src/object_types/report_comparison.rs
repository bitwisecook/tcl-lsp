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
use tcl_core_types::{BodyFrameId, ProcedureModuleId};
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
    Original(
        &'a crate::command_binding::CommandAllocation,
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

// Temporary observational control. Conditions retain their original order and
// short circuit; these traces neither issue correspondence nor change equality.
macro_rules! report_capture_refused {
    ($label:literal, $condition:expr) => {{
        let refused = $condition;
        #[cfg(test)]
        if refused && std::env::var_os("TCL_LSP_TRACE_REPORT_CAPTURE").is_some() {
            eprintln!("REPORT_CAPTURE refusal={}", $label);
        }
        refused
    }};
}
macro_rules! report_facts_match {
    ($label:literal, $condition:expr) => {{
        let matches = $condition;
        #[cfg(test)]
        if !matches && std::env::var_os("TCL_LSP_TRACE_REPORT_CAPTURE").is_some() {
            eprintln!("REPORT_CURRENTNESS refusal={}", $label);
        }
        matches
    }};
}

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
            } => Some(ReportOwner::Definition {
                body: self.bodies.get(definition)?,
                receiver: *receiver,
            }),
            ObjectFactOwner::OriginalClass {
                allocation,
                receiver,
            } => Some(ReportOwner::Original(allocation, *receiver)),
        }
    }

    fn matches_facts(&self, facts: &ObjectHandleFacts) -> bool {
        report_facts_match!("owner-spans", facts.owner_spans == self.scopes)
            && report_facts_match!("classes", facts.by_scope == self.classes)
            && report_facts_match!("returns", facts.returns_object == self.returns)
            && report_facts_match!(
                "body-coordinate-uniqueness",
                self.bodies.values().collect::<HashSet<_>>().len() == self.bodies.len()
            )
            && report_facts_match!(
                "reported-classes-totality",
                self.reported_classes().is_some()
            )
            && report_facts_match!("reported-scopes-totality", self.reported_scopes().is_some())
            && report_facts_match!(
                "reported-returns-totality",
                self.reported_returns().is_some()
            )
            && report_facts_match!(
                "reported-candidates-totality",
                self.reported_candidates(facts).is_some()
            )
    }

    fn reported_classes(&self) -> Option<HashMap<(ReportOwner<'_>, &str), &HashSet<String>>> {
        self.classes
            .iter()
            .map(|((owner, name), classes)| Some(((self.owner(owner)?, name.as_str()), classes)))
            .collect()
    }

    fn reported_scopes(
        &self,
    ) -> Option<
        Vec<(
            Span,
            &str,
            ReportFrame<'_>,
            Option<ReportOwner<'_>>,
            Option<&str>,
        )>,
    > {
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
    ) -> Option<Option<ReportCandidates<'a>>> {
        let Some(owner) = facts.logical_source_candidates.as_ref() else {
            return Some(None);
        };
        let parts = owner.report_parts(facts, &self.image, &self.input, &self.registry)?;
        Some(Some(ReportCandidates {
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

impl ObjectHandleFacts {
    pub(crate) fn retain_report_source(
        &mut self,
        analysis: &AnalysisResult,
        unit: &CompilationUnit,
        registry: &CommandRegistry,
    ) {
        self.report_source = None;
        let Some(input) = analysis.resolved_input.as_ref() else {
            report_capture_refused!("missing-input", true);
            return;
        };
        let Some(realm) = analysis.retained_command_realm_arc() else {
            report_capture_refused!("missing-realm", true);
            return;
        };
        let Some(image) = realm.original_source_image() else {
            report_capture_refused!("missing-image", true);
            return;
        };
        let module = &unit.ir_module;
        let logical = input.has_logical_source_name_context();
        // Unreached versioned source has original coordinates independently
        // of Native dispatch. This record is used only for report comparison.
        let versioned_source = !input.has_hosted_source_name_context()
            && [input.analyser_profile(), input.unit_profile()]
                .into_iter()
                .all(|profile| {
                    tcl_registry::InvocationDialect::of_profile(profile)
                        .native_name_protocol()
                        .is_some()
                });
        if report_capture_refused!("unsupported-source-domain", (!logical && !versioned_source))
            || report_capture_refused!(
                "logical-advice-unavailable",
                (logical && !analysis.allows_retained_logical_declaration_advice())
            )
            || report_capture_refused!("realm-input", !realm.matches_resolved_analysis_input(input))
            || report_capture_refused!("module-image", image != &module.source)
            || report_capture_refused!(
                "unit-source",
                image.try_text().ok() != Some(unit.source.as_str())
            )
            || report_capture_refused!(
                "module-input",
                module.source_metadata_input.as_ref() != Some(input)
            )
            || report_capture_refused!(
                "module-config",
                module.native_lexer_config() != input.lexer_config()
            )
            || report_capture_refused!("physical-entry", module.source_entry.native_entry.is_some())
            || report_capture_refused!(
                "unknown-entry",
                (module.source_entry.unknown_entry && !versioned_source)
            )
            || report_capture_refused!(
                "entry-input",
                module.source_entry.metadata_context.source_analysis_input() != Some(input)
            )
            || report_capture_refused!(
                "retained-module",
                module
                    .retained_source_bindings
                    .as_ref()
                    .is_none_or(|held| !held.matches_module(module, registry))
            )
            || report_capture_refused!(
                "registry-input",
                registry.snapshot().semantic_key()
                    != input
                        .borrowed_context_registry()
                        .commands()
                        .snapshot()
                        .semantic_key()
            )
        {
            return;
        }
        let mut bodies = HashMap::new();
        let mut coordinates = HashSet::new();
        for (id, procedure) in module.original_source_procedure_bodies() {
            let Some(original) = image.bytes().get(procedure.span.as_range()) else {
                report_capture_refused!("procedure-span", true);
                return;
            };
            if !module.procedure_ids.owns(*id) || original.is_empty() {
                report_capture_refused!("body-owner-or-empty", true);
                return;
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
                    report_capture_refused!("declaration-site-count", true);
                    return;
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
                report_capture_refused!("duplicate-coordinate-or-body", true);
                return;
            }
        }
        for (id, method) in &module.methods {
            let Some(span) = method.span else {
                report_capture_refused!("missing-method-span", true);
                return;
            };
            let Some(original) = image.bytes().get(span.as_range()) else {
                report_capture_refused!("body-span", true);
                return;
            };
            if !module.procedure_ids.owns(*id) || original.is_empty() {
                report_capture_refused!("body-owner-or-empty", true);
                return;
            }
            let coordinate = BodyCoordinate {
                kind: BodyKind::Method(method.kind),
                declaration: Some(span.start()),
                span,
                body_offset: None,
                original: original.to_vec(),
            };
            if !coordinates.insert(coordinate.clone()) || bodies.insert(*id, coordinate).is_some() {
                report_capture_refused!("duplicate-coordinate-or-body", true);
                return;
            }
        }
        for id in module.original_definition_scripts.keys() {
            let Some(source) = module.original_definition_script_source(*id, registry) else {
                report_capture_refused!("definition-source", true);
                return;
            };
            let span = source.declaration_span();
            let Some(original) = image.bytes().get(span.as_range()) else {
                report_capture_refused!("body-span", true);
                return;
            };
            let coordinate = BodyCoordinate {
                kind: BodyKind::Definition(source.family() as u8),
                declaration: Some(span.start()),
                span,
                body_offset: Some(source.content_span().start()),
                original: original.to_vec(),
            };
            if !coordinates.insert(coordinate.clone()) || bodies.insert(*id, coordinate).is_some() {
                report_capture_refused!("duplicate-coordinate-or-body", true);
                return;
            }
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
        #[cfg(test)]
        if std::env::var_os("TCL_LSP_TRACE_REPORT_CAPTURE").is_some() {
            eprintln!(
                "REPORT_COMPARISON present_left={} present_right={} current_left={} current_right={}",
                self.report_source.is_some(),
                other.report_source.is_some(),
                a.is_some(),
                b.is_some()
            );
        }
        let (a, b) = match (a, b) {
            (Some(a), Some(b)) => (a, b),
            (None, None) => return self == other,
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

    fn current_report_control(source: &str, dialect: &str, label: &str) {
        let first = Analyser::new().analyse(source, dialect);
        let second = Analyser::new().analyse(source, dialect);
        eprintln!(
            "REPORT_CONTROL label={label} dialect={dialect} bytes={} full_equal={} diagnostics_equal={} reports_present={}/{}",
            source.len(),
            first == second,
            first.diagnostics == second.diagnostics,
            first.object_handle_facts.report_source.is_some(),
            second.object_handle_facts.report_source.is_some()
        );
        eprintln!("REPORT_FIELDS {}", first.bounded_mismatch_summary(&second));
        for (ordinal, (left, right)) in first
            .diagnostics
            .iter()
            .zip(&second.diagnostics)
            .enumerate()
        {
            if left != right {
                eprintln!(
                    "REPORT_DIAGNOSTIC ordinal={ordinal} code_left={} code_right={} subject_equal={} code_equal={} span_equal={} message_equal={} severity_equal={} fixes_equal={}",
                    left.code,
                    right.code,
                    left.subject == right.subject,
                    left.code == right.code,
                    left.span == right.span,
                    left.message == right.message,
                    left.severity == right.severity,
                    left.fixes == right.fixes
                );
                if let (Some(left), Some(right)) = (left.subject.as_ref(), right.subject.as_ref()) {
                    left.trace_report_subject_pair(right);
                }
                break;
            }
        }
        assert!(
            first == second,
            "unchanged complete AnalysisResult equality: {label}"
        );
    }

    #[test]
    fn current_native_alias_report_capture_control() {
        current_report_control(
            include_str!("../../tests/data/name-consumers/aliases.tcl"),
            "tcl8.6",
            "aliases",
        );
    }
    #[test]
    fn current_native_arity_subject_report_control() {
        current_report_control(
            include_str!("../../../../samples/diagnostics/E003_proc_call_arity/example.tcl"),
            "tcl8.6",
            "E003",
        );
    }
    #[test]
    fn current_hosted_report_capture_control() {
        current_report_control(
            include_str!("../../../../samples/irules/cookbook_http_collect.irul"),
            "f5-irules",
            "hosted-http-collect",
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
        assert!(a == b);
        let incremental =
            Analyser::new().analyse_per_item(source, tcl_dialect::DialectProfile::plain_tcl().name);
        assert!(incremental == a);

        let changed = Analyser::new().analyse(
            "oo::class create C {variable {$literal}; method m {} {return DIFFERENT}; method n {} {return OTHER}}", tcl_dialect::DialectProfile::plain_tcl().name);
        assert!(changed != a);
        let mut donated = b.clone();
        donated.object_handle_facts = a.object_handle_facts.clone();
        assert!(donated == b);
        assert!(donated == a);
        assert!(donated == donated.clone());
        assert!(a == a.clone());
        assert!(b == a && a == b);
        assert!(a == b && b == donated && a == donated);
    }

    #[test]
    fn report_coordinate_mapping_refuses_missing_duplicate_and_changed_body_owners() {
        let source = "oo::class create C {method m {} {return VALUE}; method n {} {return OTHER}}";
        let a = Analyser::new().analyse(source, tcl_dialect::DialectProfile::plain_tcl().name);
        let b = Analyser::new().analyse(source, tcl_dialect::DialectProfile::plain_tcl().name);
        assert!(a == b);
        let mut missing = b.clone();
        let owner =
            std::sync::Arc::make_mut(missing.object_handle_facts.report_source.as_mut().unwrap());
        let id = *owner.bodies.keys().next().unwrap();
        owner.bodies.remove(&id);
        assert!(missing != a);
        let mut changed = b.clone();
        let owner =
            std::sync::Arc::make_mut(changed.object_handle_facts.report_source.as_mut().unwrap());
        owner.bodies.get_mut(&id).unwrap().original.push(b' ');
        assert!(changed != a);
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
        assert!(duplicate != a);
        let mut changed_facts = b.clone();
        changed_facts.object_handle_facts.owner_spans[0]
            .unit
            .push_str("changed");
        assert!(changed_facts != a);
        assert!(changed_facts == changed_facts.clone());
        assert!(a != changed_facts);
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
        assert!(a == b);
        assert!(b == a);
        let mut copied = b.clone();
        copied.object_handle_facts = a.object_handle_facts.clone();
        assert!(copied == a && copied == b);
        assert!(a == a.clone() && copied == copied.clone());
    }
}
