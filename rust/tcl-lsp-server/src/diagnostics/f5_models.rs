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

//! Reports for BIG-IP configuration and iApp APL documents.

use super::*;

/// F5 dialect dispatch: BIG-IP config and iApp APL presentation documents are
/// not Tcl source — they have model-level validators (`BIGIP6001`-`6011`,
/// `IAPP7001`-`7003`) rather than the Tcl analyser.  Compute and publish their
/// diagnostics here, before the analyser.  Returns `Some(true)` (settled) when
/// the document is an F5 model dialect; `None` to continue to the Tcl analyser.
pub(super) async fn run_diagnostics_f5_dialect(
    delivery: &DeliveryCtx<'_>,
    inputs: &LiftInputs<'_>,
    analysis_text: &str,
    language_id: &str,
) -> Option<bool> {
    if !Backend::is_bigip_dialect(inputs.dialect.name) && !is_apl_source(delivery.uri, language_id)
    {
        return None;
    }
    let diags = f5_model_document_report(
        &F5ReportInputs {
            uri: delivery.uri,
            text: inputs.text,
            analysis_text,
            decode_report: inputs.decode_report.as_ref(),
            dialect: inputs.dialect,
            language_id,
            policy_layers: inputs.policy_layers,
        },
        delivery.documents,
        delivery.store.as_ref(),
    )
    .await;
    // Publish only when this version is still current (the same revision
    // guard the analyser path applies before publishing), keep the
    // pull-diagnostic cache in lock-step, and complete the pass with the
    // delivered report so `textDocument/diagnostic` answers with this set.
    match delivery.delivered_non_tcl_report(diags).await {
        Some(result_id) => {
            delivery
                .complete_f5_model_pass(inputs.text, result_id)
                .await;
        }
        None => {
            delivery
                .diagnostic_pass_unavailable(DiagnosticUnavailable::Stale)
                .await;
        }
    }
    Some(true)
}

/// One F5 model document snapshot (BIG-IP configuration or iApp APL) and the
/// policy its report is read under.
pub(in super::super) struct F5ReportInputs<'a> {
    pub(in super::super) uri: &'a Uri,
    pub(in super::super) text: &'a str,
    pub(in super::super) analysis_text: &'a str,
    pub(in super::super) decode_report: Option<&'a tcl_lsp_core::source_decode::DecodeReport>,
    pub(in super::super) dialect: &'static tcl_dialect::DialectProfile,
    pub(in super::super) language_id: &'a str,
    pub(in super::super) policy_layers: &'a PolicyLayers,
}

/// The F5 model report for one document snapshot — the single report code
/// the push publisher and a direct `textDocument/diagnostic` answer share.
pub(in super::super) async fn f5_model_document_report(
    inputs: &F5ReportInputs<'_>,
    documents: &DocumentStore,
    store: &dyn vfs::SourceStore,
) -> Vec<tower_lsp_server::ls_types::Diagnostic> {
    // A non-text byte signature makes every parser/model verdict untrustworthy.
    // Skip the validator entirely rather than computing findings the policy
    // step's abstention would then hide.
    let produced = if tcl_lsp_core::source_decode::should_abstain(inputs.decode_report) {
        Vec::new()
    } else {
        f5_dialect_diagnostics(
            inputs.uri,
            inputs.analysis_text,
            inputs.dialect,
            inputs.language_id,
            documents,
            store,
        )
        .await
        .unwrap_or_default()
    };
    f5_model_report(
        &F5ModelDocument {
            text: inputs.text,
            analysis_text: inputs.analysis_text,
            decode_report: inputs.decode_report,
            dialect: inputs.dialect,
        },
        inputs.policy_layers,
        produced,
    )
}

