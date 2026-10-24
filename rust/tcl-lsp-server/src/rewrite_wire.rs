// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Original optimiser report/source association for diagnostic presentation.

#[cfg(test)]
use super::DiagCode;
use super::{
    AnalysisResult, Arc, DiagnosticPass, HashMap, Uri, core_code_actions, core_policy, core_report,
};

/// The actual report producer's source purpose, never decoded from wire data.
#[derive(Clone)]
pub(super) enum RewriteWireSource {
    Document(Arc<DiagnosticPass>),
    Proposal {
        uri: Uri,
        version: i32,
        original_source: Arc<str>,
        proposal: Arc<str>,
        revision: u64,
        inputs_epoch: u64,
    },
}

impl RewriteWireSource {
    fn association(
        &self,
        doc: &core_report::DocumentSource<'_>,
        analysis: &AnalysisResult,
    ) -> Option<serde_json::Value> {
        if tcl_lexer::normalise_lone_cr(doc.text).as_ref() != doc.analysis_text {
            return None;
        }
        match self {
            Self::Document(pass) => pass.rewrite_source_association(doc.text, analysis),
            Self::Proposal {
                uri,
                version,
                original_source,
                proposal,
                revision,
                inputs_epoch,
            } => {
                if proposal.as_ref() != doc.text
                    || tcl_compiler::source_graph::current_analysis(doc.analysis_text, analysis)
                        .is_none()
                {
                    return None;
                }
                Some(serde_json::json!({
                    "schema": 1,
                    "purpose": "proposed-source-optimiser-rewrite",
                    "uri": uri,
                    "version": version,
                    "originalSource": original_source.as_ref(),
                    "source": doc.text,
                    "analysisSource": doc.analysis_text,
                    "revision": revision.to_string(),
                    "inputsEpoch": inputs_epoch.to_string(),
                    "dialectId": analysis.resolved_input.as_ref()?.unit_profile().name,
                }))
            }
        }
    }
}

