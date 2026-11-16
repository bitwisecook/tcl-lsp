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

//! Conditional cross-document declaration advice from the indexed source headers.

use tcl_compiler::analyser::AnalysisResult;
use tcl_lsp_core::hover::{Hover as CoreHover, HoverKind as CoreHoverKind};
use tcl_lsp_core::{definition as core_definition, workspace_index as core_workspace_index};
use tower_lsp_server::ls_types::{Location, Position, Uri};

use crate::documents::DocumentSourceResult;
use crate::protocol::{lift_lsp_range, line_col_to_byte_offset_indexed};
use crate::{Backend, DocumentState};

use super::{
    LogicalIndexedDefinitionTargets, LogicalMemberSourceCaller, LogicalNamespaceLocationsPurpose,
    LogicalVariableLocationsPurpose, ReadonlyDeclarationLocation, ReadonlyDeclarationTarget,
};

impl Backend {
    /// Conditional cross-document Logical declaration advice retains the
    /// selected source call and actual indexed header allocations through the
    /// last await. Unavailable slots do not become report-name lookups.
    pub(super) async fn logical_definition_locations_for_snapshot(
        &self,
        caller: (Uri, DocumentState, u64),
        analysis: &AnalysisResult,
        position: Position,
        epoch: u64,
    ) -> DocumentSourceResult<Vec<Location>> {
        let Some(offset) = line_col_to_byte_offset_indexed(
            &caller.1.line_index,
            &caller.1.text,
            position.line,
            position.character,
        )
        .and_then(|offset| u32::try_from(offset).ok()) else {
            return Ok(Vec::new());
        };
        let actual = caller
            .1
            .line_index
            .position_at_utf16(offset, &caller.1.text);
        if actual.line != position.line || actual.character.get() != position.character {
            return Ok(Vec::new());
        }
        let Some(LogicalIndexedDefinitionTargets {
            mut call,
            mut targets,
        }) = self
            .logical_indexed_definition_targets(&caller.0, &caller.1.text, analysis, offset)
            .await?
        else {
            return Ok(Vec::new());
        };
        let mut resolver_revision = None;
        if targets.is_empty() {
            let Some((_, revision)) = self
                .ensure_logical_library_candidates(&caller.0, &caller.1.text, analysis, position)
                .await?
            else {
                return Ok(Vec::new());
            };
            resolver_revision = Some(revision);
            let Some(refreshed) = self
                .logical_indexed_definition_targets(&caller.0, &caller.1.text, analysis, offset)
                .await?
            else {
                return Ok(Vec::new());
            };
            call = refreshed.call;
            targets = refreshed.targets;
        }
        let selected = self
            .capture_logical_declaration_targets(&call, targets)
            .await?;
        self.with_current_declaration_sources(caller, selected, epoch, |uri, document, _, span| {
            let resolver = self.package_resolver.try_read().ok()?;
            if resolver_revision.is_some_and(|revision| resolver.revision() != revision) {
                return None;
            }
            Some(Location {
                uri: uri.clone(),
                range: lift_lsp_range(core_definition::span_to_range(
                    &document.text,
                    &document.line_index,
                    span,
                )),
            })
        })
        .await
    }

    pub(super) async fn capture_logical_declaration_targets(
        &self,
        call: &core_workspace_index::WorkspaceLogicalCommandCall,
        targets: Vec<tcl_lsp_core::original_indexed_location::LogicalIndexedDeclarationLocation>,
    ) -> DocumentSourceResult<Vec<ReadonlyDeclarationTarget>> {
        let mut selected = Vec::new();
        for location in targets {
            let Ok(uri) = location.uri().parse::<Uri>() else {
                continue;
            };
            let generation = self.live_publication_uri_generation(&uri);
            let Some(document) = self.read_document(&uri).await? else {
                continue;
            };
            let current = self
                .analysis_for(&uri, document.text.clone(), document.dialect.clone())
                .await?;
            selected.push(ReadonlyDeclarationTarget {
                uri,
                document,
                analysis: current,
                generation,
                location: ReadonlyDeclarationLocation::Logical {
                    location: Box::new(location),
                    call: Box::new(call.clone()),
                },
            });
        }
        Ok(selected)
    }

