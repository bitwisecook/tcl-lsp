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

//! Substitutions inside a word: variable reads, array index scripts and the fragments they join.

use super::super::{
    ModuleCommandBindings, SourceCommandBindings, SourceExecutionContext, SourceIndexView,
    SourceOutcomes, native_result, original_name_value, original_variable_compilation,
    source_representation,
};
use super::outcomes::{opaque_source_invocation, publish_source_branch};
use std::sync::Arc;

pub(in super::super) fn original_variable_arena(
    original: Option<&(&str, tcl_source::words::SourceSite)>,
    state: &ModuleCommandBindings,
    config: tcl_lexer::LexerConfig,
) -> Option<tcl_lexer::ExecutablePartArena> {
    original.and_then(|(raw, read_site)| {
        let image = state.current_source_origin.as_ref()?.source_image();
        let end = read_site
            .span
            .start()
            .checked_add(u32::try_from(raw.len()).ok()?)?;
        let span = tcl_lexer::Span::new(read_site.span.start(), end);
        (image.bytes().get(span.as_range())? == raw.as_bytes()).then_some(())?;
        tcl_lexer::ExecutablePartArena::decompose(
            image.clone(),
            span,
            tcl_lexer::word_parts::SubstFlags::default(),
            config,
        )
        .ok()
    })
}

pub(in super::super) fn original_variable_source<'s>(
    document: &'s str,
    base: u32,
    source: &tcl_source::words::SourceSite,
    config: tcl_lexer::LexerConfig,
) -> Option<(&'s str, tcl_source::words::SourceSite)> {
    let start = usize::try_from(source.span.start().checked_sub(base)?).ok()?;
    if document.as_bytes().get(start) != Some(&b'$') {
        return None;
    }
    let reference =
        tcl_lexer::word_parts::scan_var_ref(document.as_bytes(), start, config).ok()??;
    let spelling = document.get(start..reference.next)?;
    let span = reference.source_span(document.as_bytes(), start, base)?;
    Some((
        spelling,
        tcl_source::words::SourceSite {
            span,
            provenance: source.provenance.clone(),
        },
    ))
}

enum SourceIndexFrame {
    List {
        id: tcl_lexer::word_parts::PartListId,
        next: usize,
        outcomes: Box<SourceOutcomes>,
        value: Option<String>,
        native_value: Option<original_name_value::OriginalProducedNameValue>,
        native_complete: bool,
    },
    Variable {
        id: tcl_lexer::word_parts::PartListId,
        position: usize,
    },
}

fn join_index_fragment(
    outcomes: &mut SourceOutcomes,
    value: &mut Option<String>,
    fragment: &SourceOutcomes,
    native_value: &mut Option<original_name_value::OriginalProducedNameValue>,
    native_complete: &mut bool,
) {
    if *native_complete {
        *native_value = match (native_value.take(), fragment.normal_name_value.as_deref()) {
            (None, Some(component)) => Some(component.clone()),
            (Some(value), Some(component)) => value.concatenated(component),
            _ => None,
        };
        *native_complete = native_value.is_some();
    }
    *value = value
        .take()
        .zip(fragment.normal_value.as_ref())
        .map(|(mut text, fragment)| {
            text.push_str(&fragment.text);
            text
        });
    outcomes.join(fragment);
}

fn index_normal_result(
    outcomes: &SourceOutcomes,
    value: Option<String>,
) -> Option<Arc<native_result::EvaluatedSourceValue>> {
    outcomes.normal.as_ref().and(value).map(|text| {
        Arc::new(native_result::EvaluatedSourceValue {
            text,
            representation: tcl_syntax::value::ValueRepresentation::Unknown,
            numeric: None,
        })
    })
}

fn index_text_fragment(
    source: SourceIndexView<'_>,
    component: &tcl_lexer::word_parts::SpannedExecutablePart,
    state: &ModuleCommandBindings,
) -> SourceOutcomes {
    let mut fragment = SourceOutcomes::normal(state);
    fragment.normal_name_value = state.source_variables.execution_name_policy
        .and_then(tcl_syntax::naming::ExecutionNamePolicy::native_recipe)
        .and_then(|policy| {
            let value = crate::signature_scan::scope::SignatureSourceNameValue::from_original_executable_text_fragment(
                source.arena, source.arena.image(), source.arena.config(), component.span, policy)?;
            original_name_value::OriginalProducedNameValue::from_source_input(
                &crate::signature_scan::scope::SignatureSourceNameInput::OriginalValue(value), &state.source_variables)
        }).map(Arc::new);
    fragment.retain_complete_normal_evaluation();
    fragment.normal_value = source
        .arena
        .text(component)
        .and_then(|bytes| std::str::from_utf8(bytes).ok())
        .map(|text| {
            Arc::new(native_result::EvaluatedSourceValue {
                text: text.to_owned(),
                representation: tcl_syntax::value::ValueRepresentation::Unknown,
                numeric: None,
            })
        });
    fragment
}

