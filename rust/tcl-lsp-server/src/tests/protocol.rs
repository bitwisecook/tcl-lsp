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

//! Capability negotiation, bounded client requests and lifts.

use super::*;

/// `bounded_client_request` must turn a request that never resolves into
/// an error once its deadline passes — the load-bearing half of issue
/// #2021's fix: a server-to-client reply that will never arrive must not
/// keep a handler future (and with it, per `Server::serve`'s contract,
/// the whole process) alive forever. Goes through the parameterised inner
/// helper with a short deadline so the test does not have to wait out the
/// production 10s to prove it.
#[tokio::test]
async fn bounded_client_request_errors_once_its_deadline_elapses() {
    let never: std::future::Pending<jsonrpc::Result<()>> = std::future::pending();
    let started = std::time::Instant::now();
    let result =
        bounded_client_request_with_timeout(std::time::Duration::from_millis(20), never).await;
    assert!(
        result.is_err(),
        "a request that never resolves must time out as an error, not hang forever"
    );
    assert!(
        started.elapsed() < std::time::Duration::from_secs(5),
        "the timeout must actually bound the wait: took {:?}",
        started.elapsed()
    );
}

#[test]
fn semantic_tokens_capability_advertises_delta_and_range() {
    use tower_lsp_server::ls_types::SemanticTokensServerCapabilities as Cap;
    let Cap::SemanticTokensOptions(o) = semantic_tokens_capability() else {
        panic!("expected SemanticTokensOptions");
    };
    assert_eq!(o.range, Some(true));
    assert!(matches!(
        o.full,
        Some(SemanticTokensFullOptions::Delta { delta: Some(true) })
    ));
}

#[test]
fn position_encoding_negotiation() {
    use tower_lsp_server::ls_types::{ClientCapabilities, GeneralClientCapabilities};
    let params_with = |encs: Option<Vec<PositionEncodingKind>>| InitializeParams {
        capabilities: ClientCapabilities {
            general: Some(GeneralClientCapabilities {
                position_encodings: encs,
                ..GeneralClientCapabilities::default()
            }),
            ..ClientCapabilities::default()
        },
        ..InitializeParams::default()
    };
    let with_encodings =
        |encs: Option<Vec<PositionEncodingKind>>| negotiate_position_encoding(&params_with(encs));
    // Client offers UTF-16 → advertise it explicitly.
    assert_eq!(
        with_encodings(Some(vec![PositionEncodingKind::UTF16])),
        Some(PositionEncodingKind::UTF16)
    );
    // Client lists only UTF-8 → don't advertise an encoding it can't
    // honour; omitting it falls back to the UTF-16 default.
    assert_eq!(with_encodings(Some(vec![PositionEncodingKind::UTF8])), None);
    // No `general` capability at all → omit (default UTF-16).
    assert_eq!(
        negotiate_position_encoding(&InitializeParams::default()),
        None
    );

    // `client_lacks_utf16_support`: only true for a non-empty list that
    // omits UTF-16 — the case `initialize` warns about.
    assert!(client_lacks_utf16_support(&params_with(Some(vec![
        PositionEncodingKind::UTF8
    ]))));
    assert!(client_lacks_utf16_support(&params_with(Some(vec![
        PositionEncodingKind::UTF8,
        PositionEncodingKind::UTF32,
    ]))));
    // A list containing UTF-16, an empty list, and no list at all are all
    // fine — UTF-16 is available (or defaulted) so no warning fires.
    assert!(!client_lacks_utf16_support(&params_with(Some(vec![
        PositionEncodingKind::UTF8,
        PositionEncodingKind::UTF16,
    ]))));
    assert!(!client_lacks_utf16_support(&params_with(Some(vec![]))));
    assert!(!client_lacks_utf16_support(&InitializeParams::default()));
}

#[test]
fn pull_diagnostic_capability_detection() {
    use tower_lsp_server::ls_types::{
        ClientCapabilities, DiagnosticClientCapabilities, TextDocumentClientCapabilities,
    };
    // No `textDocument.diagnostic` → push-only client (no pull).
    assert!(!client_supports_pull_diagnostics(
        &InitializeParams::default()
    ));
    // A client advertising `textDocument/diagnostic` support → pull-capable,
    // so the worker must stop pushing to avoid a double display.
    let pull_params = InitializeParams {
        capabilities: ClientCapabilities {
            text_document: Some(TextDocumentClientCapabilities {
                diagnostic: Some(DiagnosticClientCapabilities::default()),
                ..TextDocumentClientCapabilities::default()
            }),
            ..ClientCapabilities::default()
        },
        ..InitializeParams::default()
    };
    assert!(client_supports_pull_diagnostics(&pull_params));
}

