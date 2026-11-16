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

//! iApp and configuration diagnostics gathered for the report.

use crate::query::Source;
use serde_json::{Map, Value as J};
use std::collections::HashMap;

/// Whether `uri` names an iApp presentation source. The LSP recognises the
/// same conventional names; reports use the URI because a UCS/SCF source has
/// no separate language-id field.
fn is_iapp_presentation_uri(uri: &str) -> bool {
    let basename = uri.rsplit('/').next().unwrap_or(uri);
    basename.eq_ignore_ascii_case("presentation")
        || basename
            .rsplit_once('.')
            .is_some_and(|(_, ext)| ext.eq_ignore_ascii_case("apl"))
}

/// Whether `uri` names an iApp implementation that can be paired with a
/// presentation in the same directory.
fn is_iapp_implementation_uri(uri: &str) -> bool {
    let basename = uri.rsplit('/').next().unwrap_or(uri);
    basename.eq_ignore_ascii_case("implementation")
        || basename.rsplit_once('.').is_some_and(|(_, ext)| {
            matches!(
                ext.to_ascii_lowercase().as_str(),
                "iapp" | "iappimpl" | "impl"
            )
        })
}

fn uri_directory(uri: &str) -> &str {
    uri.rsplit_once('/').map_or("", |(dir, _)| dir)
}

fn iapp_peer<'a>(uri: &str, sources: &'a [Source], presentation: bool) -> Option<&'a str> {
    sources
        .iter()
        .find(|(candidate_uri, _)| {
            uri_directory(candidate_uri) == uri_directory(uri)
                && if presentation {
                    is_iapp_implementation_uri(candidate_uri)
                } else {
                    is_iapp_presentation_uri(candidate_uri)
                }
        })
        .map(|(_, source)| source.as_str())
}

/// Evidence describing whether a report had the complete iApp pair needed for
/// cross-file validation. It is deliberately data, rather than a silent
/// absence of IAPP7001/7002, so a reviewer can distinguish "clean" from
/// "could not inspect".
#[must_use]
pub fn collect_iapp_diagnostic_evidence(uri: &str, sources: &[Source]) -> J {
    let presentation = is_iapp_presentation_uri(uri);
    let implementation = is_iapp_implementation_uri(uri);
    let peer = if presentation || implementation {
        iapp_peer(uri, sources, presentation)
    } else {
        None
    };
    let implementation_source = if presentation {
        peer
    } else if implementation {
        sources
            .iter()
            .find(|(source_uri, _)| source_uri == uri)
            .map(|(_, source)| source.as_str())
    } else {
        None
    };
    let covered = implementation_source
        .map(configured_iapp_source_references)
        .is_some_and(|references| references.complete_references().is_some());
    match (presentation, implementation, peer.is_some(), covered) {
        (true, false, true, true) => serde_json::json!({
            "state": "complete",
            "message": "Presentation and implementation were supplied; cross-file iApp checks are complete.",
        }),
        (true, false, false, _) => serde_json::json!({
            "state": "presentation_only",
            "message": "Only the iApp presentation was supplied; implementation-reference checks (IAPP7001 and IAPP7002) could not be evaluated.",
        }),
        (false, true, true, true) => serde_json::json!({
            "state": "complete",
            "message": "Implementation and presentation were supplied; cross-file iApp checks are complete.",
        }),
        (false, true, false, _) => serde_json::json!({
            "state": "implementation_only",
            "message": "Only the iApp implementation was supplied; presentation-field checks (IAPP7001) could not be evaluated.",
        }),
        (_, _, true, false) => serde_json::json!({
            "state": "partial_source",
            "message": "Both files were supplied; unknown source entry or naming leaves unused-field checks unavailable.",
        }),
        _ => J::Null,
    }
}

fn f5_diagnostic_json(diagnostic: &tcl_bigip::validator::ConfigDiagnostic) -> J {
    use tcl_bigip::validator::DiagSeverity;

    serde_json::json!({
        "code": diagnostic.code,
        "message": diagnostic.message,
        "severity": match diagnostic.severity {
            DiagSeverity::Warning => "warning",
            DiagSeverity::Hint => "hint",
        },
        "line": diagnostic.range.start.line + 1,
        "column": diagnostic.range.start.character + 1,
        "tab": diagnostic.subject.report_tab(),
    })
}

