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

//! The exact value a word has at its statement, over solved facts: statement words, `return` words and words of nested command substitutions.

use super::super::argument_words::{OperandSource, call_arguments};
use super::super::folded_type::FoldedType;
use super::super::lattice_inputs::{const_to_exact, exact_of_bytes, variable_name};
use super::super::{ArgWord, ProvenReadFacts};
use crate::analyses::LatticeValue;
use crate::ir::{CommandTokens, Statement};
use crate::ssa::{SsaStatement, Symbol, ValueKey, Version};
use std::collections::{HashMap, HashSet};
use tcl_lexer::LexerConfig;
use tcl_registry::CommandRegistry;
use tcl_registry::value_transfer::ExactValue;

/// One statement of a function unit: the block it sits in and its index
/// there, in the CFG block and the SSA block alike.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StatementId {
    /// The block.
    pub block: crate::cfg::BlockId,
    /// The statement's index in the block.
    pub index: usize,
}

impl<'a> ProvenReadFacts<'a> {
    pub(in super::super) fn execution(fu: &'a crate::compilation_unit::FunctionUnit) -> Self {
        Self {
            values: &fu.sccp.values,
            folded_types: Some(&fu.sccp.folded_types),
            reached: &fu.sccp.executable_blocks,
            folds: None,
        }
    }

    pub(crate) fn diagnostic(facts: crate::sccp::DiagnosticValueFacts<'a>) -> Self {
        Self {
            values: facts.values(),
            folded_types: None,
            reached: facts.executable_blocks(),
            folds: None,
        }
    }

    pub(crate) const fn with_folds(self, registry: &'a CommandRegistry) -> Self {
        Self {
            folds: Some(registry),
            ..self
        }
    }
}

/// The exact value a word of a statement has at that statement, with its
/// folded type — the lattice's answer, never a token relabelled as a
/// literal. `word` counts the call's words from the command's own (`0`,
/// never an operand here): a literal word is its text, and a substituted
/// word its literal runs and variable reads, each read at the statement's
/// use version, concatenated. A word the lattice holds no exact value for
/// — a command substitution, a read of a finite set or an unknown value,
/// an expansion, a word respelled after lowering — is `None`, as is any
/// word of a statement the solver never reached. Only a whole-word
/// variable read carries a folded type, the one its definition states.
/// A read of a name the statement writes is `None` once one of its words'
/// command substitutions has run: the substitution may have written it,
/// and the statement holds the one use version from before
/// (`set i 9; lindex $l [set i 0] $i` reads `$i` as 0).
/// `config` is the document's grammar, which the literal runs are decoded
/// under: the unit does not keep one.
#[must_use]
pub fn proven_word_value(
    fu: &crate::compilation_unit::FunctionUnit,
    statement: StatementId,
    word: usize,
    config: LexerConfig,
) -> Option<(ExactValue, Option<FoldedType>)> {
    proven_word_value_with_facts(
        fu,
        statement,
        word,
        config,
        ProvenReadFacts::execution(fu),
        None,
    )
}

pub(crate) fn proven_word_value_with_facts(
    fu: &crate::compilation_unit::FunctionUnit,
    statement: StatementId,
    word: usize,
    config: LexerConfig,
    facts: ProvenReadFacts<'_>,
    original: Option<(&CommandTokens, &[crate::word_subst::LiftedCall])>,
) -> Option<(ExactValue, Option<FoldedType>)> {
    if !facts.reached.contains(&statement.block) {
        return None;
    }
    let call = fu
        .cfg
        .blocks
        .get(&statement.block)?
        .statements
        .get(statement.index)?;
    let at = fu
        .ssa
        .blocks
        .get(&statement.block)?
        .statements
        .get(statement.index)?;
    let Statement::Call { args, tokens, .. } = call else {
        return None;
    };
    let tokens = original.map_or(tokens.as_ref(), |(tokens, _)| Some(tokens));
    let arguments = call_arguments(args, tokens, &config);
    let argument = arguments.get(word.checked_sub(1)?)?;
    let written = if at.defs.is_empty() {
        HashSet::new()
    } else {
        let start = tokens
            .and_then(|tokens| tokens.argv.get(word))
            .map(|span| span.start());
        let lexical;
        let calls = if let Some((_, calls)) = original {
            calls
        } else {
            lexical = crate::word_subst::lifted_calls(tokens, config);
            &lexical
        };
        written_before(at, start, calls)
    };
    let point = (statement.block, statement.index);
    proven_argument(argument, (&at.uses, point), &written, fu, (config, facts)).or_else(|| {
        let span = *tokens?.argv.get(word)?;
        let host = call.tokens()?;
        Some((
            proven_command_word(host, span, (&at.uses, point), &written, fu, facts)?,
            None,
        ))
    })
}