fn finish_index_list(
    mut outcomes: Box<SourceOutcomes>,
    value: Option<String>,
    native_value: Option<original_name_value::OriginalProducedNameValue>,
    state: &mut ModuleCommandBindings,
) -> SourceOutcomes {
    outcomes.normal_name_value = native_value
        .filter(|value| {
            outcomes
                .normal
                .as_ref()
                .is_some_and(|normal| value.is_current(&normal.source_variables))
        })
        .map(Arc::new);
    outcomes.normal_value = index_normal_result(&outcomes, value);
    outcomes.publish(state);
    *outcomes
}

impl SourceCommandBindings {
    pub(in super::super) fn walk_substitutions(
        &mut self,
        word: &tcl_source::words::WordExpr,
        document: &str,
        base: u32,
        state: &mut ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) -> SourceOutcomes {
        use tcl_source::words::WordExpr;
        match word {
            WordExpr::Variable { spelling, source } => {
                self.observe_variable_substitution(spelling, source, document, base, state, context)
            }
            WordExpr::CommandSubstitution { spelling, source } => {
                let script = spelling
                    .strip_prefix('[')
                    .and_then(|text| text.strip_suffix(']'))
                    .unwrap_or(spelling);
                self.walk_source(
                    script,
                    source.span.start() + 1,
                    state,
                    &SourceExecutionContext {
                        control: (context).control.with_source_purpose(
                            tcl_lexer::SourceSegmentationPurpose::NestedScript,
                        ),
                        ..context
                    },
                )
            }
            WordExpr::Template { parts, .. } => {
                self.walk_template_substitutions(word, parts, document, base, state, context)
            }
            WordExpr::Expand { word, .. } => {
                let mut outcomes = self.walk_substitutions(word, document, base, state, context);
                // Element expansion has an independent list conversion and arity.
                outcomes.normal_completion = None;
                outcomes
            }
            WordExpr::Literal { .. } | WordExpr::BracedLiteral { .. } => {
                let mut outcomes = SourceOutcomes::normal(state);
                outcomes.retain_complete_normal_evaluation();
                outcomes
            }
            WordExpr::Opaque { .. } => SourceOutcomes::normal(state),
        }
    }

    /// A source-model read: Logical source, or a hosted F5 rule's advisory
    /// walk of a handler body.
    fn source_model_substitution_access(
        &self,
        original: Option<&(&str, tcl_source::words::SourceSite)>,
        arena: Option<&tcl_lexer::ExecutablePartArena>,
        state: &ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) -> Option<crate::place::Place> {
        original_variable_compilation::logical_original_substitution_access(
            original, arena, state, context,
        )
        .or_else(|| {
            (self.declaration_preview_depth > 0)
                .then(|| {
                    original_variable_compilation::hosted_original_substitution_access(
                        original, arena, state, context,
                    )
                })
                .flatten()
        })
    }

