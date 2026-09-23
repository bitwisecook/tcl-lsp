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

//! The language server's passes over the diagnostic-policy truth table
//! (`docs/design/compiler/diagnostic-policy.md` § The truth table): every
//! row the server can realise, configured through the session's three
//! layers — the slot as the editor layer — and rendered through the pull
//! path, which shares `lifted_report` with both pushes, the lightbulb, and
//! `tcl-lsp.optimiseDocument`. Each rendering is held to what the row
//! expects of that surface ([`check`]).

use std::str::FromStr;
use std::sync::Arc;

use serde_json::Value;
use tcl_core_types::{DiagCode, Severity};
use tcl_lsp_core::diagnostic_policy::truth_table::{
    ActionView, Observed, ObservedState, ROWS, Row, Surface, check, offered,
};
use tower_lsp_server::LanguageServer;
use tower_lsp_server::ls_types::{
    CodeActionContext, CodeActionOrCommand, CodeActionParams, DiagnosticSeverity, DiagnosticTag,
    NumberOrString, PartialResultParams, Position, Range, TextDocumentIdentifier, Uri,
    WorkDoneProgressParams,
};

use crate::tests::test_backend;
use crate::{Backend, DocumentState, PolicyLayers};

/// The row's three layers as the session's, the slot as the editor layer.
fn session_layers(row: &Row) -> PolicyLayers {
    PolicyLayers {
        global: Row::layer(row.global),
        editor: Row::layer(row.slot),
        project: Row::layer(row.project),
    }
}

/// A backend configured with `layers`, holding the row's document at
/// `file:///truth/<name>.tcl` — with its decode report for a `bytes` row —
/// and the document's text.
async fn backend_holding(row: &Row, layers: &PolicyLayers) -> (Backend, Uri, String) {
    let backend = test_backend();
    backend.apply_session_layers(layers).await;
    let uri = Uri::from_str(&format!("file:///truth/{}.tcl", row.name)).expect("a file URI");
    let (text, decode) = row.text();
    let mut doc = DocumentState::new(text.clone(), row.dialect.to_owned());
    doc.decode_report = decode;
    backend
        .documents
        .lock("truth-table")
        .await
        .insert(uri.clone(), doc);
    (backend, uri, text)
}

/// The severity a published diagnostic carries, in the producers' terms.
fn severity_of(severity: Option<DiagnosticSeverity>) -> Option<Severity> {
    severity.and_then(|severity| match severity {
        DiagnosticSeverity::ERROR => Some(Severity::Error),
        DiagnosticSeverity::WARNING => Some(Severity::Warning),
        DiagnosticSeverity::INFORMATION => Some(Severity::Info),
        DiagnosticSeverity::HINT => Some(Severity::Hint),
        _ => None,
    })
}

/// A published diagnostic's code, when it is a catalogued one.
fn code_of(diagnostic: &tower_lsp_server::ls_types::Diagnostic) -> Option<DiagCode> {
    match &diagnostic.code {
        Some(NumberOrString::String(code)) => DiagCode::from_str(code).ok(),
        _ => None,
    }
}

