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

//! Compatibility CFG projection of shared evaluated-body completion plans.

use super::CfgBuilder;
use crate::cfg::Terminator;
use crate::execution_region::{
    EvaluatedBodyRegion, RegionRepetition, RegionSelection, RegionTarget,
};
use crate::ir::{CommandTokens, Script, SourceSite, Statement, SyntheticMarker, WordExpr};

#[derive(Clone, Copy)]
struct DictionaryPhase<'a> {
    statement: &'a Statement,
    region: &'a EvaluatedBodyRegion,
    exit: &'a str,
    write_error: &'a str,
}

impl CfgBuilder<'_> {
    pub(super) fn evaluated_body_region(statement: &Statement) -> Option<&EvaluatedBodyRegion> {
        match statement {
            Statement::Call {
                tokens: Some(tokens),
                ..
            }
            | Statement::Barrier {
                tokens: Some(tokens),
                ..
            } => tokens.evaluated_body(),
            _ => None,
        }
    }

    pub(super) fn lower_evaluated_body(
        &mut self,
        statement: &Statement,
        region: &EvaluatedBodyRegion,
        current: &str,
    ) -> String {
        if region.scope.is_some() {
            return self.lower_dictionary_scope(statement, region, current);
        }
        if region.possible_bodies.is_some() {
            return self.lower_possible_bodies(statement, region, current);
        }
        // Substitution happens once, before selection. The residual wrapper
        // boundary has separate effects and does not evaluate these words again.
        self.push_plain_statement(
            current,
            &Self::region_boundary(statement, SyntheticMarker::EvaluatedArguments),
        );
        let exit = self.new_block("evaluated_body_end");
        if region.selection == RegionSelection::Never {
            self.ensure_goto(current, &exit, Some(region.source.span));
            self.push_plain_statement(
                &exit,
                &Self::region_boundary(statement, SyntheticMarker::EvaluatedWrapper),
            );
            return exit;
        }
        let entries: Vec<String> = region
            .phases
            .iter()
            .map(|_| self.new_block("evaluated_body_phase"))
            .collect();
        self.ensure_goto(current, &entries[0], Some(region.source.span));
        if region.selection == RegionSelection::MaySkip {
            self.analysis_edges.push((current.to_owned(), exit.clone()));
        }
        for (index, phase) in region.phases.iter().enumerate() {
            let normal = Self::region_target(phase.normal, &entries, &exit);
            let abrupt = Self::region_target(phase.abrupt, &entries, &exit);
            self.lower_region_phase(&phase.script, &entries[index], normal, abrupt);
        }
        self.push_plain_statement(
            &exit,
            &Self::region_boundary(statement, SyntheticMarker::EvaluatedWrapper),
        );
        if region.repetition == RegionRepetition::MayRepeat {
            self.analysis_edges.push((exit.clone(), entries[0].clone()));
        }
        exit
    }

    fn lower_possible_bodies(
        &mut self,
        statement: &Statement,
        region: &EvaluatedBodyRegion,
        current: &str,
    ) -> String {
        use tcl_registry::native_compilation::PossibleBodyTopology;
        let selected = region
            .possible_bodies
            .as_ref()
            .expect("selected possible body region");
        self.push_plain_statement(
            current,
            &Self::region_boundary(statement, SyntheticMarker::EvaluatedArguments),
        );
        let exit = self.new_block("possible_body_end");
        let entries: Vec<_> = region
            .phases
            .iter()
            .map(|_| self.new_block("possible_body_entry"))
            .collect();
        // The converged handler may decline body entry, and selection may fail
        // before any body effect. Neither path fabricates an entered script.
        self.analysis_edges.push((current.to_owned(), exit.clone()));
        match &selected.topology {
            PossibleBodyTopology::Captured(_) => {
                self.ensure_goto(current, &entries[0], Some(region.source.span));
                self.lower_region_phase(
                    &region.phases[0].script,
                    &entries[0],
                    Some(&exit),
                    Some(&exit),
                );
            }
            PossibleBodyTopology::Sequence(arguments) => {
                self.lower_possible_sequence(region, arguments, &entries, current, &exit);
            }
            PossibleBodyTopology::Alternatives(arguments) => {
                for argument in arguments {
                    if let Some(index) = selected
                        .arguments
                        .iter()
                        .position(|candidate| candidate == argument)
                    {
                        self.analysis_edges
                            .push((current.to_owned(), entries[index].clone()));
                        self.lower_region_phase(
                            &region.phases[index].script,
                            &entries[index],
                            Some(&exit),
                            None,
                        );
                    }
                }
                self.ensure_goto(current, &exit, Some(region.source.span));
            }
            PossibleBodyTopology::Conditional(branches) => {
                self.lower_possible_conditional(
                    statement, region, branches, &entries, current, &exit,
                );
            }
            PossibleBodyTopology::CaseAlternatives(operands) => {
                for (index, entry) in entries.iter().enumerate().take(operands.len()) {
                    self.analysis_edges
                        .push((current.to_owned(), entry.clone()));
                    self.lower_region_phase(&region.phases[index].script, entry, Some(&exit), None);
                }
                self.ensure_goto(current, &exit, Some(region.source.span));
            }
            PossibleBodyTopology::Loop {
                initial,
                repeated,
                continued,
            } => {
                let header = self.new_block("possible_body_loop");
                self.lower_possible_sequence(region, initial, &entries, current, &header);
                self.analysis_edges.push((header.clone(), exit.clone()));
                let continuation = continued
                    .first()
                    .and_then(|argument| {
                        selected
                            .arguments
                            .iter()
                            .position(|candidate| candidate == argument)
                    })
                    .map_or(&header, |index| &entries[index]);
                self.loop_stack.push((exit.clone(), continuation.clone()));
                self.lower_possible_sequence(region, repeated, &entries, &header, &header);
                self.loop_stack.pop();
            }
        }
        self.push_plain_statement(
            &exit,
            &Self::region_boundary(statement, SyntheticMarker::EvaluatedWrapper),
        );
        exit
    }

    fn lower_possible_conditional(
        &mut self,
        statement: &Statement,
        region: &EvaluatedBodyRegion,
        branches: &[(Option<usize>, usize)],
        entries: &[String],
        entry: &str,
        exit: &str,
    ) {
        let selected = region
            .possible_bodies
            .as_ref()
            .expect("possible conditional");
        let mut dispatch = entry.to_owned();
        let site = statement.tokens().and_then(|tokens| {
            let source = tokens.source_binding.as_ref()?;
            let target = source.proved_handler_target()?;
            Some(crate::ir::CommandBindingSite {
                span: statement.span(),
                binding: tcl_runtime_api::CommandBindingIdentity::in_namespace(
                    source
                        .lookup_namespace
                        .strip_prefix("::")
                        .unwrap_or(&source.lookup_namespace),
                    tokens.argv_texts.first()?.clone(),
                    target.command.clone(),
                ),
                known_namespaces: Some(source.known_namespaces.clone()),
                variable_frame: Some(source.variable_frame.clone()),
                variable_context: Some(std::sync::Arc::clone(&source.variable_context)),
                existing_namespace_cells: Some(source.existing_namespace_cells.clone()),
                source_tokens: Some(Box::new(tokens.clone())),
            })
        });
        for &(condition, body) in branches {
            let Some(index) = selected
                .arguments
                .iter()
                .position(|argument| *argument == body)
            else {
                continue;
            };
            let body_entry = &entries[index];
            if let Some(argument) = condition {
                let next = self.new_block("possible_if_next");
                let expression = selected
                    .conditions
                    .iter()
                    .find(|condition| condition.argument == argument);
                if let Some(condition) = expression
                    && let Some(expression) = &condition.expression
                {
                    self.retain_condition_binding(site.as_ref(), &dispatch);
                    let true_target = self.bid(body_entry);
                    let false_target = self.bid(&next);
                    self.set_terminator(
                        &dispatch,
                        Terminator::Branch {
                            condition: expression.clone(),
                            true_target,
                            false_target,
                            span: Some(condition.source.span),
                            condition_base: condition.expression_base,
                        },
                    );
                } else {
                    // Dynamic expression values preserve both possible selections
                    // without attributing invented AST reads or constant branches.
                    self.analysis_edges
                        .push((dispatch.clone(), body_entry.clone()));
                    self.ensure_goto(&dispatch, &next, Some(region.source.span));
                }
                self.lower_region_phase(&region.phases[index].script, body_entry, Some(exit), None);
                dispatch = next;
            } else {
                self.ensure_goto(&dispatch, body_entry, Some(region.source.span));
                self.lower_region_phase(&region.phases[index].script, body_entry, Some(exit), None);
                return;
            }
        }
        self.ensure_goto(&dispatch, exit, Some(region.source.span));
    }

    fn lower_possible_sequence(
        &mut self,
        region: &EvaluatedBodyRegion,
        arguments: &[usize],
        entries: &[String],
        entry: &str,
        exit: &str,
    ) {
        let selected = region.possible_bodies.as_ref().expect("possible topology");
        let indices: Vec<_> = arguments
            .iter()
            .filter_map(|argument| {
                selected
                    .arguments
                    .iter()
                    .position(|candidate| candidate == argument)
            })
            .collect();
        self.ensure_goto(
            entry,
            indices.first().map_or(exit, |index| &entries[*index]),
            Some(region.source.span),
        );
        for (position, &index) in indices.iter().enumerate() {
            let next = indices
                .get(position + 1)
                .map_or(exit, |next| &entries[*next]);
            self.lower_region_phase(
                &region.phases[index].script,
                &entries[index],
                Some(next),
                None,
            );
        }
    }

    fn lower_dictionary_scope(
        &mut self,
        statement: &Statement,
        region: &EvaluatedBodyRegion,
        current: &str,
    ) -> String {
        use tcl_registry::completion::CompletionCode;
        use tcl_registry::completion_route::InvocationCompletionRoute as Route;
        self.push_plain_statement(
            current,
            &Self::region_boundary(statement, SyntheticMarker::EvaluatedArguments),
        );
        let entry = self.new_block("dictionary_scope_entry");
        let exit = self.new_block("dictionary_scope_end");
        let entry_error = self.new_block("dictionary_scope_entry_error");
        self.set_terminator(
            &entry_error,
            Terminator::Complete {
                route: Route::Tcl(CompletionCode::Error),
                span: Some(region.source.span),
            },
        );
        let write_error = self.new_block("dictionary_scope_write_error");
        let failure = region
            .scope
            .as_ref()
            .expect("scope")
            .plan
            .writeback_failure_route();
        if failure.normal_possible() {
            self.ensure_goto(&write_error, &exit, Some(region.source.span));
        } else {
            self.set_terminator(
                &write_error,
                Terminator::Complete {
                    route: failure,
                    span: Some(region.source.span),
                },
            );
        }
        let scope = DictionaryPhase {
            statement,
            region,
            exit: &exit,
            write_error: &write_error,
        };
        let normal = self.dictionary_completion_target(scope, Route::Tcl(CompletionCode::Ok));
        let abrupt = self.dictionary_completion_target(scope, Route::UnknownAbrupt);
        self.ensure_goto(current, &entry, Some(region.source.span));
        self.push_plain_statement(
            &entry,
            &Self::region_boundary(statement, SyntheticMarker::DictionaryScopeEntry),
        );
        self.exception_edges.push((entry.clone(), entry_error));
        self.lower_region_phase_with_scope(
            &region.phases[0].script,
            &entry,
            Some(&normal),
            Some(&abrupt),
            Some(scope),
        );
        exit
    }

    fn dictionary_completion_target(
        &mut self,
        scope: DictionaryPhase<'_>,
        route: tcl_registry::completion_route::InvocationCompletionRoute,
    ) -> String {
        use tcl_registry::completion_route::InvocationCompletionRoute as Route;
        let target = self.new_block("dictionary_scope_writeback");
        self.push_plain_statement(
            &target,
            &Self::region_boundary(
                scope.statement,
                SyntheticMarker::DictionaryScopeWriteback(route),
            ),
        );
        self.exception_edges
            .push((target.clone(), scope.write_error.to_owned()));
        let route = scope
            .region
            .scope
            .as_ref()
            .unwrap()
            .plan
            .completion_after_writeback(route);
        if route.normal_possible() {
            if route.abrupt_possible() {
                self.analysis_edges
                    .push((target.clone(), scope.exit.to_owned()));
                self.set_terminator(
                    &target,
                    Terminator::Complete {
                        route: Route::UnknownAbrupt,
                        span: Some(scope.region.source.span),
                    },
                );
            } else {
                self.ensure_goto(&target, scope.exit, Some(scope.region.source.span));
            }
        } else {
            self.set_terminator(
                &target,
                Terminator::Complete {
                    route,
                    span: Some(scope.region.source.span),
                },
            );
        }
        target
    }

    fn region_boundary(statement: &Statement, marker: SyntheticMarker) -> Statement {
        let mut boundary = statement.clone();
        match &mut boundary {
            Statement::Call {
                tokens: Some(tokens),
                defs,
                ..
            } => {
                tokens.synthetic = Some(marker);
                if marker == SyntheticMarker::EvaluatedArguments {
                    defs.clear();
                }
            }
            Statement::Barrier {
                tokens: Some(tokens),
                ..
            } => tokens.synthetic = Some(marker),
            _ => unreachable!("evaluated regions require original invocation tokens"),
        }
        boundary
    }

    fn region_target<'a>(
        target: RegionTarget,
        entries: &'a [String],
        exit: &'a str,
    ) -> Option<&'a str> {
        match target {
            RegionTarget::Phase(index) => entries.get(index).map(String::as_str),
            RegionTarget::Exit => Some(exit),
            RegionTarget::Propagate => None,
        }
    }

    fn captured_return_payload(
        terminator: &Terminator,
        fallback_span: tcl_lexer::Span,
    ) -> Option<Statement> {
        let Terminator::Return {
            span,
            value,
            value_word,
            braced,
            tokens: original_tokens,
            ..
        } = terminator
        else {
            return None;
        };
        let value = value.as_ref()?;
        let span = span.unwrap_or(fallback_span);
        let mut tokens = original_tokens
            .as_deref()
            .cloned()
            .unwrap_or_else(|| CommandTokens::marker(SyntheticMarker::EvaluatedArguments));
        tokens.synthetic = Some(SyntheticMarker::EvaluatedArguments);
        if original_tokens.is_none() {
            Self::fill_lossy_return_payload(&mut tokens, value, value_word.as_ref(), *braced, span);
        }
        Some(Statement::Call {
            span,
            command: "return".into(),
            canonical_command: None,
            args: vec![value.clone()],
            defs: Vec::new(),
            reads: Vec::new(),
            reads_own_defs: false,
            safe_on_uninit: false,
            tokens: Some(tokens),
            foreach_groups: None,
        })
    }

    fn fill_lossy_return_payload(
        tokens: &mut CommandTokens,
        value: &str,
        value_word: Option<&WordExpr>,
        braced: bool,
        span: tcl_lexer::Span,
    ) {
        tokens.argv_texts = vec!["return".into(), value.to_owned()];
        tokens.argv = vec![span, value_word.map_or(span, |word| word.source().span)];
        tokens.argv_kinds = vec![
            tcl_lexer::TokenType::Esc,
            if braced {
                tcl_lexer::TokenType::Str
            } else {
                tcl_lexer::TokenType::Esc
            },
        ];
        tokens.single_token_word = vec![true, braced];
        tokens.word_exprs = vec![
            WordExpr::Literal {
                text: "return".into(),
                source: SourceSite::opaque(span),
            },
            value_word.cloned().unwrap_or_else(|| WordExpr::Opaque {
                text: value.to_owned(),
                source: SourceSite::opaque(span),
                reason: crate::ir::WordOpacity::LossySnapshot,
            }),
        ];
    }

    fn region_process_exit_block(&self, name: &str) -> bool {
        self.blocks
            .get(name)
            .and_then(|block| block.statements.last())
            .is_some_and(|statement| {
                super::always_exits_process(
                    statement,
                    self.registry,
                    &self.embedded_head_resolver(),
                )
            })
    }

    fn region_completion_blocks(&self, before: &str, before_blocks: usize) -> Vec<String> {
        let intercepted: std::collections::HashSet<String> = self
            .totally_intercepted(
                &|id| {
                    id == self.bid(before)
                        || usize::try_from(id.0).is_ok_and(|index| index >= before_blocks)
                },
                true,
            )
            .into_iter()
            .map(str::to_owned)
            .collect();
        let captured: Vec<String> = self
            .block_ids
            .iter()
            .filter_map(|(name, id)| {
                ((name == before
                    || usize::try_from(id.0).is_ok_and(|index| index >= before_blocks))
                    && !intercepted.contains(name.as_str())
                    && self.blocks.get(name.as_str()).is_some_and(|block| {
                        matches!(
                            block.terminator,
                            Some(Terminator::Return { .. } | Terminator::Complete { .. })
                        )
                    }))
                .then_some(name)
                .filter(|name| !self.region_process_exit_block(name))
                .cloned()
            })
            .collect();
        captured
    }

    fn capture_region_returns(
        &mut self,
        before: &str,
        before_blocks: usize,
        target: &str,
        span: tcl_lexer::Span,
        scope: Option<DictionaryPhase<'_>>,
    ) {
        let captured = self.region_completion_blocks(before, before_blocks);
        for name in captured {
            let selected_target = scope.map(|scope| {
                let route = match self.blocks[&name].terminator.as_ref().unwrap() {
                    Terminator::Complete { route, .. } => *route,
                    Terminator::Return { tokens, .. } => tokens.as_deref().and_then(|tokens| crate::registry_invocation::resolved_tokens_invocation(self.registry, None, tokens)).map_or(tcl_registry::completion_route::InvocationCompletionRoute::UnknownAbrupt, |invocation| invocation.completion_route(self.registry)),
                    _ => unreachable!("completion block selected by shared owner"),
                };
                self.dictionary_completion_target(scope, route)
            });
            let target_id = self.bid(selected_target.as_deref().unwrap_or(target));
            if let Some(payload) = self.blocks[&name]
                .terminator
                .as_ref()
                .and_then(|terminator| Self::captured_return_payload(terminator, span))
            {
                let previous = self
                    .terminator_sources
                    .get(&self.bid(&name))
                    .cloned()
                    .flatten();
                let source = std::mem::replace(&mut self.current_source, previous);
                self.push_statement(&name, payload);
                self.current_source = source;
            }
            self.set_terminator(
                &name,
                Terminator::Goto {
                    target: target_id,
                    span: Some(span),
                },
            );
        }
    }

    pub(super) fn lower_region_phase(
        &mut self,
        script: &Script,
        entry: &str,
        normal: Option<&str>,
        abrupt: Option<&str>,
    ) {
        self.lower_region_phase_with_scope(script, entry, normal, abrupt, None);
    }

    fn capture_phase_failure_edges(
        &mut self,
        before_blocks: usize,
        target: &str,
        tail: Option<&str>,
    ) {
        let failure_blocks: Vec<_> = self
            .block_ids
            .iter()
            .filter(|(name, id)| {
                usize::try_from(id.0).is_ok_and(|index| index >= before_blocks)
                    && !self.region_process_exit_block(name)
            })
            .map(|(name, _)| name.clone())
            .collect();
        self.exception_edges.extend(
            failure_blocks
                .into_iter()
                .map(|name| (name, target.to_owned())),
        );
        if let Some(tail) = tail {
            self.exception_edges
                .push((tail.to_owned(), target.to_owned()));
        }
    }

    fn lower_region_phase_with_scope(
        &mut self,
        script: &Script,
        entry: &str,
        normal: Option<&str>,
        abrupt: Option<&str>,
        scope: Option<DictionaryPhase<'_>>,
    ) {
        let previous_source =
            std::mem::replace(&mut self.current_source, script.executed_source.clone());
        if script.native_compilation_failure.is_some() {
            self.lower_script(script, entry);
            if let Some(target) = abrupt {
                self.capture_region_returns(
                    entry,
                    self.block_ids.len(),
                    target,
                    script
                        .statements
                        .first()
                        .map_or(tcl_lexer::Span::new(0, 0), Statement::span),
                    scope,
                );
            }
            self.current_source = previous_source;
            return;
        }
        self.command_binding_sites
            .extend(script.command_binding_sites.iter().cloned());
        self.procedure_binding_requirements
            .extend(script.procedure_binding_requirements.iter().cloned());
        let outer_throws = self.throw_blocks.take();
        self.throw_blocks = Some(Vec::new());
        let outer_loops = abrupt.map(|_| std::mem::take(&mut self.loop_stack));
        let mut current = Some(entry.to_owned());
        for statement in &script.statements {
            let Some(before) = current.take() else { break };
            let process_exit = super::always_exits_process(
                statement,
                self.registry,
                &self.embedded_head_resolver(),
            );
            // A failure can occur before an operation commits or after a trace
            // observes its writes. Keep both states instead of inventing defs.
            if let Some(target) = abrupt.filter(|_| !process_exit) {
                self.exception_edges
                    .push((before.clone(), target.to_owned()));
            }
            let before_blocks = self.block_ids.len();
            let mut one = Script::from_statements(vec![statement.clone()]);
            one.executed_source.clone_from(&script.executed_source);
            one.implicit_math_invocations
                .clone_from(&script.implicit_math_invocations);
            one.expression_preparations
                .clone_from(&script.expression_preparations);
            let tail = self.lower_script(&one, &before);
            let terminal = self.last_terminal_block.take();
            if let Some(target) = abrupt.filter(|_| !process_exit) {
                self.capture_phase_failure_edges(before_blocks, target, tail.as_deref());
                // A captured Tcl completion is not a procedure exit. Preserve
                // the value's uses as a statement, then route its completion to
                // the same handler as failures. A real process exit stays final.
                self.capture_region_returns(
                    &before,
                    before_blocks,
                    target,
                    statement.span(),
                    scope,
                );
            }
            if terminal.is_some() {
                current = None;
            } else if let Some(tail) = tail {
                let next = self.new_block("evaluated_body_step");
                self.ensure_goto(&tail, &next, Some(statement.span()));
                current = Some(next);
            }
        }
        if let (Some(tail), Some(target)) = (current, normal) {
            self.ensure_goto(&tail, target, None);
        }
        let throws = self.throw_blocks.take().unwrap_or_default();
        self.throw_blocks = outer_throws;
        if let Some(loops) = outer_loops {
            self.loop_stack = loops;
        }
        if abrupt.is_none()
            && let Some(outer) = &mut self.throw_blocks
        {
            outer.extend(throws.iter().cloned());
        }
        if let Some(target) = abrupt {
            self.total_interceptors.insert(target.to_owned());
            let throws: Vec<_> = throws
                .into_iter()
                .filter(|from| !self.region_process_exit_block(from))
                .collect();
            self.exception_edges
                .extend(throws.into_iter().map(|from| (from, target.to_owned())));
        }
        self.current_source = previous_source;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::execution_region::RegionSelection;
    use crate::lowering::lower_to_ir;
    use crate::ssa::build_ssa;
    use tcl_registry::CommandRegistry;

    fn caller_body(source: &str, registry: &CommandRegistry) -> Script {
        lower_to_ir(&format!("proc p {{}} {{{source}}}"), registry)
            .procedures
            .remove("::p")
            .expect("real procedure caller frame")
            .body
    }

    fn read_versions(ssa: &crate::ssa::SsaFunction, name: &str) -> Vec<u32> {
        let mut versions = Vec::new();
        for (block, body) in &ssa.blocks {
            for (index, statement) in body.statements.iter().enumerate() {
                let (Statement::Call { tokens, .. } | Statement::Barrier { tokens, .. }) =
                    &statement.statement
                else {
                    continue;
                };
                if !tokens.as_ref().is_some_and(|tokens| {
                    tokens.argv_texts.first().is_some_and(|head| head == "puts")
                }) {
                    continue;
                }
                let Some(symbol) = ssa.var_symbol_at(*block, index, name) else {
                    continue;
                };
                versions.push(
                    *statement
                        .uses
                        .get(&symbol)
                        .expect("real body variable read"),
                );
            }
        }
        versions
    }

    fn fixture(
        setup: &str,
        body: &str,
        cleanup: &str,
        selection: RegionSelection,
    ) -> (CommandRegistry, Script) {
        fixture_with_continuation(setup, body, cleanup, selection, "")
    }

    fn fixture_with_continuation(
        setup: &str,
        body: &str,
        cleanup: &str,
        selection: RegionSelection,
        continuation: &str,
    ) -> (CommandRegistry, Script) {
        let registry = CommandRegistry::build_default().project_for_profile(
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
        );
        let source = format!(
            "package require tcltest\nproc p {{}} {{tcltest::test name description -setup {{{setup}}} -body {{{body}}} -cleanup {{{cleanup}}}; {continuation}}}"
        );
        let mut script = lower_to_ir(&source, &registry)
            .procedures
            .remove("::p")
            .expect("real procedure caller frame")
            .body;
        let region = script.statements[0]
            .tokens_mut()
            .and_then(|tokens| tokens.evaluated_body.as_deref_mut())
            .expect("stock provider must produce a shared source-point lifecycle");
        region.selection = selection;
        region.repetition = crate::execution_region::RegionRepetition::Once;
        (registry, script)
    }

    #[test]
    fn setup_definitions_reach_body_through_normal_completion() {
        let (registry, script) = fixture("set x 1", "puts $x", "unset x", RegionSelection::MaySkip);
        let cfg = CfgBuilder::new(true, &registry)
            .with_faithful_exceptions()
            .build_function("::p", &script);
        assert_eq!(cfg.analysis_edges.len(), 1);
        let ssa = build_ssa(&cfg, &registry);
        let reads = read_versions(&ssa, "x");
        assert!(
            !reads.is_empty(),
            "phase body read is missing from SSA: {ssa:#?}"
        );
        assert!(reads.iter().all(|version| *version != 0));
    }

    #[test]
    fn backend_cfg_keeps_original_invocation_and_omits_analysis_phases() {
        let (registry, script) = fixture("set x 1", "puts $x", "unset x", RegionSelection::MaySkip);
        let cfg = CfgBuilder::new(true, &registry).build_function("::p", &script);
        assert_eq!(
            cfg.analysis_edges,
            [] as [(crate::cfg::BlockId, crate::cfg::BlockId); 0]
        );
        assert_eq!(
            cfg.exception_edges,
            [] as [(crate::cfg::BlockId, crate::cfg::BlockId); 0]
        );
        let statements: Vec<_> = cfg
            .blocks
            .values()
            .flat_map(|block| &block.statements)
            .collect();
        assert_eq!(statements, vec![&script.statements[0]]);
    }

    #[test]
    fn skipped_region_never_manufactures_script_definitions() {
        let (registry, script) = fixture("set x 1", "set y 2", "set z 3", RegionSelection::Never);
        let cfg = CfgBuilder::new(true, &registry)
            .with_faithful_exceptions()
            .build_function("::p", &script);
        let ssa = build_ssa(&cfg, &registry);
        for name in ["x", "y", "z"] {
            assert!(ssa.var_symbol(name).is_none());
        }
    }

    #[test]
    fn captured_return_routes_to_cleanup_without_leaving_procedure() {
        let (registry, script) = fixture_with_continuation(
            "set x 1",
            "return $x",
            "set cleaned 1",
            RegionSelection::Always,
            "puts $cleaned",
        );
        let cfg = CfgBuilder::new(true, &registry)
            .with_faithful_exceptions()
            .build_function("::p", &script);
        assert!(
            cfg.blocks
                .values()
                .all(|block| !matches!(block.terminator, Some(Terminator::Return { .. })))
        );
        let ssa = build_ssa(&cfg, &registry);
        assert!(
            ssa.blocks
                .values()
                .flat_map(|block| &block.statements)
                .any(
                    |statement| statement.statement.tokens().is_some_and(|tokens| tokens
                        .argv_texts
                        .first()
                        .is_some_and(|head| head == "return"))
                        && !statement.uses.is_empty()
                )
        );
        assert!(ssa.var_symbol("cleaned").is_some());
    }

    #[test]
    fn process_exit_remains_terminal_inside_a_captured_phase() {
        let (registry, script) = fixture("", "exit 0", "set cleaned 1", RegionSelection::Always);
        let cfg = CfgBuilder::new(true, &registry)
            .with_faithful_exceptions()
            .build_function("::p", &script);
        assert!(cfg.blocks.values().any(|block| matches!(
            block.terminator,
            Some(Terminator::Complete {
                route: tcl_registry::completion_route::InvocationCompletionRoute::ProcessExit,
                ..
            })
        )));
    }

    #[test]
    fn generic_catch_shares_captured_return_and_partial_write_routing() {
        let registry = CommandRegistry::build_default().project_for_profile(
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
        );
        let script = caller_body(
            "catch {set x 1; if {$condition} {return $x}; unset x; error stop} result; puts $result; puts $x",
            &registry,
        );
        let cfg = CfgBuilder::new(true, &registry)
            .with_faithful_exceptions()
            .build_function("::p", &script);
        assert!(
            cfg.blocks
                .values()
                .all(|block| !matches!(block.terminator, Some(Terminator::Return { .. })))
        );
        assert_ne!(
            cfg.exception_edges,
            [] as [(crate::cfg::BlockId, crate::cfg::BlockId); 0]
        );
        let ssa = build_ssa(&cfg, &registry);
        for name in ["x", "result"] {
            assert_ne!(
                read_versions(&ssa, name),
                [] as [u32; 0],
                "missing {name} read: {ssa:#?}"
            );
        }
    }

    #[test]
    fn literal_exit_in_catch_has_no_captured_resume_edge() {
        let registry = CommandRegistry::build_default().project_for_profile(
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
        );
        let script = caller_body("catch {exit 0}; puts after", &registry);
        let cfg = CfgBuilder::new(true, &registry)
            .with_faithful_exceptions()
            .build_function("::p", &script);
        let exiting: Vec<_> = cfg
            .blocks
            .iter()
            .filter(|(_, block)| {
                matches!(
                    block.terminator,
                    Some(Terminator::Complete {
                        route:
                            tcl_registry::completion_route::InvocationCompletionRoute::ProcessExit,
                        ..
                    })
                )
            })
            .map(|(id, _)| id)
            .collect();
        assert_ne!(exiting, [] as [&crate::cfg::BlockId; 0]);
        assert!(
            cfg.exception_edges
                .iter()
                .all(|(from, _)| !exiting.contains(&from))
        );
    }

    #[test]
    fn nested_process_exit_does_not_gain_a_cleanup_resume_edge() {
        let (registry, script) = fixture(
            "",
            "if {$condition} {exit 0}; return done",
            "set cleaned 1",
            RegionSelection::Always,
        );
        let cfg = CfgBuilder::new(true, &registry)
            .with_faithful_exceptions()
            .build_function("::p", &script);
        let exiting: Vec<_> = cfg
            .blocks
            .iter()
            .filter_map(|(id, block)| {
                block
                    .statements
                    .last()
                    .is_some_and(|statement| {
                        statement.tokens().is_some_and(|tokens| {
                            tokens.argv_texts.first().is_some_and(|head| head == "exit")
                        })
                    })
                    .then_some(*id)
            })
            .collect();
        assert_eq!(exiting.len(), 1);
        for id in exiting {
            assert!(matches!(
                cfg.blocks[&id].terminator,
                Some(Terminator::Complete {
                    route: tcl_registry::completion_route::InvocationCompletionRoute::ProcessExit,
                    ..
                })
            ));
            assert!(cfg.exception_edges.iter().all(|(from, _)| *from != id));
        }
    }

    #[test]
    fn repeated_lifecycle_adds_a_normal_backedge_without_re_evaluating_argv() {
        let (registry, mut script) =
            fixture("set x 1", "puts $x", "unset x", RegionSelection::MaySkip);
        match &mut script.statements[0] {
            Statement::Call {
                tokens: Some(tokens),
                ..
            }
            | Statement::Barrier {
                tokens: Some(tokens),
                ..
            } => tokens.evaluated_body.as_mut().unwrap().repetition = RegionRepetition::MayRepeat,
            _ => unreachable!(),
        }
        let cfg = CfgBuilder::new(true, &registry)
            .with_faithful_exceptions()
            .build_function("::p", &script);
        assert_eq!(cfg.analysis_edges.len(), 2);
        let arguments = cfg.blocks.values().flat_map(|block| &block.statements).filter(|statement| matches!(statement, Statement::Call { tokens: Some(tokens), .. } | Statement::Barrier { tokens: Some(tokens), .. } if tokens.synthetic == Some(SyntheticMarker::EvaluatedArguments))).count();
        assert_eq!(arguments, 1);
        let ssa = build_ssa(&cfg, &registry);
        assert_ne!(read_versions(&ssa, "x"), [] as [u32; 0]);
    }
}
