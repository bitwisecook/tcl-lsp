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

//! Code actions: the published report, action commands and dialect actions.

use super::*;

#[test]
fn action_command_forwards_string_args_for_bigip_rename_partition() {
    // The BIG-IP code-action provider emits `tclLsp.renamePartition`
    // with `string_args = [uri, partition]`.  The server conversion must
    // forward those — dropping `string_args` left the rename action
    // argument-less and the client could not run it.
    let cmd = core_code_actions::ActionCommand {
        command: "tclLsp.renamePartition".to_string(),
        args: Vec::new(),
        string_args: vec!["file:///c.conf".to_string(), "MyPartition".to_string()],
    };
    let lsp = action_command_to_lsp("Rename partition".to_string(), cmd);
    assert_eq!(lsp.command, "tclLsp.renamePartition");
    assert_eq!(
        lsp.arguments,
        Some(vec![
            serde_json::json!("file:///c.conf"),
            serde_json::json!("MyPartition"),
        ]),
    );
}

#[test]
fn action_command_forwards_string_args_for_editor_rename() {
    // `editor.action.rename` carries just `[uri]`.
    let cmd = core_code_actions::ActionCommand {
        command: "editor.action.rename".to_string(),
        args: Vec::new(),
        string_args: vec!["file:///c.conf".to_string()],
    };
    let lsp = action_command_to_lsp("Rename".to_string(), cmd);
    assert_eq!(
        lsp.arguments,
        Some(vec![serde_json::json!("file:///c.conf")]),
    );
}

#[test]
fn action_command_preserves_integer_position_args() {
    // The post-extract rename (`tclLsp.renameSymbolAtPosition`) carries
    // integer position args `[line, start, end]` and no string args —
    // the existing behaviour must be unchanged by the string_args
    // forwarding (the ints stay JSON numbers, in order).
    let cmd = core_code_actions::ActionCommand {
        command: "tclLsp.renameSymbolAtPosition".to_string(),
        args: vec![0, 4, 11],
        string_args: Vec::new(),
    };
    let lsp = action_command_to_lsp("Rename symbol".to_string(), cmd);
    assert_eq!(
        lsp.arguments,
        Some(vec![
            serde_json::json!(0),
            serde_json::json!(4),
            serde_json::json!(11),
        ]),
    );
}

