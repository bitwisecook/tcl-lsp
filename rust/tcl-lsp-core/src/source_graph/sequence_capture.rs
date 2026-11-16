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

//! Capturing one document's straight-line sequence: which commands run now, defer their scripts or may exit early.

use super::{DocumentSequence, SequenceCommand, SequenceRegion};

#[derive(Debug, Clone, Copy)]
pub(super) enum ScriptScheduling {
    None,
    Deferred,
    Immediate { once: bool },
}

impl ScriptScheduling {
    fn runs_now(self) -> bool {
        matches!(self, Self::Immediate { .. })
    }
    fn defers(self) -> bool {
        matches!(self, Self::Deferred)
    }
    fn enters_once(self) -> bool {
        matches!(self, Self::Immediate { once: true })
    }
}

/// Registry control facts of one command, for the sequence.
#[derive(Debug, Clone, Copy)]
pub(super) struct CommandFlow {
    /// Traits end the block or the process (`return`, `error`, `exit`).
    terminates: bool,
    /// Traits transfer control elsewhere (`break`, `continue`, `tailcall`),
    /// or the command evaluates another file (`source`), whose own `return`
    /// ends this document's normal completion.
    may_end_load: bool,
    /// Mutually exclusive immediate/deferred/no-script scheduling.
    scheduling: ScriptScheduling,
    /// The Registry may give it a command prefix to invoke (`lsort -command`,
    /// `regsub -command`), or cannot rule one out.
    invokes_prefix: bool,
}

/// One original Registry selection per command, shared by the sequence and
/// the site inventory.
#[derive(Debug, Clone)]
pub(super) enum RegistryWordsSlot {
    Pending,
    Resolved(
        Box<Option<tcl_compiler::registry_invocation::source_structure::OriginalRegistryWords>>,
    ),
}

impl RegistryWordsSlot {
    pub(super) fn get(
        &mut self,
        resolve: impl FnOnce() -> Option<
            tcl_compiler::registry_invocation::source_structure::OriginalRegistryWords,
        >,
    ) -> Option<&tcl_compiler::registry_invocation::source_structure::OriginalRegistryWords> {
        if matches!(self, Self::Pending) {
            *self = Self::Resolved(Box::new(resolve()));
        }
        match self {
            Self::Resolved(words) => (**words).as_ref(),
            Self::Pending => None,
        }
    }

    pub(super) fn take(
        &mut self,
        resolve: impl FnOnce() -> Option<
            tcl_compiler::registry_invocation::source_structure::OriginalRegistryWords,
        >,
    ) -> Option<tcl_compiler::registry_invocation::source_structure::OriginalRegistryWords> {
        match std::mem::replace(self, Self::Pending) {
            Self::Resolved(words) => *words,
            Self::Pending => resolve(),
        }
    }
}

/// Inputs shared by every step of [`DocumentSequence::capture`].
struct SequenceCapture<'a> {
    analysis: &'a tcl_compiler::analyser::AnalysisResult,
    pub(super) input: &'a tcl_compiler::analyser::ResolvedAnalysisInput,
    pub(super) image: &'a tcl_lexer::SourceImage,
    structure: &'a crate::source_structure::SourceStructure,
    declared: rustc_hash::FxHashSet<&'a [u8]>,
}