/// Where a command substitution runs: in a statement's words, or in a
/// block's terminator — a `return` value or a branch condition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SubstitutionHost {
    /// The words of a statement.
    Statement(StatementId),
    /// The terminator of a block.
    Terminator(crate::cfg::BlockId),
}

/// The command substitutions `host` performs, innermost first
/// ([`crate::word_subst`]), which [`proven_substituted_word_value`]
/// addresses: none for a host the solver never reached, a statement with
/// no words of its own, or a branch condition with no source offset.
#[must_use]
pub fn substitution_calls(
    fu: &crate::compilation_unit::FunctionUnit,
    host: SubstitutionHost,
    config: LexerConfig,
    surface: &tcl_registry::model::DocumentCommandSurface<'_>,
) -> Vec<crate::word_subst::LiftedCall> {
    let block = match host {
        SubstitutionHost::Statement(id) => id.block,
        SubstitutionHost::Terminator(block) => block,
    };
    if !fu.sccp.executable_blocks.contains(&block) {
        return Vec::new();
    }
    let Some(cfg_block) = fu.cfg.blocks.get(&block) else {
        return Vec::new();
    };
    match host {
        SubstitutionHost::Statement(id) => match cfg_block.statements.get(id.index) {
            Some(
                Statement::Call {
                    tokens: Some(tokens),
                    ..
                }
                | Statement::AssignValue {
                    tokens: Some(tokens),
                    ..
                },
            ) if tokens.synthetic.is_none() => {
                crate::word_subst::lifted_calls(Some(tokens), config)
            }
            _ => Vec::new(),
        },
        SubstitutionHost::Terminator(_) => match &cfg_block.terminator {
            Some(crate::cfg::Terminator::Return { value_word, .. }) => {
                crate::word_subst::lifted_calls_in_word(value_word.as_ref(), config, surface)
            }
            Some(crate::cfg::Terminator::Branch {
                condition,
                condition_base: Some(base),
                ..
            }) => crate::word_subst::lifted_calls_in_expr(condition, Some(*base), config, surface),
            _ => Vec::new(),
        },
    }
}

/// The words of the command `block`'s `return` terminator stands for, as
/// the segmenter recovers them from the document `source` at the
/// terminator's span, each span an offset into `source`: `None` for a block
/// the solver never reached, any other terminator, a terminator with no
/// source span, or a span that does not hold exactly one complete command.
#[must_use]
pub fn return_command_tokens(
    fu: &crate::compilation_unit::FunctionUnit,
    block: crate::cfg::BlockId,
    source: &str,
    config: LexerConfig,
) -> Option<crate::ir::CommandTokens> {
    if !fu.sccp.executable_blocks.contains(&block) {
        return None;
    }
    let Some(crate::cfg::Terminator::Return {
        span: Some(span), ..
    }) = &fu.cfg.blocks.get(&block)?.terminator
    else {
        return None;
    };
    let span = fu.abs_span(*span);
    let text = source.get(span.start() as usize..span.end() as usize)?;
    let segments = crate::segmenter::segment_commands_with_offset_and_config(
        text,
        span.start(),
        tcl_lexer::SourceSegmentationPurpose::NestedScript.config(config),
    );
    let [segment] = segments.as_slice() else {
        return None;
    };
    if segment.is_partial {
        return None;
    }
    let map = tcl_lexer::SourceMap::new(text).with_base(span.start(), 0, 0);
    let tokens = crate::ir::CommandTokens::from_segmented(&map, config, segment);
    (!tokens.argv.is_empty()).then_some(tokens)
}

/// [`proven_word_value`] for word `word` of the `return` command `tokens`
/// holds ([`return_command_tokens`]), the terminator of `block`: read at the
/// block's exit versions, and only for a command that performs no command
/// substitution, since the exit versions hold whatever one writes.
#[must_use]
pub fn proven_return_word_value(
    fu: &crate::compilation_unit::FunctionUnit,
    block: crate::cfg::BlockId,
    tokens: &crate::ir::CommandTokens,
    word: usize,
    config: LexerConfig,
) -> Option<(ExactValue, Option<FoldedType>)> {
    proven_return_word_value_with_facts(
        fu,
        block,
        tokens,
        word,
        config,
        ProvenReadFacts::execution(fu),
    )
}

