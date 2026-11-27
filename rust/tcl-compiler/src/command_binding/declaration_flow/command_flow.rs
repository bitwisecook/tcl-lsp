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

//! The flow graph of a script: command nodes and their handler, lifecycle, branch and loop flow.

use super::super::{CommandAllocationSite, ExecutedScriptMapping};
use super::events::Event;
use super::{
    Builder, ConditionTargets, ConditionalRepresentationLoopNodes,
    ConditionalRepresentationLoopPlan, ConditionalRepresentationPlan, DeclarationFlowFrame,
    DeclarationFlowPurpose, DeclaredOverwrite, Exits, Job, Node, representation,
};
use crate::ir::CommandTokens;
use std::sync::Arc;
use tcl_registry::body_execution::BodyOperand;
use tcl_registry::script_body_flow::ScriptBodyFlow;

impl Builder<'_> {
    pub(super) fn completion_frame(&self) -> tcl_registry::VariableAliasFrame {
        if self.purpose == DeclarationFlowPurpose::Representation
            && matches!(
                self.entry,
                DeclarationFlowFrame::Existing(
                    super::super::declaration_layout::OriginalDiagnosticFrameEntry::RootScript { .. }
                )
            )
        {
            tcl_registry::VariableAliasFrame::Global
        } else {
            tcl_registry::VariableAliasFrame::Procedure
        }
    }

    pub(super) fn placeholder(&mut self) -> usize {
        let id = self.nodes.len();
        self.nodes.push(Node::default());
        id
    }

    pub(super) fn unknown(&mut self, id: usize, next: usize) {
        if self.purpose == DeclarationFlowPurpose::Representation {
            self.nodes[id]
                .representation_events
                .push(representation::Event::Barrier);
        }
        self.nodes[id].unknown = true;
        self.nodes[id].availability.unavailable_declared_prefix = true;
        self.nodes[id].successors = vec![next];
    }

    pub(super) fn script(&mut self, job: &Job) {
        let previous = self.current_representation_body.take();
        self.current_representation_body
            .clone_from(&job.representation_body);
        if let Some(context) = &job.representation_body {
            if let Some(substitution) = &context.substitution {
                self.nodes[job.entry].representation_events.push(
                    representation::Event::RequireBodyContext {
                        context: Arc::clone(&substitution.parent),
                    },
                );
            }
            self.nodes[job.entry]
                .representation_events
                .push(representation::Event::BodyContext {
                    context: Arc::clone(context),
                });
            if context.substitution.is_some() {
                self.nodes[job.entry]
                    .representation_events
                    .push(representation::Event::ClearResult);
            }
        }
        self.original_script(
            &job.source.text,
            job.source.base(),
            &job.source.origin,
            job.purpose,
            job.entry,
            job.exits,
        );
        self.current_representation_body = previous;
    }

    pub(super) fn original_script(
        &mut self,
        image: &tcl_lexer::SourceImage,
        base: u32,
        origin: &Arc<super::super::SourceOriginId>,
        purpose: tcl_lexer::SourceSegmentationPurpose,
        entry: usize,
        exits: Exits,
    ) {
        let Some(segments) = tcl_source::segment::segment_commands_image_with_offset_and_config(
            image,
            base,
            purpose.config(self.config),
        ) else {
            self.unknown(entry, exits.normal);
            return;
        };
        self.retain_read_sites(image, base, origin, &segments);
        let mut next = exits.normal;
        let mut next_original = None;
        for segment in segments.iter().rev() {
            if segment.is_partial {
                let unavailable = self.placeholder();
                self.unknown(unavailable, exits.other);
                next = unavailable;
                next_original = None;
                continue;
            }
            let tokens = CommandTokens::from_segmented(
                &image.source_map().with_base(base, 0, 0),
                self.config,
                segment,
            );
            let id = self.placeholder();
            let site = CommandAllocationSite {
                source: Arc::clone(origin),
                offset: segment.span.start(),
            };
            let exits = Exits {
                normal: next,
                ..exits
            };
            self.command(id, &tokens, &site, exits);
            if let Some((next_tokens, next_site)) = &next_original {
                self.retain_conditional_overwrite(id, &tokens, &site, next_tokens, next_site);
            }
            next_original = Some((tokens, site));
            next = id;
        }
        self.nodes[entry].successors = vec![next];
    }

    fn retain_read_sites(
        &mut self,
        image: &tcl_lexer::SourceImage,
        base: u32,
        origin: &Arc<super::super::SourceOriginId>,
        segments: &[tcl_source::segment::SegmentedCommand],
    ) {
        if let DeclarationFlowFrame::Read(read) = self.entry {
            for segment in segments {
                if segment.is_partial {
                    break;
                }
                let tokens =
                    super::super::source_command_tokens_boxed(image, base, self.config, segment);
                let site = CommandAllocationSite {
                    source: Arc::clone(origin),
                    offset: segment.span.start(),
                };
                if !read.owns_source(origin, site.offset) {
                    break;
                }
                self.read_sites.insert(site.clone());
                if self.source_read_selection(&tokens, &site).is_none()
                    && !super::super::declaration_preview::retains_successor_layout(
                        &site,
                        &tokens,
                        &read.snapshot,
                        &read.header.namespace,
                        self.config,
                        self.registry,
                    )
                {
                    break;
                }
            }
        }
    }

    fn retain_conditional_overwrite(
        &mut self,
        first_id: usize,
        first: &CommandTokens,
        first_site: &CommandAllocationSite,
        next: &CommandTokens,
        next_site: &CommandAllocationSite,
    ) {
        let Some((first_name, target)) = self.literal_local_setter(first, first_site) else {
            return;
        };
        let Some((next_name, _)) = self.literal_local_setter(next, next_site) else {
            return;
        };
        if first_name == next_name && first_site.source == next_site.source {
            self.conditional_overwrites.push((
                first_id,
                DeclaredOverwrite {
                    first: first_site.clone(),
                    next: next_site.clone(),
                    name: first_name,
                    target,
                },
            ));
        }
    }

    fn literal_local_setter(
        &self,
        tokens: &CommandTokens,
        site: &CommandAllocationSite,
    ) -> Option<(String, tcl_lexer::Span)> {
        let (selected, _) = self.selection(tokens, site)?;
        let [(name, _)] = selected.literal_stores.as_slice() else {
            return None;
        };
        if selected.effects.unknown_writes
            || selected.effects.unknown_reads
            || selected.writes.as_slice() != std::slice::from_ref(name)
            || !selected.reads.is_empty()
            || !selected.aliases.is_empty()
            || !matches!(selected.flow, ScriptBodyFlow::None)
            || tcl_syntax::naming::is_qualified(name.as_bytes())
            || tcl_syntax::naming::split_element_ref(name).is_some()
            || selected.effective.words.iter().skip(1).any(|word| {
                crate::registry_invocation::effective_invocation_word(
                    word,
                    self.dialect.lexer_grammar.escapes,
                    self.dialect.word_values,
                )
                .literal_bytes()
                .is_none()
            })
        {
            return None;
        }
        let target_at = selected.literal_store_operand?;
        let crate::registry_invocation::InvocationWordOrigin::Written(written) =
            selected.effective.origins.get(target_at)?
        else {
            return None;
        };
        let target = tokens.words().get(*written)?.source().span;
        Some((name.clone(), target))
    }

    pub(super) fn command(
        &mut self,
        id: usize,
        tokens: &CommandTokens,
        site: &CommandAllocationSite,
        exits: Exits,
    ) {
        self.nodes[id].site = Some(site.offset);
        let selected = self.selection(tokens, site);
        let loop_plan = if self.purpose == DeclarationFlowPurpose::Representation {
            selected
                .as_ref()
                .and_then(|(selected, _)| self.representation_loop_plan(tokens, site, selected))
        } else {
            None
        };
        let loop_control = loop_plan.as_ref().map(|plan| Arc::clone(&plan.source));
        let mut representation_plan = if self.purpose == DeclarationFlowPurpose::Representation {
            let plan = selected
                .as_ref()
                .and_then(|(selected, _)| self.representation_plan(tokens, site, selected));
            if let Some(context) = plan.as_ref().and_then(|plan| plan.body_context.as_ref()) {
                self.nodes[id].representation_events.push(
                    representation::Event::RequireBodyContext {
                        context: Arc::clone(context),
                    },
                );
            }
            self.nodes[id]
                .representation_events
                .push(representation::Event::ClearResult);
            plan
        } else {
            None
        };
        #[cfg(debug_assertions)]
        if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_RBS_SOURCE_OWNER").is_some() {
            if let Some((flow, _)) = &selected {
                eprintln!(
                    "RBS_GRAPH stage=selection offset={} selected=true unknown_writes={} unknown_reads={} source_catalogue={} loop_flow={} case_flow={} writes={} loop_bindings={}",
                    site.offset,
                    flow.effects.unknown_writes,
                    flow.effects.unknown_reads,
                    matches!(flow.effects.schema_origin, crate::registry_invocation::DeclarationSchemaOrigin::OriginalSourceCatalogue),
                    matches!(&flow.flow, ScriptBodyFlow::Loop { .. }),
                    matches!(&flow.flow, ScriptBodyFlow::CaseBodies(_)),
                    flow.writes.len(),
                    flow.loop_bindings.len(),
                );
            } else {
                eprintln!(
                    "RBS_GRAPH stage=selection offset={} selected=false",
                    site.offset
                );
            }
        }
        let dispatch = self.placeholder();
        let operands =
            self.representation_operand_events(tokens, site, representation_plan.as_mut());
        let next = self.events_before(operands, dispatch, exits);
        self.nodes[id].successors = vec![next];
        let Some((selected, lifecycle)) = selected else {
            self.unknown(dispatch, exits.normal);
            return;
        };
        let after = self.selected_effect_nodes(id, dispatch, site, &selected, exits.normal);
        self.representation_completion(
            representation_plan,
            loop_control,
            tokens,
            site,
            after,
            dispatch,
        );
        let successors =
            self.command_flow(site, &selected, lifecycle.as_ref(), after, exits, loop_plan);
        self.nodes[dispatch].successors = successors;
    }

    fn representation_operand_events(
        &self,
        tokens: &CommandTokens,
        site: &CommandAllocationSite,
        mut representation_plan: Option<&mut ConditionalRepresentationPlan>,
    ) -> Vec<Event> {
        let mut operands = Vec::new();
        for (index, word) in tokens.words().iter().enumerate() {
            let first_event = operands.len();
            self.word_events(word, site, &mut operands);
            if let Some(plan) = representation_plan.as_deref_mut()
                && index > 0
            {
                let Some(from) = plan.captures.pop_front() else {
                    operands.push(Event::Representation(Box::new(
                        representation::Event::Barrier,
                    )));
                    continue;
                };
                if matches!(&from, representation::CaptureSource::LastResult) {
                    let context = match &operands[first_event..] {
                        [Event::Script(source)] => self.conditional_substitution_context(
                            plan,
                            site,
                            tokens,
                            index - 1,
                            source,
                        ),
                        _ => None,
                    };
                    if let Some(context) = context {
                        let Event::Script(source) =
                            std::mem::replace(&mut operands[first_event], Event::Unknown)
                        else {
                            unreachable!("checked single script event")
                        };
                        operands[first_event] = Event::ConditionalScript(source, context);
                    }
                }
                if matches!(&from, representation::CaptureSource::Variable { .. }) {
                    for event in &mut operands[first_event..] {
                        if matches!(event, Event::ReadUnavailable) {
                            *event = Event::ConditionalReadUnavailable;
                        }
                    }
                }
                operands.push(Event::Representation(Box::new(
                    representation::Event::Capture {
                        into: representation::OperandSnapshot {
                            site: site.offset,
                            operand: tcl_registry::value_transfer::OperandId(index - 1),
                        },
                        from,
                        source: Arc::clone(&plan.source),
                    },
                )));
            }
        }
        operands
    }

    fn selected_effect_nodes(
        &mut self,
        id: usize,
        dispatch: usize,
        site: &CommandAllocationSite,
        selected: &crate::registry_invocation::DeclarationInvocationFlow,
        next: usize,
    ) -> usize {
        self.nodes[id].availability.schema_origin = selected.effects.schema_origin;
        for &operand in &selected.quoted_operands {
            match self.quoted_mentions(site, &selected.effective, operand) {
                Ok(mentions) => self.nodes[id].quoted_mentions.extend(mentions),
                Err(()) => self.nodes[id].availability.quoted_use_unavailable = true,
            }
        }
        self.nodes[dispatch].handler_site = Some(site.offset);
        self.nodes[dispatch].aliases.clone_from(&selected.aliases);
        self.nodes[dispatch]
            .caller_frame_aliases
            .clone_from(&selected.caller_frame_aliases);
        self.nodes[dispatch].unknown = selected.effects.unknown_writes;
        self.nodes[dispatch].named_reads.clone_from(&selected.reads);
        self.nodes[dispatch]
            .name_queries
            .clone_from(&selected.name_queries);
        self.nodes[dispatch].availability.unknown_reads = selected.effects.unknown_reads;
        let after = self.placeholder();
        self.nodes[after].store_site = Some(site.offset);
        self.nodes[after].writes.clone_from(&selected.writes);
        self.nodes[after]
            .possible_output_writes
            .clone_from(&selected.possible_output_writes);
        self.nodes[after]
            .literal_stores
            .clone_from(&selected.literal_stores);
        self.nodes[after]
            .increments
            .clone_from(&selected.increments);
        self.nodes[after]
            .accumulator_writes
            .clone_from(&selected.accumulator_writes);
        self.nodes[after].removes.clone_from(&selected.removes);
        self.nodes[after].successors = vec![next];
        after
    }

    fn representation_completion(
        &mut self,
        representation_plan: Option<ConditionalRepresentationPlan>,
        loop_control: Option<
            Arc<crate::registry_invocation::source_structure::OriginalRegistryWords>,
        >,
        tokens: &CommandTokens,
        site: &CommandAllocationSite,
        after: usize,
        dispatch: usize,
    ) {
        if self.purpose == DeclarationFlowPurpose::Representation {
            if let (Some(plan), Some(first)) = (representation_plan, tokens.words().first()) {
                let span = first.source().span;
                if let Some(relation) = plan.relation {
                    self.nodes[after]
                        .representation_events
                        .push(representation::Event::Apply {
                            site: site.offset,
                            span,
                            relation,
                            prior_target: plan.prior_target,
                            target: plan.target,
                            source: Arc::clone(&plan.source),
                        });
                } else if let Some(relation) = plan.list_append {
                    if let Some(target) = plan.target {
                        self.nodes[after].representation_events.push(
                            representation::Event::ListAppend {
                                site: site.offset,
                                span,
                                relation,
                                target,
                                source: Arc::clone(&plan.source),
                            },
                        );
                    } else {
                        self.nodes[dispatch]
                            .representation_events
                            .push(representation::Event::Barrier);
                    }
                }
                if let Some(coercion) = plan.coercion {
                    self.nodes[after].representation_events.push(
                        representation::Event::ListIndex {
                            site: site.offset,
                            span,
                            coercion,
                            literal: plan.list_index,
                            source: plan.source,
                        },
                    );
                }
            } else if let Some(source) = loop_control {
                self.nodes[dispatch]
                    .representation_events
                    .push(representation::Event::Control { source });
            } else {
                self.nodes[dispatch]
                    .representation_events
                    .push(representation::Event::Barrier);
            }
        }
    }

    pub(super) fn command_flow(
        &mut self,
        site: &CommandAllocationSite,
        selected: &crate::registry_invocation::DeclarationInvocationFlow,
        lifecycle: Option<&super::super::lifecycle_advice::SourceScopedLifecycleAdvice>,
        after: usize,
        exits: Exits,
        loop_plan: Option<ConditionalRepresentationLoopPlan>,
    ) -> Vec<usize> {
        let body_exits = Exits {
            normal: after,
            ..exits
        };
        let flow_entry = match &selected.flow {
            ScriptBodyFlow::None | ScriptBodyFlow::Deferred => {
                return self.handler_completion_flow(selected, body_exits);
            }
            ScriptBodyFlow::Sequence(arguments) => {
                self.sequence(site, &selected.effective, arguments, body_exits)
            }
            ScriptBodyFlow::CaseBodies(case) => {
                let mut edges = case
                    .bodies
                    .iter()
                    .map(|operand| self.body(site, &selected.effective, *operand, body_exits))
                    .collect::<Vec<_>>();
                if case.no_match_possible {
                    edges.push(after);
                }
                // An uncertain option/subject layout also has an abrupt path.
                if case.selection_unknown {
                    edges.push(exits.error);
                }
                let branch = self.placeholder();
                self.nodes[branch].successors = edges;
                branch
            }
            ScriptBodyFlow::Alternatives(arguments) => {
                let branch = self.placeholder();
                self.nodes[branch].successors = arguments
                    .iter()
                    .map(|argument| {
                        self.body(
                            site,
                            &selected.effective,
                            BodyOperand {
                                argument: *argument,
                                list_element: None,
                            },
                            body_exits,
                        )
                    })
                    .chain(std::iter::once(after))
                    .collect();
                branch
            }
            ScriptBodyFlow::Conditional(branches) => {
                self.conditional_flow(site, selected, branches, after, body_exits, exits)
            }
            flow @ ScriptBodyFlow::Loop { .. } => {
                self.loop_flow(site, selected, flow, after, exits, loop_plan)
            }
            ScriptBodyFlow::Capture(
                tcl_registry::catch_invocation::CatchInvocationSelection::Valid(capture),
            ) if capture.ignored_codes == 0 => {
                let caught = Exits {
                    normal: after,
                    error: after,
                    returned: after,
                    broken: after,
                    continued: after,
                    other: exits.other,
                    cleanup_entry: exits.cleanup_entry,
                };
                self.body(
                    site,
                    &selected.effective,
                    BodyOperand {
                        argument: capture.script_at,
                        list_element: None,
                    },
                    caught,
                )
            }
            ScriptBodyFlow::Expressions(arguments) => {
                self.expressions(site, &selected.effective, arguments, after, exits)
            }
            ScriptBodyFlow::CapturedLifecycle => {
                let Some(lifecycle) = lifecycle else {
                    let unknown = self.placeholder();
                    self.unknown(unknown, after);
                    return vec![unknown];
                };
                self.lifecycle_flow(site, selected, lifecycle, after, exits)
            }
            ScriptBodyFlow::ConcatenatedExpression { argument_offset }
                if selected.effective.words.len() == argument_offset + 2 =>
            {
                self.expressions(site, &selected.effective, &[*argument_offset], after, exits)
            }
            // Expressions may contain lazily selected script substitutions;
            // unsupported value/lifecycle protocols retain an unknown writer.
            _ => {
                let unknown = self.placeholder();
                self.unknown(unknown, after);
                unknown
            }
        };
        vec![flow_entry]
    }

    fn handler_completion_flow(
        &mut self,
        selected: &crate::registry_invocation::DeclarationInvocationFlow,
        exits: Exits,
    ) -> Vec<usize> {
        let mut successors = exits.route(selected.completion);
        if let Some(cleanup) = exits.cleanup_entry
            && selected.completion.abrupt_possible()
            && (!selected.writes.is_empty() || !selected.possible_output_writes.is_empty())
        {
            // Successful output roles do not describe partial stores
            // on failure, including a completion caught by cleanup.
            let uncertain = self.placeholder();
            self.nodes[uncertain].uncertain_writes = selected
                .writes
                .iter()
                .chain(&selected.possible_output_writes)
                .cloned()
                .collect();
            self.nodes[uncertain].successors = vec![cleanup];
            for successor in &mut successors {
                if *successor == cleanup {
                    *successor = uncertain;
                }
            }
            successors.sort_unstable();
            successors.dedup();
        }
        successors
    }

    pub(super) fn lifecycle_flow(
        &mut self,
        site: &CommandAllocationSite,
        selected: &crate::registry_invocation::DeclarationInvocationFlow,
        lifecycle: &super::super::lifecycle_advice::SourceScopedLifecycleAdvice,
        after: usize,
        exits: Exits,
    ) -> usize {
        let complete = self.placeholder();
        self.nodes[complete].successors = Exits {
            normal: after,
            ..exits
        }
        .route(selected.completion);
        let repeat = self.placeholder();
        self.nodes[repeat].successors = vec![complete];
        let captured = |normal, cleanup_entry| Exits {
            normal,
            error: normal,
            returned: normal,
            broken: normal,
            continued: normal,
            other: exits.other,
            cleanup_entry,
        };
        let phases = &lifecycle.invocation.selection;
        let phase_body = |builder: &mut Self, operand: BodyOperand, exits: Exits| {
            let Some(argument) = operand
                .argument
                .checked_add(lifecycle.invocation.argument_offset)
            else {
                let unknown = builder.placeholder();
                builder.unknown(unknown, exits.normal);
                return unknown;
            };
            builder.body(
                site,
                &selected.effective,
                BodyOperand {
                    argument,
                    ..operand
                },
                exits,
            )
        };
        let cleanup = phases.cleanup.map_or(repeat, |operand| {
            phase_body(self, operand, captured(repeat, exits.cleanup_entry))
        });
        let body = phases.body.map_or(cleanup, |operand| {
            phase_body(self, operand, captured(cleanup, Some(cleanup)))
        });
        let setup = phases.setup.map_or(body, |operand| {
            phase_body(
                self,
                operand,
                Exits {
                    normal: body,
                    ..captured(cleanup, Some(cleanup))
                },
            )
        });
        if lifecycle
            .invocation
            .contract
            .repetition(lifecycle.loader.version.as_deref())
            == tcl_registry::body_execution::BodyRepetition::MayRepeat
        {
            self.nodes[repeat].successors.push(setup);
        }
        let choice = self.placeholder();
        self.nodes[choice].successors = vec![setup, complete];
        choice
    }

    pub(super) fn conditional_flow(
        &mut self,
        site: &CommandAllocationSite,
        selected: &crate::registry_invocation::DeclarationInvocationFlow,
        branches: &[(Option<usize>, usize)],
        after: usize,
        body_exits: Exits,
        exits: Exits,
    ) -> usize {
        let mut otherwise = after;
        for &(condition, argument) in branches.iter().rev() {
            let body = self.body(
                site,
                &selected.effective,
                BodyOperand {
                    argument,
                    list_element: None,
                },
                body_exits,
            );
            otherwise = if let Some(condition) = condition {
                self.condition(
                    site,
                    &selected.effective,
                    condition,
                    ConditionTargets {
                        yes: body,
                        no: otherwise,
                        first_iteration: false,
                    },
                    exits,
                )
            } else {
                body
            };
        }
        otherwise
    }

    fn loop_flow(
        &mut self,
        site: &CommandAllocationSite,
        selected: &crate::registry_invocation::DeclarationInvocationFlow,
        flow: &ScriptBodyFlow,
        after: usize,
        exits: Exits,
        loop_plan: Option<ConditionalRepresentationLoopPlan>,
    ) -> usize {
        let ScriptBodyFlow::Loop {
            initial,
            repeated,
            continued,
            conditions,
        } = flow
        else {
            unreachable!("selected loop flow");
        };
        let header = self.placeholder();
        let bindings = self.placeholder();
        self.nodes[bindings]
            .writes
            .clone_from(&selected.loop_bindings);
        let update = self.sequence(
            site,
            &selected.effective,
            continued,
            Exits {
                normal: header,
                ..exits
            },
        );
        let body_exits = Exits {
            normal: header,
            broken: after,
            continued: update,
            ..exits
        };
        let jobs_before = self.jobs.len();
        let repeated = self.sequence(site, &selected.effective, repeated, body_exits);
        self.attach_representation_body_job(jobs_before, loop_plan.as_ref());
        self.nodes[bindings].successors = vec![repeated];
        let mut test = bindings;
        if conditions.is_empty() {
            let branch = self.placeholder();
            self.nodes[branch].successors = vec![after, bindings];
            test = branch;
        } else {
            for &condition in conditions.iter().rev() {
                test = self.condition(
                    site,
                    &selected.effective,
                    condition,
                    ConditionTargets {
                        yes: test,
                        no: after,
                        first_iteration: false,
                    },
                    exits,
                );
            }
        }
        self.nodes[header].successors = vec![test];
        let mut first_test = bindings;
        if conditions.is_empty() {
            let branch = self.placeholder();
            self.nodes[branch].successors = vec![after, bindings];
            self.nodes[branch].unknown_list_first =
                (selected.list_loop && selected.first_list_iteration.is_none()).then_some(bindings);
            self.nodes[branch].declared_successors = selected
                .first_list_iteration
                .map(|nonempty| vec![if nonempty { bindings } else { after }]);
            first_test = branch;
        } else {
            for &condition in conditions.iter().rev() {
                first_test = self.condition(
                    site,
                    &selected.effective,
                    condition,
                    ConditionTargets {
                        yes: first_test,
                        no: after,
                        first_iteration: true,
                    },
                    exits,
                );
            }
        }
        if let Some(plan) = loop_plan {
            let nodes = ConditionalRepresentationLoopNodes {
                first_gate: first_test,
                bindings,
                repeated_entry: repeated,
                header,
            };
            self.retain_representation_loop(plan, site, nodes, after, exits);
        }
        self.sequence(
            site,
            &selected.effective,
            initial,
            Exits {
                normal: first_test,
                ..exits
            },
        )
    }

    fn attach_representation_body_job(
        &mut self,
        jobs_before: usize,
        plan: Option<&ConditionalRepresentationLoopPlan>,
    ) {
        let Some(context) = plan.and_then(|plan| plan.body_context.as_ref()) else {
            return;
        };
        if self.jobs.len() != jobs_before + 1 {
            return;
        }
        let Some(job) = self.jobs.last_mut() else {
            return;
        };
        let span = context.body.content_span();
        let length = u32::try_from(job.source.text.len()).ok();
        if Arc::ptr_eq(&job.source.origin, &context.site.source)
            && job.source.mapping == (ExecutedScriptMapping::Contiguous { base: span.start() })
            && length.and_then(|length| span.start().checked_add(length)) == Some(span.end())
            && context
                .site
                .source
                .source_image()
                .bytes()
                .get(span.as_range())
                == Some(job.source.text.bytes())
        {
            job.representation_body = Some(Arc::clone(context));
        }
    }

    fn retain_representation_loop(
        &mut self,
        plan: ConditionalRepresentationLoopPlan,
        site: &CommandAllocationSite,
        nodes: ConditionalRepresentationLoopNodes,
        after: usize,
        exits: Exits,
    ) {
        self.nodes[nodes.bindings]
            .representation_events
            .push(representation::Event::Reset {
                targets: vec![plan.binder],
                source: Arc::clone(&plan.source),
            });
        self.representation_loops.push(representation::LoopRegion {
            id: representation::LoopId(self.representation_loops.len()),
            site: site.offset,
            span: plan.span,
            source: plan.source,
            count: plan.count,
            first_gate: nodes.first_gate,
            bindings: nodes.bindings,
            repeated_entry: nodes.repeated_entry,
            header: nodes.header,
            normal_exit: after,
            error_exit: exits.error,
            returned_exit: exits.returned,
            other_exit: exits.other,
        });
    }
}
