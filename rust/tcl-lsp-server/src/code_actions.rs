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

//! Code actions: the report the lightbulb reads and the lift of its actions.

use super::configuration::PolicyLayers;
use super::diagnostics::{published_findings, published_report};
use super::documents::DocumentSourceResult;
use super::protocol::lift_lsp_range;
use super::semantic_tokens::core_semantic_range;
use super::{Backend, DocumentState, readonly_reference_reports};
use std::collections::HashSet;
use std::ops::ControlFlow;
use std::sync::Arc;
use tcl_compiler::analyser::AnalysisResult;
use tcl_lsp_core::definition::LspRange as CoreLspRange;
use tcl_lsp_core::{
    code_actions as core_code_actions, definition as core_definition,
    diagnostic_policy as core_policy, diagnostic_report as core_report,
    formatting as core_formatting, workspace_index as core_workspace_index,
};
use tcl_registry::CommandRegistry;
use tower_lsp_server::jsonrpc;
use tower_lsp_server::ls_types::{
    CodeAction, CodeActionOrCommand, CodeActionParams, DocumentChanges, OneOf,
    OptionalVersionedTextDocumentIdentifier, Range, TextDocumentEdit, TextEdit, Uri, WorkspaceEdit,
};

mod requests;

/// [`code_action_report`]'s inputs beyond the document and its analysis,
/// grouped so the call site does not spell out seven parameters.
pub(super) struct CodeActionReportInputs {
    pub(super) published: Vec<tcl_compiler::analyser::Diagnostic>,
    pub(super) registry: Arc<CommandRegistry>,
    pub(super) generic_patterns: Option<Vec<String>>,
    pub(super) evidence: Option<Arc<tcl_compiler::unit_scope::CallSiteEvidence>>,
    pub(super) layers: PolicyLayers,
    pub(super) style_line_length: u32,
    pub(super) xc_for_irules: bool,
}

/// The report the lightbulb reads (`docs/design/compiler/diagnostic-policy.md`
/// § Adapters, code actions): `published_findings` over the uncached
/// compiler checks and the document's own producers (the style pass, the
/// `SslicTcl` projection, O111, the XC findings), through `published_report`
/// — the pull path's own shape, so a fix is decided against the exact set
/// the document publishes.
///
/// The checks retain this analysis's complete input and the project's
/// independent call-site evidence. Missing or stale supplied ownership withholds
/// compiler suggestions while the existing analyser findings remain available.
pub(super) fn code_action_report(
    doc: &DocumentState,
    analysis: &AnalysisResult,
    inputs: &CodeActionReportInputs,
) -> core_policy::Report {
    let checks = tcl_lsp_db::compiler_check_diagnostics_uncached_from_analysis(
        &doc.text,
        &inputs.registry,
        analysis,
        inputs.generic_patterns.as_deref(),
        inputs.evidence.as_deref(),
    );
    let produced = published_findings(
        &inputs.published,
        &checks,
        inputs.xc_for_irules.then_some(analysis),
        &doc.text,
    );
    // `doc.text` is already this snapshot's analysis form
    // (`DocumentState::normalised_for_analysis`); `doc.raw()` is the
    // client's exact buffer, kept aside for a lone-`\r` document.
    let source_doc = core_report::DocumentSource {
        text: doc.raw(),
        analysis_text: &doc.text,
        decode: doc.decode_report.as_ref(),
        dialect: tcl_lsp_core::profile_for_dialect(&doc.dialect),
        pass: core_report::SourcePass::Tcl {
            line_length: inputs.style_line_length as usize,
        },
    };
    published_report(
        &source_doc,
        produced,
        &inputs.layers,
        core_policy::Directives::from_analysis(analysis, &doc.text),
        Some(analysis),
    )
}

