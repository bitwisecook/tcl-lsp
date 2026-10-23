// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Opt-in full-report capture, separate from runtime and cache equality.
//!
//! Complete comparison retains strict typed facets except explicitly issued
//! actual-owner pairs. Missing correspondence is unavailable, never a fallback.

pub(crate) mod body_correspondence;
mod field_comparison;

use body_correspondence::{ReportBodyCorrespondence, ReportBodyInventory};
use std::sync::{Arc, Mutex};

use super::{Analyser, AnalysisResult, ResolvedAnalysisInput};
use crate::command_binding::{OriginalSourceAnalysisOwner, RetainedSourceModuleBindings};
use crate::compilation_unit::{CompilationUnit, OriginalOptimiserSourceOrigin};
use tcl_lexer::SourceImage;
use tcl_registry::{CommandRegistry, RegistrySnapshot};

/// Result of a complete, explicitly requested reporting comparison.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnalysisReportComparison {
    /// Every field and typed subject has a complete current correspondence.
    Equal,
    /// An authenticated report field differs.
    Different {
        /// Field whose reporting values differ.
        field: &'static str,
    },
    /// A required issuer or complete correspondence is unavailable.
    Unavailable(AnalysisReportUnavailable),
}

/// Terminal refusal of reporting correspondence, never evidence of equality.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnalysisReportUnavailable {
    /// The real diagnostics path did not supply a compilation unit.
    MissingActualCompilationUnit,
    /// More than one unit attempted to supply the same report request.
    AmbiguousCompilationUnit,
    /// A source, input, Registry, evaluator or actual owner no longer joins.
    CurrentOwnerUnavailable,
    /// Independent results do not share the complete source/input context
    /// required by this reporting purpose.
    IncompatibleSourceContext,
    /// The output does not retain the actual diagnostic producer's facts.
    ChangedProducedFacts,
    /// Complete typed-field correspondence is not implemented for this stage.
    CompleteFieldCorrespondenceUnavailable,
    /// A poisoned capture cannot establish a complete request inventory.
    CaptureUnavailable,
}

/// Immutable result of one actual report request. No mutable result reference
/// or public carrier constructor can donate a current source owner.
pub struct AuthenticatedAnalysisReport {
    analysis: AnalysisResult,
    capture: Result<ActualReportCapture, AnalysisReportUnavailable>,
}

impl AuthenticatedAnalysisReport {
    /// Full analysis, with its ordinary strict runtime/cache equality intact.
    #[must_use]
    pub const fn analysis(&self) -> &AnalysisResult {
        &self.analysis
    }

    /// Compare reporting data only after both real request owners rejoin.
    /// Missing coverage is terminal; ordinary equality is never a fallback.
    #[must_use]
    pub fn compare(&self, other: &Self) -> AnalysisReportComparison {
        let left = match self.current_capture() {
            Ok(value) => value,
            Err(reason) => return AnalysisReportComparison::Unavailable(reason),
        };
        let right = match other.current_capture() {
            Ok(value) => value,
            Err(reason) => return AnalysisReportComparison::Unavailable(reason),
        };
        if left.input != right.input || left.image != right.image || left.registry != right.registry
        {
            return AnalysisReportComparison::Unavailable(
                AnalysisReportUnavailable::IncompatibleSourceContext,
            );
        }
        let Some(bodies) = ReportBodyCorrespondence::capture(&left.bodies, &right.bodies) else {
            return AnalysisReportComparison::Unavailable(
                AnalysisReportUnavailable::CompleteFieldCorrespondenceUnavailable,
            );
        };
        field_comparison::compare(
            &self.analysis,
            &other.analysis,
            &bodies,
            &left.image,
            &left.registry.semantic_key(),
            &ReportOutputCorrespondence {
                left: &self.analysis,
                right: &other.analysis,
                left_owner: &left.realm_owner,
                right_owner: &right.realm_owner,
            },
        )
    }

