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

//! The read and write events of words, variables, expressions and bodies.

use super::super::source_read_scope::{trace_rbs_refusal, trace_rbs_required};
use super::{
    Builder, ConditionTargets, ConditionalRepresentationBodyContext, DeclarationFlowPurpose,
    DeclaredCondition, Exits, Job, representation,
};
use crate::command_binding::ExecutedScriptWords;
use std::collections::BTreeSet;
use std::sync::Arc;
use tcl_names::command::CommandAllocationSite;
use tcl_names::origin::{ExecutedScriptMapping, ExecutedScriptSource};
use tcl_registry::body_execution::BodyOperand;
use tcl_source::words::WordExpr;
use tcl_source::words::WordPart;

// Source graph branch only: the common Tcl Boolean constants need no selected
// Native parser. The complete source receipt supplies no Native preparation,
// error, result representation, frame, or executable rewriting permission.
pub(super) fn logical_boolean_literal_condition(
    text: &str,
    dialect: tcl_native_model::InvocationDialect,
    config: tcl_lexer::LexerConfig,
) -> Option<tcl_syntax::expr::ast::ExprNode> {
    use tcl_syntax::expr::ast::ExprNode;
    if dialect.family() != Some(tcl_dialect::model::Family::Tcl)
        || dialect.tcl_version.is_some()
        || dialect.core_point.is_some()
        || dialect.authored_name_policy()?.authority()
            != tcl_syntax::naming::NamePolicyAuthority::AuthoredSimulation
    {
        return None;
    }
    let literal = text.trim_matches(|character: char| character.is_ascii_whitespace());
    if !matches!(literal, "0" | "1") {
        return None;
    }
    let context = dialect.expression_parse_context(None);
    let original =
        tcl_syntax::expr::substitution::OriginalSourceExpression::capture(text, &context, config)?;
    let start = text.len().checked_sub(
        text.trim_start_matches(|c: char| c.is_ascii_whitespace())
            .len(),
    )?;
    let start = u32::try_from(start).ok()?;
    let expression = ExprNode::Literal {
        text: literal.to_owned(),
        start,
        end: start,
    };
    original.matches_tree(&expression).then_some(expression)
}