/// The dialect-specific code actions, appended to the generic Tcl set.
///
/// BIG-IP `.conf`/`.scf` gets the rename-partition / rename-object /
/// extract-rule actions: the generic Tcl code-action path yields nothing
/// useful on a non-Tcl config, so these extend rather than replace it —
/// mirroring the references / document-links BIG-IP routing.  iRules
/// additionally gets the `# Profiles:` header source action, and an authoring
/// dialect whose loader dropped a word gets the did-you-mean rewrite for it.
fn push_dialect_code_actions(
    actions: &mut Vec<core_code_actions::CodeAction>,
    source: &str,
    range: core_definition::LspRange,
    inputs: &DialectActionInputs<'_>,
) {
    if Backend::is_bigip_dialect(inputs.dialect.name) {
        actions.extend(core_code_actions::bigip_code_actions(
            source,
            range,
            inputs.uri,
            inputs.version,
        ));
    }
    if inputs.dialect.is_irules()
        && let Some(a) =
            core_code_actions::profiles_action(source, inputs.analysis, inputs.registry)
    {
        actions.push(a);
    }
    // A pack's load notices reach here on the request's context diagnostics,
    // like any other finding the editor is showing: they are published on the
    // pack file's own URI, merged into its set by the publisher.
    actions.extend(core_code_actions::spec_pack_quick_fixes(
        source,
        inputs.analysis,
        inputs.context_diags,
    ));
}

/// What the dialect-specific code actions read and the generic Tcl set does
/// not — grouped so a new dialect's inputs do not change every call site.
struct DialectActionInputs<'a> {
    pub(super) uri: &'a str,
    /// The document version the actions are computed from; a deferred
    /// command that edits the document carries it so the host can refuse a
    /// document that has moved on (see `bigip_code_actions`).
    pub(super) version: Option<u32>,
    pub(super) dialect: &'static tcl_dialect::DialectProfile,
    pub(super) analysis: &'a tcl_compiler::analyser::AnalysisResult,
    pub(super) registry: &'a tcl_registry::CommandRegistry,
    /// The diagnostics the editor is currently showing on the document — the
    /// channel a notice published outside the analyser pipeline arrives on.
    context_diags: &'a [core_code_actions::ContextDiagnostic],
}

/// The context-diagnostic actions, plus the fuzzy `package require`
/// suggestion.
///
/// That suggestion needs both the analysis — to prove the cursor is on an
/// unresolved *command head* rather than on a comment, a string, or a data
/// word — and the request's own diagnostics, since the editor may be showing a
/// W123 this analysis has not re-emitted.  Passing neither is what let it fire
/// anywhere an identifier-shaped word appears.
fn push_context_code_actions(
    actions: &mut Vec<core_code_actions::CodeAction>,
    source: &str,
    range: core_definition::LspRange,
    registry: &tcl_registry::CommandRegistry,
    program: core_definition::ProgramExports<'_>,
    analysis: &tcl_compiler::analyser::AnalysisResult,
    context_diags: &[core_code_actions::ContextDiagnostic],
) {
    actions.extend(core_code_actions::package_require_actions_in_program(
        source,
        range,
        core_definition::CallResolution {
            registry: Some(registry),
            program: Some(program),
        },
        Some(analysis),
        context_diags,
    ));
    actions.extend(core_code_actions::context_diagnostic_actions_in_analysis(
        source,
        analysis,
        registry,
        context_diags,
    ));
}

struct CodeActionWorkerInputs<'a> {
    uri_key: &'a str,
    pub(super) exports: &'a Arc<core_workspace_index::NamespaceExportSnapshot>,
    pub(super) doc: &'a DocumentState,
    pub(super) analysis: &'a Arc<AnalysisResult>,
    report_inputs: &'a CodeActionReportInputs,
    pub(super) range: CoreLspRange,
    pub(super) docstring_style: core_formatting::DocstringStyle,
    pub(super) registry: &'a Arc<CommandRegistry>,
    context_diags: &'a [core_code_actions::ContextDiagnostic],
    pub(super) uri_str: &'a str,
    pub(super) dialect: &'a str,
}

