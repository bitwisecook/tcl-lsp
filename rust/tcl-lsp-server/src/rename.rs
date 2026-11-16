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

//! Rename: the tiers a request tries, the workspace edits they plan and file renames.

use super::*;

mod conditional_variables;
mod cross_document;
mod file_rename;
mod namespace_cells;
mod original_commands;
mod requests;
mod tiers;

pub(super) use conditional_variables::client_supports_conditional_rename;
#[cfg(test)]
pub(super) use conditional_variables::{
    annotated_conditional_variable_edit, coalesce_conditional_variable_edits,
};
#[cfg(test)]
pub(super) use file_rename::{
    FileRenameInputSnapshot, FileRenameSourcePublication, FileRenameSources, HeldFileRename,
};
#[cfg(test)]
pub(super) use namespace_cells::describe_variable_collision;
pub(super) use original_commands::OriginalDocumentSnapshot;

/// The rename call-site facts
/// [`Backend::extend_rename_with_cross_document_edits`] needs, bundled to
/// stay within clippy's argument-count budget.
struct RenameContext<'a> {
    pub(super) uri: &'a Uri,
    pub(super) source: &'a str,
    pub(super) analysis: &'a AnalysisResult,
    pos: Position,
    pub(super) new_name: &'a str,
    pub(super) registry: &'a CommandRegistry,
}

#[derive(Clone, Copy)]
pub(super) struct WorkspaceRenameCaller<'a> {
    pub(super) uri: &'a Uri,
    pub(super) source: &'a str,
    pub(super) analysis: &'a AnalysisResult,
    pub(super) position: Position,
}

#[derive(Clone, Copy)]
pub(super) struct WorkspaceSourcePlanToken {
    pub(super) index_generation: u64,
    inventory_revision: u64,
}

/// Turn a rename safety refusal into the LSP error response the editor shows.
///
/// A refusal is deliberately **not** a `null` result: `null` means "nothing to
/// rename here" and lets the editor quietly do nothing, which is exactly the
/// wrong signal when the symbol *is* renameable but the rename would break the
/// program.  `InvalidRequest` with the gate's own reason puts that reason in
/// front of the user.
fn original_command_rename_refusal_error(
    refusal: tcl_lsp_core::original_command_rename::OriginalCommandRenameRefusal,
) -> jsonrpc::Error {
    jsonrpc::Error {
        code: jsonrpc::ErrorCode::InvalidParams,
        message: refusal.reason().into(),
        data: Some(serde_json::json!({ "code": refusal.code(), "reason": refusal.reason() })),
    }
}

fn original_member_rename_refusal_error(
    refusal: tcl_lsp_core::original_member_rename::OriginalMemberRenameRefusal,
) -> jsonrpc::Error {
    jsonrpc::Error {
        code: jsonrpc::ErrorCode::InvalidParams,
        message: refusal.reason().into(),
        data: Some(serde_json::json!({ "code": refusal.code(), "reason": refusal.reason() })),
    }
}

fn rename_refusal_error(refusal: &core_rename_safety::RenameRefusal) -> jsonrpc::Error {
    jsonrpc::Error {
        code: jsonrpc::ErrorCode::InvalidRequest,
        message: refusal.reason.clone().into(),
        data: None,
    }
}

/// Lift `spans` (byte offsets into `text`) to LSP ranges and append each as a
/// `new_name` rename edit to `bucket`, skipping any range already there — two
/// family classes in one document can report the same site.
pub(super) fn merge_span_edits(
    bucket: &mut Vec<TextEdit>,
    spans: Vec<tcl_lexer::Span>,
    line_index: &tcl_lexer::LineIndex,
    text: &str,
    new_name: &str,
) {
    for span in spans {
        let start = line_index.position_at_utf16(span.start(), text);
        let end = line_index.position_at_utf16(span.end(), text);
        let range = Range {
            start: Position {
                line: start.line,
                character: start.character.get(),
            },
            end: Position {
                line: end.line,
                character: end.character.get(),
            },
        };
        if !bucket.iter().any(|e| e.range == range) {
            bucket.push(TextEdit {
                range,
                new_text: new_name.to_owned(),
            });
        }
    }
}
