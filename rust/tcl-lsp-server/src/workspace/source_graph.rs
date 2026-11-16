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

//! The workspace index's `source`-path resolution and constant folding.

use std::path::Path;
use std::str::FromStr;

use tcl_lsp_core::workspace_index as core_workspace_index;
use tower_lsp_server::ls_types::Uri;

use crate::documents::canonical_file_uri;
use crate::uri_norm::DocumentPath as _;

/// Resolve a literal `source` path written in `parent_uri` to the child
/// document's URI string, keyed the same way the workspace index keys
/// documents ([`canonical_file_uri`]).  `None` for a non-`file:` URI or an
/// unmappable path.
#[cfg(test)]
pub(in super::super) fn resolve_source_uri(parent_uri: &str, raw_path: &str) -> Option<String> {
    let parent = Uri::from_str(parent_uri).ok()?;
    let parent_path = parent.document_path()?;
    let child = tcl_lsp_core::source_graph::resolve_source_target(parent_path.as_ref(), raw_path);
    canonical_file_uri(&child).map(|u| u.as_str().to_owned())
}

/// A [`core_workspace_index::WorkspaceIndex`] with this server's
/// `source`-path resolver installed.
///
/// The index holds no URI ↔ filesystem-path mapping of its own, so the
/// `source`-graph load order its import-lifecycle gates rank cross-document
/// events with ([`tcl_lsp_core::source_graph::RunOrder`])
/// can only be built once the host hands it [`resolve_source_edge`] — the same
/// resolver the source-site re-homing pass uses, so its statically-foldable
/// computed-path tier (`[file join [file dirname [info script]] x.tcl]`, the
/// idiom real multi-file projects write) sequences the order too. Every
/// index this server builds goes through here so none of them can silently
/// end up with an empty order and answer a different question from the rest.
pub(in super::super) fn new_workspace_index() -> core_workspace_index::WorkspaceIndex {
    let mut index = core_workspace_index::WorkspaceIndex::new();
    index.set_source_resolver(resolve_source_edge, fold_document_constants);
    index
}

/// The [`core_workspace_index::ConstantFolder`] this server installs: the
/// shared chain fold, with the document URI's filesystem path standing in
/// for `[info script]`.  A URI outside the filesystem folds path-blind
/// (literal constants still carry; `[info script]` idioms abstain).  The
/// index hands it no dialect, so it reads the commands of a document that
/// names none.
fn fold_document_constants(
    uri: &str,
    writes: &tcl_compiler::auto_path_eval::PathConstantAssignments,
    imported: &tcl_compiler::auto_path_eval::FoldedPathConstants,
) -> tcl_compiler::auto_path_eval::FoldedPathConstants {
    let parsed = Uri::from_str(uri).ok();
    let path = parsed
        .as_ref()
        .filter(|uri| uri.scheme().as_str().eq_ignore_ascii_case("file"))
        .and_then(Uri::to_file_path);
    tcl_compiler::auto_path_eval::fold_constant_assignments_with_imports(
        writes,
        path.as_deref().and_then(std::path::Path::to_str),
        imported,
    )
}

/// Resolve conditional source advice from a genuine original expression and
/// the same retained source inventory. Reported path text is not evaluated.
/// The host supplies the parent document filename and URI/path mapping; the
/// selected typed source algebra supplies no actual file evaluation.
pub(in super::super) fn resolve_source_edge(
    parent_uri: &str,
    _raw_path: &str,
    expression: Option<&tcl_compiler::auto_path_eval::OriginalSourcePathExpression>,
    source_offset: u32,
    raw_constants: &tcl_compiler::auto_path_eval::PathConstantAssignments,
    imported: &tcl_compiler::auto_path_eval::FoldedPathConstants,
) -> Option<String> {
    let expression = expression?;
    if !expression.matches_assignments(raw_constants)
        || expression.source_span().start() > source_offset
        || source_offset >= expression.source_span().end()
    {
        return None;
    }
    let parent = Uri::from_str(parent_uri).ok()?;
    let parent_path = parent.document_path()?;
    let constants = tcl_compiler::auto_path_eval::fold_constant_assignments_with_imports(
        raw_constants,
        parent_path.to_str(),
        imported,
    );
    let folded = expression.evaluate(parent_path.to_str(), &|name| {
        tcl_compiler::auto_path_eval::PathConstantLookup::path_constant(
            &constants.at(source_offset),
            name,
        )
    })?;
    let child = tcl_lsp_core::source_graph::resolve_source_target(parent_path.as_ref(), &folded);
    canonical_file_uri(&child).map(|u| u.as_str().to_owned())
}

/// Resolve a configured entry-point path (relative to `folder_root`, or
/// absolute) to its document URI string.
pub(in super::super) fn entry_point_uri(entry: &str, folder_root: Option<&Path>) -> Option<String> {
    let raw = Path::new(entry);
    let path = if raw.is_absolute() {
        raw.to_path_buf()
    } else {
        tcl_lsp_core::source_graph::resolve_under(folder_root?, entry)
    };
    canonical_file_uri(&path).map(|u| u.as_str().to_owned())
}