impl SequenceCapture<'_> {
    fn validated<'w>(
        &self,
        words: Option<
            &'w tcl_compiler::registry_invocation::source_structure::OriginalRegistryWords,
        >,
    ) -> Option<&'w tcl_compiler::registry_invocation::source_structure::OriginalRegistryWords>
    {
        words.filter(|words| {
            words.matches_source(self.image, self.input.lexer_config())
                && words.matches_registry(self.input.borrowed_context_registry().commands())
                && words.context() == Some(self.input.availability_context())
        })
    }

    /// The command's Registry control facts, selected once per command.
    pub(super) fn flow(
        &self,
        words: &tcl_compiler::registry_invocation::source_structure::OriginalRegistryWords,
    ) -> CommandFlow {
        use tcl_registry::script_body_flow::{ScriptBodyFlow, script_body_flow_for_invocation};
        words
            .with_source_schema(self.input.borrowed_context_registry(), |schema| {
                let flow =
                    script_body_flow_for_invocation(&schema.facts(), schema.words.arguments());
                CommandFlow {
                    terminates: schema.semantics.traits.intersects(
                        tcl_registry::Traits::TERMINATES_BLOCK
                            | tcl_registry::Traits::TERMINATES_PROCESS,
                    ),
                    may_end_load: schema
                        .semantics
                        .traits
                        .contains(tcl_registry::Traits::TRANSFERS_CONTROL)
                        || schema.semantics.body_execution
                            == Some(tcl_registry::body_execution::BodyExecutionSpec::SourceFile),
                    scheduling: match flow {
                        ScriptBodyFlow::Deferred => ScriptScheduling::Deferred,
                        ScriptBodyFlow::None => ScriptScheduling::None,
                        _ => ScriptScheduling::Immediate {
                            once: schema.semantics.frame_effect.is_none_or(|effect| {
                                effect.level_word
                                    == tcl_registry::frame_effect::FrameLevelWord::None
                            }) && matches!(
                                flow,
                                ScriptBodyFlow::Sequence(_)
                                    | ScriptBodyFlow::ConcatenatedScript { .. }
                            ),
                        },
                    },
                    invokes_prefix: schema.authored_source_retains_callback() != Some(false),
                }
            })
            .unwrap_or(CommandFlow {
                terminates: false,
                may_end_load: true,
                scheduling: ScriptScheduling::Immediate { once: false },
                invokes_prefix: true,
            })
    }

    /// Operands whose role may carry a script, for `Selected` words with
    /// complete roles; the written token and the whole native word are both
    /// covered.
    pub(super) fn script_operands(
        words: &tcl_compiler::registry_invocation::source_structure::OriginalRegistryWords,
    ) -> Option<Vec<tcl_lexer::Span>> {
        let mut spans = Vec::new();
        for &(argument, role) in words.roles()? {
            if role.carries_script()
                || matches!(
                    role,
                    tcl_registry::ArgRole::OpaqueScript | tcl_registry::ArgRole::CommandPrefix
                )
            {
                let operand = words.operands().get(argument)?.as_ref()?;
                let written = operand.span();
                spans.push(operand.word().map_or(written, |word| {
                    let whole = word.span();
                    tcl_lexer::Span::new(
                        whole.start().min(written.start()),
                        whole.end().max(written.end()),
                    )
                }));
            }
        }
        Some(spans)
    }

    /// Bodies this command enters once, immediately and in the current frame.
    fn entered_bodies(
        &self,
        command: &tcl_compiler::segmenter::SegmentedCommand,
        words: &tcl_compiler::registry_invocation::source_structure::OriginalRegistryWords,
        flow: CommandFlow,
    ) -> Vec<tcl_lexer::Span> {
        let at = command.span.start();
        if !flow.scheduling.enters_once()
            || self
                .structure
                .caller_context_unavailable_bodies
                .iter()
                .any(|body| body.start() <= at && at < body.end())
        {
            return Vec::new();
        }
        words
            .source_script_bodies(self.input.borrowed_context_registry())
            .iter()
            .map(tcl_compiler::registry_invocation::OriginalSourceScriptBody::content_span)
            .filter(|content| {
                self.analysis
                    .original_definition_body_span(content.start())
                    .is_none()
            })
            .collect()
    }

    /// Whether a straight command may now run a script operand this inventory
    /// does not retain as a region, which could hold an early exit. Deferred
    /// bodies run later and are no exit of the document.
    fn may_run_unretained_script(
        words: &tcl_compiler::registry_invocation::source_structure::OriginalRegistryWords,
        flow: CommandFlow,
        regions: &[SequenceRegion],
    ) -> bool {
        if !flow.scheduling.runs_now() {
            return false;
        }
        let Some(roles) = words.roles() else {
            return words
                .written_argument_roles()
                .iter()
                .any(|(_, role)| *role == tcl_registry::ArgRole::Body);
        };
        roles
            .iter()
            .filter(|(_, role)| *role == tcl_registry::ArgRole::Body)
            .any(|&(argument, _)| {
                let Some(operand) = words.operands().get(argument).and_then(Option::as_ref) else {
                    return true;
                };
                !Self::retains(regions, operand.span())
            })
    }

    /// End-only: whether a command that does not defer its scripts may run a
    /// script now that the inventory cannot see — a script-carrying operand
    /// (`Body`, `Expr`, `LambdaLiteral`, `OpaqueScript`, `CommandPrefix`) not
    /// retained as a region, an expression calling a math function (a
    /// command), or any command prefix it invokes. Such a script may
    /// `return -code return` and end the document's normal completion.
    fn may_complete_early(
        &self,
        words: &tcl_compiler::registry_invocation::source_structure::OriginalRegistryWords,
        flow: CommandFlow,
        regions: &[SequenceRegion],
    ) -> bool {
        use tcl_registry::ArgRole;
        let carries = |role: ArgRole| {
            role.carries_script() || matches!(role, ArgRole::OpaqueScript | ArgRole::CommandPrefix)
        };
        if flow.scheduling.defers() {
            return false;
        }
        if flow.invokes_prefix {
            return true;
        }
        let Some(roles) = words.roles() else {
            return words
                .written_argument_roles()
                .iter()
                .any(|&(_, role)| carries(role));
        };
        roles
            .iter()
            .filter(|&&(_, role)| carries(role))
            .any(|&(argument, role)| {
                let Some(operand) = words.operands().get(argument).and_then(Option::as_ref) else {
                    return true;
                };
                if role == ArgRole::CommandPrefix {
                    return true;
                }
                let literal = operand
                    .word()
                    .and_then(tcl_syntax::word_rules::original_static_word_unicode_value);
                if role == ArgRole::Expr {
                    return self.expression_may_complete_early(literal.as_deref())
                        || (literal.as_ref().is_some_and(|value| value.contains(&b'['))
                            && !Self::retains(regions, operand.span()));
                }
                // An empty literal body runs nothing.
                if literal.is_some_and(|value| value.iter().all(u8::is_ascii_whitespace)) {
                    return false;
                }
                !Self::retains(regions, operand.span())
            })
    }

    /// Whether an expression may run a command the inventory cannot see. The
    /// shared expression owner parses it under the input's dialect; any math
    /// function call (`f(1)`, a user `::tcl::mathfunc` procedure or an
    /// undefined name reaching `unknown`) or a parse failure counts. A word
    /// that is not a literal is substituted twice and counts too. Command
    /// substitutions are judged by their retained regions.
    fn expression_may_complete_early(&self, literal: Option<&[u8]>) -> bool {
        let Some(text) = literal.and_then(|value| std::str::from_utf8(value).ok()) else {
            return true;
        };
        let tree = tcl_syntax::expr::parser::parse_expr_for_profile(
            text,
            Some(self.input.analyser_profile()),
        );
        tree.evaluation_nodes().iter().any(|node| {
            matches!(
                node,
                tcl_syntax::expr::ast::ExprNode::Call { .. }
                    | tcl_syntax::expr::ast::ExprNode::Raw { .. }
            )
        })
    }

    /// Whether a retained script region lies inside `span`.
    pub(super) fn retains(regions: &[SequenceRegion], span: tcl_lexer::Span) -> bool {
        let first = regions.partition_point(|region| region.span.start() < span.start());
        regions.get(first).is_some_and(|region| {
            region.span.start() < span.end() && region.span.end() <= span.end()
        })
    }

    pub(super) fn selected(
        words: &tcl_compiler::registry_invocation::source_structure::OriginalRegistryWords,
    ) -> bool {
        matches!(
            words.source(),
            tcl_compiler::registry_invocation::source_structure::OriginalRegistrySource::Selected
        )
    }

    /// Bodies a deferring command stores for a later invocation.
    fn deferred_bodies(
        &self,
        words: &tcl_compiler::registry_invocation::source_structure::OriginalRegistryWords,
    ) -> Vec<tcl_lexer::Span> {
        words
            .source_script_bodies(self.input.borrowed_context_registry())
            .iter()
            .map(tcl_compiler::registry_invocation::OriginalSourceScriptBody::content_span)
            .collect()
    }

    fn declares_head(&self, command: &tcl_compiler::segmenter::SegmentedCommand) -> bool {
        command.texts.first().is_some_and(|head| {
            self.declared
                .contains(tcl_syntax::naming::key_holder_and_tail(head).1.as_bytes())
        })
    }
}