pub(crate) fn proven_return_word_value_with_facts(
    fu: &crate::compilation_unit::FunctionUnit,
    block: crate::cfg::BlockId,
    tokens: &CommandTokens,
    word: usize,
    config: LexerConfig,
    facts: ProvenReadFacts<'_>,
) -> Option<(ExactValue, Option<FoldedType>)> {
    if !facts.reached.contains(&block)
        || !crate::word_subst::lifted_calls(Some(tokens), config).is_empty()
    {
        return None;
    }
    let versions = &fu.ssa.blocks.get(&block)?.exit_versions;
    let arguments = call_arguments(tokens.argv_texts.get(1..)?, Some(tokens), &config);
    proven_argument(
        arguments.get(word.checked_sub(1)?)?,
        (versions, (block, usize::MAX)),
        &HashSet::new(),
        fu,
        (config, facts),
    )
}

/// [`proven_word_value`] for word `word` of `calls[call]`, a command
/// substitution `host` performs, where `calls` is what
/// [`substitution_calls`] lifts there. A statement's substitution reads at
/// the statement's use versions, under the same rule for a name the
/// statement writes; a terminator's reads at its block's exit versions,
/// which the definition point ahead of it has already given every name its
/// words write.
#[must_use]
pub fn proven_substituted_word_value(
    fu: &crate::compilation_unit::FunctionUnit,
    host: SubstitutionHost,
    calls: &[crate::word_subst::LiftedCall],
    (call, word): (usize, usize),
    config: LexerConfig,
) -> Option<(ExactValue, Option<FoldedType>)> {
    proven_substituted_word_value_with_facts(
        fu,
        host,
        calls,
        (call, word),
        config,
        ProvenReadFacts::execution(fu),
    )
}

pub(crate) fn proven_substituted_word_value_with_facts(
    fu: &crate::compilation_unit::FunctionUnit,
    host: SubstitutionHost,
    calls: &[crate::word_subst::LiftedCall],
    (call, word): (usize, usize),
    config: LexerConfig,
    facts: ProvenReadFacts<'_>,
) -> Option<(ExactValue, Option<FoldedType>)> {
    let tokens = calls.get(call)?.words.as_ref()?;
    let start = tokens.argv.get(word)?.start();
    let (versions, written, point) = match host {
        SubstitutionHost::Statement(id) => {
            if !facts.reached.contains(&id.block) {
                return None;
            }
            let at = fu.ssa.blocks.get(&id.block)?.statements.get(id.index)?;
            (
                &at.uses,
                written_before(at, Some(start), calls),
                (id.block, id.index),
            )
        }
        SubstitutionHost::Terminator(block) => {
            if !facts.reached.contains(&block) {
                return None;
            }
            (
                &fu.ssa.blocks.get(&block)?.exit_versions,
                HashSet::new(),
                (block, usize::MAX),
            )
        }
    };
    let arguments = call_arguments(tokens.argv_texts.get(1..)?, Some(tokens), &config);
    proven_argument(
        arguments.get(word.checked_sub(1)?)?,
        (versions, point),
        &written,
        fu,
        (config, facts),
    )
    .or_else(|| {
        let SubstitutionHost::Statement(id) = host else {
            return None;
        };
        let statement = fu.cfg.blocks.get(&id.block)?.statements.get(id.index)?;
        let value = proven_command_word(
            statement.tokens()?,
            *tokens.argv.get(word)?,
            (versions, point),
            &written,
            fu,
            facts,
        )?;
        Some((value, None))
    })
}

/// The value the whole-word command substitution `span` holds, one of those
/// the statement `host` performs, folds to through the registry's
/// const-folds, each variable it reads at `uses` under `facts`: `None` when
/// `facts` carries no registry, the substitution reads a name the statement
/// has already written, or any part of it declines.
fn proven_command_word(
    host: &CommandTokens,
    span: tcl_lexer::Span,
    (uses, point): (&HashMap<Symbol, Version>, (crate::cfg::BlockId, usize)),
    written: &HashSet<Symbol>,
    fu: &crate::compilation_unit::FunctionUnit,
    facts: ProvenReadFacts<'_>,
) -> Option<ExactValue> {
    let registry = facts.folds?;
    let input = fu.source_metadata_input.as_ref()?;
    let metadata =
        crate::registry_invocation::InvocationMetadataContext::for_analysis_input(registry, input)?;
    let calls = crate::word_subst::checked_lifted_calls(host, input.lexer_config().nested())?;
    // The word's span stops before its `]`; the call's covers it.
    let mut selected = calls
        .iter()
        .filter(|call| call.span.start() == span.start());
    let call = selected.next()?;
    selected.next().is_none().then_some(())?;
    let lookup = |name: &str| {
        let symbol = read_symbol(fu, point, name)?;
        if written.contains(&symbol) {
            return None;
        }
        match facts.values.get(&(symbol, *uses.get(&symbol)?))? {
            LatticeValue::Const(value) => String::from_utf8(const_to_exact(value).bytes).ok(),
            _ => None,
        }
    };
    let trust = |_: &str| false;
    crate::const_subst::ConstSubstCtx {
        registry,
        resolution_namespace: "::",
        namespace_context: None,
        version: input.unit_profile().const_fold_version(),
        defining_class: None,
        trusts: &trust,
        lookup_var: &lookup,
        script_encoding: tcl_registry::value_transfer::ScriptEncoding::Auto,
    }
    .fold_retained_call_with_metadata_context(
        call,
        crate::const_subst::RetainedCalls::new(&calls),
        metadata,
    )
    .map(|text| exact_of_bytes(text.into_bytes()))
}