    pub(super) async fn logical_variable_locations_for_snapshot(
        &self,
        caller: (Uri, DocumentState, u64),
        analysis: &AnalysisResult,
        position: Position,
        epoch: u64,
        purpose: LogicalVariableLocationsPurpose,
    ) -> DocumentSourceResult<Vec<Location>> {
        let std::ops::ControlFlow::Break(Some(occurrence)) = tcl_lsp_core::variable_symbol::select(
            &caller.1.text,
            analysis,
            position.line,
            position.character,
        ) else {
            return Ok(Vec::new());
        };
        let selected = {
            let _rehoming = self.rehomed_index_guard().await?;
            let index = self.workspace_index.read().await;
            let Some(request) = index.logical_variable_location_for_occurrence(
                caller.0.as_str(),
                &caller.1.text,
                analysis,
                occurrence,
            ) else {
                return Ok(Vec::new());
            };
            let locations = index.logical_variable_source_locations(&request);
            locations
                .into_iter()
                .filter(|location| match purpose {
                    LogicalVariableLocationsPurpose::Definitions => {
                        location.uri() != caller.0.as_str() && location.is_declaration()
                    }
                    LogicalVariableLocationsPurpose::References {
                        include_declaration,
                    } => include_declaration || !location.is_declaration(),
                })
                .map(|location| ReadonlyDeclarationLocation::LogicalVariable {
                    location: Box::new(location),
                    caller: Box::new(request.clone()),
                })
                .collect::<Vec<_>>()
        };
        let mut targets = Vec::new();
        for location in selected {
            let Ok(uri) = location.uri().parse::<Uri>() else {
                continue;
            };
            let generation = self.live_publication_uri_generation(&uri);
            let Some(document) = self.read_document(&uri).await? else {
                continue;
            };
            let current = self
                .analysis_for(&uri, document.text.clone(), document.dialect.clone())
                .await?;
            targets.push(ReadonlyDeclarationTarget {
                uri,
                document,
                analysis: current,
                generation,
                location,
            });
        }
        self.resolve_current_declaration_locations(caller, targets, epoch)
            .await
    }

    pub(super) async fn logical_namespace_locations_for_snapshot(
        &self,
        caller: (Uri, DocumentState, u64),
        analysis: &AnalysisResult,
        position: Position,
        epoch: u64,
        purpose: LogicalNamespaceLocationsPurpose,
    ) -> DocumentSourceResult<Vec<Location>> {
        let Some(offset) = line_col_to_byte_offset_indexed(
            &caller.1.line_index,
            &caller.1.text,
            position.line,
            position.character,
        )
        .and_then(|offset| u32::try_from(offset).ok()) else {
            return Ok(Vec::new());
        };
        let actual = caller
            .1
            .line_index
            .position_at_utf16(offset, &caller.1.text);
        if actual.line != position.line || actual.character.get() != position.character {
            return Ok(Vec::new());
        }
        let Some(symbol) = Self::original_namespace_symbol(&caller.1.text, analysis, position)
        else {
            return Ok(Vec::new());
        };
        let (request, locations, parents) = {
            let _rehoming = self.rehomed_index_guard().await?;
            let index = self.workspace_index.read().await;
            let Some(request) = index.logical_namespace_location_at(
                caller.0.as_str(),
                &caller.1.text,
                analysis,
                offset,
            ) else {
                return Ok(Vec::new());
            };
            let locations = index
                .logical_namespace_source_locations(&request, &symbol)
                .into_iter()
                .filter(|location| match purpose {
                    LogicalNamespaceLocationsPurpose::Definitions => location.is_declaration(),
                    LogicalNamespaceLocationsPurpose::References {
                        include_declaration,
                    } => include_declaration || !location.is_declaration(),
                })
                .collect::<Vec<_>>();
            let parents = if matches!(purpose, LogicalNamespaceLocationsPurpose::Definitions) {
                index.logical_namespace_parent_locations(&request, &symbol)
            } else {
                Vec::new()
            };
            (request, locations, parents)
        };
        let locations = self
            .resolve_logical_namespace_sources(caller.clone(), &request, locations, epoch)
            .await?;
        if !locations.is_empty()
            || !matches!(purpose, LogicalNamespaceLocationsPurpose::Definitions)
        {
            return Ok(locations);
        }
        self.resolve_logical_namespace_sources(caller, &request, parents, epoch)
            .await
    }

