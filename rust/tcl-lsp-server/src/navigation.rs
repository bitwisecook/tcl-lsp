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

//! Go-to-definition, hover, references, hierarchies and workspace symbols across documents.

use std::sync::Arc;

use tcl_compiler::analyser::AnalysisResult;
use tcl_lsp_core::definition::LspRange as CoreLspRange;
use tcl_lsp_core::original_indexed_location::OriginalIndexedSourceLocation;
use tcl_lsp_core::workspace_index as core_workspace_index;
use tower_lsp_server::ls_types::{Location, Uri};

use crate::DocumentState;
use crate::documents::DocumentPathAdmission;
use crate::protocol::lift_lsp_range;

mod declaration_identity;
mod declaration_sources;
mod definition;
mod hierarchies;
mod hover;
mod ilx;
mod logical_declarations;
mod method_targets;
mod namespaces;
mod references;
mod requests;
mod symbol_resolution;
mod workspace_symbols;

pub(super) use definition::position_is_command_head;
pub(super) use workspace_symbols::WorkspaceSymbolDiagnostic;

enum LogicalNamespaceLocationsPurpose {
    Definitions,
    References { include_declaration: bool },
}

enum LogicalVariableLocationsPurpose {
    Definitions,
    References { include_declaration: bool },
}

struct LogicalIndexedDefinitionTargets {
    call: core_workspace_index::WorkspaceLogicalCommandCall,
    pub(super) targets:
        Vec<tcl_lsp_core::original_indexed_location::LogicalIndexedDeclarationLocation>,
}

pub(super) struct ReadonlySavedSource {
    pub(super) admission: Arc<DocumentPathAdmission>,
    pub(super) bytes: Vec<u8>,
}

#[derive(Clone)]
enum LogicalMemberSourceCaller {
    Command(Box<core_workspace_index::WorkspaceLogicalCommandCall>),
    Member(Box<tcl_lsp_core::original_indexed_location::LogicalIndexedMemberLocation>),
}

pub(super) struct ReadonlyDeclarationTarget {
    pub(super) uri: Uri,
    pub(super) document: DocumentState,
    pub(super) analysis: Arc<AnalysisResult>,
    pub(super) generation: u64,
    pub(super) location: ReadonlyDeclarationLocation,
}

pub(super) enum ReadonlyDeclarationLocation {
    Original(Box<OriginalIndexedSourceLocation>),
    OriginalMethod {
        location: Box<OriginalIndexedSourceLocation>,
        caller_uri: String,
        input: tcl_compiler::analyser::ResolvedAnalysisInput,
        query: Box<tcl_lsp_core::method_symbol::OriginalMethodQuery>,
        candidate: Box<tcl_lsp_core::method_symbol::OriginalMethodCandidate>,
    },
    Logical {
        location: Box<tcl_lsp_core::original_indexed_location::LogicalIndexedDeclarationLocation>,
        call: Box<core_workspace_index::WorkspaceLogicalCommandCall>,
    },
    LogicalVariable {
        location: Box<tcl_lsp_core::original_indexed_location::LogicalIndexedVariableLocation>,
        caller: Box<tcl_lsp_core::original_indexed_location::LogicalIndexedVariableLocation>,
    },
    LogicalNamespace {
        location: Box<tcl_lsp_core::original_indexed_location::LogicalIndexedNamespaceLocation>,
        caller: Box<tcl_lsp_core::original_indexed_location::LogicalIndexedNamespaceLocation>,
    },
    LogicalSelfMember {
        location: Box<tcl_lsp_core::original_indexed_location::LogicalIndexedMemberLocation>,
        caller: Box<tcl_lsp_core::original_indexed_location::LogicalIndexedMemberLocation>,
    },
    LogicalMember {
        location: Box<tcl_lsp_core::original_indexed_location::LogicalIndexedMemberLocation>,
        call: Box<core_workspace_index::WorkspaceLogicalCommandCall>,
    },
}