/// The names the statement `at` writes, when one of the command
/// substitutions `calls` its words perform has run before the read at
/// `start` (or the read's position is unknown); otherwise none.
pub(in super::super) fn written_before(
    at: &SsaStatement,
    start: Option<u32>,
    calls: &[crate::word_subst::LiftedCall],
) -> HashSet<Symbol> {
    let ran = start.is_none_or(|start| calls.iter().any(|call| call.span.end() <= start));
    if ran {
        at.defs.keys().copied().collect()
    } else {
        HashSet::new()
    }
}

/// The symbol `name` reads at `point` (`usize::MAX` for a terminator): the
/// binding at that operation when the SSA records one per operation.
fn read_symbol(
    fu: &crate::compilation_unit::FunctionUnit,
    (block, index): (crate::cfg::BlockId, usize),
    name: &str,
) -> Option<Symbol> {
    if fu.ssa.point_contexts.is_some() {
        fu.ssa.var_symbol_at(block, index, name)
    } else {
        fu.ssa.var_symbol(name)
    }
}

/// One argument's proven value: a literal is its text, a substituted word
/// [`proven_substitution`] at `versions`, never reading a name in `written`.
fn proven_argument(
    argument: &ArgWord<'_>,
    versions: (&HashMap<Symbol, Version>, (crate::cfg::BlockId, usize)),
    written: &HashSet<Symbol>,
    fu: &crate::compilation_unit::FunctionUnit,
    (config, facts): (LexerConfig, ProvenReadFacts<'_>),
) -> Option<(ExactValue, Option<FoldedType>)> {
    match argument.source {
        OperandSource::BracedLiteral | OperandSource::Literal | OperandSource::QuotedLiteral => {
            Some((ExactValue::from_literal(&argument.text), None))
        }
        OperandSource::Substituted => {
            proven_substitution(&argument.text, versions, written, fu, (config, facts))
        }
        OperandSource::Unknown => None,
    }
}

/// [`proven_word_value`] of a substituted word: its literal runs decoded,
/// each variable read at the use version `uses` gives it, and nothing else.
fn proven_substitution(
    text: &str,
    (uses, point): (&HashMap<Symbol, Version>, (crate::cfg::BlockId, usize)),
    written: &HashSet<Symbol>,
    fu: &crate::compilation_unit::FunctionUnit,
    (config, facts): (LexerConfig, ProvenReadFacts<'_>),
) -> Option<(ExactValue, Option<FoldedType>)> {
    use tcl_lexer::word_parts::{SubstFlags, WordBody, WordPart as Part, decompose};
    let read = |name: &str| -> Option<(ExactValue, ValueKey)> {
        let symbol = read_symbol(fu, point, name)?;
        if written.contains(&symbol) {
            return None;
        }
        let key = (symbol, *uses.get(&symbol)?);
        match facts.values.get(&key)? {
            LatticeValue::Const(value) => Some((const_to_exact(value), key)),
            _ => None,
        }
    };
    if let Some(name) = tcl_syntax::naming::whole_variable_reference_literal_name(text, config) {
        let (value, key) = read(name)?;
        return Some((
            value,
            facts
                .folded_types
                .and_then(|types| types.get(&key).cloned()),
        ));
    }
    let parts = match decompose(text.as_bytes(), SubstFlags::default(), config) {
        WordBody::Literal(bytes) => return Some((exact_of_bytes(bytes.to_vec()), None)),
        WordBody::Parts(parts) => parts,
    };
    let mut bytes = Vec::with_capacity(text.len());
    for part in parts {
        match part {
            Part::Text(run) => bytes.extend_from_slice(&run),
            Part::Variable(reference) => {
                let (value, _) = read(&variable_name(&reference).ok()?)?;
                bytes.extend_from_slice(&value.bytes);
            }
            Part::Command(_) | Part::Expression(_) | Part::ParseError(_) => return None,
        }
    }
    Some((exact_of_bytes(bytes), None))
}
