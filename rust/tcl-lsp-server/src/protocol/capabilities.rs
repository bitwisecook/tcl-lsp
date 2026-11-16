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

//! What the client supports and what the server advertises.

use super::super::workspace::tcl_source_glob;
use tcl_lsp_core::semantic_tokens as core_semantic_tokens;
use tower_lsp_server::ls_types::{
    CallHierarchyServerCapability, CodeActionProviderCapability, CodeLensOptions,
    CompletionOptions, DeclarationCapability, DocumentLinkOptions, ExecuteCommandOptions,
    FileOperationFilter, FileOperationPattern, FileOperationPatternOptions,
    FileOperationRegistrationOptions, FoldingRangeProviderCapability, HoverProviderCapability,
    ImplementationProviderCapability, InitializeParams, OneOf, PositionEncodingKind, RenameOptions,
    SelectionRangeProviderCapability, SemanticTokensFullOptions, SemanticTokensLegend,
    SemanticTokensOptions, SemanticTokensServerCapabilities, ServerCapabilities,
    SignatureHelpOptions, TextDocumentSyncCapability, TextDocumentSyncKind,
    TextDocumentSyncOptions, TypeDefinitionProviderCapability, WorkDoneProgressOptions,
    WorkspaceFileOperationsServerCapabilities, WorkspaceFoldersServerCapabilities,
    WorkspaceServerCapabilities,
};

/// Return a fresh `result_id` for a semantic-tokens response.
/// Monotonically increasing via an atomic counter so each
/// snapshot we hand out is uniquely identifiable, letting
/// clients consult our cache via
/// `semanticTokens/full/delta { previousResultId }`.
/// Negotiate the position encoding the server will use against the
/// client's advertised `general.positionEncodings`.
///
/// Every provider in this server computes columns in UTF-16 code units
/// (via [`tcl_lexer::LineIndex::position_at_utf16`]), which is also the
/// LSP default — so we always operate in UTF-16 and only advertise it
/// explicitly when the client listed it (declaring an encoding the client
/// didn't offer is a protocol violation; omitting the field falls back to
/// the UTF-16 default either way).
pub(in super::super) fn negotiate_position_encoding(
    params: &InitializeParams,
) -> Option<PositionEncodingKind> {
    let supported = params
        .capabilities
        .general
        .as_ref()
        .and_then(|g| g.position_encodings.as_ref())?;
    supported
        .contains(&PositionEncodingKind::UTF16)
        .then_some(PositionEncodingKind::UTF16)
}

/// True when the client advertised a non-empty `general.positionEncodings`
/// list that omits UTF-16.
///
/// This is the one case the server cannot satisfy cleanly: every provider
/// emits UTF-16 columns, so a client that explicitly excludes UTF-16 would
/// mis-map any non-ASCII range.  The server still proceeds in UTF-16 (the
/// LSP default for an omitted server `positionEncoding`) rather than
/// refusing the session — UTF-16 is the universal LSP baseline and no
/// real client ships without it — but [`Backend::initialize`] logs a
/// warning so the mismatch is not silent.
pub(in super::super) fn client_lacks_utf16_support(params: &InitializeParams) -> bool {
    params
        .capabilities
        .general
        .as_ref()
        .and_then(|g| g.position_encodings.as_ref())
        .is_some_and(|encs| !encs.is_empty() && !encs.contains(&PositionEncodingKind::UTF16))
}

/// Whether the client advertised pull-diagnostic support
/// (`textDocument.diagnostic`).  Such a client (e.g. `vscode-languageclient`)
/// issues `textDocument/diagnostic` requests itself, so the server must not
/// *also* push diagnostics — pushing and pulling the same set lands them in
/// two diagnostic collections and shows each one twice.
pub(in super::super) fn client_supports_pull_diagnostics(params: &InitializeParams) -> bool {
    params
        .capabilities
        .text_document
        .as_ref()
        .and_then(|td| td.diagnostic.as_ref())
        .is_some()
}