impl ReadonlyDeclarationLocation {
    fn retained_in(&self, index: &core_workspace_index::WorkspaceIndex) -> bool {
        match self {
            Self::Original(_) => true,
            Self::OriginalMethod {
                caller_uri,
                input,
                query,
                candidate,
                ..
            } => {
                index.source_analysis_input(caller_uri) == Some(input)
                    && index.source_analysis_input(candidate.uri()) == Some(input)
                    && matches!(tcl_lsp_core::method_symbol::candidate(index, caller_uri, query),
                    std::ops::ControlFlow::Break(Some(current)) if current == **candidate)
            }
            Self::LogicalSelfMember { location, caller } => {
                index.logical_member_location_is_retained(caller)
                    && index.logical_member_location_is_retained(location)
            }
            Self::LogicalMember { location, call } => {
                call.is_retained_in(index) && index.logical_member_location_is_retained(location)
            }
            Self::LogicalNamespace { location, caller } => {
                index.logical_namespace_location_is_retained(caller)
                    && index.logical_namespace_location_is_retained(location)
            }
            Self::Logical { location, call } => {
                call.is_retained_in(index)
                    && index.logical_declaration_location_is_retained(location)
            }
            Self::LogicalVariable { location, caller } => {
                index.logical_variable_location_is_retained(caller)
                    && index.logical_variable_location_is_retained(location)
            }
        }
    }

    pub(super) fn uri(&self) -> &str {
        match self {
            Self::Original(location) | Self::OriginalMethod { location, .. } => location.uri(),
            Self::Logical { location, .. } => location.uri(),
            Self::LogicalVariable { location, .. } => location.uri(),
            Self::LogicalNamespace { location, .. } => location.uri(),
            Self::LogicalMember { location, .. } | Self::LogicalSelfMember { location, .. } => {
                location.uri()
            }
        }
    }

    pub(super) fn validated_span(
        &self,
        source: &str,
        analysis: &AnalysisResult,
    ) -> Option<tcl_lexer::Span> {
        match self {
            Self::Original(location) | Self::OriginalMethod { location, .. } => {
                location.validated_span(source, analysis)
            }
            Self::Logical { location, .. } => location.validated_span(source, analysis),
            Self::LogicalVariable { location, .. } => location.validated_span(source, analysis),
            Self::LogicalNamespace { location, .. } => location.validated_span(source, analysis),
            Self::LogicalMember { location, .. } | Self::LogicalSelfMember { location, .. } => {
                location.validated_span(source, analysis)
            }
        }
    }
}

pub(super) fn type_definition_locations(uri: &Uri, ranges: Vec<CoreLspRange>) -> Vec<Location> {
    ranges
        .into_iter()
        .map(|r| Location {
            uri: uri.clone(),
            range: lift_lsp_range(r),
        })
        .collect()
}

pub(super) fn implementation_inventory(
    documents: &[(String, Arc<str>, Arc<AnalysisResult>)],
) -> Vec<tcl_lsp_core::original_declaration::OriginalDeclarationDocument<'_>> {
    documents
        .iter()
        .map(|(uri, source, analysis)| {
            tcl_lsp_core::original_declaration::OriginalDeclarationDocument {
                uri,
                source,
                analysis,
            }
        })
        .collect::<Vec<_>>()
}

/// Deduplicate `Location`s by `(uri, range)`, preserving first-
/// seen order.  Used to merge current-document and cross-
/// document reference hits without double-listing a location.
/// Sort `(uri, span)` targets into a stable document-then-position order and
/// drop exact duplicates.
///
/// The union tiers (namespace declarations / references) read the same site
/// from two places — the request document's own analysis and the workspace
/// index — so overlap is the normal case, not an error.  Deduplicating on the
/// pair rather than on the rendered `Location` keeps it independent of the
/// (async, source-reading) span-to-range conversion.
pub(super) fn dedup_span_targets(targets: &mut Vec<(String, tcl_lexer::Span)>) {
    targets.sort_by(|(au, a), (bu, b)| {
        au.cmp(bu)
            .then_with(|| a.start().cmp(&b.start()))
            .then_with(|| a.end().cmp(&b.end()))
    });
    targets.dedup();
}

pub(super) fn dedup_locations(locations: &mut Vec<Location>) {
    let mut seen: std::collections::HashSet<(String, u32, u32, u32, u32)> =
        std::collections::HashSet::new();
    locations.retain(|loc| {
        seen.insert((
            loc.uri.to_string(),
            loc.range.start.line,
            loc.range.start.character,
            loc.range.end.line,
            loc.range.end.character,
        ))
    });
}