/// Serialize only a complete independently issued worker group for this source.
/// The JSON is presentation data. Existing action/transform owners remain the
/// only consumers that can grant an edit after their own currentness checks.
pub(super) fn original_rewrite_payloads(
    doc: &core_report::DocumentSource<'_>,
    report: &core_policy::Report,
    analysis: Option<&AnalysisResult>,
    source: Option<&RewriteWireSource>,
) -> HashMap<*const core_policy::Finding, serde_json::Value> {
    let mut payloads = HashMap::new();
    let Some((analysis, source)) = analysis.zip(source) else {
        return payloads;
    };
    let Some(guard) =
        core_code_actions::DiagnosticEditSource::for_analysis(doc.analysis_text, analysis)
    else {
        return payloads;
    };
    let Some(association) = source.association(doc, analysis) else {
        return payloads;
    };
    for rewrite in report.applicable_rewrites() {
        if !guard.matches_optimiser_rewrite_group(&rewrite.members) {
            continue;
        }
        let edits = rewrite
            .members
            .iter()
            .map(|finding| {
                let (replacement, _, _) = finding.rewrite_payload()?;
                Some(serde_json::json!({
                    "replacement": replacement,
                    "startOffset": finding.span.start(),
                    "endOffset": finding.span.end(),
                }))
            })
            .collect::<Option<Vec<_>>>();
        let Some(edits) = edits else { continue };
        let payload = if let Some(group) = rewrite.group {
            serde_json::json!({
                "group": group,
                "edits": edits,
                "sourceAssociation": association,
            })
        } else {
            let [edit] = edits.as_slice() else { continue };
            let mut edit = edit.clone();
            edit["sourceAssociation"] = association.clone();
            edit
        };
        payloads.extend(
            rewrite
                .members
                .iter()
                .map(|finding| (std::ptr::from_ref(*finding), payload.clone())),
        );
    }
    payloads
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_compiler::analyser::Analyser;
    use tcl_compiler::compilation_unit::{CompilationUnit, UnitBuildOptions};
    use tcl_compiler::optimiser::optimise_unit;

    const SOURCE: &str = "proc p {y} {\n    set x [llength $y]\n    puts $x\n}\n";

    fn produced() -> (AnalysisResult, Vec<core_policy::Finding>) {
        let mut analyser = Analyser::new();
        let input = analyser.prepare_analysis_input("tcl8.6").unwrap();
        let registry = input.borrowed_context_registry().commands();
        let unit = CompilationUnit::build_standalone_with_analysis_input(
            SOURCE,
            UnitBuildOptions {
                registry,
                defer_top_level: false,
                config: input.lexer_config(),
                dialect: Some(input.unit_profile()),
                external_call_sites: None,
                declared_commands: None,
            },
            &input,
        )
        .unwrap()
        .with_interprocedural(registry, Some(input.unit_profile()));
        let findings: Vec<_> = optimise_unit(&unit, registry, Some(input.unit_profile()))
            .into_iter()
            .filter(|member| member.code == DiagCode::O127)
            .map(core_policy::Finding::from)
            .collect();
        assert_eq!(findings.len(), 2);
        let mut analyser = Analyser::new().with_resolved_input(input);
        analyser.set_cu_override(Arc::new(unit));
        (analyser.analyse(SOURCE, "tcl8.6"), findings)
    }

    fn doc(text: &str) -> core_report::DocumentSource<'_> {
        core_report::DocumentSource {
            text,
            analysis_text: text,
            decode: None,
            dialect: tcl_lsp_core::profile_for_dialect("tcl8.6"),
            pass: core_report::SourcePass::IntegrityOnly,
        }
    }

    fn proposal() -> RewriteWireSource {
        RewriteWireSource::Proposal {
            uri: "file:///rewrite-wire.tcl".parse().unwrap(),
            version: 7,
            original_source: Arc::from("puts previous\n"),
            proposal: Arc::from(SOURCE),
            revision: 17,
            inputs_epoch: 23,
        }
    }

    #[test]
    fn issued_whole_worker_group_has_explicit_proposal_source_on_every_member() {
        let (analysis, findings) = produced();
        let report = core_policy::apply(findings, &core_policy::Policy::unrestricted());
        let payloads =
            original_rewrite_payloads(&doc(SOURCE), &report, Some(&analysis), Some(&proposal()));
        assert_eq!(payloads.len(), 2);
        for payload in payloads.values() {
            assert_eq!(payload["edits"].as_array().unwrap().len(), 2);
            let association = &payload["sourceAssociation"];
            assert_eq!(association["purpose"], "proposed-source-optimiser-rewrite");
            assert_eq!(association["source"], SOURCE);
            assert_eq!(association["originalSource"], "puts previous\n");
            assert_eq!(association["version"], 7);
            assert_eq!(association["revision"], "17");
            assert_eq!(association["inputsEpoch"], "23");
        }
    }

    #[test]
    fn missing_owner_foreign_analysis_changed_bytes_and_partial_group_have_no_payload() {
        let (analysis, findings) = produced();
        let report = core_policy::apply(findings.clone(), &core_policy::Policy::unrestricted());
        let source = proposal();
        assert!(original_rewrite_payloads(&doc(SOURCE), &report, Some(&analysis), None).is_empty());
        assert!(original_rewrite_payloads(&doc(SOURCE), &report, None, Some(&source)).is_empty());
        let foreign = Analyser::new()
            .with_resolved_input(analysis.resolved_input.clone().unwrap())
            .analyse(SOURCE, "tcl8.6");
        assert!(
            original_rewrite_payloads(&doc(SOURCE), &report, Some(&foreign), Some(&source))
                .is_empty()
        );
        assert!(
            original_rewrite_payloads(
                &doc("puts changed\n"),
                &report,
                Some(&analysis),
                Some(&source)
            )
            .is_empty()
        );
        let partial = core_policy::apply(
            vec![findings[0].clone()],
            &core_policy::Policy::unrestricted(),
        );
        assert!(
            original_rewrite_payloads(&doc(SOURCE), &partial, Some(&analysis), Some(&source))
                .is_empty()
        );
    }
    #[test]
    fn manually_constructed_rewrite_cannot_acquire_source_association() {
        let (analysis, _) = produced();
        let manual = tcl_compiler::optimiser::Optimisation::new(
            DiagCode::O114,
            "manual",
            tcl_lexer::Span::new(0, 0),
            "incr x",
        );
        let report = core_policy::apply(vec![manual.into()], &core_policy::Policy::unrestricted());
        assert!(!report.applicable_rewrites().is_empty());
        assert!(
            original_rewrite_payloads(&doc(SOURCE), &report, Some(&analysis), Some(&proposal()))
                .is_empty()
        );
    }
    #[test]
    fn mixed_workers_and_mismatched_normalised_source_withhold_payloads() {
        let (analysis, first) = produced();
        let (_, second) = produced();
        let report = core_policy::apply(
            vec![first[0].clone(), second[1].clone()],
            &core_policy::Policy::unrestricted(),
        );
        assert!(
            original_rewrite_payloads(&doc(SOURCE), &report, Some(&analysis), Some(&proposal()))
                .is_empty()
        );
        let report = core_policy::apply(first, &core_policy::Policy::unrestricted());
        let mut mismatched = doc(SOURCE);
        mismatched.analysis_text = "puts different\n";
        assert!(
            original_rewrite_payloads(&mismatched, &report, Some(&analysis), Some(&proposal()))
                .is_empty()
        );
    }
}