/// The report of a non-Tcl F5 model document — BIG-IP configuration or iApp
/// APL — on the wire: the validator's findings plus the byte-integrity codes
/// and W305, under the document's policy. These families never run the Tcl
/// analyser, so the directives are scanned here rather than read off an
/// analysis, and no analyser skip is declared: the validators compute every
/// code and the policy step decides what shows.
pub(in super::super) fn f5_model_report(
    doc: &F5ModelDocument<'_>,
    layers: &PolicyLayers,
    produced: Vec<core_policy::Finding>,
) -> Vec<tower_lsp_server::ls_types::Diagnostic> {
    let source = core_report::DocumentSource {
        text: doc.text,
        analysis_text: doc.analysis_text,
        decode: doc.decode_report,
        dialect: doc.dialect,
        pass: core_report::SourcePass::IntegrityOnly,
    };
    lifted_report(
        &source,
        produced,
        layers,
        core_policy::Directives::scan(doc.text, doc.dialect),
        None,
    )
}

/// Default BIG-IP partition assumed when a config carries no explicit
/// one.
const BIGIP_DEFAULT_PARTITION: &str = "Common";

/// The BIG-IP config / iApp model validators' output as findings. A
/// [`tcl_bigip::validator::ConfigDiagnostic`] carries an **inclusive** end
/// offset; the conversion makes it exclusive, as every other producer's is.
/// A spelling the catalogue lacks is a conversion failure and is dropped.
pub(in super::super) fn model_findings(
    diagnostics: &[tcl_bigip::validator::ConfigDiagnostic],
) -> Vec<core_policy::Finding> {
    diagnostics
        .iter()
        .filter_map(|d| core_policy::Finding::try_from(d).ok())
        .collect()
}

/// BIG-IP config findings (`BIGIP6001`–
/// `BIGIP6012`).  BIG-IP `.conf` text is not Tcl source — it has its own
/// model-level validator
/// ([`tcl_bigip::validator::validate_bigip_source`]). A conversion; the
/// policy step decides what shows.
pub(in super::super) fn bigip_config_findings(text: &str) -> Vec<core_policy::Finding> {
    model_findings(&tcl_bigip::validator::validate_bigip_source(
        text,
        BIGIP_DEFAULT_PARTITION,
    ))
}

/// iApp APL presentation diagnostics
/// (`IAPP7001`–`IAPP7003`).  Parses the APL presentation, optionally
/// cross-checks it against the sibling implementation's
/// `$::section__field` references, and lifts the validator output.  The
/// validator is gated on the `f5-iapps` dialect (we only reach here for
/// APL sources, so the gate is always satisfied — see [`is_apl_source`]).
pub(in super::super) fn apl_presentation_findings(
    text: &str,
    impl_var_refs: Option<&[tcl_bigip::apl::IappVarRef]>,
) -> Vec<core_policy::Finding> {
    let model = tcl_bigip::apl::parse_apl(text);
    model_findings(&tcl_bigip::apl::validate_iapp_presentation(
        &model,
        impl_var_refs,
        tcl_lsp_core::profile_for_dialect(IAPPS_DIALECT),
    ))
}

/// Whether `uri` (with its editor `language_id`) is an iApp APL
/// presentation document: an explicit APL
/// language id, or a basename of `*.apl` / `presentation`.
pub(in super::super) fn is_apl_source(uri: &Uri, language_id: &str) -> bool {
    if matches!(
        language_id.to_ascii_lowercase().as_str(),
        "tcl-apl" | "apl-lang" | "apl"
    ) {
        return true;
    }
    let path = uri.as_str();
    let basename = path.rsplit('/').next().unwrap_or(path);
    if basename.eq_ignore_ascii_case("presentation") {
        return true;
    }
    Path::new(basename)
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("apl"))
}

/// Locate the iApp *implementation* that pairs with the APL presentation
/// at `uri` and extract its `$::section__field` variable references.
/// Prefers an open buffer in
/// the same directory (so unsaved edits to the implementation are
/// reflected), then falls back to reading a sibling from disk.  The sibling
/// is named `implementation` or carries a `.iapp` / `.iappimpl` / `.impl`
/// extension.
async fn configured_iapp_source_references(
    source: String,
) -> Option<tcl_bigip::apl::IappSourceReferences> {
    crate::rt::spawn_blocking(move || {
        let environment = tcl_registry::model::ingress::resolve_environment(IAPPS_DIALECT);
        tcl_bigip::apl::extract_iapp_var_refs_with_context(
            &source,
            environment.analyser_profile(),
            environment.default_context_registry(),
            tcl_lexer::LexerConfig::from_grammar(environment.grammar()),
        )
    })
    .await
    .ok()
}