    fn current_capture(&self) -> Result<&ActualReportCapture, AnalysisReportUnavailable> {
        let capture = self.capture.as_ref().map_err(|reason| *reason)?;
        let matched = capture.matches_output(&self.analysis);
        #[cfg(test)]
        if std::env::var_os("TCL_LSP_TRACE_AUTHENTICATED_REPORT").is_some() {
            eprintln!("REPORT_CAPTURE stage=current-output result={matched:?}");
        }
        matched?;
        Ok(capture)
    }
}

impl std::fmt::Debug for AuthenticatedAnalysisReport {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("AuthenticatedAnalysisReport")
            .field("capture_available", &self.capture.is_ok())
            .finish_non_exhaustive()
    }
}

/// Only the same analyser's real diagnostic CU may fulfil this request. A Weak
/// field in the analyser cannot keep a completed or unwound request active.
#[derive(Debug, Default)]
pub(super) struct ReportCaptureRequest(Mutex<CaptureState>);

#[derive(Default)]
enum CaptureState {
    #[default]
    Pending,
    Captured(Box<ActualReportCapture>),
    Refused(AnalysisReportUnavailable),
}

impl std::fmt::Debug for CaptureState {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::Pending => "Pending",
            Self::Captured(_) => "Captured",
            Self::Refused(_) => "Refused",
        })
    }
}

impl ReportCaptureRequest {
    pub(super) fn capture(
        &self,
        analysis: &AnalysisResult,
        unit: &CompilationUnit,
        registry: &CommandRegistry,
    ) {
        let Ok(mut state) = self.0.lock() else {
            return;
        };
        if !matches!(*state, CaptureState::Pending) {
            *state = CaptureState::Refused(AnalysisReportUnavailable::AmbiguousCompilationUnit);
            return;
        }
        *state = match ActualReportCapture::from_actual_unit(analysis, unit, registry) {
            Some(capture) => CaptureState::Captured(Box::new(capture)),
            None => CaptureState::Refused(AnalysisReportUnavailable::CurrentOwnerUnavailable),
        };
    }

    fn finish(&self, analysis: AnalysisResult) -> AuthenticatedAnalysisReport {
        let capture = match self.0.lock() {
            Ok(mut state) => match std::mem::take(&mut *state) {
                CaptureState::Pending => {
                    Err(AnalysisReportUnavailable::MissingActualCompilationUnit)
                }
                CaptureState::Captured(capture) => Ok(*capture),
                CaptureState::Refused(reason) => Err(reason),
            },
            Err(_) => Err(AnalysisReportUnavailable::CaptureUnavailable),
        };
        let mut report = AuthenticatedAnalysisReport { analysis, capture };
        if let Err(reason) = report.current_capture() {
            report.capture = Err(reason);
        }
        report
    }
}

/// Sealed current actual diagnostic-output pair. Only successful full capture
/// and complete source/input/registry/body joins can construct this token.
pub(crate) struct ReportOutputCorrespondence<'a> {
    left: &'a AnalysisResult,
    right: &'a AnalysisResult,
    left_owner: &'a Arc<OriginalSourceAnalysisOwner>,
    right_owner: &'a Arc<OriginalSourceAnalysisOwner>,
}
impl ReportOutputCorrespondence<'_> {
    pub(crate) fn analyses(&self) -> (&AnalysisResult, &AnalysisResult) {
        (self.left, self.right)
    }
    pub(crate) fn analysis_owners(
        &self,
        left: &Arc<OriginalSourceAnalysisOwner>,
        right: &Arc<OriginalSourceAnalysisOwner>,
    ) -> bool {
        Arc::ptr_eq(left, self.left_owner) && Arc::ptr_eq(right, self.right_owner)
    }
}

// Preserve each original Option exactly once; diagnostic output is test-only.
fn trace_capture_option<T>(value: Option<T>, stage: &str) -> Option<T> {
    #[cfg(test)]
    if std::env::var_os("TCL_LSP_TRACE_AUTHENTICATED_REPORT").is_some() {
        eprintln!("REPORT_CAPTURE stage={stage} available={}", value.is_some());
    }
    #[cfg(not(test))]
    let _ = stage;
    value
}