    async fn resolve_logical_namespace_sources(
        &self,
        caller: (Uri, DocumentState, u64),
        request: &tcl_lsp_core::original_indexed_location::LogicalIndexedNamespaceLocation,
        locations: Vec<tcl_lsp_core::original_indexed_location::LogicalIndexedNamespaceLocation>,
        epoch: u64,
    ) -> DocumentSourceResult<Vec<Location>> {
        let mut targets = Vec::new();
        for location in locations {
            let Ok(uri) = location.uri().parse::<Uri>() else {
                continue;
            };
            let generation = self.live_publication_uri_generation(&uri);
            let Some(document) = self.read_document(&uri).await? else {
                continue;
            };
            let current = self
                .analysis_for(&uri, document.text.clone(), document.dialect.clone())
                .await?;
            targets.push(ReadonlyDeclarationTarget {
                uri,
                document,
                analysis: current,
                generation,
                location: ReadonlyDeclarationLocation::LogicalNamespace {
                    location: Box::new(location),
                    caller: Box::new(request.clone()),
                },
            });
        }
        self.resolve_current_declaration_locations(caller, targets, epoch)
            .await
    }

    async fn logical_member_source_targets_for_snapshot(
        &self,
        caller: &(Uri, DocumentState, u64),
        analysis: &AnalysisResult,
        position: Position,
    ) -> DocumentSourceResult<Option<Vec<ReadonlyDeclarationTarget>>> {
        let Some(offset) = line_col_to_byte_offset_indexed(
            &caller.1.line_index,
            &caller.1.text,
            position.line,
            position.character,
        )
        .and_then(|offset| u32::try_from(offset).ok()) else {
            return Ok(None);
        };
        let actual = caller
            .1
            .line_index
            .position_at_utf16(offset, &caller.1.text);
        if actual.line != position.line || actual.character.get() != position.character {
            return Ok(None);
        }
        let selected = {
            let _rehoming = self.rehomed_index_guard().await?;
            let index = self.workspace_index.read().await;
            if let Some(selected) = index.logical_class_member_targets_at(
                caller.0.as_str(),
                &caller.1.text,
                analysis,
                offset,
            ) {
                Some((
                    LogicalMemberSourceCaller::Command(Box::new(selected.call)),
                    selected.targets,
                ))
            } else {
                index
                    .logical_self_member_targets_at(
                        caller.0.as_str(),
                        &caller.1.text,
                        analysis,
                        offset,
                    )
                    .map(|selected| {
                        (
                            LogicalMemberSourceCaller::Member(Box::new(selected.caller)),
                            selected.targets,
                        )
                    })
            }
        };
        let Some((original_caller, selected_targets)) = selected else {
            return Ok(None);
        };
        let mut targets = Vec::new();
        for location in selected_targets {
            let Ok(uri) = location.uri().parse::<Uri>() else {
                continue;
            };
            let generation = self.live_publication_uri_generation(&uri);
            let Some(document) = self.read_document(&uri).await? else {
                continue;
            };
            let current = self
                .analysis_for(&uri, document.text.clone(), document.dialect.clone())
                .await?;
            targets.push(ReadonlyDeclarationTarget {
                uri,
                document,
                analysis: current,
                generation,
                location: match &original_caller {
                    LogicalMemberSourceCaller::Command(call) => {
                        ReadonlyDeclarationLocation::LogicalMember {
                            location: Box::new(location),
                            call: call.clone(),
                        }
                    }
                    LogicalMemberSourceCaller::Member(caller) => {
                        ReadonlyDeclarationLocation::LogicalSelfMember {
                            location: Box::new(location),
                            caller: caller.clone(),
                        }
                    }
                },
            });
        }
        Ok(Some(targets))
    }