async fn find_sibling_impl_vars(
    uri: &Uri,
    documents: &DocumentStore,
    store: &dyn vfs::SourceStore,
) -> Option<tcl_bigip::apl::IappSourceReferences> {
    let path = uri.document_path()?;
    let dir = path.parent()?.to_path_buf();
    let is_impl_name = |p: &Path| {
        p.file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n.eq_ignore_ascii_case("implementation"))
            || p.extension().and_then(|e| e.to_str()).is_some_and(|ext| {
                matches!(
                    ext.to_ascii_lowercase().as_str(),
                    "iapp" | "iappimpl" | "impl"
                )
            })
    };

    // 1. Prefer an open implementation buffer in the same directory: an unsaved
    //    edit to the implementation must drive the presentation's cross-file
    //    diagnostics, not a stale on-disk copy.
    let open_source = {
        let docs = documents.lock("find_sibling_impl_vars").await;
        docs.iter().find_map(|(doc_uri, doc)| {
            if doc_uri == uri {
                return None;
            }
            let doc_path = doc_uri.document_path()?;
            (doc_path.parent() == Some(dir.as_path()) && is_impl_name(&doc_path))
                .then(|| doc.text.to_string())
        })
    };
    if let Some(source) = open_source {
        return configured_iapp_source_references(source).await;
    }

    // 2. Fall back to a sibling on disk — the literal `implementation` file
    //    first, then any `.iapp` / `.iappimpl` / `.impl` file (sorted for a
    //    deterministic pick).
    let impl_path = dir.join("implementation");
    if store.is_file(&impl_path)
        && let Ok(content) = store.read_to_string(&impl_path)
    {
        return configured_iapp_source_references(content).await;
    }
    let mut ext_candidates: Vec<PathBuf> = store
        .read_dir(&dir)
        .ok()?
        .into_iter()
        .map(|e| e.path)
        .filter(|p| {
            *p != path
                && p.extension().and_then(|s| s.to_str()).is_some_and(|ext| {
                    matches!(
                        ext.to_ascii_lowercase().as_str(),
                        "iapp" | "iappimpl" | "impl"
                    )
                })
        })
        .collect();
    ext_candidates.sort();
    for cand in ext_candidates {
        if let Ok(content) = store.read_to_string(&cand) {
            return configured_iapp_source_references(content).await;
        }
    }
    None
}

/// File-type dispatch: if `uri` is a non-Tcl F5 dialect document
/// (BIG-IP config or iApp APL presentation), compute its model-level
/// validator diagnostics; otherwise return `None` so the caller runs the
/// normal Tcl analyser path.  The validator runs on
/// `spawn_blocking` for the same parser-panic containment the analyser path
/// uses.
pub(in super::super) async fn f5_dialect_diagnostics(
    uri: &Uri,
    text: &str,
    dialect: &'static tcl_dialect::DialectProfile,
    language_id: &str,
    documents: &DocumentStore,
    store: &dyn vfs::SourceStore,
) -> Option<Vec<core_policy::Finding>> {
    if Backend::is_bigip_dialect(dialect.name) {
        let t = text.to_owned();
        let diags = crate::rt::spawn_blocking(move || bigip_config_findings(&t))
            .await
            .unwrap_or_default();
        return Some(diags);
    }
    if is_apl_source(uri, language_id) {
        let impl_refs = find_sibling_impl_vars(uri, documents, store).await;
        let t = text.to_owned();
        let diags = crate::rt::spawn_blocking(move || {
            apl_presentation_findings(
                &t,
                impl_refs
                    .as_ref()
                    .and_then(tcl_bigip::apl::IappSourceReferences::complete_references),
            )
        })
        .await
        .unwrap_or_default();
        return Some(diags);
    }
    None
}