/// Whether the client can match a watch pattern against a base URI
/// (`workspace.didChangeWatchedFiles.relativePatternSupport`, LSP 3.17).
///
/// This decides whether the server may watch a pack directory that sits
/// *outside* every workspace folder. A client without it has no way to be
/// told "watch that other directory": handed a bare absolute pattern, eglot
/// walks the whole project instead, adding one file-notify watch per
/// directory, and on a real repository dies with `File watching not
/// possible, no file descriptor left` (kqueue spends a descriptor per watch,
/// so macOS hits the ceiling first). An absolute pattern is not portably
/// declinable, so [`Backend::refresh_external_pack_watchers`] sends none.
pub(in super::super) fn client_supports_relative_watch_patterns(params: &InitializeParams) -> bool {
    params
        .capabilities
        .workspace
        .as_ref()
        .and_then(|w| w.did_change_watched_files.as_ref())
        .and_then(|w| w.relative_pattern_support)
        .unwrap_or(false)
}

/// Whether the client can render the actions of a `window/showMessageRequest`
/// (`window.showMessage.messageActionItem`). Without it the notice is a plain
/// `window/showMessage`, which every client renders.
pub(in super::super) fn client_supports_message_action_items(params: &InitializeParams) -> bool {
    params
        .capabilities
        .window
        .as_ref()
        .and_then(|w| w.show_message.as_ref())
        .is_some_and(|m| m.message_action_item.is_some())
}

/// Whether the client can open a URL for the server (`window.showDocument`).
pub(in super::super) fn client_supports_show_document(params: &InitializeParams) -> bool {
    params
        .capabilities
        .window
        .as_ref()
        .and_then(|w| w.show_document.as_ref())
        .is_some_and(|d| d.support)
}