impl Backend {
    async fn code_action_source_for_range(
        &self,
        uri: &Uri,
        doc: &DocumentState,
        range: Range,
    ) -> DocumentSourceResult<
        ControlFlow<(), Option<readonly_reference_reports::OriginalCodeActionSource>>,
    > {
        if doc.dialect == "f5-bigip" {
            return Ok(ControlFlow::Continue(None));
        }
        let Some(source) = self.capture_code_action_source(uri, doc).await? else {
            return Ok(ControlFlow::Break(()));
        };
        if !source.contains_range(range) {
            return Ok(ControlFlow::Break(()));
        }
        Ok(ControlFlow::Continue(Some(source)))
    }

    async fn analysis_for_code_action_source(
        &self,
        uri: &Uri,
        doc: &DocumentState,
        source: Option<&readonly_reference_reports::OriginalCodeActionSource>,
    ) -> DocumentSourceResult<Arc<AnalysisResult>> {
        match source {
            Some(source) => Ok(source.analysis()),
            None => {
                self.analysis_for(uri, doc.text.clone(), doc.dialect.clone())
                    .await
            }
        }
    }

    async fn code_action_published_diagnostics(
        &self,
        uri: &Uri,
        doc: &DocumentState,
        analysis: &Arc<AnalysisResult>,
        registry: &Arc<CommandRegistry>,
        disabled_codes: &HashSet<String>,
    ) -> Vec<tcl_compiler::analyser::Diagnostic> {
        self.published_analyser_diagnostics(
            uri,
            &doc.text,
            analysis,
            tcl_lsp_core::profile_for_dialect(&doc.dialect),
            registry,
            disabled_codes,
        )
        .await
    }

    fn build_document_code_actions(
        inputs: &CodeActionWorkerInputs<'_>,
    ) -> Vec<core_code_actions::CodeAction> {
        let CodeActionWorkerInputs {
            uri_key,
            exports,
            doc,
            analysis,
            report_inputs,
            range,
            docstring_style,
            registry,
            context_diags,
            uri_str,
            dialect,
        } = *inputs;
        let program = core_definition::ProgramExports {
            uri: uri_key,
            oracle: exports.as_ref(),
        };
        let report = code_action_report(doc, analysis, report_inputs);
        // `program` decides what an inlined call reaches; `report`
        // decides what this document shows.
        let mut actions = core_code_actions::code_actions_in_program(
            &doc.text,
            range,
            Some(analysis),
            &report,
            Some(program),
            docstring_style,
        );
        push_context_code_actions(
            &mut actions,
            &doc.text,
            range,
            registry,
            program,
            analysis,
            context_diags,
        );
        let dialect_inputs = DialectActionInputs {
            uri: uri_str,
            version: doc.version.and_then(|version| u32::try_from(version).ok()),
            dialect: tcl_lsp_core::profile_for_dialect(dialect),
            analysis,
            registry,
            context_diags,
        };
        push_dialect_code_actions(&mut actions, &doc.text, range, &dialect_inputs);
        actions
    }
}