    pub(in super::super) fn observe_variable_substitution(
        &mut self,
        spelling: &str,
        source: &tcl_source::words::SourceSite,
        document: &str,
        base: u32,
        state: &mut ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) -> SourceOutcomes {
        let original = original_variable_source(document, base, source, context.config);
        let arena = original_variable_arena(original.as_ref(), state, context.config);
        let root = arena.as_ref().zip(original.as_ref()).and_then(|(arena, (_, read_site))| {
            let policy = state.source_variables.execution_name_policy?.native_recipe()?;
            crate::signature_scan::variable_name::SignatureSourceVariableRoot::from_original_executable(
                arena, arena.image(), context.config, read_site.span,
                tcl_syntax::word_rules::WordValueRules::from_config(&context.config), policy)
        });
        let lexical = original_variable_compilation::OriginalSourceLexicalCompilation::at_source(
            state, context,
        );
        let mut outcomes = SourceOutcomes::normal(state);
        let mut evaluated_index = root.as_ref().and_then(
            crate::signature_scan::variable_name::SignatureSourceVariableRoot::static_index_input,
        );
        if let Some(arena) = &arena {
            for part in arena.list(arena.root()) {
                if let tcl_lexer::ExecutablePart::Variable {
                    index: Some(index), ..
                } = part.part
                {
                    let Some(continuing) = outcomes.normal.take() else {
                        break;
                    };
                    publish_source_branch(state, continuing);
                    let index_outcomes = self.walk_variable_index_arena(
                        SourceIndexView { arena, offset: 0 },
                        index,
                        state,
                        context,
                    );
                    if evaluated_index.is_none() {
                        evaluated_index = index_outcomes.normal_name_value.as_deref().map(|value|
                            crate::signature_scan::scope::SignatureSourceNameInput::OriginalValue(
                                crate::signature_scan::scope::SignatureSourceNameValue::from_original_produced_value(value)));
                    }
                    outcomes.join(&index_outcomes);
                }
            }
        }
        if let Some(continuing) = outcomes.normal.take() {
            *state = *continuing;
            let spelling = original.as_ref().map_or(spelling, |(original, _)| original);
            let logical = self.source_model_substitution_access(
                original.as_ref(),
                arena.as_ref(),
                state,
                context,
            );
            let access = logical.unwrap_or_else(|| {
                root.as_ref().zip(lexical.as_ref()).map_or_else(
                    crate::place::unknown_top,
                    |(root, lexical)| {
                        lexical.resolve_root(
                            root,
                            evaluated_index.as_ref(),
                            &state.source_variables,
                            context.registry,
                            tcl_registry::TraceOperation::Read,
                        )
                    },
                )
            });
            if let Some((original, read_site)) = &original
                && read_site.provenance == tcl_source::words::Provenance::Source
            {
                self.record_variable_access(
                    read_site,
                    original,
                    state,
                    context.variable_read_owner,
                    context.registry,
                    &access,
                );
            }
            if access.observed {
                let observed = self.walk_captured_observed_read(
                    source.span.start(),
                    &access,
                    Some(tcl_syntax::naming::var_reference_for_style(
                        spelling,
                        context.config.braced_var,
                    )),
                    state,
                    context,
                );
                outcomes.join(&observed);
            } else {
                let mut normal =
                    source_representation::unobserved_read_result(state, &access, context.registry);
                normal.normal_object = state.object_read_proof(&access, context.registry);
                outcomes.join(&normal);
            }
        }
        outcomes.publish(state);
        outcomes
    }

    fn walk_variable_index_arena(
        &mut self,
        source: SourceIndexView<'_>,
        root: tcl_lexer::word_parts::PartListId,
        state: &mut ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) -> SourceOutcomes {
        use SourceIndexFrame as Frame;
        use tcl_lexer::word_parts::ExecutablePart;
        use tcl_registry::completion_route::InvocationCompletionRoute as Route;

        let new_list = |id, state: &ModuleCommandBindings| Frame::List {
            id,
            next: 0,
            outcomes: Box::new(SourceOutcomes::normal(state)),
            value: Some(String::new()),
            native_value: None,
            native_complete: true,
        };
        let mut pending = vec![new_list(root, state)];
        let mut completed: Option<SourceOutcomes> = None;
        while let Some(frame) = pending.pop() {
            match frame {
                Frame::Variable { id, position } => {
                    let index = completed.take().expect("completed native index list");
                    completed = Some(self.finish_index_variable(
                        source,
                        &source.arena.list(id)[position],
                        index,
                        state,
                        context,
                    ));
                }
                Frame::List {
                    id,
                    next,
                    mut outcomes,
                    mut value,
                    mut native_value,
                    mut native_complete,
                } => {
                    if let Some(fragment) = completed.take() {
                        join_index_fragment(
                            &mut outcomes,
                            &mut value,
                            &fragment,
                            &mut native_value,
                            &mut native_complete,
                        );
                    }
                    let component = source.arena.list(id).get(next);
                    if component.is_none() || outcomes.normal.is_none() {
                        completed = Some(finish_index_list(outcomes, value, native_value, state));
                        continue;
                    }
                    let component = component.expect("remaining index component");
                    publish_source_branch(
                        state,
                        outcomes.normal.take().expect("continuing index branch"),
                    );
                    pending.push(Frame::List {
                        id,
                        next: next + 1,
                        outcomes,
                        value,
                        native_value,
                        native_complete,
                    });
                    match component.part {
                        ExecutablePart::Variable {
                            index: Some(index), ..
                        } => {
                            pending.push(Frame::Variable { id, position: next });
                            pending.push(new_list(index, state));
                        }
                        ExecutablePart::Variable { index: None, .. } => {
                            completed = Some(self.finish_index_variable(
                                source,
                                component,
                                SourceOutcomes::normal(state),
                                state,
                                context,
                            ));
                        }
                        ExecutablePart::Command { body } => {
                            completed = Some(self.walk_index_command(source, body, state, context));
                        }
                        ExecutablePart::Expression { .. } => {
                            completed = Some(opaque_source_invocation(state));
                        }
                        ExecutablePart::ParseError(_) => {
                            completed = Some(SourceOutcomes::invocation(
                                state,
                                Route::Tcl(tcl_registry::completion::CompletionCode::Error),
                            ));
                        }
                        ExecutablePart::Text(_) => {
                            completed = Some(index_text_fragment(source, component, state));
                        }
                    }
                }
            }
        }
        completed.expect("completed original index evaluation")
    }