/// Build the `ServerCapabilities` advertised in the response
/// to `initialize`.  Kept as a free function so the
/// `LanguageServer::initialize` handler stays focused on
/// state setup and result construction — the long capability
/// literal lives here rather than inside the trait method.
pub(in super::super) fn build_server_capabilities(
    position_encoding: Option<PositionEncodingKind>,
) -> ServerCapabilities {
    ServerCapabilities {
        // UTF-16 columns, matching the LSP default and what every provider
        // emits.  Advertised explicitly only when the client offered it
        // (see `negotiate_position_encoding`).
        position_encoding,
        // Options form (rather than the bare `Kind`) so we can advertise
        // `willSaveWaitUntil` alongside incremental sync. The handler is
        // gated by `tclLsp.features.willSaveWaitUntil` (default off).
        text_document_sync: Some(TextDocumentSyncCapability::Options(
            TextDocumentSyncOptions {
                open_close: Some(true),
                change: Some(TextDocumentSyncKind::INCREMENTAL),
                will_save: Some(false),
                will_save_wait_until: Some(true),
                save: None,
            },
        )),
        folding_range_provider: Some(FoldingRangeProviderCapability::Simple(true)),
        document_symbol_provider: Some(OneOf::Left(true)),
        hover_provider: Some(HoverProviderCapability::Simple(true)),
        completion_provider: Some(CompletionOptions {
            trigger_characters: Some(vec!["$".to_owned()]),
            ..CompletionOptions::default()
        }),
        signature_help_provider: Some(SignatureHelpOptions {
            trigger_characters: Some(vec![" ".to_owned()]),
            retrigger_characters: None,
            work_done_progress_options: WorkDoneProgressOptions::default(),
        }),
        definition_provider: Some(OneOf::Left(true)),
        declaration_provider: Some(DeclarationCapability::Simple(true)),
        type_definition_provider: Some(TypeDefinitionProviderCapability::Simple(true)),
        implementation_provider: Some(ImplementationProviderCapability::Simple(true)),
        references_provider: Some(OneOf::Left(true)),
        document_highlight_provider: Some(OneOf::Left(true)),
        rename_provider: Some(OneOf::Right(RenameOptions {
            prepare_provider: Some(true),
            work_done_progress_options: WorkDoneProgressOptions::default(),
        })),
        selection_range_provider: Some(SelectionRangeProviderCapability::Simple(true)),
        document_formatting_provider: Some(OneOf::Left(true)),
        document_range_formatting_provider: Some(OneOf::Left(true)),
        document_link_provider: Some(DocumentLinkOptions {
            resolve_provider: Some(false),
            work_done_progress_options: WorkDoneProgressOptions::default(),
        }),
        inlay_hint_provider: Some(OneOf::Left(true)),
        code_lens_provider: Some(CodeLensOptions {
            resolve_provider: Some(true),
        }),
        code_action_provider: Some(CodeActionProviderCapability::Simple(true)),
        call_hierarchy_provider: Some(CallHierarchyServerCapability::Simple(true)),
        // `ServerCapabilities` (ls-types 0.0.6) carries no
        // `type_hierarchy_provider` field, and dynamic
        // `client/registerCapability` does not surface in the client's
        // `initializeResult.capabilities` (which editors inspect to decide the
        // provider is present).  The type-hierarchy capability is therefore
        // injected into the serialised `initialize` response by the
        // `inject_type_hierarchy_provider` service layer in `main.rs`; the
        // request handlers (`prepare_type_hierarchy` / `supertypes` /
        // `subtypes`) back it here.
        semantic_tokens_provider: Some(semantic_tokens_capability()),
        workspace_symbol_provider: Some(OneOf::Left(true)),
        linked_editing_range_provider: Some(
            tower_lsp_server::ls_types::LinkedEditingRangeServerCapabilities::Simple(true),
        ),
        // Pull-model diagnostics are **opt-in** and intentionally NOT
        // advertised by default: `vscode-languageclient` (and most clients)
        // switch to pull mode the moment `diagnosticProvider` is present, which
        // silently disables our richer push pipeline (`publish_diagnostics`)
        // and makes clients render each diagnostic twice.  The
        // `textDocument/diagnostic` + `workspace/diagnostic` handlers still
        // exist for a client that requests them directly, but the default
        // capability set leaves this absent so push stays the sole delivery
        // channel. Switching models is an initialize-time client/server
        // contract, not a live feature toggle.
        diagnostic_provider: None,
        // Every workspace command an editor may invoke. A command absent
        // here is not offered by conforming clients, so this list and the
        // `execute_command` dispatch move together.
        execute_command_provider: Some(server_execute_command_options()),
        // Advertise willRename /
        // didRename so the editor consults us before/after a `.tcl`
        // rename — the willRename handler rewrites dependents' `source`
        // literals, the didRename handler reindexes the moved file.
        workspace: Some(server_workspace_capabilities()),
        ..ServerCapabilities::default()
    }
}

fn server_workspace_capabilities() -> WorkspaceServerCapabilities {
    WorkspaceServerCapabilities {
        // Advertise multi-root support and ask the client to send
        // `workspace/didChangeWorkspaceFolders` so folders added or removed
        // mid-session update the index and configuration (the
        // `did_change_workspace_folders` handler), not just at startup.
        workspace_folders: Some(WorkspaceFoldersServerCapabilities {
            supported: Some(true),
            change_notifications: Some(OneOf::Left(true)),
        }),
        file_operations: Some(WorkspaceFileOperationsServerCapabilities {
            will_rename: Some(rename_file_operation_options()),
            did_rename: Some(rename_file_operation_options()),
            ..WorkspaceFileOperationsServerCapabilities::default()
        }),
    }
}