/// Every row the server publishes is published as the row expects: its
/// shown findings, at their severities, and nothing it suppresses. The row
/// that checks a tag also checks it on the wire.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn every_row_publishes_its_shown_set() {
    let mut failures: Vec<String> = Vec::new();
    for row in ROWS.iter().filter(|row| row.runs_on(Surface::Lsp)) {
        let (backend, uri, text) = backend_holding(row, &session_layers(row)).await;
        let published = backend
            .full_diagnostics_for(
                &uri,
                Arc::from(text.as_str()),
                row.dialect.to_owned(),
                "tcl",
            )
            .await;
        let observed: Vec<Observed> = published
            .iter()
            .filter_map(|diagnostic| {
                Some(Observed {
                    code: code_of(diagnostic)?,
                    line: Some(diagnostic.range.start.line + 1),
                    state: ObservedState::Shown(severity_of(diagnostic.severity)),
                })
            })
            .collect();
        if let Err(failure) = check(row, Surface::Lsp, &observed) {
            failures.push(failure);
        }
        if row.name == "a_tagged_code_carries_its_tag" {
            let tags: Vec<_> = published
                .iter()
                .filter(|diagnostic| code_of(diagnostic) == Some(DiagCode::W211))
                .map(|diagnostic| diagnostic.tags.clone())
                .collect();
            if tags != [Some(vec![DiagnosticTag::UNNECESSARY])] {
                failures.push(format!(
                    "row `{}` on Lsp: W211 carries tags {tags:?}, not [UNNECESSARY]",
                    row.name
                ));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// The lightbulb over the whole document offers a subject's fix exactly
/// when the row shows the subject: the brace refactor for W100, the
/// shimmer `# noqa` action for S100, the fold quick-fix for O101.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn every_row_offers_fixes_for_shown_findings_only() {
    let mut failures: Vec<String> = Vec::new();
    for row in ROWS.iter().filter(|row| row.runs_on(Surface::LspActions)) {
        let (backend, uri, text) = backend_holding(row, &session_layers(row)).await;
        let end_line = u32::try_from(text.lines().count()).expect("a small program");
        let actions = backend
            .code_action(CodeActionParams {
                text_document: TextDocumentIdentifier { uri: uri.clone() },
                range: Range::new(Position::new(0, 0), Position::new(end_line, 0)),
                context: CodeActionContext::default(),
                work_done_progress_params: WorkDoneProgressParams::default(),
                partial_result_params: PartialResultParams::default(),
            })
            .await
            .expect("the code-action request succeeds")
            .unwrap_or_default();
        let views: Vec<ActionView<'_>> = actions
            .iter()
            .filter_map(|action| match action {
                CodeActionOrCommand::CodeAction(action) => Some(action),
                CodeActionOrCommand::Command(_) => None,
            })
            .map(|action| ActionView {
                title: &action.title,
                kind: action.kind.as_ref().map_or("", |kind| kind.as_str()),
                edits: action
                    .edit
                    .as_ref()
                    .and_then(|edit| edit.changes.as_ref())
                    .and_then(|changes| changes.get(&uri))
                    .into_iter()
                    .flatten()
                    .map(|edit| (edit.range.start.line + 1, edit.new_text.as_str()))
                    .collect(),
            })
            .collect();
        let observed: Vec<Observed> = row
            .expected(Surface::LspActions)
            .iter()
            .filter_map(|expect| {
                let line = expect.line?;
                Some(Observed {
                    code: expect.code,
                    line: Some(line),
                    state: ObservedState::Offered(offered(expect.code, line, &views)),
                })
            })
            .collect();
        if let Err(failure) = check(row, Surface::LspActions, &observed) {
            failures.push(failure);
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// `tcl-lsp.optimiseDocument` applies the fold exactly when the row shows
/// it: the slot's profile is the command's argument, its per-code keys sit
/// in the editor layer, and the other layers are the session's.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn every_rewrite_row_applies_through_optimise_document() {
    let mut failures: Vec<String> = Vec::new();
    for row in ROWS.iter().filter(|row| row.runs_on(Surface::LspRewrite)) {
        let mut layers = session_layers(row);
        let profile = layers
            .editor
            .get_mut("optimiser")
            .and_then(Value::as_object_mut)
            .and_then(|optimiser| optimiser.remove("profile"));
        let (backend, uri, _) = backend_holding(row, &layers).await;
        let mut args = vec![serde_json::json!(uri.as_str())];
        args.extend(profile);
        let result = backend
            .optimise_document_command(&args)
            .await
            .expect("the command succeeds")
            .expect("a result for a held document");
        let applied = result["source"]
            .as_str()
            .is_some_and(|source| source.contains("set x 3"));
        let observed: Vec<Observed> = row
            .expected(Surface::LspRewrite)
            .iter()
            .map(|expect| Observed {
                code: expect.code,
                line: expect.line,
                state: ObservedState::Applied(applied),
            })
            .collect();
        if let Err(failure) = check(row, Surface::LspRewrite, &observed) {
            failures.push(failure);
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