    pub(super) async fn logical_class_member_locations_for_snapshot(
        &self,
        caller: (Uri, DocumentState, u64),
        analysis: &AnalysisResult,
        position: Position,
        epoch: u64,
    ) -> DocumentSourceResult<Option<Vec<Location>>> {
        let Some(targets) =
            Box::pin(self.logical_member_source_targets_for_snapshot(&caller, analysis, position))
                .await?
        else {
            return Ok(None);
        };
        Ok(Some(
            self.resolve_current_declaration_locations(caller, targets, epoch)
                .await?,
        ))
    }

    pub(in super::super) async fn logical_member_source_hover(
        &self,
        uri: &Uri,
        document: &DocumentState,
        analysis: &AnalysisResult,
        position: Position,
    ) -> DocumentSourceResult<Option<CoreHover>> {
        let epoch = self.diag_inputs_epoch();
        let caller = (
            uri.clone(),
            document.clone(),
            self.live_publication_uri_generation(uri),
        );
        let Some(targets) =
            Box::pin(self.logical_member_source_targets_for_snapshot(&caller, analysis, position))
                .await?
        else {
            return Ok(None);
        };
        let cards = targets
            .iter()
            .filter_map(|target| {
                let (ReadonlyDeclarationLocation::LogicalMember { location, .. }
                | ReadonlyDeclarationLocation::LogicalSelfMember { location, .. }) =
                    &target.location
                else {
                    return None;
                };
                let member = location.source_member();
                let span = member.original_name_word()?.span();
                let label =
                    if member.side() == tcl_compiler::analyser::types::MemberSide::ClassObject {
                        "classmethod"
                    } else {
                        "method"
                    };
                Some((
                    target.uri.clone(),
                    span,
                    format!(
                        "**{label}** `{}::{}` ({} param(s))",
                        member.class().metadata().qualified_name,
                        member.metadata().name,
                        member.metadata().params.len()
                    ),
                ))
            })
            .collect::<Vec<_>>();
        let mut rendered = self
            .with_current_declaration_sources(caller, targets, epoch, |uri, _, _, span| {
                cards
                    .iter()
                    .find(|(owner, selected, _)| owner == uri && *selected == span)
                    .map(|(_, _, text)| text.clone())
            })
            .await?;
        rendered.sort();
        rendered.dedup();
        Ok((!rendered.is_empty()).then(|| CoreHover {
            value: format!("Possible source declarations\n\n{}", rendered.join("\n\n")),
            kind: CoreHoverKind::Markdown,
        }))
    }

    pub(super) async fn logical_indexed_definition_targets(
        &self,
        uri: &Uri,
        source: &str,
        analysis: &AnalysisResult,
        offset: u32,
    ) -> DocumentSourceResult<Option<LogicalIndexedDefinitionTargets>> {
        let _rehoming = self.rehomed_index_guard().await?;
        let index = self.workspace_index.read().await;
        let Some(call) = index.logical_command_call_at(uri.as_str(), source, analysis, offset)
        else {
            return Ok(None);
        };
        let Some(targets) = index.logical_declaration_targets_for_call(&call) else {
            return Ok(None);
        };
        Ok(Some(LogicalIndexedDefinitionTargets { call, targets }))
    }
}