/// The actual code-action handler demands the same authentic current
/// external signature projection. Readonly callback advice grants no edit.
#[tokio::test]
async fn original_callback_code_action_demands_current_source_header_projection() {
    // naming.database.original-project-callback-projection
    // docs/design/analysis/name-resolution-proofs/database-original-project-callback-projection.md
    // naming.source.original-callback-procedure-target
    // docs/design/analysis/name-resolution-proofs/source-original-callback-procedure-target.md
    let executed: Arc<std::sync::Mutex<Vec<String>>> = Arc::default();
    let sink = Arc::clone(&executed);
    let backend = test_backend_over(tcl_lsp_db::TclDatabase::with_event_logger(move |key| {
        sink.lock().unwrap().push(key);
    }));
    let caller = Uri::from_str("file:///callback-actions.tcl").unwrap();
    let provider = Uri::from_str("file:///callback-actions-provider.tcl").unwrap();
    let source = "interp alias {} cb {} ::external FIXED\nlsort -command {cb BAKED} {2 1}";
    register_callback_source(&backend, &provider, "proc external {a b c} {}")
        .await
        .unwrap();
    register_callback_source(&backend, &caller, source)
        .await
        .unwrap();
    set_callback_project_advice(&backend, true).await;
    let old_report = callback_pull_report(&backend, &caller, source)
        .await
        .unwrap();
    assert_eq!(diag_codes(&old_report), ["E003"]);
    register_callback_source(&backend, &provider, "proc external {a b c d e} {}")
        .await
        .unwrap();
    executed.lock().unwrap().clear();
    let actions = backend
        .code_action(CodeActionParams {
            text_document: TextDocumentIdentifier {
                uri: caller.clone(),
            },
            range: Range::new(Position::new(1, 16), Position::new(1, 18)),
            context: tower_lsp_server::ls_types::CodeActionContext {
                diagnostics: old_report,
                ..Default::default()
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
        })
        .await
        .unwrap()
        .unwrap_or_default();
    assert!(
        executed
            .lock()
            .unwrap()
            .iter()
            .any(|key| key.contains("original_command_signatures")),
        "code actions must demand authentic held-target headers, not nominal arities"
    );
    assert!(
        actions.iter().all(|action| match action {
            CodeActionOrCommand::CodeAction(action) => action
                .diagnostics
                .as_ref()
                .is_none_or(|diags| { callback_source_wire_diagnostics(diags).is_empty() }),
            CodeActionOrCommand::Command(_) => true,
        }),
        "readonly source callback subjects cannot license a naming edit"
    );
    assert_eq!(
        diag_codes(
            &callback_pull_report(&backend, &caller, source)
                .await
                .unwrap()
        ),
        ["E002"]
    );
}

#[tokio::test]
async fn code_action_handler_bigip_conf_offers_partition_rename() {
    // A BIG-IP `.conf` document routes code actions through the dialect
    // provider (`tcl-lsp-core::bigip_code_actions`): an `auth partition`
    // stanza offers the `tclLsp.renamePartition` command, which the generic
    // Tcl code-action path never would. Guards the provider's wiring into
    // `Backend::code_action`.
    let backend = test_backend();
    let uri = Uri::from_str("file:///bigip.conf").unwrap();
    let src = "auth partition Team1 {\n    description test\n}\n";
    backend.documents.lock("test").await.insert(
        uri.clone(),
        DocumentState::with_version(src.to_owned(), "f5-bigip".to_owned(), 3),
    );
    let params = CodeActionParams {
        text_document: TextDocumentIdentifier { uri: uri.clone() },
        range: Range::new(Position::new(0, 0), Position::new(0, 0)),
        context: tower_lsp_server::ls_types::CodeActionContext::default(),
        work_done_progress_params: WorkDoneProgressParams::default(),
        partial_result_params: PartialResultParams::default(),
    };
    let result = backend
        .code_action(params)
        .await
        .expect("ok")
        .expect("some");
    let has_rename_partition = result.iter().any(|a| match a {
        CodeActionOrCommand::CodeAction(ca) => ca
            .command
            .as_ref()
            .is_some_and(|c| c.command == "tclLsp.renamePartition"),
        CodeActionOrCommand::Command(c) => c.command == "tclLsp.renamePartition",
    });
    assert!(
        has_rename_partition,
        "expected a renamePartition code action: {result:?}",
    );
    // The deferred rename names the document, the partition and the
    // version the stanza was read from — the version the server's
    // `tcl-lsp.renamePartition` refuses any other document state against.
    let arguments = result.iter().find_map(|a| match a {
        CodeActionOrCommand::CodeAction(ca) => ca
            .command
            .as_ref()
            .filter(|c| c.command == "tclLsp.renamePartition")
            .and_then(|c| c.arguments.clone()),
        CodeActionOrCommand::Command(c) => (c.command == "tclLsp.renamePartition")
            .then(|| c.arguments.clone())
            .flatten(),
    });
    assert_eq!(
        arguments,
        Some(vec![
            serde_json::json!(uri.as_str()),
            serde_json::json!("Team1"),
            serde_json::json!(3),
        ]),
    );
}

/// The code-action handler builds its report the pull path's
/// way — `published_findings` then `published_report` — so the
/// lightbulb's shown set is exactly what the document publishes.
#[tokio::test]
async fn the_lightbulb_reads_the_published_report() {
    let backend = test_backend();
    let uri = Uri::from_str("file:///lightbulb.tcl").unwrap();
    let src = "set x 1   \nputs $x\n";
    register(&backend, &uri, src).await;

    let published = backend
        .full_diagnostics_for(&uri, Arc::from(src), "tcl8.6".to_owned(), "tcl")
        .await
        .unwrap();
    let published_codes: std::collections::BTreeSet<String> = published
        .iter()
        .filter_map(|d| match &d.code {
            Some(tower_lsp_server::ls_types::NumberOrString::String(c)) => Some(c.clone()),
            _ => None,
        })
        .collect();
    assert!(published_codes.contains("W112"), "{published_codes:?}");

    // The lightbulb's own assembly (`code_action`): the published
    // analyser set through `published_findings`, then `published_report`
    // under the same layers.
    let dialect = tcl_lsp_core::profile_for_dialect("tcl8.6");
    let analysis = backend
        .analysis_for(&uri, Arc::from(src), "tcl8.6".to_owned())
        .await
        .unwrap();
    let registry = backend.registry_for_dialect("tcl8.6").await;
    let (disabled, _) = backend.resolved_analysis_settings(&uri).await;
    let published_diags = backend
        .published_analyser_diagnostics(&uri, src, &analysis, dialect, &disabled)
        .await;
    let checks =
        tcl_lsp_db::compiler_check_diagnostics_uncached(src, &registry, "tcl8.6", None, None);
    let produced = published_findings(&published_diags, &checks, None, src);
    let doc = core_report::DocumentSource {
        text: src,
        analysis_text: src,
        decode: None,
        dialect,
        pass: core_report::SourcePass::Tcl {
            line_length: backend.resolved_style_line_length(&uri).await as usize,
        },
    };
    let policy_layers = backend.resolved_policy_layers(&uri).await;
    let report = published_report(
        &doc,
        produced,
        &policy_layers,
        core_policy::Directives::from_analysis(&analysis, src),
        Some(&analysis),
    );
    let report_codes: std::collections::BTreeSet<String> = report
        .shown()
        .map(|shown| shown.finding.code.to_string())
        .collect();
    assert_eq!(report_codes, published_codes, "{report:?}");
}