impl DocumentSequence {
    pub(super) fn capture(
        source: &str,
        analysis: &tcl_compiler::analyser::AnalysisResult,
        input: &tcl_compiler::analyser::ResolvedAnalysisInput,
        image: &tcl_lexer::SourceImage,
        structure: &crate::source_structure::SourceStructure,
        slots: &mut [RegistryWordsSlot],
    ) -> Option<Self> {
        let end = u32::try_from(source.len()).ok()?;
        let capture = SequenceCapture {
            analysis,
            input,
            image,
            structure,
            declared: analysis
                .original_procedure_declarations()
                .map(|record| record.name().slot().simple.as_bytes())
                .chain(
                    analysis
                        .original_class_declarations()
                        .map(|record| record.name().slot().simple.as_bytes()),
                )
                .collect(),
        };
        let mut sequence = Self {
            end,
            cutoff: None,
            end_withheld: false,
            regions: Self::regions(structure, end),
            definition_bodies: Self::definition_bodies(analysis),
        };
        // Single start-ordered sweep: the innermost region of each command
        // is reached from the last region starting at or before it through
        // parent links, and the open straight commands form a stack.
        let mut next_region = 0;
        let mut current: Option<usize> = None;
        let mut open: Vec<tcl_lexer::Span> = Vec::new();
        for (index, command) in structure.commands.iter().enumerate() {
            let at = command.span.start();
            while sequence
                .regions
                .get(next_region)
                .is_some_and(|region| region.span.start() <= at)
            {
                current = Some(next_region);
                next_region += 1;
            }
            current = sequence.innermost_from(current, at);
            if Self::in_bodies(&sequence.definition_bodies, at) {
                continue;
            }
            while open.last().is_some_and(|span| span.end() <= at) {
                open.pop();
            }
            let straight = current.is_some_and(|region| sequence.regions[region].straight);
            if straight {
                open.push(command.span);
            }
            let words = capture.validated(slots[index].get(|| {
                tcl_compiler::registry_invocation::source_structure::source_registry_words(
                    source, analysis, command,
                )
            }));
            let flow = words.map(|words| capture.flow(words));
            let completes_early = words.zip(flow).is_some_and(|(words, flow)| {
                capture.may_complete_early(words, flow, &sequence.regions)
            });
            sequence.record_exits(
                completes_early,
                current,
                words,
                flow,
                open.last().copied(),
                at,
            );
            if let (Some(words), Some(flow)) = (words, flow)
                && flow.scheduling.defers()
            {
                sequence.mark(&capture.deferred_bodies(words), |region| {
                    region.deferred = true;
                });
            }
            if let Some(region) = current.filter(|_| straight) {
                sequence.record_straight(&capture, region, command, words, flow);
            }
        }
        Some(sequence)
    }

