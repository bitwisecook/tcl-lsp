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

use super::*;

#[tokio::test]
async fn a_workspace_data_group_promotes_a_standalone_irule_signed_scan_hint() {
    use tower_lsp_server::ls_types::{DiagnosticSeverity, NumberOrString};

    let mut backend = crate::tests::test_backend();
    let store = Arc::new(crate::vfs::MemoryStore::new());
    backend.state_mut_for_test().store = store.clone();
    backend.workspace_scan_ready.mark_complete();
    *backend.workspace_folders.lock().await = vec![Uri::from_str("file:///signed-scan").unwrap()];

    let rule = Uri::from_str("file:///signed-scan/sample.irule").unwrap();
    let config = Uri::from_str("file:///signed-scan/bigip.conf").unwrap();
    let duplicate = Uri::from_str("file:///signed-scan/bigip_local.conf").unwrap();
    let rule_text = "when CLIENT_DATA {\n binary scan [TCP::payload] c item\n class match -- $item equals int_dg\n}";
    let configuration = |key| {
        format!(
            "ltm data-group internal /Common/int_dg {{ type integer\n records {{ {key} {{ data hit }} }} }}"
        )
    };
    let config_text = configuration(129);
    let duplicate_text = configuration(130);
    let edited_text = configuration(127);
    for (uri, path, text) in [
        (&rule, "/signed-scan/sample.irule", rule_text),
        (&config, "/signed-scan/bigip.conf", config_text.as_str()),
        (
            &duplicate,
            "/signed-scan/bigip_local.conf",
            duplicate_text.as_str(),
        ),
    ] {
        store.upsert_uri(uri.as_str(), path, text.as_bytes().to_vec());
    }
    let admission =
        |uri| Arc::new(crate::DocumentPathAdmission::capture(uri, store.as_ref()).unwrap());
    let rule_state = DocumentState::with_version(rule_text.to_owned(), "f5-irules".to_owned(), 1)
        .with_path_admission(admission(&rule));
    backend
        .documents
        .lock("workspace_signed_scan")
        .await
        .insert(rule.clone(), rule_state);
    backend
        .db_set_source(&rule, rule_text, "f5-irules".to_owned())
        .await
        .unwrap();
    let mut previous = None;
    let primary = (&config, admission(&config));
    let conflict = (&duplicate, admission(&duplicate));
    let (hint, warning) = (DiagnosticSeverity::HINT, DiagnosticSeverity::WARNING);
    for ((uri, path_admission), replacement, expected) in [
        (&primary, Some((&config_text, 1)), warning),
        (&conflict, Some((&duplicate_text, 1)), hint),
        (&conflict, None, warning),
        (&primary, Some((&edited_text, 2)), hint),
        (&primary, Some((&config_text, 3)), warning),
        (&primary, None, hint),
    ] {
        let uri = *uri;
        {
            let mut documents = backend
                .documents
                .lock("workspace_configuration_revision")
                .await;
            if let Some((text, version)) = replacement {
                documents.insert(
                    uri.clone(),
                    DocumentState::with_version(text.clone(), "f5-bigip".to_owned(), version)
                        .with_path_admission(Arc::clone(path_admission)),
                );
            } else {
                documents.remove(uri);
            }
        }
        if let Some(report) = previous.take() {
            assert!(matches!(
                backend
                    .publish_direct_diagnostic_report(&rule, report)
                    .await
                    .unwrap(),
                Err(DiagnosticUnavailable::Stale)
            ));
        }
        let report = backend
            .produce_direct_diagnostic_report(&rule)
            .await
            .unwrap();
        let signed = report
            .diagnostics
            .iter()
            .find(|diagnostic| {
                matches!(&diagnostic.code, Some(NumberOrString::String(code)) if code == "W156")
            })
            .expect("signed data-group finding");
        assert_eq!(signed.severity, Some(expected), "{uri:?}: {replacement:?}");
        assert_eq!(
            signed.message.contains("Configured key 129"),
            expected == warning,
        );
        previous = Some(report);
    }
}

#[tokio::test]
async fn a_direct_report_is_recomputed_after_initial_scheduler_admission() {
    let backend = crate::tests::test_backend();
    let expected = DiagnosticSourceExpectation {
        uri: Uri::from_str("file:///direct-initial-admission.tcl").unwrap(),
        version: 1,
        source: "set value 7\n".into(),
    };
    open_source(&backend, &expected).await;
    let initial = backend
        .produce_direct_diagnostic_report(&expected.uri)
        .await
        .unwrap();
    assert_eq!(initial.receipt.source.slot_generation, None);
    let mut slot = DiagSlot::default();
    slot.mark_dirty();
    backend
        .diag_slots
        .lock()
        .await
        .insert(expected.uri.clone(), slot);
    assert!(matches!(
        backend
            .publish_direct_diagnostic_report(&expected.uri, initial)
            .await
            .unwrap(),
        Err(DiagnosticUnavailable::Stale)
    ));
    // The scheduler has admitted generation 1 but has not drained its
    // dirty flag yet. A new receipt still cannot publish while pending.
    let pending = backend
        .produce_direct_diagnostic_report(&expected.uri)
        .await
        .unwrap();
    assert!(matches!(
        backend
            .publish_direct_diagnostic_report(&expected.uri, pending)
            .await
            .unwrap(),
        Err(DiagnosticUnavailable::Pending)
    ));
    // Model the real worker's drain boundary without changing generation.
    backend
        .diag_slots
        .lock()
        .await
        .get_mut(&expected.uri)
        .unwrap()
        .dirty = false;
    let report = backend
        .ensure_current_diagnostic_report(&expected.uri)
        .await
        .unwrap();
    assert_eq!(report.uri, expected.uri);
    let current = backend.pull_diag_cache.lock().await;
    let receipt = current
        .get(&expected.uri)
        .unwrap()
        .currency
        .as_ref()
        .unwrap();
    assert_eq!(receipt.source.slot_generation, Some(1));
}

#[tokio::test]
async fn successive_scheduler_changes_refuse_each_actual_direct_receipt() {
    let backend = crate::tests::test_backend();
    let expected = DiagnosticSourceExpectation {
        uri: Uri::from_str("file:///direct-repeated-admission.tcl").unwrap(),
        version: 1,
        source: "set value 7\n".into(),
    };
    open_source(&backend, &expected).await;
    for _ in 0..2 {
        let produced = backend
            .produce_direct_diagnostic_report(&expected.uri)
            .await
            .unwrap();
        backend
            .diag_slots
            .lock()
            .await
            .entry(expected.uri.clone())
            .or_default()
            .mark_dirty();
        assert!(matches!(
            backend
                .publish_direct_diagnostic_report(&expected.uri, produced)
                .await
                .unwrap(),
            Err(DiagnosticUnavailable::Stale)
        ));
    }
    assert!(
        !backend
            .pull_diag_cache
            .lock()
            .await
            .contains_key(&expected.uri)
    );
}

#[tokio::test]
async fn actual_direct_and_push_rewrites_retain_their_captured_document_source() {
    let backend = crate::tests::test_backend();
    backend
        .apply_global_config(&serde_json::json!({"optimiser": {"profile": "full"}}))
        .await
        .expect("the fixture configuration is valid");
    let expected = DiagnosticSourceExpectation {
        uri: Uri::from_str("file:///rewrite-wire-captured-source.tcl").unwrap(),
        version: 7,
        source: "proc p {y} {\n    set x [llength $y]\n    puts $x\n}\n".into(),
    };
    open_source(&backend, &expected).await;
    let direct = backend
        .produce_direct_diagnostic_report(&expected.uri)
        .await
        .unwrap_or_else(|_| panic!("direct report"));
    assert_document_rewrites(&direct.diagnostics, &expected);
    run_provider(&backend, &expected.uri).await;
    let CurrentDiagnostics::Complete { report } =
        backend.current_diagnostics(&expected).await.unwrap()
    else {
        panic!("actual completed push report");
    };
    assert_document_rewrites(&report.diagnostics, &expected);
}