impl Backend {
    pub(super) async fn code_action_attempt(
        &self,
        uri: &Uri,
        params: &CodeActionParams,
    ) -> jsonrpc::Result<ControlFlow<Option<Vec<CodeActionOrCommand>>>> {
        let Some(doc) = self.read_document(uri).await? else {
            return Ok(ControlFlow::Break(None));
        };
        let range = core_semantic_range(params.range);
        // The dedicated BIG-IP adapter retains its existing routing. Ordinary
        // source edits require an actual open, versioned document receipt.
        let ControlFlow::Continue(source) = self
            .code_action_source_for_range(uri, &doc, params.range)
            .await?
        else {
            return Ok(ControlFlow::Break(None));
        };
        let analysis = self
            .analysis_for_code_action_source(uri, &doc, source.as_ref())
            .await?;
        // `S-code-actions-rich`: walks the analyser's
        // diagnostics for fixes whose span overlaps the
        // requested range, plus fuzzy `package require`
        // suggestions for the word at the cursor.  Run on a worker.
        let Some(registry) = analysis
            .resolved_registry()
            .map(|registry| registry.snapshot().shared_registry())
        else {
            return Ok(ControlFlow::Break(None));
        };
        // The resolved per-check disabled set — the same one the diagnostics
        // path resolves for this document (folder overrides included) — so the
        // compiler-checks code-action path does not re-surface a quick-fix for
        // a diagnostic the user disabled.
        let (disabled_codes, ..) = self.resolved_analysis_settings(uri).await;
        // The analyser diagnostics this document actually publishes.  Code
        // actions lift their quick-fixes from this set, never from the
        // analyser's raw one, so a W123 the workspace refinements decided to
        // suppress can never leave a "did you mean…?" rewrite behind
        // when the diagnostic itself is gone. What the document *shows* of
        // it is the policy step's decision, taken below with the compiler
        // checks and the optimiser's rewrites in the same report.
        let published = self
            .code_action_published_diagnostics(uri, &doc, &analysis, &registry, &disabled_codes)
            .await;
        // Lift the request-context diagnostics (the editor sends the ones it
        // currently shows) so context-driven quick-fixes — e.g. the iRules
        // taint encode-wrap fixes — can act on them even when the analyser
        // didn't re-emit them.
        let context_diags = lift_context_diagnostics(&params.context.diagnostics);
        let dialect = doc.dialect.clone();
        let uri_str = uri.as_str().to_string();
        // IRULE4002 generic-name patterns for the iRules-only compiler-checks
        // code-action lowering below (uncached, off the salsa path).
        let generic_patterns = self.generic_variable_patterns.lock().await.clone();
        // The report the lightbulb reads is built the pull path's way
        // (`published_findings`, `published_report`): the style line length
        // and the XC switch feed the same producers, so a fix is decided
        // against the exact set the document publishes.
        let xc_for_irules = tcl_lsp_core::profile_for_dialect(&dialect).is_irules()
            && self.xc_diagnostics_enabled(uri).await;
        let policy_layers = self.resolved_policy_layers(uri).await;
        let report_inputs = CodeActionReportInputs {
            published,
            registry: Arc::clone(&registry),
            generic_patterns,
            evidence: self.cross_file_evidence_for(uri).await,
            layers: policy_layers,
            style_line_length: self.resolved_style_line_length(uri).await,
            xc_for_irules,
        };
        // The action builders compose their inserted text with plain `\n`;
        // this is the line ending it is retargeted onto below, so a docstring
        // / `package require` / `# noqa` / extracted `set` inserted into a
        // CRLF or old-Mac document keeps that document's terminators.
        let line_ending = self.resolved_edit_line_ending(uri, doc.raw()).await;
        // The inline-proc refactor substitutes the reached proc's body, so
        // it needs the same export view definition uses.
        let exports = match &source {
            Some(source) => source.exports(),
            None => self.export_snapshot().await,
        };
        // `tclLsp.formatting.docstringStyle` — resolved the same way
        // as every other folder-overridable formatting setting, then handed
        // to the docstring source action so it actually governs where (or
        // whether) a generated stub is inserted.
        let docstring_style = self.resolved_docstring_style(uri).await;
        let uri_key = uri.as_str().to_owned();
        let mut actions = crate::rt::spawn_blocking(move || {
            Self::build_document_code_actions(&CodeActionWorkerInputs {
                uri_key: &uri_key,
                exports: &exports,
                doc: &doc,
                analysis: &analysis,
                report_inputs: &report_inputs,
                range,
                docstring_style,
                registry: &registry,
                context_diags: &context_diags,
                uri_str: &uri_str,
                dialect: &dialect,
            })
        })
        .await
        .map_err(|err| jsonrpc::Error {
            code: jsonrpc::ErrorCode::InternalError,
            message: format!("code_action worker panicked: {err}").into(),
            data: None,
        })?;
        if actions.is_empty() {
            return Ok(ControlFlow::Break(None));
        }
        core_code_actions::retarget_newlines(&mut actions, &line_ending);
        let lifted = if let Some(source) = &source {
            match self
                .code_action_response_transaction(source, actions, params.context.only.as_ref())
                .await?
            {
                readonly_reference_reports::ReadonlyTransaction::Published(lifted) => lifted,
                readonly_reference_reports::ReadonlyTransaction::PublishRefused => {
                    return Ok(ControlFlow::Break(None));
                }
                readonly_reference_reports::ReadonlyTransaction::CurrencyRefused => {
                    return Ok(ControlFlow::Continue(()));
                }
            }
        } else {
            let mut lifted = lift_code_actions(actions, uri, params.context.only.as_ref());
            for action in &mut lifted {
                attach_source_action_data(action, serde_json::json!({"kind": "legacy-bigip"}));
            }
            lifted
        };
        if lifted.is_empty() {
            return Ok(ControlFlow::Break(None));
        }
        Ok(ControlFlow::Break(Some(lifted)))
    }
}

