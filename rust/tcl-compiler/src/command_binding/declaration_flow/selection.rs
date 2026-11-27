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

//! The declaration layout a command's reads select: selection scopes, read points and layouts.

use super::super::source_read_scope::{trace_rbs_refusal, trace_rbs_required};
use super::super::{CommandAllocationSite, ExecutedScriptMapping};
use super::{
    Builder, DeclarationFlowFrame, DeclarationFlowPurpose, DeclarationReadPointScope,
    DeclarationSelectionScope,
};
use crate::ir::CommandTokens;
use std::sync::Arc;
use tcl_registry::script_body_flow::ScriptBodyFlow;

impl DeclarationReadPointScope<'_> {
    fn matches_frame(&self, frame: &crate::var_resolve::VariableExecutionFrame) -> bool {
        match self {
            Self::Observed(scopes) => scopes
                .iter()
                .any(|scope| scope.snapshot.state.variable_frame == *frame),
            Self::ConditionalDeclared(body) => body.frame == *frame,
            Self::ConditionalRoot(entry) => entry.frame() == frame,
            Self::ConditionalRootRelated { root, .. } => root.frame() == frame,
        }
    }

    fn accepts_source_scope(
        &self,
        scope: &super::super::source_read_scope::SourceReadScope,
        config: tcl_lexer::LexerConfig,
    ) -> bool {
        match self {
            Self::ConditionalDeclared(body) => body.matches_logical_read_scope(scope, config),
            Self::Observed(_) | Self::ConditionalRoot(_) | Self::ConditionalRootRelated { .. } => {
                true
            }
        }
    }
}