    pub(super) fn definition_bodies(
        analysis: &tcl_compiler::analyser::AnalysisResult,
    ) -> Vec<(tcl_lexer::Span, u32)> {
        let mut bodies = analysis
            .original_procedure_declarations()
            .map(|record| record.metadata().body_span)
            .chain(
                analysis
                    .original_class_declarations()
                    .map(|record| record.metadata().body_span),
            )
            .map(|span| (span, span.end()))
            .collect::<Vec<_>>();
        bodies.sort_by_key(|(span, _)| (span.start(), span.end()));
        let mut reach = 0;
        for (span, running) in &mut bodies {
            reach = reach.max(span.end());
            *running = reach;
        }
        bodies
    }

    /// The innermost region containing `at`, from a region starting at or
    /// before it, through parent links.
    fn innermost_from(&self, mut current: Option<usize>, at: u32) -> Option<usize> {
        while let Some(index) = current
            && !(self.regions[index].span.start() <= at && at < self.regions[index].span.end())
        {
            current = self.regions[index].parent;
        }
        current
    }

    fn in_deferred(&self, mut current: Option<usize>) -> bool {
        while let Some(index) = current {
            if self.regions[index].deferred {
                return true;
            }
            current = self.regions[index].parent;
        }
        false
    }

    /// Apply `update` to every retained region with exactly one of `spans`.
    fn mark(&mut self, spans: &[tcl_lexer::Span], update: impl Fn(&mut SequenceRegion)) {
        for content in spans {
            if let Ok(body) = self.regions.binary_search_by(|region| {
                (region.span.start(), std::cmp::Reverse(region.span.end()))
                    .cmp(&(content.start(), std::cmp::Reverse(content.end())))
            }) {
                update(&mut self.regions[body]);
            }
        }
    }

