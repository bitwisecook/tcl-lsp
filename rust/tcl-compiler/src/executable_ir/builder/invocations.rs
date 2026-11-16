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

//! Emitting a generic invocation or an evaluated region: planning argv words, staging their evaluation and retaining the original word sources.

use super::super::{
    ArgvEntry, CompletionId, EvaluatedRegionCompletion, ExecutableArgvId, ExecutableBlockId,
    ExecutableInstruction, ExecutableTerminator, ExecutableValueId, GenericInvoke,
    OriginalExecutableArgvSource, OriginalExecutableWordSource, SourceCompatibilityDecline,
};
use super::{
    ControlContext, FunctionBuilder, IdAllocator, RegionEmissionSite, StatementTail, child_path,
};
use crate::ir::{NodeId, SourceSite, Statement, WordExpr};
use crate::registry_invocation::{
    InvocationMetadataContext, RegistryInvocationDecline,
    RegistryInvocationResolution as InvocationResolution,
    resolve_command_tokens_with_metadata_context,
};
use tcl_registry::CommandRegistry;

impl FunctionBuilder {
    fn retain_original_word_sources(
        &mut self,
        registry: &CommandRegistry,
        context: Option<InvocationMetadataContext<'_>>,
        tokens: &crate::ir::CommandTokens,
        stages: &[Stage],
    ) {
        let owner = (|| {
            let binding = tokens.source_binding.as_ref()?;
            if let Some(context) = context {
                context.matches_registry(registry).then_some(())?;
                if let Some(input) = context.source_analysis_input() {
                    binding.original_argument_metadata_for_input(tokens, input, registry)?;
                }
            }
            let config = binding.original_argument_lexer_config(tokens)?;
            let image = binding.original_argument_source_image(tokens)?;
            let words = crate::registry_invocation::original_native_compiler_words(
                image,
                tokens.words(),
                tokens.argv.first()?.start(),
                config,
            )?;
            Some(std::sync::Arc::new(OriginalExecutableArgvSource {
                tokens: tokens.clone(),
                words,
                metadata: match context {
                    // Only the legacy no-context entry or actual static adapter
                    // selects standalone compatibility. A borrowed availability
                    // view cannot reconstruct its original Arc/source owner.
                    None => crate::registry_invocation::OwnedInvocationMetadataContext::Standalone,
                    Some(context) if context.is_standalone() => {
                        crate::registry_invocation::OwnedInvocationMetadataContext::Standalone
                    }
                    Some(context) => {
                        crate::registry_invocation::OwnedInvocationMetadataContext::for_source_input(
                            context.source_analysis_input(),
                        )
                    }
                },
            }))
        })();
        // Each original argv word emits exactly one Evaluate stage; an
        // expanded word's separate Expand stage has no source evaluation.
        for (ordinal, (value, word)) in stages
            .iter()
            .filter_map(|stage| {
                if let Stage::Evaluate { value, word, .. } = stage {
                    Some((*value, word))
                } else {
                    None
                }
            })
            .enumerate()
        {
            let retained = owner.as_ref().map(|owner| OriginalExecutableWordSource {
                owner: std::sync::Arc::clone(owner),
                ordinal,
                evaluated: word.clone(),
            });
            self.original_word_sources.insert(value, retained);
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn emit_call(
        &mut self,
        registry: &CommandRegistry,
        context: Option<InvocationMetadataContext<'_>>,
        statement: &Statement,
        node: &NodeId,
        block: ExecutableBlockId,
        control: ControlContext,
        source: SourceSite,
        tail: bool,
    ) -> Result<StatementTail, SourceCompatibilityDecline> {
        let statement_index = node.path().last().copied().unwrap_or(0) as usize;
        let source_call = source_call(statement, statement_index)?;
        let words = exact_words(
            source_call.tokens,
            statement_index,
            source_call.command,
            source_call.args,
        )?;
        let resolution =
            resolve_invocation_facts(registry, context, source_call.tokens, statement_index)?;
        let mut stages = Vec::new();
        let entries = plan_argv_entries(&words, node, &mut self.allocator, &mut stages);
        self.retain_original_word_sources(registry, context, source_call.tokens, &stages);
        let argv = self.allocator.argv();
        stages.push(Stage::BuildArgv {
            argv,
            completion: self.allocator.completion(),
            entries,
        });
        stages.push(Stage::Invoke {
            argv,
            completion: self.allocator.completion(),
            resolution,
            original_words: words,
            source_binding: source_call.tokens.source_binding.clone().map(Box::new),
            effective_words: crate::registry_invocation::effective_command_words(
                source_call.tokens,
            ),
            node: node.clone(),
            source,
        });
        let stage_count = stages.len();
        let mut current = block;
        let mut outcome = None;
        for (index, stage) in stages.into_iter().enumerate() {
            let completion = stage.completion();
            for instruction in stage.into_instructions() {
                self.push(current, instruction);
            }
            let is_last = index + 1 == stage_count;
            match self.finish(current, completion, control, tail && is_last) {
                StatementTail::Returns => return Ok(StatementTail::Returns),
                StatementTail::Falls { block, completion } => {
                    current = block;
                    outcome = Some(StatementTail::Falls { block, completion });
                }
            }
        }
        Ok(outcome.expect("a call always plans at least one stage"))
    }

    pub(super) fn emit_evaluated_region(
        &mut self,
        registry: &CommandRegistry,
        site: &RegionEmissionSite<'_>,
        block: ExecutableBlockId,
        region: &crate::execution_region::EvaluatedBodyRegion,
    ) -> Result<StatementTail, SourceCompatibilityDecline> {
        use crate::execution_region::RegionSelection;
        let statement_index = site.node.path().last().copied().unwrap_or(0) as usize;
        if region.scope.is_some() || region.possible_bodies.is_some() {
            return Err(SourceCompatibilityDecline::UnsupportedStatement {
                statement_index,
                kind: "dictionary scope effect projection",
            });
        }
        let call = source_call(site.statement, statement_index)?;
        let words = exact_words(call.tokens, statement_index, call.command, call.args)?;
        let resolution =
            resolve_invocation_facts(registry, site.context, call.tokens, statement_index)?;
        let mut stages = Vec::new();
        let entries = plan_argv_entries(&words, site.node, &mut self.allocator, &mut stages);
        self.retain_original_word_sources(registry, site.context, call.tokens, &stages);
        let argv = self.allocator.argv();
        stages.push(Stage::BuildArgv {
            argv,
            completion: self.allocator.completion(),
            entries,
        });
        let mut current = block;
        for stage in stages {
            let completion = stage.completion();
            for instruction in stage.into_instructions() {
                self.push(current, instruction);
            }
            current = self.dispatch(current, completion, site.control);
        }
        let phase_entries: Vec<ExecutableBlockId> =
            region.phases.iter().map(|_| self.new_block()).collect();
        let join = self.new_block();
        match region.selection {
            RegionSelection::Always => {
                self.terminate(current, ExecutableTerminator::Goto(phase_entries[0]));
            }
            RegionSelection::Never => self.terminate(current, ExecutableTerminator::Goto(join)),
            RegionSelection::MaySkip => self.terminate(
                current,
                ExecutableTerminator::RegionChoice {
                    enter: phase_entries[0],
                    skip: join,
                },
            ),
        }
        self.emit_evaluated_phases(registry, site, region, &phase_entries, join)?;
        let completion = self.allocator.completion();
        self.push(
            join,
            ExecutableInstruction::CompleteEvaluatedRegion(EvaluatedRegionCompletion {
                invocation: GenericInvoke {
                    completion,
                    argv,
                    resolution,
                    original_words: words,
                    source_binding: call.tokens.source_binding.clone(),
                    effective_words: crate::registry_invocation::effective_command_words(
                        call.tokens,
                    ),
                    node: site.node.clone(),
                    source: region.source.clone(),
                },
                region: region.clone(),
                entry: phase_entries[0],
            }),
        );
        if region.repetition == crate::execution_region::RegionRepetition::MayRepeat
            && region.selection != RegionSelection::Never
        {
            let next = self.dispatch(join, completion, site.control);
            let done = self.new_block();
            self.terminate(
                next,
                ExecutableTerminator::RegionChoice {
                    enter: phase_entries[0],
                    skip: done,
                },
            );
            Ok(self.finish(done, completion, site.control, site.tail))
        } else {
            Ok(self.finish(join, completion, site.control, site.tail))
        }
    }

    fn emit_evaluated_phases(
        &mut self,
        registry: &CommandRegistry,
        site: &RegionEmissionSite<'_>,
        region: &crate::execution_region::EvaluatedBodyRegion,
        phase_entries: &[ExecutableBlockId],
        join: ExecutableBlockId,
    ) -> Result<(), SourceCompatibilityDecline> {
        use crate::execution_region::RegionTarget;
        for (index, phase) in region.phases.iter().enumerate() {
            let target = |route| match route {
                RegionTarget::Phase(next) => Some(phase_entries[next]),
                RegionTarget::Exit => Some(join),
                RegionTarget::Propagate => None,
            };
            let handler = target(phase.abrupt).map(|_| self.new_block());
            let phase_control = handler.map_or(site.control, ControlContext::caught_body);
            let path = child_path(
                site.node,
                u32::try_from(index).expect("phase count fits source node") + 1,
            );
            let flow = self.emit_nested(
                registry,
                site.context,
                &phase.script,
                &path,
                phase_entries[index],
                phase_control,
            )?;
            self.terminate(
                flow,
                ExecutableTerminator::Goto(target(phase.normal).unwrap_or(join)),
            );
            if let (Some(handler), Some(destination)) = (handler, target(phase.abrupt)) {
                let caught = self.allocator.completion();
                self.push(
                    handler,
                    ExecutableInstruction::JoinCompletion {
                        completion: caught,
                        node: site.node.clone(),
                        source: region.source.clone(),
                    },
                );
                self.terminate(handler, ExecutableTerminator::Goto(destination));
            }
        }
        Ok(())
    }
}

fn plan_argv_entries(
    words: &[WordExpr],
    node: &NodeId,
    allocator: &mut IdAllocator,
    stages: &mut Vec<Stage>,
) -> Vec<ArgvEntry> {
    let mut entries = Vec::with_capacity(words.len());
    for word in words {
        entries.push(plan_word_entry(word, node, allocator, stages));
    }
    entries
}

fn plan_word_entry(
    word: &WordExpr,
    node: &NodeId,
    allocator: &mut IdAllocator,
    stages: &mut Vec<Stage>,
) -> ArgvEntry {
    if let WordExpr::Expand {
        word: inner,
        source: expansion_source,
    } = word
    {
        let input = allocator.value();
        stages.push(Stage::evaluate(
            input,
            allocator.completion(),
            inner.as_ref().clone(),
            node.clone(),
            inner.source().clone(),
        ));
        let value = allocator.value();
        stages.push(Stage::expand(
            value,
            allocator.completion(),
            input,
            word.clone(),
            node.clone(),
            expansion_source.clone(),
        ));
        ArgvEntry::Expanded(value)
    } else {
        let value = allocator.value();
        stages.push(Stage::evaluate(
            value,
            allocator.completion(),
            word.clone(),
            node.clone(),
            word.source().clone(),
        ));
        ArgvEntry::Value(value)
    }
}

struct SourceCall<'a> {
    command: &'a str,
    args: &'a [String],
    pub(in super::super) tokens: &'a crate::ir::CommandTokens,
}

fn source_call(
    statement: &Statement,
    statement_index: usize,
) -> Result<SourceCall<'_>, SourceCompatibilityDecline> {
    match statement {
        Statement::Call {
            command,
            args,
            tokens: Some(tokens),
            ..
        }
        | Statement::Barrier {
            command,
            args,
            tokens: Some(tokens),
            ..
        } => Ok(SourceCall {
            command,
            args,
            tokens,
        }),
        Statement::Call { .. } | Statement::Barrier { .. } | Statement::NativeCall { .. } => {
            Err(SourceCompatibilityDecline::MissingCommandTokens { statement_index })
        }
        other => Err(SourceCompatibilityDecline::UnsupportedStatement {
            statement_index,
            kind: statement_kind_name(other),
        }),
    }
}