impl Builder<'_> {
    pub(super) fn selection_scopes(
        &self,
        tokens: &CommandTokens,
        site: &CommandAllocationSite,
    ) -> Option<Vec<DeclarationSelectionScope<'_>>> {
        Some(match self.entry {
            DeclarationFlowFrame::Existing(entry) => {
                let observations = trace_rbs_required(
                    super::super::declaration_layout::original_declaration_layouts(
                        trace_rbs_required(
                            self.inventory.layouts.get(site),
                            site.offset,
                            "scopes.layouts",
                        )?,
                    ),
                    site.offset,
                    "scopes.observations",
                )?;
                let mut scopes = Vec::new();
                for observation in observations {
                    if observation.entry.as_ref() != entry
                        || observation.config != self.config
                        || observation.words.as_ref() != tokens.words()
                    {
                        trace_rbs_refusal(site.offset, "scopes.row-current");
                        return None;
                    }
                    scopes.push(DeclarationSelectionScope {
                        snapshot: &observation.snapshot,
                        namespace: &observation.namespace,
                        config: observation.config,
                    });
                }
                scopes
            }
            DeclarationFlowFrame::Read(entry) => {
                if !self.read_sites.contains(site) || !entry.owns_source(&site.source, site.offset)
                {
                    trace_rbs_refusal(site.offset, "scopes.read-current");
                    return None;
                }
                let source = entry.header.declaration.source.source_image();
                let text = source.try_text().ok()?;
                let first = tokens.words().first()?.original_written_span(text)?;
                let last = tokens.words().last()?.original_written_span(text)?;
                if first.start() != site.offset {
                    return None;
                }
                let image = tcl_lexer::SourceImage::from_bytes(
                    source
                        .bytes()
                        .get(first.start() as usize..last.end() as usize)?,
                    source.channel(),
                );
                let segments = tcl_source::segment::segment_commands_image_with_offset_and_config(
                    &image,
                    first.start(),
                    self.config.nested(),
                )?;
                let [segment] = segments.as_slice() else {
                    return None;
                };
                let original = super::super::source_command_tokens_boxed(
                    &image,
                    first.start(),
                    self.config,
                    segment,
                );
                if original.words() != tokens.words() {
                    return None;
                }
                vec![DeclarationSelectionScope {
                    snapshot: &entry.snapshot,
                    namespace: &entry.header.namespace,
                    config: self.config,
                }]
            }
        })
    }

    fn conditional_root_scope(
        &self,
        catalogue: &super::super::source_read_scope::SourceReadCatalogue,
        entry: &super::super::declaration_layout::OriginalDiagnosticFrameEntry,
        site: &CommandAllocationSite,
        tokens: &CommandTokens,
        required_bodies: &[tcl_lexer::Span],
    ) -> Option<super::super::source_read_scope::SourceReadScope> {
        use super::super::declaration_layout::OriginalDiagnosticFrameEntry;
        let OriginalDiagnosticFrameEntry::RootScript { source, frame, .. } = entry else {
            return None;
        };
        let seed = self.inventory.source_seed.as_ref()?;
        let owner = seed.analysis_owner.as_ref()?;
        let input = seed.baseline.metadata_context.source_analysis_input()?;
        let metadata = crate::registry_invocation::InvocationMetadataContext::for_source_input(
            self.registry,
            input,
            self.config,
            Some(input.unit_profile()),
        )?;
        if *frame != crate::var_resolve::VariableExecutionFrame::Global
            || source.mapping != (ExecutedScriptMapping::Contiguous { base: 0 })
            || source.text != *source.origin.source_image()
            || !Arc::ptr_eq(&source.origin, &seed.origin)
            || !Arc::ptr_eq(&source.origin, &owner.source)
            || !Arc::ptr_eq(&source.origin, &site.source)
            || owner.config != self.config
            || seed.config != self.config
            || input.lexer_config() != self.config
            || seed.logical_input.is_some()
            || owner.metadata != seed.baseline.metadata_context
            || owner.metadata.source_analysis_input() != Some(input)
            || !seed.matches_registry(self.registry)
            || tokens.synthetic.is_some()
            || !tokens.original_words_match_argv_text()
        {
            return None;
        }
        catalogue.scope_with_ancestry(site, tokens, self.config, |advice, context| {
            if !std::ptr::eq(Arc::as_ptr(&input.context_registry()), context) {
                return false;
            }
            let seen_bodies = std::cell::RefCell::new(Vec::new());
            let valid = |candidate: &super::super::OriginalSourceCommandTransitionAdvice| {
                let current = Arc::ptr_eq(&candidate.site().source, &source.origin)
                    && catalogue.admits_original_site(candidate.site())
                    && candidate.matches_context(context)
                    && candidate.matches_source(source.origin.source_image(), self.config)
                    && candidate.logical_source_input().is_none()
                    && candidate
                        .original_words()
                        .iter()
                        .all(|word| !word.group().expand)
                    && candidate
                        .arguments()
                        .iter()
                        .enumerate()
                        .all(|(index, word)| {
                            word.origin
                                == crate::registry_invocation::InvocationWordOrigin::Written(
                                    index + 1,
                                )
                        });
                if current && let Some(body) = candidate.original_catalogue_source_body() {
                    seen_bodies.borrow_mut().push(body.content_span());
                }
                current
            };
            let endpoint = |candidate: &super::super::OriginalSourceCommandTransitionAdvice| {
                self.conditional_root_endpoint(
                    candidate,
                    entry,
                    metadata,
                    required_bodies,
                    &seen_bodies.borrow(),
                )
            };
            super::super::source_read_scope::matches_read_body_ancestry_until(
                advice,
                context,
                self.config,
                true,
                valid,
                endpoint,
            )
        })
    }

    fn conditional_root_endpoint(
        &self,
        candidate: &super::super::OriginalSourceCommandTransitionAdvice,
        entry: &super::super::declaration_layout::OriginalDiagnosticFrameEntry,
        metadata: crate::registry_invocation::InvocationMetadataContext<'_>,
        required_bodies: &[tcl_lexer::Span],
        seen_bodies: &[tcl_lexer::Span],
    ) -> bool {
        use super::super::declaration_layout::OriginalDiagnosticFrameEntry;
        let OriginalDiagnosticFrameEntry::RootScript {
            source,
            frame,
            namespace,
        } = entry
        else {
            return false;
        };
        if required_bodies
            .iter()
            .any(|body| !seen_bodies.contains(body))
        {
            return false;
        }
        let Some(rows) = self.inventory.layouts.get(candidate.site()) else {
            return false;
        };
        let Some(rows) = super::super::declaration_layout::original_declaration_layouts(rows)
        else {
            return false;
        };
        rows.into_iter().all(|row| {
            row.entry.as_ref() == entry
                && Arc::ptr_eq(&row.entry.source().origin, &source.origin)
                && row.namespace == *namespace
                && row.config == self.config
                && row.snapshot.state.variable_frame == *frame
                && row
                    .snapshot
                    .state
                    .retained_original_source_purpose(metadata, self.registry)
                    == Some(super::super::OriginalInvocationSourcePurpose::AuthoredSimulation)
                && row
                    .snapshot
                    .state
                    .current_source_origin
                    .as_ref()
                    .is_some_and(|origin| Arc::ptr_eq(origin, &source.origin))
                && crate::registry_invocation::original_native_compiler_words(
                    source.origin.source_image(),
                    &row.words,
                    candidate.site().offset,
                    self.config,
                )
                .as_deref()
                    == Some(candidate.original_words())
        })
    }

    pub(super) fn source_read_point_scope(
        &self,
        tokens: &CommandTokens,
        site: &CommandAllocationSite,
    ) -> Option<DeclarationReadPointScope<'_>> {
        if let Some(scopes) = self.selection_scopes(tokens, site) {
            return Some(DeclarationReadPointScope::Observed(scopes));
        }
        if self.purpose == DeclarationFlowPurpose::Representation
            && let DeclarationFlowFrame::Existing(
                root @ super::super::declaration_layout::OriginalDiagnosticFrameEntry::RootScript {
                    source,
                    frame,
                    namespace,
                },
            ) = self.entry
            && let Some(rows) = self.inventory.layouts.get(site)
        {
            let input = self
                .inventory
                .source_seed
                .as_ref()?
                .baseline
                .metadata_context
                .source_analysis_input()?;
            let metadata = crate::registry_invocation::InvocationMetadataContext::for_source_input(
                self.registry,
                input,
                self.config,
                Some(input.unit_profile()),
            )?;
            let rows = super::super::declaration_layout::original_declaration_layouts(rows)?;
            let mut bodies = Vec::new();
            for row in rows {
                let super::super::declaration_layout::OriginalDiagnosticFrameEntry::ScriptActivation {
                    source: child,
                    frame: child_frame,
                    namespace: child_namespace,
                    config,
                } = row.entry.as_ref()
                else {
                    return None;
                };
                let ExecutedScriptMapping::Contiguous { base } = child.mapping else {
                    return None;
                };
                let end = base.checked_add(u32::try_from(child.text.len()).ok()?)?;
                if !Arc::ptr_eq(&child.origin, &source.origin)
                    || !Arc::ptr_eq(&child.origin, &site.source)
                    || child_frame != frame
                    || child_namespace != namespace
                    || *config != self.config
                    || row.config != self.config
                    || row.words.as_ref() != tokens.words()
                    || row.namespace != *namespace
                    || row.snapshot.state.variable_frame != *frame
                    || row
                        .snapshot
                        .state
                        .retained_original_source_purpose(metadata, self.registry)
                        != Some(super::super::OriginalInvocationSourcePurpose::AuthoredSimulation)
                    || !row
                        .snapshot
                        .state
                        .current_source_origin
                        .as_ref()
                        .is_some_and(|origin| Arc::ptr_eq(origin, &source.origin))
                    || !row.entry.owns_source(&site.source, site.offset)
                    || source
                        .origin
                        .source_image()
                        .bytes()
                        .get(base as usize..end as usize)
                        != Some(child.text.bytes())
                {
                    return None;
                }
                bodies.push(tcl_lexer::Span::new(base, end));
            }
            return Some(DeclarationReadPointScope::ConditionalRootRelated { root, bodies });
        }
        // An original row that is present but invalid cannot borrow source
        // absence. Only an independently retained declaration may proceed.
        if self.inventory.layouts.contains_key(site) {
            return None;
        }
        if self.purpose == DeclarationFlowPurpose::Representation
            && let DeclarationFlowFrame::Existing(
                entry
                @ super::super::declaration_layout::OriginalDiagnosticFrameEntry::RootScript {
                    ..
                },
            ) = self.entry
        {
            return Some(DeclarationReadPointScope::ConditionalRoot(entry));
        }
        let DeclarationFlowFrame::Existing(
            super::super::declaration_layout::OriginalDiagnosticFrameEntry::DeclaredProcedure(body),
        ) = self.entry
        else {
            return None;
        };
        Some(DeclarationReadPointScope::ConditionalDeclared(body))
    }

    fn source_read_body(&self, site: &CommandAllocationSite) -> Option<tcl_lexer::Span> {
        Some(match self.entry {
            DeclarationFlowFrame::Existing(entry) => {
                if !(matches!(
                    entry.frame(),
                    crate::var_resolve::VariableExecutionFrame::Procedure { .. }
                        | crate::var_resolve::VariableExecutionFrame::ReceiverMethod { .. }
                ) || self.purpose == DeclarationFlowPurpose::Representation
                    && matches!(
                        entry,
                        super::super::declaration_layout::OriginalDiagnosticFrameEntry::RootScript { .. }
                    ))
                {
                    trace_rbs_refusal(site.offset, "selection.frame");
                    return None;
                }
                let source = entry.source();
                if source.origin != site.source || !entry.owns_source(&site.source, site.offset) {
                    trace_rbs_refusal(site.offset, "selection.source-owner");
                    return None;
                }
                tcl_lexer::Span::new(
                    source.base(),
                    source
                        .base()
                        .checked_add(u32::try_from(source.text.len()).ok()?)?,
                )
            }
            DeclarationFlowFrame::Read(entry) => {
                trace_rbs_required(
                    entry.owns_source(&site.source, site.offset).then_some(()),
                    site.offset,
                    "selection.read-owner",
                )?;
                trace_rbs_required(
                    entry.read.original_body_span(),
                    site.offset,
                    "selection.read-body",
                )?
            }
        })
    }

    pub(super) fn source_read_selection(
        &self,
        tokens: &CommandTokens,
        site: &CommandAllocationSite,
    ) -> Option<crate::registry_invocation::DeclarationInvocationFlow> {
        let body = self.source_read_body(site)?;
        // Original layout corruption is terminal even for a source-only query.
        let scopes = trace_rbs_required(
            self.source_read_point_scope(tokens, site),
            site.offset,
            "selection.scopes",
        )?;
        let mut retained = tokens.clone();
        let points = self
            .inventory
            .points
            .iter()
            .chain(
                self.inventory
                    .origin_points
                    .get(&site.source)
                    .into_iter()
                    .flatten(),
            )
            .filter(|point| {
                point.dispatch
                    && point.offset == site.offset
                    && point.state.current_source_origin.as_ref() == Some(&site.source)
                    && (self.purpose != DeclarationFlowPurpose::Representation
                        || point
                            .state
                            .current_source_origin
                            .as_ref()
                            .is_some_and(|origin| Arc::ptr_eq(origin, &site.source)))
                    && scopes.matches_frame(&point.state.variable_frame)
            });
        let mut points = points.peekable();
        if points.peek().is_some() {
            retained.source_binding = Some(
                super::super::SourceCommandBindings::query_dispatch_points(points),
            );
        }
        let catalogue = trace_rbs_required(
            self.inventory
                .source_read_catalogue(self.registry, self.purpose),
            site.offset,
            "selection.catalogue",
        )?;
        let scope = trace_rbs_required(
            match self.entry {
                DeclarationFlowFrame::Existing(
                    entry @ super::super::declaration_layout::OriginalDiagnosticFrameEntry::RootScript {
                        ..
                    },
                ) if self.purpose == DeclarationFlowPurpose::Representation => {
                    let bodies = match &scopes {
                        DeclarationReadPointScope::ConditionalRootRelated { bodies, .. } => {
                            bodies.as_slice()
                        }
                        _ => &[],
                    };
                    self.conditional_root_scope(catalogue, entry, site, &retained, bodies)
                }
                _ => catalogue.scope(site, &retained, body, self.config),
            },
            site.offset,
            "selection.scope",
        )?;
        if !scopes.accepts_source_scope(&scope, self.config) {
            return None;
        }
        if scope.advice().dialect() != self.dialect {
            trace_rbs_refusal(site.offset, "selection.dialect");
            return None;
        }
        trace_rbs_required(
            crate::registry_invocation::declaration_source_read_flow(
                self.registry,
                &retained,
                &scope,
                self.completion_frame(),
                self.purpose == DeclarationFlowPurpose::Representation,
            ),
            site.offset,
            "selection.flow",
        )
    }

    pub(super) fn selection(
        &self,
        tokens: &CommandTokens,
        site: &CommandAllocationSite,
    ) -> Option<(
        crate::registry_invocation::DeclarationInvocationFlow,
        Option<super::super::lifecycle_advice::SourceScopedLifecycleAdvice>,
    )> {
        if self.purpose == DeclarationFlowPurpose::Representation {
            let Some(source) = self.source_read_selection(tokens, site) else {
                let fallback = self.representation_observed_selection(tokens, site);
                return fallback.map(|selected| (selected, None));
            };
            if matches!(
                self.source_read_point_scope(tokens, site)?,
                DeclarationReadPointScope::Observed(_)
            ) {
                let (_, lifecycle) =
                    self.original_layout_selection_checked(tokens, site, Some(&source))?;
                if lifecycle.is_some() {
                    return None;
                }
            }
            return Some((source, None));
        }
        self.original_layout_selection(tokens, site).or_else(|| {
            self.source_read_selection(tokens, site)
                .map(|selected| (selected, None))
        })
    }

    /// Reuse the retained module realm for an already observed root command.
    /// The original word-source variant and all of its obligations remain held.
    fn representation_observed_selection(
        &self,
        tokens: &CommandTokens,
        site: &CommandAllocationSite,
    ) -> Option<crate::registry_invocation::DeclarationInvocationFlow> {
        if !matches!(
            self.entry,
            DeclarationFlowFrame::Existing(
                super::super::declaration_layout::OriginalDiagnosticFrameEntry::RootScript { .. }
            )
        ) || !matches!(
            self.source_read_point_scope(tokens, site)?,
            DeclarationReadPointScope::Observed(_)
        ) {
            return None;
        }
        let (observed, lifecycle) = self.original_layout_selection(tokens, site)?;
        if lifecycle.is_some()
            || tokens.synthetic.is_some()
            || !tokens.original_words_match_argv_text()
        {
            return None;
        }
        let realm = self.representation_realm?;
        let input = self
            .inventory
            .source_seed
            .as_ref()?
            .baseline
            .metadata_context
            .source_analysis_input()?;
        let mut retained = tokens.clone();
        realm.stamp_original_tokens(&mut retained);
        let words = crate::registry_invocation::source_structure::original_registry_words_for_supplied_tokens(
            realm, input, &retained,
        );
        let words = words?;
        let source = crate::registry_invocation::declaration_original_words_flow(
            self.registry,
            &retained,
            words,
            input.borrowed_context_registry(),
            self.completion_frame(),
            true,
        )?;
        let (verified, _) = self.original_layout_selection_checked(tokens, site, Some(&source))?;
        (verified == observed).then_some(source)
    }

    pub(super) fn original_layout_selection(
        &self,
        tokens: &CommandTokens,
        site: &CommandAllocationSite,
    ) -> Option<(
        crate::registry_invocation::DeclarationInvocationFlow,
        Option<super::super::lifecycle_advice::SourceScopedLifecycleAdvice>,
    )> {
        self.original_layout_selection_checked(tokens, site, None)
    }

    fn original_layout_selection_checked(
        &self,
        tokens: &CommandTokens,
        site: &CommandAllocationSite,
        source: Option<&crate::registry_invocation::DeclarationInvocationFlow>,
    ) -> Option<(
        crate::registry_invocation::DeclarationInvocationFlow,
        Option<super::super::lifecycle_advice::SourceScopedLifecycleAdvice>,
    )> {
        let mut agreed = None;
        for DeclarationSelectionScope {
            snapshot,
            namespace,
            config,
        } in self.selection_scopes(tokens, site)?
        {
            let advice = super::super::original_site_operand_layout_advice(
                site, tokens, snapshot, namespace, config,
            )?;
            if advice.dialect() != self.dialect {
                return None;
            }
            let mut retained = tokens.clone();
            let points = self
                .inventory
                .points
                .iter()
                .chain(
                    self.inventory
                        .origin_points
                        .get(&site.source)
                        .into_iter()
                        .flatten(),
                )
                .filter(|point| {
                    point.dispatch
                        && point.offset == site.offset
                        && point.state.current_source_origin.as_ref() == Some(&site.source)
                        && point.state.variable_frame == snapshot.state.variable_frame
                });
            let mut points = points.peekable();
            if points.peek().is_some() {
                retained.source_binding = Some(
                    super::super::SourceCommandBindings::query_dispatch_points(points),
                );
            }
            let selected = crate::registry_invocation::declaration_invocation_flow_in_frame(
                self.registry,
                &retained,
                &advice,
                self.completion_frame(),
            )?;
            if let Some(source) = source {
                let input = self
                    .inventory
                    .source_seed
                    .as_ref()?
                    .baseline
                    .metadata_context
                    .source_analysis_input()?;
                crate::registry_invocation::declaration_source_flow_corresponds(
                    self.registry,
                    &retained,
                    &advice,
                    source,
                    &selected,
                    input.borrowed_context_registry(),
                )?;
            }
            let lifecycle = if selected.flow == ScriptBodyFlow::CapturedLifecycle {
                Some(match self.entry {
                    DeclarationFlowFrame::Existing(entry) => advice
                        .scoped_declaration_lifecycle_advice(&retained, self.registry, entry)?,
                    DeclarationFlowFrame::Read(entry) => advice
                        .scoped_procedure_read_lifecycle_advice(&retained, self.registry, entry)?,
                })
            } else {
                None
            };
            let selected = (selected, lifecycle);
            if agreed.as_ref().is_some_and(|old| old != &selected) {
                return None;
            }
            agreed = Some(selected);
        }
        agreed
    }
}