fn trace_capture_gate(value: bool, stage: &str) -> bool {
    #[cfg(test)]
    if std::env::var_os("TCL_LSP_TRACE_AUTHENTICATED_REPORT").is_some() {
        eprintln!("REPORT_CAPTURE stage={stage} refuses={value}");
    }
    #[cfg(not(test))]
    let _ = stage;
    value
}

struct ActualReportCapture {
    image: SourceImage,
    input: ResolvedAnalysisInput,
    registry: RegistrySnapshot,
    origin: OriginalOptimiserSourceOrigin,
    realm_owner: Arc<OriginalSourceAnalysisOwner>,
    module_owner: Arc<OriginalSourceAnalysisOwner>,
    positioned: Arc<RetainedSourceModuleBindings>,
    facts: crate::object_types::ObjectHandleFacts,
    bodies: ReportBodyInventory,
}

impl ActualReportCapture {
    fn from_actual_unit(
        analysis: &AnalysisResult,
        unit: &CompilationUnit,
        registry: &CommandRegistry,
    ) -> Option<Self> {
        let input = trace_capture_option(analysis.resolved_input.as_ref(), "capture.input")?;
        let realm = trace_capture_option(analysis.retained_command_realm(), "capture.realm")?;
        let bindings = realm.source_bindings_ref();
        let realm_owner =
            trace_capture_option(bindings.original_analysis_owner(), "capture.realm-owner")?;
        let module = &unit.ir_module;
        let positioned = trace_capture_option(
            module.retained_source_bindings.as_ref(),
            "capture.retained-module",
        )?;
        let module_bindings = trace_capture_option(
            positioned.positioned_for_module(module, registry),
            "capture.positioned-module",
        )?;
        let module_owner = trace_capture_option(
            module_bindings.original_analysis_owner(),
            "capture.module-owner",
        )?;
        let image = trace_capture_option(realm.original_source_image(), "capture.image")?;
        let origin =
            trace_capture_option(unit.original_optimiser_source_origin(), "capture.origin")?;
        let prepared = trace_capture_option(
            unit.original_full_report_prepared_unit(registry),
            "capture.prepared",
        )?;
        let bodies = trace_capture_option(
            ReportBodyInventory::capture(prepared, registry),
            "capture.bodies",
        )?;
        if trace_capture_gate(
            analysis.analysis_context_unavailable.is_some(),
            "capture.analysis-context",
        ) || trace_capture_gate(
            analysis.original_source_region.is_some(),
            "capture.source-region",
        ) || trace_capture_gate(
            !realm_owner.matches(bindings, input),
            "capture.realm-owner-match",
        ) || trace_capture_gate(
            !module_owner.matches(module_bindings, input),
            "capture.module-owner-match",
        ) || trace_capture_gate(image != &module.source, "capture.module-image")
            || trace_capture_gate(image.try_text().ok()? != unit.source, "capture.unit-source")
            || trace_capture_gate(
                module.source_metadata_input.as_ref() != Some(input),
                "capture.module-input",
            )
            || trace_capture_gate(
                !analysis.matches_original_source_image(image, input.lexer_config()),
                "capture.analysis-image",
            )
            || trace_capture_gate(
                registry.snapshot() != input.borrowed_context_registry().commands().snapshot(),
                "capture.registry",
            )
            || !analysis
                .optimiser_source_origin
                .as_ref()?
                .same_request_allocation(origin)
        {
            return None;
        }
        Some(Self {
            image: image.clone(),
            input: input.clone(),
            registry: registry.snapshot(),
            origin: origin.clone(),
            realm_owner: Arc::clone(realm_owner),
            module_owner: Arc::clone(module_owner),
            positioned: Arc::clone(positioned),
            facts: analysis.object_handle_facts.clone(),
            bodies,
        })
    }