/// Lift the editor's request-context diagnostics (the ones it currently shows)
/// into the core's [`core_code_actions::ContextDiagnostic`] shape so
/// context-driven quick-fixes can act on them even when the analyser did not
/// re-emit them.
fn lift_context_diagnostics(
    diagnostics: &[tower_lsp_server::ls_types::Diagnostic],
) -> Vec<core_code_actions::ContextDiagnostic> {
    diagnostics
        .iter()
        .filter_map(|d| {
            let code = match d.code.as_ref()? {
                tower_lsp_server::ls_types::NumberOrString::String(s) => s.clone(),
                tower_lsp_server::ls_types::NumberOrString::Number(n) => n.to_string(),
            };
            Some(core_code_actions::ContextDiagnostic {
                data: d.data.as_ref().and_then(|data| {
                    core_code_actions::ContextDiagnosticData::from_value(data, &code)
                }),
                code,
                message: d.message.clone(),
                range: CoreLspRange {
                    start_line: d.range.start.line,
                    start_character: d.range.start.character,
                    end_line: d.range.end.line,
                    end_character: d.range.end.character,
                },
            })
        })
        .collect()
}

/// Filter the computed actions by the client's `only` kinds and lift each into
/// an LSP [`CodeActionOrCommand`] for document `uri`.
fn lift_code_actions(
    actions: Vec<core_code_actions::CodeAction>,
    uri: &Uri,
    only: Option<&Vec<tower_lsp_server::ls_types::CodeActionKind>>,
) -> Vec<CodeActionOrCommand> {
    // Honour the client's `only` filter (e.g. `["refactor.extract"]`): an
    // action is kept when its kind prefix-matches a requested kind in
    // either direction (`refactor` matches `refactor.extract` and vice
    // versa).
    let only: Option<Vec<String>> = only.map(|kinds| {
        kinds
            .iter()
            .map(|k| k.as_str().to_owned())
            .collect::<Vec<_>>()
    });
    actions
        .into_iter()
        .filter(|a| {
            only.as_ref().is_none_or(|wanted| {
                let k = a.kind.as_str();
                // Keep an action when its kind exactly matches a requested
                // kind, or is a *subtype* of one (`refactor.extract`
                // satisfies a `refactor` request). A requested
                // `refactor.extract` must NOT pull in a generic `refactor`
                // action — the requested kind is not a parent of the
                // action's kind in that direction (LSP CodeActionKind
                // matching is prefix-on-the-action-side only).
                wanted
                    .iter()
                    .any(|w| k == w || k.starts_with(&format!("{w}.")))
            })
        })
        .map(|a| {
            let mut changes = std::collections::HashMap::new();
            let lifted_edits: Vec<TextEdit> = a
                .edits
                .into_iter()
                .map(|e| TextEdit {
                    range: lift_lsp_range(e.range),
                    new_text: e.new_text,
                })
                .collect();
            changes.insert(uri.clone(), lifted_edits);
            let command = a.command.map(|c| action_command_to_lsp(a.title.clone(), c));
            // Surface the rendered tmsh data-group definition (the
            // extract-to-datagroup refactor) as the action's `data`
            // payload.
            let data = a
                .data_group_definition
                .map(|def| serde_json::json!({ "data_group_definition": def }));
            // A refactoring that found its subject but cannot preserve
            // behaviour is surfaced *greyed out* with its reason, rather than
            // silently omitted: LSP's `disabled.reason` is what the editor
            // shows, and without it a user cannot tell "does not apply here"
            // from "is broken".
            let disabled = a
                .disabled
                .map(|reason| tower_lsp_server::ls_types::CodeActionDisabled { reason });
            CodeActionOrCommand::CodeAction(CodeAction {
                title: a.title,
                kind: Some(tower_lsp_server::ls_types::CodeActionKind::new(
                    a.kind.as_str(),
                )),
                diagnostics: None,
                edit: Some(WorkspaceEdit {
                    changes: Some(changes),
                    document_changes: None,
                    change_annotations: None,
                }),
                command,
                is_preferred: None,
                disabled,
                data,
            })
        })
        .collect()
}