async fn folder_registry_fixture() -> (
    Backend,
    DiagnosticSourceExpectation,
    tcl_lsp_db::AnalyserConfig,
) {
    let backend = crate::tests::test_backend();
    let packs = tcl_spectcl::pack::load_in_memory(vec![(
            tcl_spectcl::PackFile {
                tier: tcl_spectcl::Tier::Workspace,
                path: PathBuf::from("/diagnostic-registry/owned.tclspec"),
                origin: tcl_spectcl::discovery::Origin::DotDir,
                dependency_tier: None,
            },
            "speclib diagnostic-registry 1.0 {command registry_probe {arity 3; arg 0 -role Name; arg 1 -role ParamList; arg 2 -role Body; analyser_hook -native Proc; command_table_effect DefinesProcedure}}".to_owned(),
        )]);
    assert!(packs.notices.is_empty(), "{:?}", packs.notices);
    let _installed = tcl_spectcl::install::registry_for_dialect_with_packs("tcl8.6", &packs);
    let config = {
        let mut db = backend.db.lock().await;
        tcl_lsp_db::set_overlay_epoch(&mut db, tcl_registry::overlay_epoch());
        tcl_lsp_db::AnalyserConfig::new(
            &*db,
            Vec::new(),
            tcl_compiler::analyser::NonAsciiMode::Default,
            Vec::new(),
            None,
            None,
            packs.key,
            Vec::new(),
            Vec::new(),
        )
    };
    backend.folder_db_configs.lock().await.push((
        crate::canonical_file_uri("/diagnostic-registry").unwrap(),
        config,
    ));
    let expected = DiagnosticSourceExpectation {
        uri: crate::canonical_file_uri("/diagnostic-registry/source.tcl").unwrap(),
        version: 1,
        source: "registry_probe provided {one} {return $one}\n".into(),
    };
    open_source(&backend, &expected).await;
    (backend, expected, config)
}

#[tokio::test]
async fn diagnostic_metadata_and_pass_retain_the_actual_folder_registry() {
    let (backend, expected, _) = folder_registry_fixture().await;
    let inputs = backend
        .diag_inputs(&expected.uri, tcl_lsp_core::profile_for_dialect("tcl8.6"))
        .await;
    let job = inputs.capture_job(&expected.uri).await.unwrap();
    let pass = job.diagnostic_pass.as_ref().unwrap();
    let input = pass.input.as_ref().unwrap();
    assert!(inputs.registry.get("registry_probe").is_some());
    let analysis = tcl_compiler::analyser::Analyser::new()
        .with_resolved_input((**input).clone())
        .structure_only()
        .analyse(&expected.source, "tcl8.6");
    assert!(pass.matches_completed_analysis(&analysis));
    assert_eq!(
        inputs.registry.snapshot().semantic_key(),
        analysis
            .resolved_registry()
            .unwrap()
            .snapshot()
            .semantic_key()
    );
}

#[tokio::test]
async fn an_old_diagnostic_registry_cannot_cover_a_changed_folder_input() {
    use salsa::Setter as _;
    let (backend, expected, config) = folder_registry_fixture().await;
    let inputs = backend
        .diag_inputs(&expected.uri, tcl_lsp_core::profile_for_dialect("tcl8.6"))
        .await;
    {
        let mut db = backend.db.lock().await;
        config.set_spec_pack_key(&mut *db).to(0);
    }
    let job = inputs.capture_job(&expected.uri).await.unwrap();
    let pass = job.diagnostic_pass.as_ref().unwrap();
    let input = pass.input.as_ref().unwrap();
    let analysis = tcl_compiler::analyser::Analyser::new()
        .with_resolved_input((**input).clone())
        .structure_only()
        .analyse(&expected.source, "tcl8.6");
    assert_eq!(analysis.resolved_input.as_ref(), Some(input.as_ref()));
    assert!(!pass.matches_completed_analysis(&analysis));
}

#[tokio::test]
async fn an_invalid_source_capture_is_refreshed_by_an_ordinary_same_epoch_schedule() {
    let (backend, expected, _) = folder_registry_fixture().await;
    {
        let mut docs = backend
            .documents
            .lock("pending diagnostic registry control")
            .await;
        docs.get_mut(&expected.uri).unwrap().publication = DocumentPublication::Pending;
    }
    let invalid = backend
        .diag_inputs(&expected.uri, tcl_lsp_core::profile_for_dialect("tcl8.6"))
        .await;
    assert!(invalid.inputs_epoch.is_none());
    {
        let mut docs = backend
            .documents
            .lock("indexed diagnostic registry control")
            .await;
        docs.get_mut(&expected.uri).unwrap().publication = DocumentPublication::Indexed;
    }
    let epoch = backend.diag_inputs_epoch();
    backend.diag_slots.lock().await.insert(
        expected.uri.clone(),
        DiagSlot {
            latest_inputs: Some(invalid),
            inputs_epoch: epoch,
            running: true,
            ..DiagSlot::default()
        },
    );
    backend
        .schedule_diagnostics(expected.uri.clone(), "tcl8.6".to_owned())
        .await;
    let slots = backend.diag_slots.lock().await;
    let slot = slots.get(&expected.uri).unwrap();
    assert_eq!(slot.inputs_epoch, epoch);
    assert_eq!(
        slot.latest_inputs.as_ref().unwrap().inputs_epoch,
        Some(epoch)
    );
}

fn assert_document_rewrites(
    diagnostics: &[tower_lsp_server::ls_types::Diagnostic],
    expected: &DiagnosticSourceExpectation,
) {
    let rewrites: Vec<_> = diagnostics
        .iter()
        .filter_map(|diagnostic| diagnostic.data.as_ref())
        .filter(|data| data.get("sourceAssociation").is_some())
        .collect();
    assert!(
        !rewrites.is_empty(),
        "actual worker must retain an applicable rewrite"
    );
    for rewrite in rewrites {
        let source = &rewrite["sourceAssociation"];
        assert_eq!(source["purpose"], "original-document-optimiser-rewrite");
        assert_eq!(source["uri"], expected.uri.as_str());
        assert_eq!(source["version"], expected.version);
        assert_eq!(source["source"], expected.source);
        assert!(source["revision"].is_string());
        assert!(source["inputsEpoch"].is_string());
        assert!(source.get("originalSource").is_none());
    }
}

/// Reproduction of the sourced-file fault seen end to end: a document that
/// `source`s an open sibling must still report an unknown command as W123
/// (tclsh: `invalid command name`). With the source of a file that does not
/// exist the report abstains, which `cross-file-diagnostics.md` states.
/// tclsh 8.6.18 and 9.1.0, with lib.tcl beside the script: `source lib.tcl`
/// then an undefined command -> `invalid command name`.
#[tokio::test]
async fn an_unknown_command_in_a_sourcing_document_draws_w123() {
    let mut silent = Vec::new();
    for dialect in ["tcl", "tcl8.6"] {
        let backend = crate::tests::test_backend();
        backend.workspace_scan_ready.mark_complete();
        let lib = Uri::from_str("file:///w/lib.tcl").unwrap();
        let main = Uri::from_str("file:///w/main.tcl").unwrap();
        for (uri, text) in [
            (&lib, "proc libtest {} {}\n"),
            (&main, "source lib.tcl\nneverDefinedAnywhereAtAll x\n"),
        ] {
            backend
                .documents
                .lock("sourcing_w123_fixture")
                .await
                .insert(
                    uri.clone(),
                    DocumentState::with_version(text.into(), dialect.into(), 1),
                );
            backend
                .db_set_source(uri, text, dialect.into())
                .await
                .unwrap();
        }
        let report = backend
            .produce_direct_diagnostic_report(&main)
            .await
            .unwrap_or_else(|_| panic!("direct report"));
        let w123: Vec<_> = report
            .diagnostics
            .iter()
            .filter(|d| {
                matches!(&d.code, Some(tower_lsp_server::ls_types::NumberOrString::String(c)) if c == "W123")
            })
            .collect();
        if w123.len() != 1 {
            silent.push(format!("{dialect}: {:?}", report.diagnostics));
        }
    }
    assert!(
        silent.is_empty(),
        "no W123 for the unknown command: {silent:?}"
    );
}