    fn matches_output(&self, analysis: &AnalysisResult) -> Result<(), AnalysisReportUnavailable> {
        let unavailable = AnalysisReportUnavailable::CurrentOwnerUnavailable;
        let source = self.image.try_text().map_err(|_| unavailable)?;
        let registry = analysis.resolved_registry().ok_or(unavailable)?;
        let origin = analysis
            .original_optimiser_request_owner(source, registry)
            .ok_or(unavailable)?;
        let realm = analysis.retained_command_realm().ok_or(unavailable)?;
        let bindings = realm.source_bindings_ref();
        let owner = bindings.original_analysis_owner().ok_or(unavailable)?;
        if trace_capture_gate(
            analysis.resolved_input.as_ref() != Some(&self.input),
            "output.input",
        ) || trace_capture_gate(registry.snapshot() != self.registry, "output.registry")
            || trace_capture_gate(
                !self.origin.same_request_allocation(origin),
                "output.origin-allocation",
            )
            || trace_capture_gate(!self.bodies.same_request(origin), "output.body-request")
            || trace_capture_gate(
                !Arc::ptr_eq(owner, &self.realm_owner),
                "output.realm-allocation",
            )
            || trace_capture_gate(
                !self.realm_owner.matches(bindings, &self.input),
                "output.realm-match",
            )
            || trace_capture_gate(
                !analysis.matches_original_source_image(&self.image, self.input.lexer_config()),
                "output.image",
            )
            || !self.positioned.matches_full_report_capture_owner(
                &self.image,
                &self.input,
                registry,
                &self.module_owner,
            )
        {
            return Err(unavailable);
        }
        if analysis.object_handle_facts != self.facts {
            return Err(AnalysisReportUnavailable::ChangedProducedFacts);
        }
        Ok(())
    }
}

impl Analyser {
    /// Capture an actual complete analysis for the separate reporting API.
    /// This does not alter diagnostics, runtime equality or cache eligibility.
    pub fn analyse_for_report(
        &mut self,
        source: &str,
        dialect: &str,
    ) -> AuthenticatedAnalysisReport {
        self.capture_report(|analyser| analyser.analyse(source, dialect))
    }

    /// Capture the existing per-item analysis path, including its original
    /// fallbacks, with the same separate full-report obligations.
    pub fn analyse_per_item_for_report(
        &mut self,
        source: &str,
        dialect: &str,
    ) -> AuthenticatedAnalysisReport {
        self.capture_report(|analyser| analyser.analyse_per_item(source, dialect))
    }

    fn capture_report(
        &mut self,
        run: impl FnOnce(&mut Self) -> AnalysisResult,
    ) -> AuthenticatedAnalysisReport {
        let request = Arc::new(ReportCaptureRequest::default());
        let previous = self
            .authenticated_report_request
            .replace(Arc::downgrade(&request));
        let analysis = run(self);
        self.authenticated_report_request = previous;
        request.finish(analysis)
    }
}

#[cfg(test)]
mod capture_controls {
    use super::*;

    fn actual_report() -> AuthenticatedAnalysisReport {
        let report = Analyser::new().analyse_for_report("set value VALUE", "tcl8.6");
        assert!(
            report.current_capture().is_ok(),
            "a genuine diagnostics CU is required"
        );
        report
    }

    #[test]
    fn report_capture_self_comparison_requires_complete_current_owners() {
        let report = actual_report();
        assert!(report.analysis() == report.analysis());
        assert_eq!(report.compare(&report), AnalysisReportComparison::Equal);
    }

    #[test]
    fn actual_e003_subjects_join_their_current_diagnostic_output_owners() {
        let source = "proc retained {} {}\nretained EXTRA";
        let mut analyser = Analyser::new();
        let left = analyser.analyse_for_report(source, "tcl8.6");
        let right = analyser.analyse_for_report(source, "tcl8.6");
        assert!(
            left.analysis.diagnostics.iter().any(|diagnostic| matches!(
                diagnostic.subject,
                Some(crate::analyser::DiagnosticSubject::UserSourceArity(_))
            )),
            "genuine original user arity producer required"
        );
        assert!(left.current_capture().is_ok());
        assert!(right.current_capture().is_ok());
        assert_eq!(left.compare(&right), AnalysisReportComparison::Equal);
    }