impl Builder<'_> {
    pub(super) fn events_before(&mut self, events: Vec<Event>, next: usize, exits: Exits) -> usize {
        let mut next = next;
        for event in events.into_iter().rev() {
            let id = self.placeholder();
            match event {
                Event::AuthoredRead(name, span) => {
                    self.nodes[id].authored_reads.push((name, span));
                    self.nodes[id].successors = vec![next];
                }
                Event::Read(name, span) => {
                    self.nodes[id].reads.push((name, span));
                    self.nodes[id].successors = vec![next];
                }
                Event::Script(source) => {
                    if self.purpose == DeclarationFlowPurpose::Representation {
                        self.nodes[id]
                            .representation_events
                            .push(representation::Event::ClearResult);
                    }
                    self.jobs.push(Job {
                        source,
                        representation_body: None,
                        purpose: tcl_lexer::SourceSegmentationPurpose::NestedScript,
                        entry: id,
                        exits: Exits {
                            normal: next,
                            ..exits
                        },
                    });
                }
                Event::ConditionalScript(source, context) => {
                    self.jobs.push(Job {
                        source,
                        representation_body: Some(context),
                        purpose: tcl_lexer::SourceSegmentationPurpose::NestedScript,
                        entry: id,
                        exits: Exits {
                            normal: next,
                            ..exits
                        },
                    });
                }
                Event::Representation(event) => {
                    self.nodes[id].representation_events.push(*event);
                    self.nodes[id].successors = vec![next];
                }
                Event::ReadUnavailable => {
                    self.unknown(id, next);
                    self.nodes[id].declared_read_unavailable = true;
                    self.nodes[id].availability.unavailable_declared_prefix = false;
                }
                Event::ConditionalReadUnavailable => {
                    self.nodes[id].unknown = true;
                    self.nodes[id].successors = vec![next];
                    self.nodes[id].declared_read_unavailable = true;
                    self.nodes[id].availability.unavailable_declared_prefix = false;
                }
                Event::Unknown => self.unknown(id, next),
            }
            next = id;
        }
        next
    }

    pub(super) fn condition(
        &mut self,
        site: &CommandAllocationSite,
        effective: &crate::registry_invocation::EffectiveCommandWords,
        argument: usize,
        targets: ConditionTargets,
        exits: Exits,
    ) -> usize {
        let ConditionTargets {
            yes,
            no,
            first_iteration,
        } = targets;
        let branch = self.placeholder();
        let (events, value, expression) = self.expression_events(site, effective, argument);
        self.nodes[branch].condition = expression.map(|expression| DeclaredCondition {
            expression,
            yes,
            no,
            first_iteration,
        });
        self.nodes[branch].successors = match value {
            Some(true) => vec![yes],
            Some(false) => vec![no],
            None => vec![yes, no, exits.error],
        };
        self.events_before(events, branch, exits)
    }

    pub(super) fn expressions(
        &mut self,
        site: &CommandAllocationSite,
        effective: &crate::registry_invocation::EffectiveCommandWords,
        arguments: &[usize],
        next: usize,
        exits: Exits,
    ) -> usize {
        let mut next = next;
        for &argument in arguments.iter().rev() {
            let (events, _, _) = self.expression_events(site, effective, argument);
            next = self.events_before(events, next, exits);
        }
        next
    }

    fn expression_events(
        &self,
        site: &CommandAllocationSite,
        effective: &crate::registry_invocation::EffectiveCommandWords,
        argument: usize,
    ) -> (
        Vec<Event>,
        Option<bool>,
        Option<tcl_syntax::expr::ast::ExprNode>,
    ) {
        use tcl_syntax::expr::ast::{BinOp, ExprNode};
        use tcl_syntax::expr::parser::{CheckedExprParse, parse_expr_checked_with_context};
        let source = self.body_source(
            site,
            effective,
            BodyOperand {
                argument,
                list_element: None,
            },
        );
        let Some(source) = trace_rbs_required(source, site.offset, "condition.source") else {
            return (vec![Event::Unknown], None, None);
        };
        let dialect = self.entry_dialect();
        let Ok(text) = source.try_text() else {
            trace_rbs_refusal(site.offset, "condition.text");
            return (vec![Event::Unknown], None, None);
        };
        let parse_context = dialect.expression_parse_context(None);
        let expression = match parse_expr_checked_with_context(text, &parse_context) {
            CheckedExprParse::Parsed(expression) => Some(expression),
            CheckedExprParse::Unsupported(
                tcl_syntax::expr::parser::ExprParseUnsupported::UnknownNativeSyntax,
            ) => logical_boolean_literal_condition(text, dialect, self.config),
            _ => None,
        };
        let Some(expression) = expression else {
            trace_rbs_refusal(site.offset, "condition.parse");
            return (vec![Event::Unknown], None, None);
        };
        let value = if let ExprNode::Literal { text, .. } = &expression {
            tcl_syntax::boolean::truthiness_with(text, dialect.numbers)
        } else {
            None
        };
        let mut pending = vec![&expression];
        let mut events = Vec::new();
        let mut scalar_condition = true;
        while let Some(node) = pending.pop() {
            match node {
                ExprNode::Literal { .. } => {}
                ExprNode::Var { text, start, .. } => {
                    let Some(scalar) =
                        self.expression_variable_events(site, &source, text, *start, &mut events)
                    else {
                        return (vec![Event::Unknown], None, None);
                    };
                    scalar_condition &= scalar;
                }
                ExprNode::Unary { operand, .. } => pending.push(operand),
                ExprNode::Binary { op, left, right }
                    if !matches!(op, BinOp::And | BinOp::Or | BinOp::WordAnd | BinOp::WordOr) =>
                {
                    pending.extend([right.as_ref(), left.as_ref()]);
                }
                // Lazy operands and dispatched math/script/string substitutions
                // need their independently retained evaluation topology.
                _ => return (vec![Event::Unknown], None, None),
            }
        }
        (events, value, scalar_condition.then_some(expression))
    }

    fn expression_variable_events(
        &self,
        site: &CommandAllocationSite,
        source: &ExecutedScriptSource,
        text: &str,
        start: u32,
        events: &mut Vec<Event>,
    ) -> Option<bool> {
        let reference =
            tcl_lexer::word_parts::whole_var_ref(text.as_bytes(), self.config).ok()??;
        let start = source.base().checked_add(start)?;
        let span = reference.source_span(text.as_bytes(), 0, start)?;
        let name = std::str::from_utf8(reference.name).ok()?;
        if reference.index.is_some() {
            let end = start.checked_add(u32::try_from(text.len()).ok()?)?;
            let first_event = events.len();
            self.variable(text, tcl_lexer::Span::new(start, end), site, events);
            // The index keeps its original read/script evaluation order, but
            // this expression projection cannot certify the final array read.
            for event in &mut events[first_event..] {
                if matches!(event, Event::Read(_, read_span) if *read_span == span) {
                    *event = Event::ReadUnavailable;
                }
            }
            return Some(false);
        }
        if !tcl_syntax::naming::is_qualified(name.as_bytes())
            && self.entry.owns_read(&site.source, span)
        {
            events.push(Event::AuthoredRead(name.to_owned(), span));
        }
        if !self.retained_local_read(site, span, text, name, true) {
            events.push(Event::ReadUnavailable);
        } else if !tcl_syntax::naming::is_qualified(name.as_bytes()) {
            events.push(Event::Read(name.to_owned(), span));
        }
        Some(true)
    }

    pub(super) fn sequence(
        &mut self,
        site: &CommandAllocationSite,
        effective: &crate::registry_invocation::EffectiveCommandWords,
        arguments: &[usize],
        exits: Exits,
    ) -> usize {
        let mut next = exits.normal;
        for argument in arguments.iter().rev() {
            next = self.body(
                site,
                effective,
                BodyOperand {
                    argument: *argument,
                    list_element: None,
                },
                Exits {
                    normal: next,
                    ..exits
                },
            );
        }
        next
    }

    pub(super) fn body(
        &mut self,
        site: &CommandAllocationSite,
        effective: &crate::registry_invocation::EffectiveCommandWords,
        operand: BodyOperand,
        exits: Exits,
    ) -> usize {
        let id = self.placeholder();
        let source = self.body_source(site, effective, operand);
        if let Some(source) = source {
            self.jobs.push(Job {
                source,
                representation_body: None,
                purpose: tcl_lexer::SourceSegmentationPurpose::NestedScript,
                entry: id,
                exits,
            });
        } else {
            self.unknown(id, exits.normal);
        }
        id
    }

    pub(super) fn body_source(
        &self,
        site: &CommandAllocationSite,
        effective: &crate::registry_invocation::EffectiveCommandWords,
        operand: BodyOperand,
    ) -> Option<ExecutedScriptSource> {
        let written = effective.written_argument(operand.argument)?;
        let word = effective.words.get(operand.argument + 1)?;
        let dialect = self.entry_dialect();
        let crate::registry_invocation::EffectiveInvocationWord::Literal(value) =
            crate::registry_invocation::effective_invocation_word(
                word,
                self.config.escapes,
                dialect.word_values,
            )
        else {
            return None;
        };
        let mut source =
            ExecutedScriptSource::from_word(site.clone(), written, word, &value, self.config);
        if let Some(element) = operand.list_element {
            source = source.list_element(site.clone(), written, element, dialect.word_values)?;
        }
        matches!(source.mapping, ExecutedScriptMapping::Contiguous { .. }).then_some(source)
    }

    pub(super) fn quoted_mentions(
        &self,
        site: &CommandAllocationSite,
        effective: &crate::registry_invocation::EffectiveCommandWords,
        operand: BodyOperand,
    ) -> Result<BTreeSet<String>, ()> {
        use tcl_lexer::word_parts::{ExecutablePart, ExecutablePartArena, SubstFlags};
        if !matches!(
            effective.words.get(operand.argument + 1),
            Some(WordExpr::BracedLiteral { .. })
        ) {
            return Ok(BTreeSet::new());
        }
        let source = self.body_source(site, effective, operand).ok_or(())?;
        let end = source
            .base()
            .checked_add(u32::try_from(source.text.len()).map_err(|_| ())?)
            .ok_or(())?;
        let mut regions = vec![tcl_lexer::Span::new(source.base(), end)];
        let mut mentions = BTreeSet::new();
        while let Some(region) = regions.pop() {
            let arena = ExecutablePartArena::decompose(
                site.source.source_image().clone(),
                region,
                SubstFlags::default(),
                self.config,
            )
            .map_err(|_| ())?;
            let mut lists = vec![arena.root()];
            while let Some(list) = lists.pop() {
                for part in arena.list(list) {
                    match &part.part {
                        ExecutablePart::Variable { name, index } => {
                            let name = std::str::from_utf8(arena.bytes(*name).ok_or(())?)
                                .map_err(|_| ())?;
                            if !tcl_syntax::naming::is_qualified(name.as_bytes()) {
                                mentions.insert(name.to_owned());
                            }
                            lists.extend(*index);
                        }
                        ExecutablePart::Command { body } => regions.push(*body),
                        ExecutablePart::Expression { expression } => regions.push(*expression),
                        ExecutablePart::Text(_) => {}
                        ExecutablePart::ParseError(_) => return Err(()),
                    }
                }
            }
        }
        Ok(mentions)
    }

    fn entry_dialect(&self) -> tcl_native_model::InvocationDialect {
        self.dialect
    }

    pub(super) fn word_events(
        &self,
        word: &WordExpr,
        site: &CommandAllocationSite,
        output: &mut Vec<Event>,
    ) {
        match word {
            WordExpr::Variable { spelling, source } => {
                self.variable(spelling, source.span, site, output);
            }
            WordExpr::CommandSubstitution { spelling, source } => {
                Self::substitution(spelling, source.span, site, output);
            }
            WordExpr::Template { parts, .. } => {
                for part in parts {
                    match part {
                        WordPart::Variable { spelling, source } => {
                            self.variable(spelling, source.span, site, output);
                        }
                        WordPart::CommandSubstitution { spelling, source } => {
                            Self::substitution(spelling, source.span, site, output);
                        }
                        WordPart::Opaque { .. } => output.push(Event::Unknown),
                        WordPart::Text { .. } => {}
                    }
                }
            }
            WordExpr::Expand { .. } | WordExpr::Opaque { .. } => output.push(Event::Unknown),
            _ => {}
        }
    }

    pub(super) fn variable(
        &self,
        spelling: &str,
        span: tcl_lexer::Span,
        site: &CommandAllocationSite,
        output: &mut Vec<Event>,
    ) {
        #[derive(Clone, Copy)]
        enum Task<'a> {
            Part(&'a tcl_lexer::word_parts::SpannedExecutablePart),
            Read(&'a tcl_lexer::word_parts::SpannedExecutablePart),
        }
        use tcl_lexer::word_parts::{ExecutablePart, ExecutablePartArena, SubstFlags};
        let Some(end) = u32::try_from(spelling.len())
            .ok()
            .and_then(|len| span.start().checked_add(len))
        else {
            output.push(Event::Unknown);
            return;
        };
        if site
            .source
            .source_image()
            .bytes()
            .get(span.start() as usize..end as usize)
            != Some(spelling.as_bytes())
        {
            output.push(Event::Unknown);
            return;
        }
        let Ok(arena) = ExecutablePartArena::decompose(
            site.source.source_image().clone(),
            tcl_lexer::Span::new(span.start(), end),
            SubstFlags::default(),
            self.config,
        ) else {
            output.push(Event::Unknown);
            return;
        };
        let mut tasks = arena
            .list(arena.root())
            .iter()
            .rev()
            .map(Task::Part)
            .collect::<Vec<_>>();
        while let Some(task) = tasks.pop() {
            let part = match task {
                Task::Part(part) | Task::Read(part) => part,
            };
            match task {
                Task::Part(_) => match &part.part {
                    ExecutablePart::Variable { index, .. } => {
                        tasks.push(Task::Read(part));
                        if let Some(index) = index {
                            tasks.extend(arena.list(*index).iter().rev().map(Task::Part));
                        }
                    }
                    ExecutablePart::Command { body } => {
                        if let Some(source) = Self::original_region(site, *body) {
                            output.push(Event::Script(source));
                        } else {
                            output.push(Event::Unknown);
                        }
                    }
                    ExecutablePart::Text(_) => {}
                    _ => output.push(Event::Unknown),
                },
                Task::Read(_) => {
                    let ExecutablePart::Variable { name, .. } = part.part else {
                        unreachable!()
                    };
                    let Some(name) = arena
                        .bytes(name)
                        .and_then(|bytes| std::str::from_utf8(bytes).ok())
                    else {
                        output.push(Event::Unknown);
                        continue;
                    };
                    let Some(read_span) = arena.source_span(part) else {
                        output.push(Event::Unknown);
                        continue;
                    };
                    let Some(original) = arena
                        .bytes(part.span)
                        .and_then(|bytes| std::str::from_utf8(bytes).ok())
                    else {
                        output.push(Event::Unknown);
                        continue;
                    };
                    if !tcl_syntax::naming::is_qualified(name.as_bytes())
                        && self.entry.owns_read(&site.source, read_span)
                    {
                        output.push(Event::AuthoredRead(name.to_owned(), read_span));
                    }
                    if self.retained_local_read(site, read_span, original, name, false) {
                        if !tcl_syntax::naming::is_qualified(name.as_bytes()) {
                            output.push(Event::Read(name.to_owned(), read_span));
                        }
                    } else {
                        output.push(Event::ReadUnavailable);
                    }
                }
            }
        }
    }

    fn retained_local_read(
        &self,
        site: &CommandAllocationSite,
        span: tcl_lexer::Span,
        spelling: &str,
        name: &str,
        expression: bool,
    ) -> bool {
        use super::super::declaration_layout::{
            direct_declaration_operand_owner, local_read_scope_is_excluded,
        };
        let mut found = false;
        for access in self
            .inventory
            .accesses
            .get(&span.start())
            .into_iter()
            .flatten()
            .chain(
                self.inventory
                    .origin_accesses
                    .get(&site.source)
                    .and_then(|accesses| accesses.get(&span.start()))
                    .into_iter()
                    .flatten(),
            )
        {
            if access.source.span != span || access.original_spelling != spelling {
                continue;
            }
            let owned = if expression {
                strict_expression_owner(&access.owner, site)
            } else {
                direct_declaration_operand_owner(&access.owner, site)
            };
            if !owned {
                return false;
            }
            let Some(observations) = self.inventory.layouts.get(site).and_then(|observations| {
                super::super::declaration_layout::original_declaration_layouts(observations)
            }) else {
                return false;
            };
            let contexts = access
                .context_alternatives()
                .iter()
                .filter(|context| self.entry.owns_original_context(context))
                .collect::<Vec<_>>();
            if contexts.is_empty() {
                continue;
            }
            if observations.clone().any(|observation| {
                !observation.entry.owns_source(&site.source, span.start())
                    || local_read_scope_is_excluded(
                        &observation.snapshot.state.source_variables,
                        name,
                    )
                    || contexts.iter().any(|context| {
                        local_read_scope_is_excluded(context, name)
                            || context.activation
                                != observation.snapshot.state.source_variables.activation
                    })
            }) {
                return false;
            }
            found = true;
        }
        found
    }

    fn original_region(
        site: &CommandAllocationSite,
        span: tcl_lexer::Span,
    ) -> Option<ExecutedScriptSource> {
        let image = tcl_lexer::SourceImage::from_bytes(
            site.source.source_image().bytes().get(span.as_range())?,
            site.source.source_image().channel(),
        );
        ExecutedScriptSource::contiguous_image(Arc::clone(&site.source), image, span.start())
    }

    pub(super) fn substitution(
        spelling: &str,
        span: tcl_lexer::Span,
        site: &CommandAllocationSite,
        output: &mut Vec<Event>,
    ) {
        let bytes = spelling.as_bytes();
        if bytes.first() != Some(&b'[')
            || bytes.last() != Some(&b']')
            || site.source.source_image().bytes().get(span.as_range()) != Some(bytes)
        {
            output.push(Event::Unknown);
            return;
        }
        let image = tcl_lexer::SourceImage::from_bytes(
            &bytes[1..bytes.len() - 1],
            site.source.source_image().channel(),
        );
        if let Some(source) = ExecutedScriptSource::contiguous_image(
            Arc::clone(&site.source),
            image,
            span.start() + 1,
        ) {
            output.push(Event::Script(source));
        } else {
            output.push(Event::Unknown);
        }
    }
}

pub(super) enum Event {
    AuthoredRead(String, tcl_lexer::Span),
    Read(String, tcl_lexer::Span),
    Script(ExecutedScriptSource),
    ConditionalScript(
        ExecutedScriptSource,
        Arc<ConditionalRepresentationBodyContext>,
    ),
    Representation(Box<representation::Event>),
    ConditionalReadUnavailable,
    ReadUnavailable,
    Unknown,
}

fn strict_expression_owner(
    owner: &super::super::SourceVariableEvaluationOwner,
    site: &CommandAllocationSite,
) -> bool {
    let mut pending = vec![owner];
    while let Some(owner) = pending.pop() {
        match owner {
            super::super::SourceVariableEvaluationOwner::NativeExpression {
                invocation, ..
            } if invocation == site => {}
            super::super::SourceVariableEvaluationOwner::Alternatives(owners)
                if !owners.is_empty() =>
            {
                pending.extend(owners);
            }
            _ => return false,
        }
    }
    true
}