/// A `.tclspec` document is analysed like any document: an unknown bare word
/// in it draws the analyser's W123 hint beside the loader's notices. The core
/// analyser reports it for dialect `spectcl` (`tcl-lsp analyze`).
#[tokio::test]
async fn an_unknown_word_in_a_pack_document_draws_the_analyser_hint() {
    let mut silent = Vec::new();
    for dialect in ["tcl", "spectcl"] {
        let backend = crate::tests::test_backend();
        backend.workspace_scan_ready.mark_complete();
        let uri = Uri::from_str("file:///w/.tcl-lsp/mylib.tclspec").unwrap();
        let text = "speclib mylib 1 {\n    command mylib::x {\n        arity 1\n    }\n}\na_word_only_this_edit_carries\n";
        backend.documents.lock("pack_w123_fixture").await.insert(
            uri.clone(),
            DocumentState::with_version(text.into(), dialect.into(), 1),
        );
        backend
            .db_set_source(&uri, text, dialect.into())
            .await
            .unwrap();
        let report = backend
            .produce_direct_diagnostic_report(&uri)
            .await
            .unwrap_or_else(|_| panic!("direct report"));
        let w123 = report.diagnostics.iter().any(|d| {
            matches!(&d.code, Some(tower_lsp_server::ls_types::NumberOrString::String(c)) if c == "W123")
        });
        let core = tcl_compiler::analyser::Analyser::new()
            .analyse(text, dialect)
            .clone();
        if !w123 {
            silent.push(format!(
                "{dialect}: server {:?}; core unresolved sites {}",
                report.diagnostics,
                core.unresolved_command_sites.len()
            ));
        }
    }
    assert!(
        silent.is_empty(),
        "no W123 for the unknown word: {silent:?}"
    );
}

async fn open_source(backend: &Backend, expected: &DiagnosticSourceExpectation) {
    // A running server has finished its startup scan before a current report
    // can settle; the fixture models that, as the scan-dependent tests do.
    backend.workspace_scan_ready.mark_complete();
    backend
        .documents
        .lock("diagnostic_completion_fixture")
        .await
        .insert(
            expected.uri.clone(),
            DocumentState::with_version(expected.source.clone(), "tcl8.6".into(), expected.version),
        );
    backend
        .db_set_source(&expected.uri, &expected.source, "tcl8.6".into())
        .await
        .unwrap();
}

async fn run_provider(backend: &Backend, uri: &Uri) {
    // A running server has finished its startup scan before a current report
    // can settle; the fixture models that, as the scan-dependent tests do.
    backend.workspace_scan_ready.mark_complete();
    // Select the provider profile from the actual published Salsa source,
    // and require it to agree with the live document before dispatch.
    let document = backend
        .documents
        .lock("diagnostic_provider_fixture")
        .await
        .get(uri)
        .cloned()
        .expect("fixture has a live document");
    let file = backend
        .db_files
        .lock()
        .await
        .get(uri)
        .copied()
        .expect("fixture publishes its source to the database");
    let dialect = {
        let db = backend.db.lock().await;
        assert_eq!(file.dialect(&*db), &document.dialect);
        assert_eq!(
            file.text(&*db),
            tcl_lexer::normalise_lone_cr(&document.text).as_ref()
        );
        file.dialect(&*db).clone()
    };
    let inputs = backend
        .diag_inputs(uri, tcl_lsp_core::profile_for_dialect(&dialect))
        .await;
    let job = inputs.capture_job(uri).await.unwrap();
    assert!(
        job.diagnostic_pass.is_some(),
        "fixture retains an actual source input"
    );
    assert!(run_diagnostics_core(inputs, uri, job).await);
}

#[tokio::test]
async fn publication_validation_reborrows_exact_publisher_without_reissuing() {
    let backend = crate::tests::test_backend();
    let expected = DiagnosticSourceExpectation {
        uri: Uri::from_str("file:///diagnostic-validation-purpose.tcl").unwrap(),
        version: 1,
        source: "set x 1".into(),
    };
    backend.documents.lock("validation_fixture").await.insert(
        expected.uri.clone(),
        DocumentState::with_version(expected.source.clone(), "tcl9.0".into(), 1),
    );
    backend
        .db_set_source(&expected.uri, &expected.source, "tcl9.0".into())
        .await
        .unwrap();
    run_provider(&backend, &expected.uri).await;
    let mut identity =
        serde_json::to_value(backend.current_diagnostics(&expected).await.unwrap()).unwrap();
    assert_eq!(identity["status"], "complete");
    identity.as_object_mut().unwrap().remove("status");
    identity.as_object_mut().unwrap().remove("diagnostics");
    let before = backend.pull_diag_cache.lock().await[&expected.uri]
        .result_id
        .clone();
    let validation = backend
        .validate_diagnostic_publication_command(&[identity.clone()])
        .await
        .unwrap()
        .unwrap();
    assert_eq!(validation["status"], "current");
    assert_eq!(
        backend.pull_diag_cache.lock().await[&expected.uri].result_id,
        before
    );
    for field in ["resultId", "revision", "inputsEpoch", "dialectId", "source"] {
        let mut foreign = identity.clone();
        foreign[field] = serde_json::json!("foreign");
        let refused = backend
            .validate_diagnostic_publication_command(&[foreign])
            .await
            .unwrap()
            .unwrap();
        assert_eq!(refused["status"], "unavailable", "{field}");
    }
    backend
        .diagnostic_publisher
        .completion
        .lock()
        .await
        .remove(&expected.uri);
    assert_eq!(
        backend
            .validate_diagnostic_publication_command(&[identity])
            .await
            .unwrap()
            .unwrap()["status"],
        "unavailable"
    );
}

#[tokio::test]
async fn current_diagnostics_serializes_dialect_from_the_completed_publisher_pass() {
    let backend = crate::tests::test_backend();
    backend.workspace_scan_ready.mark_complete();
    let expected = DiagnosticSourceExpectation {
        uri: Uri::from_str("file:///diagnostic-published-dialect.tcl").unwrap(),
        version: 1,
        source: "set x 1".into(),
    };
    let dialect = "tcl9.0";
    backend
        .documents
        .lock("diagnostic_completed_dialect")
        .await
        .insert(
            expected.uri.clone(),
            DocumentState::with_version(expected.source.clone(), dialect.into(), expected.version),
        );
    backend
        .db_set_source(&expected.uri, &expected.source, dialect.into())
        .await
        .unwrap();
    let inputs = backend
        .diag_inputs(&expected.uri, tcl_lsp_core::profile_for_dialect(dialect))
        .await;
    let job = inputs.capture_job(&expected.uri).await.unwrap();
    assert!(job.diagnostic_pass.is_some());
    assert!(run_diagnostics_core(inputs, &expected.uri, job).await);
    let response =
        serde_json::to_value(backend.current_diagnostics(&expected).await.unwrap()).unwrap();
    assert_eq!(response["status"], "complete");
    assert_eq!(response["uri"], expected.uri.to_string());
    assert_eq!(response["version"], expected.version);
    assert_eq!(response["source"], expected.source);
    assert_eq!(response["dialectId"], dialect);
    assert!(response["resultId"].is_string());
    assert!(response["inputsEpoch"].is_string());
}