fn exact_words(
    tokens: &crate::ir::CommandTokens,
    statement_index: usize,
    command: &str,
    args: &[String],
) -> Result<Vec<WordExpr>, SourceCompatibilityDecline> {
    if tokens.words().is_empty() {
        return Err(SourceCompatibilityDecline::MissingCommandHead { statement_index });
    }
    if !tokens.words_align_with_argv_text()
        || tokens.argv_texts.len() != args.len().saturating_add(1)
        || tokens.argv_texts.first().map(String::as_str) != Some(command)
        || tokens.argv_texts.get(1..) != Some(args)
    {
        return Err(SourceCompatibilityDecline::InconsistentCommandTokens { statement_index });
    }
    Ok(tokens.words().to_vec())
}

/// Resolve source-aware word facts without re-evaluating, flattening, or
/// re-parsing any Tcl word. The small borrowed vector exists only for this
/// registry call; `InvocationFacts`/the unresolved projection own every fact
/// retained in executable IR.
fn resolve_invocation_facts(
    registry: &CommandRegistry,
    context: Option<InvocationMetadataContext<'_>>,
    tokens: &crate::ir::CommandTokens,
    statement_index: usize,
) -> Result<InvocationResolution, SourceCompatibilityDecline> {
    match resolve_command_tokens_with_metadata_context(registry, context, tokens) {
        Ok(resolution) => Ok(resolution),
        Err(RegistryInvocationDecline::MissingCommandHead) => {
            Err(SourceCompatibilityDecline::MissingCommandHead { statement_index })
        }
        Err(RegistryInvocationDecline::IncompleteResolution) => {
            Err(SourceCompatibilityDecline::IncompleteRegistryResolution { statement_index })
        }
    }
}