    fn walk_index_command(
        &mut self,
        source: SourceIndexView<'_>,
        body: tcl_lexer::Span,
        state: &mut ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) -> SourceOutcomes {
        match source
            .arena
            .bytes(body)
            .and_then(|bytes| std::str::from_utf8(bytes).ok())
            .zip(source.offset.checked_add(body.start()))
        {
            Some((script, offset)) => self.walk_source(
                script,
                offset,
                state,
                &SourceExecutionContext {
                    control: (context)
                        .control
                        .with_source_purpose(tcl_lexer::SourceSegmentationPurpose::NestedScript),
                    depth: context.depth + 1,
                    ..context
                },
            ),
            None => opaque_source_invocation(state),
        }
    }

    fn finish_index_variable(
        &mut self,
        source: SourceIndexView<'_>,
        component: &tcl_lexer::word_parts::SpannedExecutablePart,
        mut outcomes: SourceOutcomes,
        state: &mut ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) -> SourceOutcomes {
        let Some(continuing) = outcomes.normal.take() else {
            return outcomes;
        };
        publish_source_branch(state, continuing);
        let Some(spelling) = source
            .arena
            .bytes(component.span)
            .and_then(|bytes| std::str::from_utf8(bytes).ok())
        else {
            outcomes.join(&opaque_source_invocation(state));
            return outcomes;
        };
        let tcl_lexer::word_parts::ExecutablePart::Variable { index, .. } = component.part else {
            unreachable!("selected index variable component")
        };
        let root = state.source_variables.execution_name_policy
            .and_then(tcl_syntax::naming::ExecutionNamePolicy::native_recipe)
            .and_then(|policy| crate::signature_scan::variable_name::SignatureSourceVariableRoot::from_original_executable(
                source.arena, source.arena.image(), context.config, component.span,
                tcl_syntax::word_rules::WordValueRules::from_config(&context.config), policy));
        let evaluated_index = root.as_ref().and_then(crate::signature_scan::variable_name::SignatureSourceVariableRoot::static_index_input)
            .or_else(|| index.and(outcomes.normal_name_value.as_deref()).map(|value|
                crate::signature_scan::scope::SignatureSourceNameInput::OriginalValue(
                    crate::signature_scan::scope::SignatureSourceNameValue::from_original_produced_value(value))));
        let lexical = original_variable_compilation::OriginalSourceLexicalCompilation::at_source(
            state, context,
        );
        let access = root.as_ref().zip(lexical.as_ref()).map_or_else(
            crate::place::unknown_top,
            |(root, lexical)| {
                lexical.resolve_root(
                    root,
                    evaluated_index.as_ref(),
                    &state.source_variables,
                    context.registry,
                    tcl_registry::TraceOperation::Read,
                )
            },
        );
        if let Some(span) = source.arena.source_span(component)
            && let Some((start, end)) = source
                .offset
                .checked_add(span.start())
                .zip(source.offset.checked_add(span.end()))
        {
            self.record_variable_access(
                &tcl_source::words::SourceSite::source(tcl_lexer::Span::new(start, end)),
                spelling,
                state,
                context.variable_read_owner,
                context.registry,
                &access,
            );
        } else {
            outcomes.join(&opaque_source_invocation(state));
            return outcomes;
        }
        let read = if access.observed {
            self.walk_captured_observed_read(
                context.invocation_offset,
                &access,
                Some(tcl_syntax::naming::var_reference_for_style(
                    spelling,
                    context.config.braced_var,
                )),
                state,
                context,
            )
        } else {
            source_representation::unobserved_read_result(state, &access, context.registry)
        };
        outcomes.join(&read);
        outcomes
    }
}