/// A BIG-IP configuration or iApp APL document answers
/// `textDocument/diagnostic` with its F5 model report — the same report
/// code the push publisher runs — both directly before any push and by
/// reborrowing the completed push afterwards.
#[tokio::test]
async fn f5_model_documents_answer_pull_with_their_model_report() {
    for (uri, dialect, source) in [
        (
            "file:///pulled-config.scf",
            "f5-bigip",
            "ltm pool /Common/p {\n    members {\n        /Common/10.0.0.1:80 { }\n    }\n}\n",
        ),
        (
            "file:///pulled.apl",
            "f5-iapps",
            "section basic {\n    string name\n}\n",
        ),
    ] {
        let backend = crate::tests::test_backend();
        let expected = DiagnosticSourceExpectation {
            uri: Uri::from_str(uri).unwrap(),
            version: 1,
            source: source.into(),
        };
        backend.documents.lock("f5_pull_fixture").await.insert(
            expected.uri.clone(),
            DocumentState::with_version(expected.source.clone(), dialect.into(), expected.version),
        );
        backend
            .db_set_source(&expected.uri, &expected.source, dialect.into())
            .await
            .unwrap();
        let direct = full_pull(
            backend
                .diagnostic(pull_params(&expected.uri, None))
                .await
                .unwrap_or_else(|error| panic!("{uri}: direct pull refused: {error:?}")),
        );
        run_provider(&backend, &expected.uri).await;
        let pushed = backend
            .pull_diag_cache
            .lock()
            .await
            .get(&expected.uri)
            .map(|entry| (entry.result_id.clone(), entry.diagnostics.clone()))
            .expect("the push primed the pull cache");
        let reborrowed = full_pull(
            backend
                .diagnostic(pull_params(&expected.uri, None))
                .await
                .unwrap_or_else(|error| panic!("{uri}: pull after push refused: {error:?}")),
        );
        assert_eq!(
            reborrowed.result_id.as_deref(),
            Some(pushed.0.as_str()),
            "{uri}"
        );
        assert_eq!(reborrowed.items, pushed.1, "{uri}");
        assert_eq!(direct.items, pushed.1, "{uri}: one report code");
        let current =
            serde_json::to_value(backend.current_diagnostics(&expected).await.unwrap()).unwrap();
        assert_eq!(current["status"], "complete", "{uri}: {current}");
        assert_eq!(current["dialectId"], dialect, "{uri}");
    }
}

#[tokio::test]
async fn current_diagnostics_requires_an_actual_completed_provider_even_for_empty_reports() {
    let backend = crate::tests::test_backend();
    let mut expected = DiagnosticSourceExpectation {
        uri: Uri::from_str("file:///diagnostic-completion.tcl").unwrap(),
        version: 1,
        source: String::new(),
    };
    open_source(&backend, &expected).await;
    assert!(matches!(
        backend.current_diagnostics(&expected).await.unwrap(),
        CurrentDiagnostics::Unavailable {
            reason: DiagnosticUnavailable::Pending
        }
    ));
    // A published/displayed empty collection supplies no provider receipt.
    backend.pull_diag_cache.lock().await.insert(
        expected.uri.clone(),
        PullDiagEntry {
            currency: None,
            result_id: next_pull_diag_result_id(),
            revision: 0,
            diagnostics: Vec::new(),
        },
    );
    assert!(matches!(
        backend.current_diagnostics(&expected).await.unwrap(),
        CurrentDiagnostics::Unavailable { .. }
    ));
    run_provider(&backend, &expected.uri).await;
    assert!(
        matches!(backend.current_diagnostics(&expected).await.unwrap(),
            CurrentDiagnostics::Complete { report } if report.diagnostics.is_empty())
    );

    // A fresh admission cannot authenticate an unretained publication.
    let admission = backend
        .document_path_admission(&expected.uri)
        .await
        .unwrap();
    let Ok(current) = backend
        .current_diagnostic_input(&expected, &admission)
        .await
    else {
        panic!("fixture retains the actual published SourceFile input");
    };
    let retained = backend
        .source_path_admissions
        .lock()
        .unwrap()
        .remove(&expected.uri)
        .unwrap();
    assert!(matches!(
        current.lock_current(&backend, &expected.uri, &admission),
        Err(DiagnosticUnavailable::Context)
    ));
    assert!(matches!(
        backend
            .current_diagnostic_input(&expected, &admission)
            .await,
        Err(DiagnosticUnavailable::Context)
    ));
    assert!(backend.current_diagnostics(&expected).await.is_err());
    backend
        .source_path_admissions
        .lock()
        .unwrap()
        .insert(expected.uri.clone(), retained);
    assert!(matches!(
        backend.current_diagnostics(&expected).await.unwrap(),
        CurrentDiagnostics::Complete { .. }
    ));

    let old = DiagnosticSourceExpectation {
        uri: expected.uri.clone(),
        version: 1,
        source: String::new(),
    };
    expected.version = 2;
    expected.source = "proc check {} {}\ncheck unexpected\n".into();
    open_source(&backend, &expected).await;
    assert!(matches!(
        backend.current_diagnostics(&old).await.unwrap(),
        CurrentDiagnostics::Unavailable { .. }
    ));
    assert!(matches!(
        backend.current_diagnostics(&expected).await.unwrap(),
        CurrentDiagnostics::Unavailable { .. }
    ));
    run_provider(&backend, &expected.uri).await;
    assert!(
        matches!(backend.current_diagnostics(&expected).await.unwrap(),
            CurrentDiagnostics::Complete { report } if !report.diagnostics.is_empty())
    );
}

#[tokio::test]
async fn current_diagnostics_cannot_reborrow_a_replaced_equal_path_publication() {
    let mut backend = crate::tests::test_backend();
    let store = Arc::new(vfs::MemoryStore::new());
    let expected = DiagnosticSourceExpectation {
        uri: Uri::from_str("file:///diagnostic-publication-aba.tcl").unwrap(),
        version: 1,
        source: String::new(),
    };
    let path = "/same/diagnostic-publication-aba.tcl";
    store.upsert_uri(expected.uri.as_str(), path, Vec::new());
    backend.state_mut_for_test().store = store.clone();
    open_source(&backend, &expected).await;
    run_provider(&backend, &expected.uri).await;
    let old_pass = backend
        .diagnostic_publisher
        .completion
        .lock()
        .await
        .get(&expected.uri)
        .unwrap()
        .pass
        .clone();
    assert!(matches!(
        backend.current_diagnostics(&expected).await.unwrap(),
        CurrentDiagnostics::Complete { .. }
    ));
    store.upsert_uri(
        expected.uri.as_str(),
        "/different/diagnostic-publication-aba.tcl",
        Vec::new(),
    );
    store.upsert_uri(expected.uri.as_str(), path, Vec::new());
    // Republish the same bytes/version/configuration under the replacement
    // provider binding without completing another diagnostic pass.
    open_source(&backend, &expected).await;
    let admission = backend
        .document_path_admission(&expected.uri)
        .await
        .unwrap();
    assert_ne!(old_pass.path_admission.as_ref(), admission.as_ref());
    let Ok(current) = backend
        .current_diagnostic_input(&expected, &admission)
        .await
    else {
        panic!("replacement publication has an actual current input");
    };
    assert_eq!(old_pass.input.as_deref(), Some(current.input.as_ref()));
    assert!(matches!(
        backend.current_diagnostics(&expected).await.unwrap(),
        CurrentDiagnostics::Unavailable {
            reason: DiagnosticUnavailable::Stale
        }
    ));
    run_provider(&backend, &expected.uri).await;
    assert!(matches!(
        backend.current_diagnostics(&expected).await.unwrap(),
        CurrentDiagnostics::Complete { .. }
    ));
}