fn statement_kind_name(statement: &Statement) -> &'static str {
    match statement {
        Statement::AssignConst { .. } => "AssignConst",
        Statement::AssignExpr { .. } => "AssignExpr",
        Statement::AssignValue { .. } => "AssignValue",
        Statement::Incr { .. } => "Incr",
        Statement::ExprEval { .. } => "ExprEval",
        Statement::Call { .. } => "Call",
        Statement::Return { .. } => "Return",
        Statement::Barrier { .. } => "Barrier",
        Statement::NativeCall { .. } => "NativeCall",
        Statement::Block { .. } => "Block",
        Statement::UpFrame { .. } => "UpFrame",
        Statement::If { .. } => "If",
        Statement::For { .. } => "For",
        Statement::While { .. } => "While",
        Statement::Foreach { .. } => "Foreach",
        Statement::Catch { .. } => "Catch",
        Statement::Try { .. } => "Try",
        Statement::Switch { .. } => "Switch",
    }
}

enum Stage {
    Evaluate {
        value: ExecutableValueId,
        completion: CompletionId,
        word: WordExpr,
        node: NodeId,
        source: SourceSite,
    },
    Expand {
        value: ExecutableValueId,
        completion: CompletionId,
        input: ExecutableValueId,
        original: WordExpr,
        node: NodeId,
        source: SourceSite,
    },
    BuildArgv {
        argv: ExecutableArgvId,
        completion: CompletionId,
        entries: Vec<ArgvEntry>,
    },
    Invoke {
        argv: ExecutableArgvId,
        completion: CompletionId,
        resolution: InvocationResolution,
        original_words: Vec<WordExpr>,
        source_binding: Option<Box<crate::command_binding::SourceInvocationBinding>>,
        effective_words: Option<crate::registry_invocation::EffectiveCommandWords>,
        node: NodeId,
        source: SourceSite,
    },
}