/// Retain edit versions for LSP clients and the original source for the
/// checked editor adapter. The caller holds source/input/provider currency.
pub(super) fn lift_versioned_code_actions(
    actions: Vec<core_code_actions::CodeAction>,
    uri: &Uri,
    version: i32,
    source: &str,
    only: Option<&Vec<tower_lsp_server::ls_types::CodeActionKind>>,
) -> Vec<CodeActionOrCommand> {
    let mut lifted = lift_code_actions(actions, uri, only);
    for action in &mut lifted {
        attach_source_action_data(
            action,
            serde_json::json!({
                "kind": "versioned", "uri": uri.as_str(), "version": version, "source": source,
            }),
        );
        if let CodeActionOrCommand::CodeAction(action) = action
            && let Some(edit) = &mut action.edit
            && let Some(mut changes) = edit.changes.take()
        {
            let edits = changes.remove(uri).unwrap_or_default();
            edit.document_changes = Some(DocumentChanges::Edits(vec![TextDocumentEdit {
                text_document: OptionalVersionedTextDocumentIdentifier {
                    uri: uri.clone(),
                    version: Some(version),
                },
                edits: edits.into_iter().map(OneOf::Left).collect(),
            }]));
        }
    }
    lifted
}

fn attach_source_action_data(action: &mut CodeActionOrCommand, intent: serde_json::Value) {
    if let CodeActionOrCommand::CodeAction(action) = action {
        let data = action.data.get_or_insert_with(|| serde_json::json!({}));
        if let Some(object) = data.as_object_mut() {
            object.insert("sourceAction".to_owned(), intent);
        }
    }
}

/// Convert a core [`ActionCommand`](core_code_actions::ActionCommand) into an
/// LSP [`Command`](tower_lsp_server::ls_types::Command), forwarding **both**
/// argument kinds.
///
/// A given action command carries exactly one kind:
/// * integer-position commands (`tclLsp.renameSymbolAtPosition`) use `args`
///   (`[line, start, end]`), and
/// * BIG-IP / editor string commands use `string_args` —
///   `tclLsp.renamePartition` → `[uri, partition]`, `editor.action.rename`
///   → `[uri]`.
///
/// String args are emitted first, then int args. Because each command uses
/// only one kind, the concatenation yields the exact `arguments` array the
/// client expects, without this
/// conversion needing to know which kind it is handling. Forwarding
/// `string_args` here is what stops the BIG-IP rename actions from reaching
/// the editor argument-less.
pub(super) fn action_command_to_lsp(
    title: String,
    c: core_code_actions::ActionCommand,
) -> tower_lsp_server::ls_types::Command {
    let mut arguments: Vec<serde_json::Value> = c
        .string_args
        .into_iter()
        .map(serde_json::Value::from)
        .collect();
    arguments.extend(c.args.into_iter().map(serde_json::Value::from));
    tower_lsp_server::ls_types::Command {
        title,
        command: c.command,
        arguments: Some(arguments),
    }
}