    /// Record the two kinds of early exit a command outside definition bodies
    /// may take. A known terminator (or an unretained script that runs now)
    /// cuts every later position and the end; anything else that may end the
    /// document's normal completion early — control transfer, a sourced
    /// file, a script-carrying operand that is not retained or a command
    /// prefix, words that are not selected Registry words (user procedures,
    /// dynamic heads) — withholds only the end, since a later position is
    /// reached only when control did pass.
    pub(super) fn record_exits(
        &mut self,
        completes_early: bool,
        current: Option<usize>,
        words: Option<&tcl_compiler::registry_invocation::source_structure::OriginalRegistryWords>,
        flow: Option<CommandFlow>,
        straight_ancestor: Option<tcl_lexer::Span>,
        at: u32,
    ) {
        if self.in_deferred(current) {
            return;
        }
        let straight = current.is_some_and(|region| self.regions[region].straight);
        let unretained = words.zip(flow).is_some_and(|(words, flow)| {
            SequenceCapture::may_run_unretained_script(words, flow, &self.regions)
        });
        if flow.is_some_and(|flow| flow.terminates) || (straight && unretained) {
            let exit = straight_ancestor.map_or(at, tcl_lexer::Span::end);
            self.cutoff = Some(self.cutoff.map_or(exit, |cutoff| cutoff.min(exit)));
        }
        self.end_withheld |= unretained
            || completes_early
            || flow.is_none_or(|flow| flow.may_end_load)
            || !words.is_some_and(SequenceCapture::selected);
    }

    /// Admit one direct command of a straight region and mark the bodies it
    /// enters once, immediately and in the current frame.
    fn record_straight(
        &mut self,
        capture: &SequenceCapture<'_>,
        region: usize,
        command: &tcl_compiler::segmenter::SegmentedCommand,
        words: Option<&tcl_compiler::registry_invocation::source_structure::OriginalRegistryWords>,
        flow: Option<CommandFlow>,
    ) {
        let selected = words
            .filter(|words| SequenceCapture::selected(words) && !capture.declares_head(command));
        let script_operands = selected.and_then(SequenceCapture::script_operands);
        if script_operands.is_some()
            && let Some(words) = selected
            && let Some(flow) = flow
        {
            self.mark(&capture.entered_bodies(command, words, flow), |region| {
                region.straight = true;
            });
        }
        self.regions[region].commands.push(SequenceCommand {
            span: command.span,
            head: command.argv.first().map(|word| word.span),
            script_operands,
        });
    }

    /// Retained script regions ordered by start, outermost first, with their
    /// nesting parents; only the document root starts straight.
    pub(super) fn regions(
        structure: &crate::source_structure::SourceStructure,
        end: u32,
    ) -> Vec<SequenceRegion> {
        let mut spans = structure
            .scripts
            .iter()
            .map(|(span, _)| *span)
            .collect::<Vec<_>>();
        spans.sort_by_key(|span| (span.start(), std::cmp::Reverse(span.end())));
        spans.dedup();
        let root = tcl_lexer::Span::new(0, end);
        let mut regions = Vec::with_capacity(spans.len());
        let mut stack: Vec<usize> = Vec::new();
        for span in spans {
            while let Some(&open) = stack.last() {
                let outer: &SequenceRegion = &regions[open];
                if outer.span.start() <= span.start() && span.end() <= outer.span.end() {
                    break;
                }
                stack.pop();
            }
            regions.push(SequenceRegion {
                span,
                parent: stack.last().copied(),
                straight: span == root,
                deferred: false,
                commands: Vec::new(),
            });
            stack.push(regions.len() - 1);
        }
        regions
    }

    fn in_bodies(bodies: &[(tcl_lexer::Span, u32)], at: u32) -> bool {
        let index = bodies.partition_point(|(span, _)| span.start() <= at);
        index > 0 && bodies[index - 1].1 > at
    }

    pub(super) fn contains(&self, at: u32) -> bool {
        if at >= self.end {
            return self.cutoff.is_none() && !self.end_withheld;
        }
        if self.cutoff.is_some_and(|cutoff| at >= cutoff) {
            return false;
        }
        if Self::in_bodies(&self.definition_bodies, at) {
            return false;
        }
        let mut current = self
            .regions
            .partition_point(|region| region.span.start() <= at)
            .checked_sub(1);
        while let Some(index) = current
            && !(self.regions[index].span.start() <= at && at < self.regions[index].span.end())
        {
            current = self.regions[index].parent;
        }
        let Some(region) = current.map(|index| &self.regions[index]) else {
            return false;
        };
        if !region.straight {
            return false;
        }
        let next = region
            .commands
            .partition_point(|command| command.span.start() <= at);
        let Some(command) = next.checked_sub(1).map(|index| &region.commands[index]) else {
            return true;
        };
        at >= command.span.end()
            || command
                .head
                .is_some_and(|head| head.start() <= at && at < head.end())
            || command.script_operands.as_ref().is_some_and(|operands| {
                !operands
                    .iter()
                    .any(|operand| operand.start() <= at && at < operand.end())
            })
    }
}