#[test]
fn relative_watch_pattern_capability_detection() {
    use tower_lsp_server::ls_types::{
        ClientCapabilities, DidChangeWatchedFilesClientCapabilities, WorkspaceClientCapabilities,
    };
    let params_with = |support: Option<bool>| InitializeParams {
        capabilities: ClientCapabilities {
            workspace: Some(WorkspaceClientCapabilities {
                did_change_watched_files: Some(DidChangeWatchedFilesClientCapabilities {
                    relative_pattern_support: support,
                    ..DidChangeWatchedFilesClientCapabilities::default()
                }),
                ..WorkspaceClientCapabilities::default()
            }),
            ..ClientCapabilities::default()
        },
        ..InitializeParams::default()
    };
    // Only an explicit `true` earns an external-pack watcher: handed a
    // bare absolute pattern instead, a client without relative patterns
    // matches it against the workspace and watches the whole project.
    assert!(client_supports_relative_watch_patterns(&params_with(Some(
        true
    ))));
    assert!(!client_supports_relative_watch_patterns(&params_with(
        Some(false)
    )));
    // Field absent, `didChangeWatchedFiles` absent, and no `workspace`
    // capability at all all mean "cannot express a base URI".
    assert!(!client_supports_relative_watch_patterns(&params_with(None)));
    assert!(!client_supports_relative_watch_patterns(
        &InitializeParams {
            capabilities: ClientCapabilities {
                workspace: Some(WorkspaceClientCapabilities::default()),
                ..ClientCapabilities::default()
            },
            ..InitializeParams::default()
        }
    ));
    assert!(!client_supports_relative_watch_patterns(
        &InitializeParams::default()
    ));
}

#[test]
fn message_capability_detection() {
    use tower_lsp_server::ls_types::{
        ClientCapabilities, MessageActionItemCapabilities, ShowDocumentClientCapabilities,
        ShowMessageRequestClientCapabilities, WindowClientCapabilities,
    };
    let params_with = |window: Option<WindowClientCapabilities>| InitializeParams {
        capabilities: ClientCapabilities {
            window,
            ..ClientCapabilities::default()
        },
        ..InitializeParams::default()
    };
    let none = InitializeParams::default();
    assert!(!client_supports_message_action_items(&none));
    assert!(!client_supports_show_document(&none));

    // `showMessage` without `messageActionItem` cannot render actions.
    let bare_show_message = params_with(Some(WindowClientCapabilities {
        show_message: Some(ShowMessageRequestClientCapabilities::default()),
        ..WindowClientCapabilities::default()
    }));
    assert!(!client_supports_message_action_items(&bare_show_message));

    let actions = params_with(Some(WindowClientCapabilities {
        show_message: Some(ShowMessageRequestClientCapabilities {
            message_action_item: Some(MessageActionItemCapabilities::default()),
        }),
        ..WindowClientCapabilities::default()
    }));
    assert!(client_supports_message_action_items(&actions));
    assert!(!client_supports_show_document(&actions));

    // `showDocument` counts only when its `support` flag is set.
    let document = |support| {
        params_with(Some(WindowClientCapabilities {
            show_document: Some(ShowDocumentClientCapabilities { support }),
            ..WindowClientCapabilities::default()
        }))
    };
    assert!(client_supports_show_document(&document(true)));
    assert!(!client_supports_show_document(&document(false)));
}

#[test]
fn lsp_position_helpers_use_utf16_columns() {
    let text = "é😀x\n";
    assert_eq!(line_col_to_byte_offset(text, 0, 0), Some(0));
    assert_eq!(line_col_to_byte_offset(text, 0, 1), Some(2));
    assert_eq!(line_col_to_byte_offset(text, 0, 3), Some(6));

    let line_index = tcl_lexer::LineIndex::new_lsp(text);
    let range = lift_span(text, &line_index, tcl_lexer::Span::new(6, 7));
    assert_eq!(range.start, pos(0, 3));
    assert_eq!(range.end, pos(0, 4));
}

/// The folding-range lift maps `FoldKind::Comment` to the LSP
/// `Comment` kind and `FoldKind::Region` to `Region`. The
/// `start_character` / `end_character` fields stay `None`
/// because the core provider returns line-only ranges.
#[test]
fn lift_folding_range_preserves_kind_and_lines() {
    let r = tcl_lsp_core::folding::FoldingRange {
        start_line: 2,
        end_line: 5,
        kind: FoldKind::Region,
    };
    let lifted = lift_folding_range(r);
    assert_eq!(lifted.start_line, 2);
    assert_eq!(lifted.end_line, 5);
    assert_eq!(lifted.kind, Some(FoldingRangeKind::Region));
    assert!(lifted.start_character.is_none());
    assert!(lifted.end_character.is_none());

    let comment = tcl_lsp_core::folding::FoldingRange {
        start_line: 0,
        end_line: 3,
        kind: FoldKind::Comment,
    };
    assert_eq!(
        lift_folding_range(comment).kind,
        Some(FoldingRangeKind::Comment),
    );
}