fn assert_diagnostics_unavailable(report: &CurrentDiagnostics) {
    assert!(matches!(report, CurrentDiagnostics::Unavailable { .. }));
}

fn legacy_full_report_id(legacy: DocumentDiagnosticReportResult) -> String {
    match legacy {
        DocumentDiagnosticReportResult::Report(DocumentDiagnosticReport::Full(report)) => {
            report.full_document_diagnostic_report.result_id.unwrap()
        }
        _ => panic!("a real cache-miss pull must publish a new full report"),
    }
}

#[tokio::test]
async fn current_diagnostics_withdraws_changed_inputs_generation_and_source() {
    let backend = crate::tests::test_backend();
    let expected = DiagnosticSourceExpectation {
        uri: Uri::from_str("file:///diagnostic-input-currency.tcl").unwrap(),
        version: 4,
        source: String::new(),
    };
    open_source(&backend, &expected).await;
    run_provider(&backend, &expected.uri).await;
    assert!(matches!(
        backend.current_diagnostics(&expected).await.unwrap(),
        CurrentDiagnostics::Complete { .. }
    ));
    // An unchanged editing input/revision/epoch cannot donate changed report configuration.
    let (prior_input, prior_configuration) = {
        let config = backend.resolved_db_config(&expected.uri).await;
        let db = backend.db.lock().await;
        let file = *backend.db_files.lock().await.get(&expected.uri).unwrap();
        (
            tcl_lsp_db::document_analysis_input(&*db, file, config).unwrap(),
            config.diagnostic_report_configuration(&*db),
        )
    };
    let prior_epoch = backend.diag_inputs_epoch();
    {
        use salsa::Setter as _;
        let config = backend.resolved_db_config(&expected.uri).await;
        let mut db = backend.db.lock().await;
        config
            .set_targets(&mut *db)
            .to(vec![("tcl".into(), "8.4-9.0".into())]);
    }
    {
        let config = backend.resolved_db_config(&expected.uri).await;
        let db = backend.db.lock().await;
        let file = *backend.db_files.lock().await.get(&expected.uri).unwrap();
        assert_eq!(
            prior_input,
            tcl_lsp_db::document_analysis_input(&*db, file, config).unwrap()
        );
        assert_ne!(
            prior_configuration,
            config.diagnostic_report_configuration(&*db)
        );
    }
    assert_eq!(prior_epoch, backend.diag_inputs_epoch());
    assert_diagnostics_unavailable(&backend.current_diagnostics(&expected).await.unwrap());
    run_provider(&backend, &expected.uri).await;
    assert!(matches!(
        backend.current_diagnostics(&expected).await.unwrap(),
        CurrentDiagnostics::Complete { .. }
    ));
    *backend.class_factory_generation.write().await += 1;
    assert_diagnostics_unavailable(&backend.current_diagnostics(&expected).await.unwrap());
    run_provider(&backend, &expected.uri).await;
    backend
        .feature_toggles
        .lock()
        .await
        .set
        .insert("diagnostics".into(), false);
    backend.invalidate_diag_inputs();
    assert_diagnostics_unavailable(&backend.current_diagnostics(&expected).await.unwrap());
    run_provider(&backend, &expected.uri).await;
    assert!(matches!(
        backend.current_diagnostics(&expected).await.unwrap(),
        CurrentDiagnostics::Suppressed {
            reason: DiagnosticSuppression::Disabled,
            ..
        }
    ));
    let admission = backend
        .document_path_admission(&expected.uri)
        .await
        .unwrap();
    backend
        .documents
        .lock("completion-close")
        .await
        .remove(&expected.uri);
    backend
        .evict_diag_slot(&expected.uri, &admission)
        .await
        .unwrap();
    assert!(
        backend
            .diagnostic_publisher
            .completion
            .lock()
            .await
            .get(&expected.uri)
            .is_none()
    );
    assert_diagnostics_unavailable(&backend.current_diagnostics(&expected).await.unwrap());
}

#[tokio::test]
async fn cancelled_and_failed_passes_cannot_reuse_a_prior_complete_report() {
    let backend = crate::tests::test_backend();
    let expected = DiagnosticSourceExpectation {
        uri: Uri::from_str("file:///diagnostic-pass-outcomes.tcl").unwrap(),
        version: 1,
        source: String::new(),
    };
    open_source(&backend, &expected).await;
    run_provider(&backend, &expected.uri).await;
    let inputs = backend
        .diag_inputs(&expected.uri, tcl_lsp_core::profile_for_dialect("tcl8.6"))
        .await;
    let job = inputs.capture_job(&expected.uri).await.unwrap();
    let pass = job.diagnostic_pass.as_ref().unwrap();
    backend
        .diagnostic_publisher
        .begin_diagnostic_pass(&expected.uri, pass)
        .await;
    assert!(matches!(
        backend.current_diagnostics(&expected).await.unwrap(),
        CurrentDiagnostics::Unavailable {
            reason: DiagnosticUnavailable::Pending
        }
    ));
    for reason in [
        DiagnosticUnavailable::Cancelled,
        DiagnosticUnavailable::ProviderError,
    ] {
        backend
            .diagnostic_publisher
            .finish_diagnostic_pass(
                &expected.uri,
                pass,
                DiagnosticPassState::Unavailable(reason),
            )
            .await;
        assert!(matches!(
            backend.current_diagnostics(&expected).await.unwrap(),
            CurrentDiagnostics::Unavailable { .. }
        ));
    }
    // A superseded pass cannot overwrite the actual newer pass state.
    let successor = inputs.capture_job(&expected.uri).await.unwrap();
    let successor = successor.diagnostic_pass.as_ref().unwrap();
    backend
        .diagnostic_publisher
        .begin_diagnostic_pass(&expected.uri, successor)
        .await;
    backend
        .diagnostic_publisher
        .finish_diagnostic_pass(
            &expected.uri,
            pass,
            DiagnosticPassState::Unavailable(DiagnosticUnavailable::ProviderError),
        )
        .await;
    assert!(matches!(
        backend.current_diagnostics(&expected).await.unwrap(),
        CurrentDiagnostics::Unavailable {
            reason: DiagnosticUnavailable::Pending
        }
    ));
}