fn server_execute_command_options() -> ExecuteCommandOptions {
    ExecuteCommandOptions {
        commands: vec![
            "tcl-lsp.minifyDocument".to_owned(),
            "tcl-lsp.captureDocumentSourceContext".to_owned(),
            "tcl-lsp.validateDocumentSourceContext".to_owned(),
            "tcl-lsp.validateDocumentTransform".to_owned(),
            "tcl-lsp.optimiseDocument".to_owned(),
            "tcl-lsp.unminifyError".to_owned(),
            "tcl-lsp.describeIruleEvent".to_owned(),
            "tcl-lsp.describeIruleCommand".to_owned(),
            "tcl-lsp.listIruleEvents".to_owned(),
            "tcl-lsp.diagramData".to_owned(),
            "tcl-lsp.xcTranslate".to_owned(),
            "tcl-lsp.xcTranslateDocument".to_owned(),
            "tcl-lsp.listRules".to_owned(),
            "tcl-lsp.extractRule".to_owned(),
            "tcl-lsp.writeRuleBack".to_owned(),
            "tcl-lsp.extractLinkedObjects".to_owned(),
            "tcl-lsp.bigipCleanup".to_owned(),
            "tcl-lsp.minimizeDiagnostic".to_owned(),
            "tcl-lsp.renamePartition".to_owned(),
            "tcl-lsp.currentDiagnostics".to_owned(),
            "tcl-lsp.validateDiagnosticPublication".to_owned(),
            "tcl-lsp.proposalDiagnostics".to_owned(),
            "tcl-lsp.getEffectiveConfig".to_owned(),
            "tcl-lsp.fixAllSafeIssues".to_owned(),
            "tcl-lsp.listSubcommands".to_owned(),
            "tcl-lsp.insertPackageRequire".to_owned(),
            "tcl-lsp.listKnownPackages".to_owned(),
            "tcl-lsp.suggestPackagesForSymbol".to_owned(),
            "tcl-lsp.exportConfig".to_owned(),
            "tcl-lsp.listTclInstallations".to_owned(),
            "tcl-lsp.listDialects".to_owned(),
            "tcl-lsp.setDialect".to_owned(),
            "tcl-lsp.setSessionDialectOverride".to_owned(),
            "tcl-lsp.setDocumentDialectOverride".to_owned(),
            "tcl-lsp.studioSource".to_owned(),
            "tcl-lsp.compilerExplorer".to_owned(),
            "tcl-lsp.compilerExplorerView".to_owned(),
            "tcl-lsp.tkPreview".to_owned(),
            "tcl-lsp.ilxReferences".to_owned(),
        ],
        work_done_progress_options: WorkDoneProgressOptions::default(),
    }
}

/// File-operation filter for `willRename` / `didRename`: match the Tcl
/// source extensions the workspace scan indexes ([`TCL_SOURCE_EXTENSIONS`]).
///
/// `ignoreCase` so the filter agrees with [`is_tcl_source`], which case-folds:
/// an `UPPER.TCL` the scan indexed must still have its `source` references
/// rewritten when it is renamed.
pub(in super::super) fn rename_file_operation_options() -> FileOperationRegistrationOptions {
    FileOperationRegistrationOptions {
        filters: vec![FileOperationFilter {
            scheme: Some("file".to_owned()),
            pattern: FileOperationPattern {
                glob: tcl_source_glob(),
                matches: None,
                options: Some(FileOperationPatternOptions {
                    ignore_case: Some(true),
                }),
            },
        }],
    }
}

/// Build the semantic-tokens capability advertising the classifier's legend,
/// `range` support, and `full` **with** `delta` support.
///
/// The `full/delta` handler ([`Backend::semantic_tokens_full_delta`]) returns a
/// real minimal edit (via [`core_semantic_tokens::diff`]) whenever the client's
/// `previousResultId` matches the last stream we served, so advertising delta
/// is a genuine incremental win for every client that uses it — the same shape
/// rust-analyzer and clangd advertise.
pub(in super::super) fn semantic_tokens_capability() -> SemanticTokensServerCapabilities {
    SemanticTokensServerCapabilities::SemanticTokensOptions(SemanticTokensOptions {
        work_done_progress_options: WorkDoneProgressOptions::default(),
        legend: SemanticTokensLegend {
            token_types: core_semantic_tokens::legend_token_types()
                .into_iter()
                .map(tower_lsp_server::ls_types::SemanticTokenType::new)
                .collect(),
            token_modifiers: core_semantic_tokens::legend_token_modifiers()
                .into_iter()
                .map(tower_lsp_server::ls_types::SemanticTokenModifier::new)
                .collect(),
        },
        range: Some(true),
        full: Some(SemanticTokensFullOptions::Delta { delta: Some(true) }),
    })
}