impl Stage {
    fn evaluate(
        value: ExecutableValueId,
        completion: CompletionId,
        word: WordExpr,
        node: NodeId,
        source: SourceSite,
    ) -> Self {
        Self::Evaluate {
            value,
            completion,
            word,
            node,
            source,
        }
    }

    fn expand(
        value: ExecutableValueId,
        completion: CompletionId,
        input: ExecutableValueId,
        original: WordExpr,
        node: NodeId,
        source: SourceSite,
    ) -> Self {
        Self::Expand {
            value,
            completion,
            input,
            original,
            node,
            source,
        }
    }

    pub(in super::super) const fn completion(&self) -> CompletionId {
        match self {
            Self::Evaluate { completion, .. }
            | Self::Expand { completion, .. }
            | Self::BuildArgv { completion, .. }
            | Self::Invoke { completion, .. } => *completion,
        }
    }

    fn into_instructions(self) -> Vec<ExecutableInstruction> {
        match self {
            Self::Evaluate {
                value,
                completion,
                word,
                node,
                source,
            } => vec![ExecutableInstruction::EvaluateWord {
                value,
                completion,
                word,
                node,
                source,
            }],
            Self::Expand {
                value,
                completion,
                input,
                original,
                node,
                source,
            } => vec![ExecutableInstruction::ExpandWord {
                value,
                completion,
                input,
                original,
                node,
                source,
            }],
            Self::BuildArgv {
                argv,
                completion,
                entries,
            } => vec![ExecutableInstruction::BuildArgv {
                argv,
                completion,
                entries,
            }],
            Self::Invoke {
                argv,
                completion,
                resolution,
                original_words,
                source_binding,
                effective_words,
                node,
                source,
            } => vec![ExecutableInstruction::Invoke(GenericInvoke {
                completion,
                argv,
                resolution,
                original_words,
                source_binding: source_binding.map(|binding| *binding),
                effective_words,
                node,
                source,
            })],
        }
    }
}