    #[test]
    fn e003_equal_source_foreign_diagnostic_attachment_is_unavailable() {
        let source = "proc retained {} {}\nretained EXTRA";
        let mut left = Analyser::new().analyse_for_report(source, "tcl8.6");
        let right = Analyser::new().analyse_for_report(source, "tcl8.6");
        let foreign = Analyser::new().analyse_for_report(source, "tcl8.6");
        assert!(
            !Arc::ptr_eq(
                &left.current_capture().unwrap().realm_owner,
                &foreign.current_capture().unwrap().realm_owner
            ),
            "independent actual analysis issuers required"
        );
        let subject = foreign
            .analysis
            .diagnostics
            .iter()
            .find_map(|diagnostic| match &diagnostic.subject {
                Some(crate::analyser::DiagnosticSubject::UserSourceArity(subject)) => {
                    Some(subject.clone())
                }
                _ => None,
            })
            .expect("genuine E003 source subject");
        let target = left
            .analysis
            .diagnostics
            .iter_mut()
            .find(|diagnostic| {
                matches!(
                    &diagnostic.subject,
                    Some(crate::analyser::DiagnosticSubject::UserSourceArity(_))
                )
            })
            .unwrap();
        target.subject = Some(crate::analyser::DiagnosticSubject::UserSourceArity(subject));
        assert!(
            left.current_capture().is_ok(),
            "outer output owner alone does not authenticate a transplanted subject"
        );
        assert_eq!(
            left.compare(&right),
            AnalysisReportComparison::Unavailable(
                AnalysisReportUnavailable::CompleteFieldCorrespondenceUnavailable
            )
        );
    }

    #[test]
    fn genuine_hosted_event_report_retains_a_separate_actual_domain() {
        let report = Analyser::new()
            .analyse_for_report("when HTTP_REQUEST { set value VALUE }", "f5-irules");
        let capture = report
            .current_capture()
            .expect("actual hosted diagnostics CU");
        assert!(capture.input.has_hosted_source_name_context());
        assert!(capture.bodies.hosted_domain_for_test());
        assert_eq!(report.compare(&report), AnalysisReportComparison::Equal);
    }

    #[test]
    fn changed_plain_report_value_is_a_difference_after_current_owners_join() {
        let left = actual_report();
        let mut right = actual_report();
        right.analysis.library_versions.tool_version = Some("VERSION".into());
        assert_eq!(
            left.compare(&right),
            AnalysisReportComparison::Different {
                field: "library_versions"
            }
        );
    }

    #[test]
    fn report_capture_rejects_equal_source_foreign_output_attachment() {
        let mut report = actual_report();
        let foreign = actual_report();
        report.analysis.command_realm = foreign.analysis.command_realm;
        assert_eq!(
            report.current_capture().err(),
            Some(AnalysisReportUnavailable::CurrentOwnerUnavailable)
        );
    }

    #[test]
    fn report_capture_requires_its_actual_source_build_request() {
        let mut report = actual_report();
        let foreign = actual_report();
        report.analysis.optimiser_source_origin = foreign.analysis.optimiser_source_origin;
        assert_eq!(
            report.current_capture().err(),
            Some(AnalysisReportUnavailable::CurrentOwnerUnavailable)
        );
    }

    #[test]
    fn report_capture_rejects_changed_producer_facts_and_unavailable_output() {
        let mut report = actual_report();
        report
            .analysis
            .object_handle_facts
            .any_scope
            .insert("changed".into(), Default::default());
        assert_eq!(
            report.current_capture().err(),
            Some(AnalysisReportUnavailable::ChangedProducedFacts)
        );
        let mut unavailable = actual_report();
        unavailable.analysis.body_lexer_config = None;
        assert_eq!(
            unavailable.current_capture().err(),
            Some(AnalysisReportUnavailable::CurrentOwnerUnavailable)
        );
    }

    #[test]
    fn completed_report_request_cannot_capture_a_later_ordinary_analysis() {
        let mut analyser = Analyser::new();
        let report = analyser.analyse_for_report("set value VALUE", "tcl8.6");
        assert!(report.current_capture().is_ok());
        assert!(analyser.authenticated_report_request.is_none());
        let ordinary = analyser.analyse("set other OTHER", "tcl8.6");
        assert!(analyser.authenticated_report_request.is_none());
        assert!(report.current_capture().is_ok());
        assert!(report.analysis().resolved_input.is_some());
        assert!(!ordinary.all_variables.is_empty());
    }
}
