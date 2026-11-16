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

//! BIG-IP configuration and iApp APL reports.

use super::*;

/// The F5 model report of `src` under an editor layer, with no bytes.
fn f5_model_set(
    src: &str,
    dialect: &str,
    editor: serde_json::Value,
    produced: Vec<core_policy::Finding>,
) -> Vec<tower_lsp_server::ls_types::Diagnostic> {
    let layers = PolicyLayers {
        editor,
        ..PolicyLayers::default()
    };
    f5_model_report(
        &F5ModelDocument {
            text: src,
            analysis_text: src,
            decode_report: None,
            dialect: tcl_lsp_core::profile_for_dialect(dialect),
        },
        &layers,
        produced,
    )
}

/// A BIG-IP config that references a pool not defined in the config
/// surfaces `BIGIP6002` through the validator's findings and the F5
/// report, tagged `tcl-lsp`.
#[test]
fn bigip_config_findings_surface_codes() {
    let src =
        "ltm rule /Common/r {\n  when HTTP_REQUEST {\n    pool /Common/no_such_pool\n  }\n}\n";
    let findings = bigip_config_findings(src);
    assert!(
        findings.iter().any(|f| f.code == DiagCode::Bigip6002),
        "expected BIGIP6002, got: {findings:?}",
    );
    let diags = f5_model_set(src, "f5-bigip", serde_json::json!({}), findings);
    let codes = diag_codes(&diags);
    assert!(
        codes.iter().any(|c| c == "BIGIP6002"),
        "expected BIGIP6002 on the wire, got: {codes:?}",
    );
    assert!(diags.iter().all(|d| d.source.as_deref() == Some("tcl-lsp")));
}

/// A disabled BIG-IP code (`tclLsp.diagnostics.BIGIP6002 = false`) is
/// hidden by the report.
#[test]
fn the_f5_report_honours_a_disabled_model_code() {
    let src =
        "ltm rule /Common/r {\n  when HTTP_REQUEST {\n    pool /Common/no_such_pool\n  }\n}\n";
    let codes = diag_codes(&f5_model_set(
        src,
        "f5-bigip",
        serde_json::json!({ "diagnostics": { "BIGIP6002": false } }),
        bigip_config_findings(src),
    ));
    assert!(
        !codes.iter().any(|c| c == "BIGIP6002"),
        "BIGIP6002 should be hidden, got: {codes:?}",
    );
}

/// An iApp APL presentation field never referenced by the (cross-file)
/// implementation surfaces `IAPP7002`.
#[test]
fn apl_presentation_findings_surface_codes() {
    let refs = tcl_bigip::apl::extract_iapp_var_refs("set x $::basic__addr");
    let src = "section basic {\n  string addr\n  string port\n}\n";
    let findings = apl_presentation_findings(src, Some(&refs));
    assert!(
        findings.iter().any(|f| f.code.as_str() == "IAPP7002"),
        "expected IAPP7002 for the unreferenced 'port' field, got: {findings:?}",
    );
    let diags = f5_model_set(src, "f5-iapps", serde_json::json!({}), findings);
    assert!(diag_codes(&diags).iter().any(|c| c == "IAPP7002"));
    assert!(diags.iter().all(|d| d.source.as_deref() == Some("tcl-lsp")));
}

/// `is_apl_source` matches by APL language id or `*.apl` / `presentation`
/// basename.
#[test]
fn is_apl_source_detects_apl_documents() {
    let apl_ext = Uri::from_str("file:///app/iapp/foo.apl").unwrap();
    let presentation = Uri::from_str("file:///app/iapp/presentation").unwrap();
    let plain = Uri::from_str("file:///app/util.tcl").unwrap();
    assert!(is_apl_source(&apl_ext, "tcl"));
    assert!(is_apl_source(&presentation, "tcl"));
    // An explicit APL language id wins regardless of basename.
    assert!(is_apl_source(&plain, "tcl-apl"));
    // A plain Tcl document is not an APL source.
    assert!(!is_apl_source(&plain, "tcl"));
}