/// `lift_symbol_kind` maps every `CoreSymbolKind` variant to the
/// matching LSP `SymbolKind` constant. The mapping is exhaustive
/// so additions to `CoreSymbolKind` force an explicit decision
/// about the corresponding LSP `SymbolKind`.
#[test]
fn lift_symbol_kind_covers_all_variants() {
    assert_eq!(
        lift_symbol_kind(CoreSymbolKind::Function),
        SymbolKind::FUNCTION
    );
    assert_eq!(lift_symbol_kind(CoreSymbolKind::Method), SymbolKind::METHOD);
    assert_eq!(lift_symbol_kind(CoreSymbolKind::Class), SymbolKind::CLASS);
    assert_eq!(
        lift_symbol_kind(CoreSymbolKind::Property),
        SymbolKind::PROPERTY
    );
    assert_eq!(
        lift_symbol_kind(CoreSymbolKind::Constructor),
        SymbolKind::CONSTRUCTOR,
    );
    assert_eq!(
        lift_symbol_kind(CoreSymbolKind::Namespace),
        SymbolKind::NAMESPACE
    );
    assert_eq!(
        lift_symbol_kind(CoreSymbolKind::Variable),
        SymbolKind::VARIABLE
    );
    assert_eq!(lift_symbol_kind(CoreSymbolKind::Test), SymbolKind::FUNCTION);
    assert_eq!(
        lift_symbol_kind(CoreSymbolKind::Constant),
        SymbolKind::CONSTANT
    );
    assert_eq!(
        lift_symbol_kind(CoreSymbolKind::Operator),
        SymbolKind::OPERATOR
    );
    assert_eq!(lift_symbol_kind(CoreSymbolKind::Module), SymbolKind::MODULE);
}

/// `lift_document_symbol` preserves the name / detail / kind /
/// range / selection-range fields and recursively lifts
/// children. An empty child list maps to `None` rather than
/// `Some(Vec::new())` so the serialised JSON omits the field
/// entirely (per `#[serde(skip_serializing_if)]` on the LSP
/// type).
#[test]
fn lift_document_symbol_preserves_fields_and_recurses() {
    let inner = core_symbols::DocumentSymbol {
        name: "child".to_owned(),
        detail: None,
        kind: CoreSymbolKind::Variable,
        range: core_symbols::LineRange {
            start_line: 2,
            start_character: 4,
            end_line: 2,
            end_character: 9,
        },
        selection_range: core_symbols::LineRange {
            start_line: 2,
            start_character: 4,
            end_line: 2,
            end_character: 9,
        },
        children: Vec::new(),
    };
    let outer = core_symbols::DocumentSymbol {
        name: "demo".to_owned(),
        detail: Some("()".to_owned()),
        kind: CoreSymbolKind::Function,
        range: core_symbols::LineRange {
            start_line: 0,
            start_character: 0,
            end_line: 4,
            end_character: 1,
        },
        selection_range: core_symbols::LineRange {
            start_line: 0,
            start_character: 5,
            end_line: 0,
            end_character: 9,
        },
        children: vec![inner],
    };

    let lifted = lift_document_symbol(outer);
    assert_eq!(lifted.name, "demo");
    assert_eq!(lifted.detail.as_deref(), Some("()"));
    assert_eq!(lifted.kind, SymbolKind::FUNCTION);
    assert_eq!(
        lifted.range.start,
        Position {
            line: 0,
            character: 0
        }
    );
    assert_eq!(
        lifted.range.end,
        Position {
            line: 4,
            character: 1
        }
    );
    assert_eq!(
        lifted.selection_range.start,
        Position {
            line: 0,
            character: 5
        },
    );
    let kids = lifted.children.expect("non-empty children lift to Some");
    assert_eq!(kids.len(), 1);
    assert_eq!(kids[0].name, "child");
    assert_eq!(kids[0].kind, SymbolKind::VARIABLE);
    assert!(
        kids[0].children.is_none(),
        "leaf symbol's empty children must lift to None",
    );
}

#[test]
fn materialise_selection_range_none_falls_back_to_cursor() {
    // The fallback the handler applies when a position yields no chain:
    // `materialise_selection_range` returns `None` for an empty chain, and
    // the handler substitutes a degenerate range at the cursor so the
    // position is never dropped.
    assert!(materialise_selection_range(&[]).is_none());
}

/// The `willRename` / `didRename` filter must carry `ignoreCase`, because
/// [`is_tcl_source`] case-folds and a glob cannot express "any casing":
/// without it an `UPPER.TCL` the scan indexed would be renamed on a
/// case-sensitive file system without its `source` references being
/// rewritten.
#[test]
fn rename_filter_matches_case_insensitively() {
    let options = rename_file_operation_options().filters[0]
        .pattern
        .options
        .clone();
    assert_eq!(
        options.and_then(|o| o.ignore_case),
        Some(true),
        "the rename filter must set ignoreCase",
    );
}
