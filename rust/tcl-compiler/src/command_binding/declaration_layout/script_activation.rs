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

//! Script activations retained for an original body and the member frame at an offset.

use super::super::{ModuleCommandBindings, SourceCommandBindings, SourceExecutionContext};
use super::variable_frame::direct_script_cursor;
use super::{
    DeclarationLayoutIssuer, OriginalDiagnosticFrameEntry, SourceOriginalVariableFrame,
    original_declaration_layouts, original_source_catalogue_layouts,
};
use std::sync::Arc;

impl SourceCommandBindings {
    /// Original Logical receiver-body frame geometry for source navigation.
    /// Only the retained member/body/vector and actual parent declaration may
    /// supply this recipe. Native entry and receiver namespace stay unavailable.
    pub(crate) fn logical_source_member_frame_at_offset(
        &self,
        input: &crate::analyser::ResolvedAnalysisInput,
        image: &tcl_lexer::SourceImage,
        offset: u32,
        config: tcl_lexer::LexerConfig,
    ) -> Option<SourceOriginalVariableFrame> {
        if !input.has_logical_source_name_context()
            || input.lexer_config() != config
            || self.original_logical_source_name_advice_input() != Some(input)
            || !self.matches_original_source_image(image, config)
        {
            return None;
        }
        let origin = self.source_origin()?;
        let mut selected: Option<SourceOriginalVariableFrame> = None;
        for (site, rows) in &self.declaration_layouts {
            let Some(rows) = original_source_catalogue_layouts(rows) else {
                continue;
            };
            let first = rows.clone().next()?;
            let OriginalDiagnosticFrameEntry::LogicalReceiver(body) = first.entry.as_ref() else {
                continue;
            };
            if body.input != *input
                || !Arc::ptr_eq(&body.source.origin, origin)
                || !body.body.matches_source(image, config)
                || !body.body.matches_context(input.borrowed_context_registry())
                || !first.entry.owns_source_cursor(origin, offset)
                || !direct_script_cursor(image, config, body.body.content_span(), offset, 0)
            {
                continue;
            }
            let crate::registry_invocation::source_structure::OriginalRegistrySource::SourceTransitions(parent) =
                body.body.original_parent_body().source_words().source()
            else {
                return None;
            };
            let parents =
                original_declaration_layouts(self.declaration_layouts.get(parent.site())?)?;
            if parent.logical_source_input() != Some(input)
                || !Arc::ptr_eq(&parent.site().source, origin)
                || rows.clone().any(|row| {
                    row.issuer != DeclarationLayoutIssuer::ConditionalReceiverSource
                        || row.config != config
                        || row.snapshot.state.logical_source_name_advice_input() != Some(input)
                        || !parents.clone().any(|parent| {
                            parent.snapshot == row.snapshot && parent.namespace == row.namespace
                        })
                        || origin
                            .original_lexical_command(&row.words, site.offset, config)
                            .is_none_or(|original| {
                                original.native_words().iter().any(|word| {
                                    word.span().start() < body.body.content_span().start()
                                        || word.span().end() > body.body.content_span().end()
                                })
                            })
                })
            {
                return None;
            }
            let frame = SourceOriginalVariableFrame {
                source: body.source.clone(),
                frame: body.frame.clone(),
                config,
                logical_lambda: None,
            };
            match &selected {
                Some(previous) if previous.source.text.len() < frame.source.text.len() => {}
                Some(previous) if previous.source.text.len() == frame.source.text.len() => {
                    if previous != &frame {
                        return None;
                    }
                }
                _ => selected = Some(frame),
            }
        }
        selected
    }

    pub(in super::super) fn retain_original_script_activation(
        &mut self,
        image: &tcl_lexer::SourceImage,
        base: u32,
        state: &ModuleCommandBindings,
        context: &SourceExecutionContext<'_>,
    ) {
        if &state.variable_frame != context.frame {
            return;
        }
        let Some(origin) = state.current_source_origin.as_ref() else {
            return;
        };
        let namespace = context.namespace_identity();
        if state.source_variables.namespace_identity.as_ref() != Some(&namespace) {
            return;
        }
        // A nested parser does not issue a frame. Reborrow only a genuine
        // entered operand observation carrying this exact source and frame.
        let entered = (context.source_purpose()
            == tcl_lexer::SourceSegmentationPurpose::NestedScript)
            .then(|| {
                self.entered_scripts
                    .values()
                    .flat_map(|arguments| arguments.values().flatten())
                    .find(|observation| {
                        observation.source.origin == *origin
                            && observation.source.text == *image
                            && observation.source.base() == base
                            && observation.frame.as_ref() == Some(context.frame)
                            && observation.namespace.as_ref() == Some(&namespace)
                    })
                    .map(|observation| observation.source.clone())
            })
            .flatten();
        let source = if let Some(source) = entered {
            source
        } else {
            if self.root_origin.as_ref() != Some(origin)
                || !matches!(
                    state.variable_frame.layout(),
                    crate::var_resolve::VariableExecutionFrame::NamespaceActivation { .. }
                )
            {
                return;
            }
            let Some(source) = super::super::ExecutedScriptSource::contiguous_image(
                Arc::clone(origin),
                image.clone(),
                base,
            ) else {
                return;
            };
            source
        };
        let logical = context.body_entry() == super::super::SourceBodyEntryPurpose::Conditional
            && state.baseline.logical_source_input.is_some();
        if logical && super::super::logical_definition::logical_entry(state, *context).is_none() {
            return;
        }
        let entry = Arc::new(if logical {
            OriginalDiagnosticFrameEntry::LogicalScriptActivation {
                source,
                frame: state.variable_frame.clone(),
                namespace,
                config: context.config,
            }
        } else {
            OriginalDiagnosticFrameEntry::ScriptActivation {
                source,
                frame: state.variable_frame.clone(),
                namespace,
                config: context.config,
            }
        });
        if !self.original_script_activations.contains(&entry) {
            self.original_script_activations.push(entry);
        }
    }

    pub(super) fn original_script_activation_at(
        &self,
        state: &ModuleCommandBindings,
        context: &SourceExecutionContext<'_>,
        origin: &Arc<super::super::SourceOriginId>,
        offset: u32,
    ) -> Option<OriginalDiagnosticFrameEntry> {
        let namespace = context.namespace_identity();
        let mut matching = self.original_script_activations.iter().filter(|entry| {
            matches!(entry.as_ref(), OriginalDiagnosticFrameEntry::ScriptActivation {
                frame, namespace: selected, config, ..
            } | OriginalDiagnosticFrameEntry::LogicalScriptActivation {
                frame, namespace: selected, config, ..
            } if frame == &state.variable_frame && frame == context.frame && selected == &namespace && *config == context.config)
                && (matches!(entry.as_ref(), OriginalDiagnosticFrameEntry::LogicalScriptActivation { .. })
                    == (context.body_entry() == super::super::SourceBodyEntryPurpose::Conditional
                        && super::super::logical_definition::logical_entry(state, *context).is_some()))
                && entry.owns_source(origin, offset)
                && entry.owns_original_context(&state.source_variables)
        });
        let first = matching.next()?;
        matching
            .all(|entry| entry == first)
            .then(|| first.as_ref().clone())
    }
}