async fn await_pending_notification(backend: &Backend, uri: &Uri) {
    crate::rt::timeout(std::time::Duration::from_secs(5), async {
        loop {
            let pending = backend
                .diagnostic_publisher
                .state
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .pending
                .contains_key(uri);
            if pending {
                break;
            }
            crate::rt::yield_now().await;
        }
    })
    .await
    .expect("the actual provider must enqueue its notification");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn suppression_requires_delivered_clear_and_superseded_clear_cannot_settle() {
    use std::sync::atomic::Ordering;
    let backend = Arc::new(crate::tests::test_backend());
    let expected = DiagnosticSourceExpectation {
        uri: Uri::from_str("file:///diagnostic-clear-receipt.tcl").unwrap(),
        version: 7,
        source: String::new(),
    };
    open_source(&backend, &expected).await;
    backend
        .feature_toggles
        .lock()
        .await
        .set
        .insert("diagnostics".into(), false);
    backend.invalidate_diag_inputs();
    // Reuse the existing held-mailbox seam: no new production instrumentation.
    backend
        .diagnostic_publisher
        .started
        .store(true, Ordering::Release);
    let runner = Arc::clone(&backend);
    let uri = expected.uri.clone();
    let first = crate::rt::spawn(async move { run_provider(&runner, &uri).await });
    await_pending_notification(&backend, &expected.uri).await;
    assert!(matches!(
        backend.current_diagnostics(&expected).await.unwrap(),
        CurrentDiagnostics::Unavailable {
            reason: DiagnosticUnavailable::Pending
        }
    ));
    // This real mailbox replacement settles the first clear as Superseded.
    let replacement = backend.diagnostic_publisher.enqueue_assembled(
        expected.uri.clone(),
        Vec::new(),
        Some(expected.version),
        false,
        false,
    );
    first.await.unwrap();
    assert!(matches!(
        backend.current_diagnostics(&expected).await.unwrap(),
        CurrentDiagnostics::Unavailable { .. }
    ));
    let runner = Arc::clone(&backend);
    let uri = expected.uri.clone();
    let second = crate::rt::spawn(async move { run_provider(&runner, &uri).await });
    // The second actual clear must supersede this queued notification.
    assert_eq!(
        crate::rt::timeout(std::time::Duration::from_secs(5), replacement)
            .await
            .expect("the second clear must reach the mailbox")
            .unwrap(),
        DiagnosticPublishOutcome::Superseded
    );
    assert!(matches!(
        backend.current_diagnostics(&expected).await.unwrap(),
        CurrentDiagnostics::Unavailable { .. }
    ));
    backend
        .diagnostic_publisher
        .started
        .store(false, Ordering::Release);
    backend.diagnostic_publisher.ensure_started();
    second.await.unwrap();
    assert!(
        matches!(backend.current_diagnostics(&expected).await.unwrap(),
            CurrentDiagnostics::Suppressed { reason: DiagnosticSuppression::Disabled, report } if report.diagnostics.is_empty())
    );

    backend
        .feature_toggles
        .lock()
        .await
        .set
        .insert("diagnostics".into(), true);
    *backend.diagnostics_exclude.lock().await = vec!["*.tcl".into()];
    backend.invalidate_diag_inputs();
    run_provider(&backend, &expected.uri).await;
    assert!(
        matches!(backend.current_diagnostics(&expected).await.unwrap(),
            CurrentDiagnostics::Suppressed { reason: DiagnosticSuppression::Excluded, report } if report.diagnostics.is_empty())
    );
}

#[tokio::test]
async fn issued_publication_id_cannot_be_replaced_by_the_actual_legacy_pull_handler() {
    use std::sync::atomic::Ordering;
    let backend = crate::tests::test_backend();
    let expected = DiagnosticSourceExpectation {
        uri: Uri::from_str("file:///diagnostic-issued-publication.tcl").unwrap(),
        version: 1,
        source: String::new(),
    };
    open_source(&backend, &expected).await;
    let inputs = backend
        .diag_inputs(&expected.uri, tcl_lsp_core::profile_for_dialect("tcl8.6"))
        .await;
    let job = inputs.capture_job(&expected.uri).await.unwrap();
    let pass = job.diagnostic_pass.as_ref().unwrap();
    let analysis = backend
        .analysis_for(&expected.uri, Arc::clone(&job.text), job.dialect.clone())
        .await
        .unwrap();
    backend
        .diagnostic_publisher
        .begin_diagnostic_pass(&expected.uri, pass)
        .await;
    let mut delivery = inputs.delivery_for_job(
        Some(pass),
        &job.text,
        &expected.uri,
        job.currency,
        job.version,
    );
    delivery.client_supports_pull = false;
    // Hold actual delivery while the legacy handler independently replaces
    // the cache. No production scheduling or publisher hook is added.
    backend
        .diagnostic_publisher
        .started
        .store(true, Ordering::Release);
    let publication = {
        let docs = backend.documents.lock("issued-publication-control").await;
        assert!(delivery.is_current(&docs).await);
        delivery.cache_and_enqueue(Vec::new()).await
    };
    assert_eq!(
        backend
            .pull_diag_cache
            .lock()
            .await
            .get(&expected.uri)
            .unwrap()
            .result_id,
        publication.result_id
    );
    backend.pull_diag_cache.lock().await.remove(&expected.uri);
    let params = serde_json::from_value(serde_json::json!({
        "textDocument": {"uri": expected.uri}
    }))
    .unwrap();
    let legacy = backend.diagnostic(params).await.unwrap();
    let legacy_id = legacy_full_report_id(legacy);
    assert_ne!(publication.result_id, legacy_id);
    backend
        .diagnostic_publisher
        .started
        .store(false, Ordering::Release);
    backend.diagnostic_publisher.ensure_started();
    assert_eq!(
        publication.receipt.await.unwrap(),
        DiagnosticPublishOutcome::Delivered
    );
    // Exercise the actual post-ack issuer facade with its own transported
    // publication ID. This control isolates publication identity; it does
    // not claim another complete provider pass was run by this direct call.
    delivery
        .complete_diagnostic_pass(&analysis, publication.result_id.clone())
        .await;
    {
        let ledger = backend.diagnostic_publisher.completion.lock().await;
        let entry = ledger.get(&expected.uri).unwrap();
        assert!(matches!(
            &entry.state,
            DiagnosticPassState::Complete { result_id }
                if result_id == &publication.result_id && result_id != &legacy_id
        ));
    }
    assert!(matches!(
        backend.current_diagnostics(&expected).await.unwrap(),
        CurrentDiagnostics::Unavailable {
            reason: DiagnosticUnavailable::Stale
        }
    ));
    // A new genuine provider pass remains able to issue current completion.
    run_provider(&backend, &expected.uri).await;
    assert!(matches!(
        backend.current_diagnostics(&expected).await.unwrap(),
        CurrentDiagnostics::Complete { .. }
    ));
}

#[tokio::test]
async fn actual_failed_lift_worker_cannot_issue_completed_empty_diagnostics() {
    // Preserve the original failed-worker assertion surface. This adapter
    // delegates to the current publication owner; a failed worker produced
    // no command-query receipt and cannot enter dependency admission.
    async fn publish_diagnostics_result(
        delivery: &DeliveryCtx<'_>,
        workspace_index: &Arc<TrackedRwLock<core_workspace_index::WorkspaceIndex>>,
        rehomed_source_seeds: &Arc<Mutex<HashMap<String, Vec<String>>>>,
        rehoming_gate: &Arc<tokio::sync::Mutex<()>>,
        analysis: &Arc<AnalysisResult>,
        result: (
            Result<(), DiagnosticUnavailable>,
            Result<Vec<tower_lsp_server::ls_types::Diagnostic>, crate::rt::JoinError>,
        ),
        timing: PublishTiming<'_>,
    ) -> bool {
        crate::publish_diagnostics_result(
            delivery,
            workspace_index,
            rehomed_source_seeds,
            rehoming_gate,
            analysis,
            (result.0, result.1, None),
            timing,
        )
        .await
    }
    let backend = crate::tests::test_backend();
    let expected = DiagnosticSourceExpectation {
        uri: Uri::from_str("file:///diagnostic-failed-lift.tcl").unwrap(),
        version: 2,
        source: String::new(),
    };
    open_source(&backend, &expected).await;
    run_provider(&backend, &expected.uri).await;
    let inputs = backend
        .diag_inputs(&expected.uri, tcl_lsp_core::profile_for_dialect("tcl8.6"))
        .await;
    let job = inputs.capture_job(&expected.uri).await.unwrap();
    let pass = job.diagnostic_pass.as_ref().unwrap();
    backend
        .diagnostic_publisher
        .begin_diagnostic_pass(&expected.uri, pass)
        .await;
    let analysis = backend
        .analysis_for(&expected.uri, Arc::clone(&job.text), job.dialect.clone())
        .await
        .unwrap();
    let mut delivery = inputs.delivery_for_job(
        Some(pass),
        &job.text,
        &expected.uri,
        job.currency,
        job.version,
    );
    delivery.client_supports_pull = false;

    let failed = crate::rt::spawn_blocking(|| -> Vec<tower_lsp_server::ls_types::Diagnostic> {
        panic!("actual diagnostic lift worker failure control")
    })
    .await;
    assert!(failed.is_err());
    assert!(
        publish_diagnostics_result(
            &delivery,
            &backend.workspace_index,
            &backend.rehomed_source_seeds,
            &backend.rehoming_gate,
            &analysis,
            (Ok(()), failed),
            PublishTiming {
                started: crate::rt::Instant::now(),
                uri_str: expected.uri.as_str(),
                line_count: 1
            }
        )
        .await
    );
    assert!(matches!(
        backend.current_diagnostics(&expected).await.unwrap(),
        CurrentDiagnostics::Unavailable {
            reason: DiagnosticUnavailable::ProviderError
        }
    ));
}

#[tokio::test]
async fn actual_foreign_provider_registry_cannot_borrow_completed_analysis_currency() {
    let backend = crate::tests::test_backend();
    let expected = DiagnosticSourceExpectation {
        uri: Uri::from_str("file:///diagnostic-provider-registry.tcl").unwrap(),
        version: 1,
        source: String::new(),
    };
    open_source(&backend, &expected).await;
    run_provider(&backend, &expected.uri).await;
    assert!(matches!(
        backend.current_diagnostics(&expected).await.unwrap(),
        CurrentDiagnostics::Complete { .. }
    ));
    let mut inputs = backend
        .diag_inputs(&expected.uri, tcl_lsp_core::profile_for_dialect("tcl8.6"))
        .await;
    let foreign = inputs
        .registry
        .project_for_profile(tcl_lsp_core::profile_for_dialect("tcl8.4"));
    assert_ne!(
        inputs.registry.snapshot().semantic_key(),
        foreign.snapshot().semantic_key()
    );
    inputs.registry = Arc::new(foreign);
    let job = inputs.capture_job(&expected.uri).await.unwrap();
    assert!(run_diagnostics_core(inputs, &expected.uri, job).await);
    assert!(matches!(
        backend.current_diagnostics(&expected).await.unwrap(),
        CurrentDiagnostics::Unavailable {
            reason: DiagnosticUnavailable::Context
        }
    ));
}

#[tokio::test]
async fn delivered_policy_suppression_does_not_claim_missing_analysis_context() {
    use salsa::Setter as _;
    let backend = crate::tests::test_backend();
    let expected = DiagnosticSourceExpectation {
        uri: Uri::from_str("file:///diagnostic-disabled-missing-context.tcl").unwrap(),
        version: 1,
        source: String::new(),
    };
    open_source(&backend, &expected).await;
    run_provider(&backend, &expected.uri).await;
    assert!(matches!(
        backend.current_diagnostics(&expected).await.unwrap(),
        CurrentDiagnostics::Complete { .. }
    ));
    {
        let config = backend.resolved_db_config(&expected.uri).await;
        let mut db = backend.db.lock().await;
        config.set_spec_pack_key(&mut *db).to(u64::MAX - 390_037);
    }
    set_diagnostics_enabled(&backend, false).await;
    backend.invalidate_diag_inputs();
    for registry_at_drain in [false, true] {
        let inputs = backend
            .diag_inputs_with(
                &expected.uri,
                tcl_lsp_core::profile_for_dialect("tcl8.6"),
                registry_at_drain,
            )
            .await;
        let job = inputs.capture_job(&expected.uri).await.unwrap();
        assert!(job.diagnostic_pass.as_ref().unwrap().input.is_none());
        assert!(run_diagnostics_core(inputs, &expected.uri, job).await);
        assert!(matches!(
            backend.current_diagnostics(&expected).await.unwrap(),
            CurrentDiagnostics::Suppressed {
                reason: DiagnosticSuppression::Disabled,
                ..
            }
        ));
    }
    assert!(matches!(
        backend.current_diagnostics(&expected).await.unwrap(),
        CurrentDiagnostics::Suppressed {
            reason: DiagnosticSuppression::Disabled,
            ..
        }
    ));
    assert_policy_receipt_checks_source_and_configuration(&backend, &expected).await;
    let admission = backend
        .document_path_admission(&expected.uri)
        .await
        .unwrap();
    assert!(matches!(
        backend
            .current_diagnostic_input(&expected, &admission)
            .await,
        Err(DiagnosticUnavailable::Context)
    ));
    set_diagnostics_enabled(&backend, true).await;
    *backend.diagnostics_exclude.lock().await = vec!["*.tcl".into()];
    backend.invalidate_diag_inputs();
    for registry_at_drain in [false, true] {
        let inputs = backend
            .diag_inputs_with(
                &expected.uri,
                tcl_lsp_core::profile_for_dialect("tcl8.6"),
                registry_at_drain,
            )
            .await;
        let job = inputs.capture_job(&expected.uri).await.unwrap();
        assert!(job.diagnostic_pass.as_ref().unwrap().input.is_none());
        assert!(run_diagnostics_core(inputs, &expected.uri, job).await);
        assert!(matches!(
            backend.current_diagnostics(&expected).await.unwrap(),
            CurrentDiagnostics::Suppressed {
                reason: DiagnosticSuppression::Excluded,
                ..
            }
        ));
    }
    backend.diagnostics_exclude.lock().await.clear();
    backend.invalidate_diag_inputs();
    let inputs = backend
        .diag_inputs(&expected.uri, tcl_lsp_core::profile_for_dialect("tcl8.6"))
        .await;
    let job = inputs.capture_job(&expected.uri).await.unwrap();
    assert!(job.diagnostic_pass.is_none());
    assert!(run_diagnostics_core(inputs, &expected.uri, job).await);
    assert!(matches!(
        backend.current_diagnostics(&expected).await.unwrap(),
        CurrentDiagnostics::Unavailable { .. }
    ));
}
async fn set_diagnostics_enabled(backend: &Backend, enabled: bool) {
    backend
        .feature_toggles
        .lock()
        .await
        .set
        .insert("diagnostics".into(), enabled);
}

async fn assert_policy_receipt_checks_source_and_configuration(
    backend: &Backend,
    expected: &DiagnosticSourceExpectation,
) {
    use salsa::Setter as _;
    let foreign = DiagnosticSourceExpectation {
        uri: expected.uri.clone(),
        version: expected.version + 1,
        source: expected.source.clone(),
    };
    assert!(matches!(
        backend.current_diagnostics(&foreign).await.unwrap(),
        CurrentDiagnostics::Unavailable {
            reason: DiagnosticUnavailable::Stale
        }
    ));
    let epoch = backend.diag_inputs_epoch();
    backend
        .db_set_source(&expected.uri, "set changed 1\n", "tcl8.6".into())
        .await
        .unwrap();
    assert!(matches!(
        backend.current_diagnostics(expected).await.unwrap(),
        CurrentDiagnostics::Unavailable {
            reason: DiagnosticUnavailable::Stale
        }
    ));
    backend
        .db_set_source(&expected.uri, &expected.source, "tcl8.6".into())
        .await
        .unwrap();
    let file = *backend.db_files.lock().await.get(&expected.uri).unwrap();
    {
        let mut db = backend.db.lock().await;
        file.set_path(&mut *db)
            .to(Some("/foreign/source.tcl".into()));
    }
    assert!(matches!(
        backend.current_diagnostics(expected).await.unwrap(),
        CurrentDiagnostics::Unavailable {
            reason: DiagnosticUnavailable::Stale
        }
    ));
    backend
        .db_set_source(&expected.uri, &expected.source, "tcl8.6".into())
        .await
        .unwrap();
    let config = backend.resolved_db_config(&expected.uri).await;
    {
        let mut db = backend.db.lock().await;
        config
            .set_targets(&mut *db)
            .to(vec![("tcl".into(), "8.4-9.0".into())]);
    }
    assert_eq!(backend.diag_inputs_epoch(), epoch);
    assert!(matches!(
        backend.current_diagnostics(expected).await.unwrap(),
        CurrentDiagnostics::Unavailable {
            reason: DiagnosticUnavailable::Stale
        }
    ));
    {
        let mut db = backend.db.lock().await;
        config.set_targets(&mut *db).to(Vec::new());
    }
}

fn pull_params(uri: &Uri, previous: Option<&str>) -> DocumentDiagnosticParams {
    serde_json::from_value(serde_json::json!({
        "textDocument": { "uri": uri },
        "previousResultId": previous,
    }))
    .unwrap()
}

fn full_pull(report: DocumentDiagnosticReportResult) -> FullDocumentDiagnosticReport {
    match report {
        DocumentDiagnosticReportResult::Report(DocumentDiagnosticReport::Full(report)) => {
            report.full_document_diagnostic_report
        }
        _ => panic!("a changed original report requires a Full response"),
    }
}

fn workspace_params(uri: &Uri, previous: &str) -> WorkspaceDiagnosticParams {
    serde_json::from_value(serde_json::json!({
        "previousResultIds": [{ "uri": uri, "value": previous }]
    }))
    .unwrap()
}

#[tokio::test]
async fn legacy_cache_withdraws_config_only_changes_before_unchanged() {
    let backend = crate::tests::test_backend();
    let expected = DiagnosticSourceExpectation {
        uri: Uri::from_str("file:///legacy-config-currency.tcl").unwrap(),
        version: 3,
        source: "proc check {} {}\ncheck unexpected\n".into(),
    };
    open_source(&backend, &expected).await;
    let original = full_pull(
        backend
            .diagnostic(pull_params(&expected.uri, None))
            .await
            .unwrap(),
    );
    assert_ne!(original.items.len(), 0);
    let previous = original.result_id.unwrap();
    backend
        .feature_toggles
        .lock()
        .await
        .set
        .insert("diagnostics".into(), false);
    backend.invalidate_diag_inputs();
    let cleared = full_pull(
        backend
            .diagnostic(pull_params(&expected.uri, Some(&previous)))
            .await
            .unwrap(),
    );
    assert_ne!(cleared.result_id.as_deref(), Some(previous.as_str()));
    assert_eq!(
        cleared.items,
        [] as [tower_lsp_server::ls_types::Diagnostic; 0]
    );
    {
        let cache = backend.pull_diag_cache.lock().await;
        assert!(matches!(
            cache
                .get(&expected.uri)
                .unwrap()
                .currency
                .as_ref()
                .unwrap()
                .origin,
            DiagnosticCacheOrigin::Direct {
                policy: Some(DiagnosticSuppression::Disabled)
            }
        ));
    }
    assert!(matches!(
        backend.current_diagnostics(&expected).await.unwrap(),
        CurrentDiagnostics::Unavailable { .. }
    ));
    run_provider(&backend, &expected.uri).await;
    assert!(matches!(
        backend.current_diagnostics(&expected).await.unwrap(),
        CurrentDiagnostics::Suppressed {
            reason: DiagnosticSuppression::Disabled,
            ..
        }
    ));
}

#[tokio::test]
async fn workspace_cache_preserves_actual_report_version_and_whole_source() {
    let backend = crate::tests::test_backend();
    let mut expected = DiagnosticSourceExpectation {
        uri: Uri::from_str("file:///workspace-cache-currency.tcl").unwrap(),
        version: 1,
        source: "proc check {} {}\ncheck unexpected\n".into(),
    };
    open_source(&backend, &expected).await;
    let original = full_pull(
        backend
            .diagnostic(pull_params(&expected.uri, None))
            .await
            .unwrap(),
    );
    assert_ne!(original.items.len(), 0);
    let previous = original.result_id.unwrap();
    expected.version = 2;
    expected.source.clear();
    open_source(&backend, &expected).await;
    let report = backend
        .workspace_diagnostic(workspace_params(&expected.uri, &previous))
        .await
        .unwrap();
    let WorkspaceDiagnosticReportResult::Report(report) = report else {
        panic!("workspace report retains actual participants");
    };
    let [WorkspaceDocumentDiagnosticReport::Full(report)] = report.items.as_slice() else {
        panic!("a changed source cannot borrow Unchanged from the older payload");
    };
    assert_eq!(report.version, Some(2));
    assert_ne!(
        report.full_document_diagnostic_report.result_id.as_deref(),
        Some(previous.as_str())
    );
    assert_eq!(
        report.full_document_diagnostic_report.items,
        [] as [tower_lsp_server::ls_types::Diagnostic; 0]
    );
}

#[tokio::test]
async fn legacy_cache_missing_context_or_closed_owner_cannot_issue_empty_current() {
    use salsa::Setter as _;
    let backend = crate::tests::test_backend();
    let expected = DiagnosticSourceExpectation {
        uri: Uri::from_str("file:///legacy-unavailable-currency.tcl").unwrap(),
        version: 1,
        source: String::new(),
    };
    open_source(&backend, &expected).await;
    let original = full_pull(
        backend
            .diagnostic(pull_params(&expected.uri, None))
            .await
            .unwrap(),
    );
    let previous = original.result_id.unwrap();
    {
        let config = backend.resolved_db_config(&expected.uri).await;
        let mut db = backend.db.lock().await;
        config.set_spec_pack_key(&mut *db).to(u64::MAX - 391_111);
    }
    backend.invalidate_diag_inputs();
    assert!(
        backend
            .diagnostic(pull_params(&expected.uri, Some(&previous)))
            .await
            .is_err()
    );
    assert!(
        backend
            .workspace_diagnostic(workspace_params(&expected.uri, &previous))
            .await
            .is_err()
    );
    backend
        .documents
        .lock("legacy-cache-close-control")
        .await
        .remove(&expected.uri);
    assert!(
        backend
            .diagnostic(pull_params(&expected.uri, None))
            .await
            .is_err()
    );
}

#[tokio::test]
async fn legacy_cache_equal_path_replacement_cannot_reuse_completed_payload() {
    let mut backend = crate::tests::test_backend();
    let store = Arc::new(vfs::MemoryStore::new());
    let expected = DiagnosticSourceExpectation {
        uri: Uri::from_str("file:///legacy-cache-aba.tcl").unwrap(),
        version: 1,
        source: String::new(),
    };
    let path = "/same/legacy-cache-aba.tcl";
    store.upsert_uri(expected.uri.as_str(), path, Vec::new());
    backend.state_mut_for_test().store = store.clone();
    open_source(&backend, &expected).await;
    let original = full_pull(
        backend
            .diagnostic(pull_params(&expected.uri, None))
            .await
            .unwrap(),
    );
    let previous = original.result_id.unwrap();
    store.upsert_uri(
        expected.uri.as_str(),
        "/different/legacy-cache-aba.tcl",
        Vec::new(),
    );
    store.upsert_uri(expected.uri.as_str(), path, Vec::new());
    assert!(
        backend
            .current_cached_diagnostic_reports(std::slice::from_ref(&expected.uri))
            .await
            .is_err()
    );
    open_source(&backend, &expected).await;
    let current = full_pull(
        backend
            .diagnostic(pull_params(&expected.uri, Some(&previous)))
            .await
            .unwrap(),
    );
    assert_ne!(current.result_id.as_deref(), Some(previous.as_str()));
}