/// Run the shared BIG-IP config validator and expose every finding to the
/// report. The complete list is retained as `configDiagnostics`; routed lists
/// let the template render findings in the tab where they are actionable.
#[must_use]
pub fn collect_config_diagnostics(source: &str) -> J {
    J::Array(
        tcl_bigip::validator::validate_bigip_source(source, "Common")
            .iter()
            .map(f5_diagnostic_json)
            .collect(),
    )
}

/// Collect model and iApp diagnostics for one report source. The conventional
/// iApp presentation/implementation filenames let a multi-file report perform
/// every IAPP7001–7003 check offline; partial input remains explicit through
/// [`collect_iapp_diagnostic_evidence`].
#[must_use]
fn configured_iapp_source_references(source: &str) -> tcl_bigip::apl::IappSourceReferences {
    let environment = tcl_registry::model::ingress::resolve_environment("f5-iapps");
    tcl_bigip::apl::extract_iapp_var_refs_with_context(
        source,
        environment.analyser_profile(),
        environment.default_context_registry(),
        tcl_lexer::LexerConfig::from_grammar(environment.grammar()),
    )
}

pub fn collect_report_diagnostics(uri: &str, source: &str, sources: &[Source]) -> J {
    let mut diagnostics = tcl_bigip::validator::validate_bigip_source(source, "Common");
    if is_iapp_presentation_uri(uri) {
        let model = tcl_bigip::apl::parse_apl(source);
        let refs = iapp_peer(uri, sources, true).map(configured_iapp_source_references);
        diagnostics.extend(tcl_bigip::apl::validate_iapp_presentation(
            &model,
            refs.as_ref()
                .and_then(tcl_bigip::apl::IappSourceReferences::complete_references),
            tcl_registry::model::ingress::resolve_environment("f5-iapps").analyser_profile(),
        ));
    } else if is_iapp_implementation_uri(uri)
        && let Some(presentation) = iapp_peer(uri, sources, false)
    {
        let model = tcl_bigip::apl::parse_apl(presentation);
        let refs = configured_iapp_source_references(source);
        diagnostics.extend(tcl_bigip::apl::validate_iapp_implementation(
            refs.references(),
            Some(&model),
            tcl_registry::model::ingress::resolve_environment("f5-iapps").analyser_profile(),
        ));
    }
    J::Array(diagnostics.iter().map(f5_diagnostic_json).collect())
}

pub(super) fn add_config_diagnostics(
    device: &mut Map<String, J>,
    uri: &str,
    source: &str,
    sources: &[Source],
) {
    let J::Array(all) = collect_report_diagnostics(uri, source, sources) else {
        unreachable!("config diagnostics are always an array")
    };
    let mut routed: HashMap<String, Vec<J>> = HashMap::new();
    for item in &all {
        let tab = item.get("tab").and_then(J::as_str).unwrap_or("virtuals");
        routed.entry(tab.to_owned()).or_default().push(item.clone());
    }

    for subject in tcl_bigip::validator::ConfigDiagnosticSubject::ALL {
        let tab = subject.report_tab();
        let key = match tab {
            "virtuals" => "virtualDiagnostics",
            "rules" => "ruleDiagnostics",
            "pools" => "poolDiagnostics",
            "dataGroups" => "dataGroupDiagnostics",
            "apps" => "appDiagnostics",
            "objectIndex" => "objectDiagnostics",
            _ => unreachable!("every configuration diagnostic subject has a report tab"),
        };
        device.insert(key.into(), J::Array(routed.remove(tab).unwrap_or_default()));
    }
    device.insert("configDiagnostics".into(), J::Array(all));
    device.insert(
        "iappDiagnosticEvidence".into(),
        collect_iapp_diagnostic_evidence(uri, sources),
    );
}