/// A `tcl_bigip` validator range carries an *inclusive* end; the
/// conversion to a finding makes it exclusive, so the wire range ends
/// one past the last character.
#[test]
fn a_model_finding_lifts_with_an_exclusive_end() {
    let src = "ltm rule /Common/r {\n  x\n}\n";
    // `x` is byte 23: line 1, character 2.
    let at_x = tcl_bigip::Position {
        line: 1,
        character: 2,
        offset: 23,
    };
    let d = tcl_bigip::validator::ConfigDiagnostic {
        code: "BIGIP6002".to_owned(),
        message: "x".to_owned(),
        severity: tcl_bigip::validator::DiagSeverity::Warning,
        subject: tcl_bigip::validator::ConfigDiagnosticSubject::IRule,
        range: tcl_bigip::Range {
            start: at_x,
            end: at_x,
        },
    };
    let diags = f5_model_set(src, "f5-bigip", serde_json::json!({}), model_findings(&[d]));
    let lifted = diags
            .iter()
            .find(|d| matches!(&d.code, Some(tower_lsp_server::ls_types::NumberOrString::String(c)) if c == "BIGIP6002"))
            .expect("the model finding is shown");
    assert_eq!(lifted.range.start.line, 1);
    assert_eq!(lifted.range.start.character, 2);
    assert_eq!(lifted.range.end.line, 1);
    assert_eq!(lifted.range.end.character, 3);
    assert_eq!(
        lifted.severity,
        Some(tower_lsp_server::ls_types::DiagnosticSeverity::WARNING)
    );
}

/// End-to-end: the pull path's [`Backend::full_diagnostics_for`] routes a
/// `f5-bigip` document to the BIG-IP validator instead of the Tcl
/// analyser, so the editor receives `BIGIP6xxx` codes.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn full_diagnostics_for_routes_bigip_to_validator() {
    let backend = test_backend();
    let uri = Uri::from_str("file:///Common/bigip.conf").unwrap();
    let src =
        "ltm rule /Common/r {\n  when HTTP_REQUEST {\n    pool /Common/no_such_pool\n  }\n}\n";
    let diags = backend
        .full_diagnostics_for(&uri, Arc::from(src), "f5-bigip".to_owned(), "tcl-bigip")
        .await
        .unwrap();
    let codes = diag_codes(&diags);
    assert!(
        codes.iter().any(|c| c == "BIGIP6002"),
        "expected BIGIP6002 via the pull path, got: {codes:?}",
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn f5_pull_paths_compose_byte_and_bidi_integrity_with_model_diagnostics() {
    for (uri_text, dialect, language_id, model_code, mut bytes) in [
            (
                "file:///Common/bigip.conf",
                "f5-bigip",
                "tcl-bigip",
                "BIGIP6002",
                "ltm rule /Common/r {\n  when HTTP_REQUEST {\n    # \u{202e}hidden\n    pool /Common/no_such_pool\n  }\n}\n"
                    .as_bytes()
                    .to_vec(),
            ),
            (
                "file:///app/presentation.apl",
                "f5-iapps",
                "tcl-apl",
                "IAPP7003",
                "# \u{202e}hidden\n#include \"missing.apl\"\nsection basic {\n  string addr\n}\n"
                    .as_bytes()
                    .to_vec(),
            ),
        ] {
            // Keep the malformed byte inside a comment. The model remains
            // structurally readable, proving that W107 composes with the
            // dialect validator instead of replacing it.
            let insert_at = bytes
                .windows(b"hidden".len())
                .position(|window| window == b"hidden")
                .map(|start| start + b"hidden".len())
                .unwrap();
            bytes.insert(insert_at, 0xff);
            let (text, report) = tcl_lsp_core::source_decode::decode_source(&bytes);
            let backend = test_backend();
            let uri = Uri::from_str(uri_text).unwrap();
            let mut doc = DocumentState::with_version(text.clone(), dialect.to_owned(), 1)
                .with_language_id(language_id.to_owned());
            doc.decode_report = Some(report);
            backend.documents.lock("test").await.insert(uri.clone(), doc);
            let diagnostics = backend
                .full_diagnostics_for(
                    &uri,
                    Arc::from(text),
                    dialect.to_owned(),
                    language_id,
                )
                .await.unwrap();
            let codes = diag_codes(&diagnostics);
            assert!(codes.iter().any(|c| c == "W107"), "{codes:?}");
            assert!(codes.iter().any(|c| c == "W305"), "{codes:?}");
            assert!(codes.iter().any(|c| c == model_code), "{codes:?}");
        }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn publish_analyser_diagnostics_routes_bigip_dialect_to_pull_cache() {
    // A BIG-IP document takes the `f5_dialect_diagnostics` branch of
    // `run_diagnostics_core` (model validators, not the Tcl analyser) and
    // still lands a pull-cache entry.
    let backend = test_backend();
    let uri = Uri::from_str("file:///life.conf").unwrap();
    let src = "ltm pool /Common/p {\n    members {\n        /Common/n:80 { }\n    }\n}\n";
    backend.documents.lock("test").await.insert(
        uri.clone(),
        DocumentState::new(src.to_owned(), "f5-bigip".to_owned()),
    );
    backend
        .publish_analyser_diagnostics(
            uri.clone(),
            src.to_owned(),
            "f5-bigip".to_owned(),
            0,
            Some(1),
        )
        .await;
    assert!(
        backend.pull_diag_cache.lock().await.contains_key(&uri),
        "BIG-IP dialect should populate the pull cache via the f5 branch",
    );
}
