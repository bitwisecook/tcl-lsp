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

//! The literal-only checks over proven words: a check the walk abstains
//! from because a word it reads is not literal runs again, once the
//! compilation unit exists, over the value the lattice proves for that word
//! at its statement ([`crate::value_transfer::proven_word_value`]).
//!
//! The walk keeps every call with such a word ([`ProvenSite`]); the pass
//! retains each original operand and supplies its lattice value separately
//! to the selected source descriptor. It never manufactures an original
//! literal token or Native argument vector. What a check reports is kept only at a word the lattice proved,
//! so nothing the walk reported is reported twice and nothing lands at a
//! span the user did not write; the index checks, which report at the
//! literal index a proven list or string makes checkable, keep what they
//! report over the proven words and did not report over the written ones.
//! A finding over a proven word carries no fix: its word is a substitution
//! the user wrote, which a fix would replace with a constant. A call nested
//! in a command substitution is read at the statement or terminator that
//! performs the substitution, as a call written on its own line is read at
//! its statement, and a `return`'s own words — its options — at the
//! terminator it lowers to. An index whose `$var` value the interval checks
//! bound is theirs: the re-run leaves a site they reported to them.

use rustc_hash::FxHashMap;
use tcl_core_types::DiagCode;
use tcl_lexer::{Span, Token, TokenType};

use crate::analyser::state::Analyser;
use crate::compilation_unit::{CompilationUnit, FunctionUnit};
use crate::value_transfer::{
    OriginalDiagnosticValues, StatementId, SubstitutionHost, proven_return_word_value,
    proven_substituted_word_value, proven_word_value, return_command_tokens, substitution_calls,
};
use crate::word_subst::LiftedCall;

/// The codes the pass may report, each at a proven word.
const PROVEN_CODES: [DiagCode; 10] = [
    DiagCode::W121,
    DiagCode::W127,
    DiagCode::W137,
    DiagCode::W138,
    DiagCode::W141,
    DiagCode::W145,
    DiagCode::W146,
    DiagCode::W200,
    DiagCode::W202,
    DiagCode::W303,
];

/// A dispatch site's words as the walk reads them, borrowed.
#[derive(Clone, Copy)]
pub(in crate::analyser) struct CallWords<'a> {
    pub cmd_tok: Token,
    pub args: &'a [String],
    pub arg_tokens: &'a [Token],
    pub arg_single: &'a [bool],
    /// Parallel to the whole argv, the command word at `0`.
    pub arg_expand_in: &'a [bool],
}

/// A call the walk dispatched with a word a literal-only check could not
/// read, owned until the pass reads the words' proven values.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::analyser) struct ProvenSite {
    cmd_tok: Token,
    args: Vec<String>,
    arg_tokens: Vec<Token>,
    arg_single: Vec<bool>,
    arg_expand_in: Vec<bool>,
}

impl ProvenSite {
    /// Syntax-only coordinate adjustment for a memoised body fragment. The
    /// whole-source invocation and actual CU revalidate every eventual value.
    pub(in crate::analyser) fn rebase(&mut self, delta: u32) {
        self.cmd_tok.span = tcl_lexer::Span::new(
            self.cmd_tok.span.start() + delta,
            self.cmd_tok.span.end() + delta,
        );
        for token in &mut self.arg_tokens {
            token.span = tcl_lexer::Span::new(token.span.start() + delta, token.span.end() + delta);
        }
    }
}

/// A site's words with every proven word substituted.
struct ProvenWords {
    args: Vec<String>,
    /// The spans of the substituted words: the only places a finding over
    /// the proven words is kept.
    proven: Vec<Span>,
    /// The substituted argument indices.
    at: Vec<usize>,
    /// The absolute span of the statement or terminator whose words the
    /// site's are, where the interval checks anchor a finding.
    host: Option<Span>,
}

/// Where a call word sits in a unit the solver reached.
#[derive(Clone, Copy, PartialEq, Eq)]
enum WordAddress {
    /// Word `word` of a call statement.
    Statement(StatementId, usize),
    /// Word `word` of call `call` of host `host`'s command substitutions.
    Substituted {
        host: usize,
        call: usize,
        word: usize,
    },
    /// Word `word` of the `return` command of `returns[at]`.
    Return { at: usize, word: usize },
}

/// Every call word of the unit's reached statements, of the command
/// substitutions their words and their blocks' terminators perform, and of
/// the `return` commands their blocks end with, by its absolute span.
struct WordIndex<'u> {
    /// Identity of this borrowed function is only local index bookkeeping;
    /// each value owner validates its actual Module and projection inputs.
    logical_values: FxHashMap<*const FunctionUnit, OriginalDiagnosticValues<'u>>,
    words: FxHashMap<(u32, u32), (&'u FunctionUnit, WordAddress)>,
    /// Each host's command substitutions, as [`substitution_calls`] lifts
    /// them, with the host's absolute span.
    hosts: Vec<(SubstitutionHost, Vec<LiftedCall>, Option<Span>)>,
    /// Each reached `return` terminator's block and words
    /// ([`return_command_tokens`]), with its absolute span.
    returns: Vec<(crate::cfg::BlockId, crate::ir::CommandTokens, Span)>,
}

impl<'u> WordIndex<'u> {
    /// `wanted` is the sorted start of every word a site may prove: a host
    /// holding none has its substitutions left unlifted.
    fn build(
        cu: &'u CompilationUnit,
        wanted: &[u32],
        (source, config): (&str, tcl_lexer::LexerConfig),
        surface: &tcl_registry::model::DocumentCommandSurface<'_>,
    ) -> Self {
        let mut index = Self {
            logical_values: FxHashMap::default(),
            words: FxHashMap::default(),
            hosts: Vec::new(),
            returns: Vec::new(),
        };
        let holds = |span: Option<Span>| {
            span.is_some_and(|span| {
                let at = wanted.partition_point(|&start| start < span.start());
                wanted.get(at).is_some_and(|&start| start < span.end())
            })
        };
        let units = std::iter::once(&cu.top_level)
            .chain(cu.procedures.values())
            .chain(cu.methods.values())
            .chain(cu.body_units.values());
        let registry = surface.commands();
        if !cu
            .ir_module
            .retained_source_bindings
            .as_ref()
            .is_some_and(|owner| owner.matches_module(&cu.ir_module, registry))
        {
            return index;
        }

        let Some(module_metadata) =
            crate::registry_invocation::InvocationMetadataContext::for_module(
                registry,
                &cu.ir_module,
            )
        else {
            return index;
        };
        let Some(input) = module_metadata.source_analysis_input() else {
            return index;
        };
        if input.lexer_config().normalized() != config.normalized()
            || cu.ir_module.source.bytes() != source.as_bytes()
        {
            return index;
        }
        for fu in units {
            let Some(metadata) = fu.invocation_metadata_context_for_module(registry, &cu.ir_module)
            else {
                continue;
            };
            if metadata.permits_logical_source_names() {
                let Some(values) =
                    OriginalDiagnosticValues::for_module_function(&cu.ir_module, fu, registry)
                else {
                    continue;
                };
                index.logical_values.insert(std::ptr::from_ref(fu), values);
            }
            let config = fu.source_lexer_config();
            let blocks: Vec<_> = fu
                .cfg
                .blocks
                .keys()
                .copied()
                .filter(|block| {
                    index
                        .logical_values
                        .get(&std::ptr::from_ref(fu))
                        .map_or_else(
                            || fu.sccp.executable_blocks.contains(block),
                            |values| values.reaches(*block),
                        )
                })
                .collect();
            for block in blocks {
                let Some(cfg_block) = fu.cfg.blocks.get(&block) else {
                    continue;
                };
                for (at, statement) in cfg_block.statements.iter().enumerate() {
                    let id = StatementId { block, index: at };
                    if matches!(statement, crate::ir::Statement::Call { .. })
                        && let Some(tokens) = fu.cfg.source_tokens_at(block, at)
                        && tokens.synthetic.is_none()
                    {
                        for (word, &span) in tokens.argv.iter().enumerate() {
                            index.insert(fu, span, WordAddress::Statement(id, word));
                        }
                    }
                    let span = Some(fu.abs_span(statement.span()));
                    if holds(span) {
                        index.add_host(
                            fu,
                            SubstitutionHost::Statement(id),
                            span,
                            (config, surface),
                        );
                    }
                }
                let span = match &cfg_block.terminator {
                    Some(
                        crate::cfg::Terminator::Return { span, .. }
                        | crate::cfg::Terminator::Branch { span, .. },
                    ) => span.map(|span| fu.abs_span(span)),
                    _ => None,
                };
                if holds(span) {
                    index.add_host(
                        fu,
                        SubstitutionHost::Terminator(block),
                        span,
                        (config, surface),
                    );
                    index.add_return(fu, block, source, config);
                }
            }
        }
        index
    }

    /// Index the words of `block`'s `return` terminator, when it has one.
    fn add_return(
        &mut self,
        fu: &'u FunctionUnit,
        block: crate::cfg::BlockId,
        source: &str,
        config: tcl_lexer::LexerConfig,
    ) {
        let tokens = self
            .logical_values
            .get(&std::ptr::from_ref(fu))
            .map_or_else(
                || return_command_tokens(fu, block, source, config),
                |values| values.return_tokens(block),
            );
        let Some(tokens) = tokens else {
            return;
        };
        let at = self.returns.len();
        for (word, &span) in tokens.argv.iter().enumerate() {
            self.words
                .entry((span.start(), span.end()))
                .or_insert((fu, WordAddress::Return { at, word }));
        }
        let span = Span::new(
            tokens.argv.first().map_or(0, |span| span.start()),
            tokens.argv.last().map_or(0, |span| span.end()),
        );
        self.returns.push((block, tokens, span));
    }

    fn insert(&mut self, fu: &'u FunctionUnit, span: Span, address: WordAddress) {
        let span = fu.abs_span(span);
        self.words
            .entry((span.start(), span.end()))
            .or_insert((fu, address));
    }

    fn add_host(
        &mut self,
        fu: &'u FunctionUnit,
        host: SubstitutionHost,
        span: Option<Span>,
        (config, surface): (
            tcl_lexer::LexerConfig,
            &tcl_registry::model::DocumentCommandSurface<'_>,
        ),
    ) {
        let calls = self
            .logical_values
            .get(&std::ptr::from_ref(fu))
            .map_or_else(
                || substitution_calls(fu, host, config, surface),
                |values| values.substitution_calls(host).unwrap_or_default(),
            );
        if calls.is_empty() {
            return;
        }
        let at = self.hosts.len();
        for (call, lifted) in calls.iter().enumerate() {
            let Some(tokens) = &lifted.words else {
                continue;
            };
            for (word, &span) in tokens.argv.iter().enumerate().skip(1) {
                self.insert(
                    fu,
                    span,
                    WordAddress::Substituted {
                        host: at,
                        call,
                        word,
                    },
                );
            }
        }
        self.hosts.push((host, calls, span));
    }

    /// The value the lattice proves at `address`, and the absolute span of
    /// the statement or terminator whose words hold it.
    fn proven(
        &self,
        fu: &FunctionUnit,
        address: WordAddress,
        _config: tcl_lexer::LexerConfig,
    ) -> Option<(String, Option<Span>)> {
        let config = fu.source_lexer_config();
        let logical = self.logical_values.get(&std::ptr::from_ref(fu));
        match address {
            WordAddress::Statement(id, word) => {
                let host = fu
                    .cfg
                    .blocks
                    .get(&id.block)
                    .and_then(|block| block.statements.get(id.index))
                    .map(|statement| fu.abs_span(statement.span()));
                let value = logical.map_or_else(
                    || {
                        proven_word_value(fu, id, word, config)
                            .and_then(|(value, _)| value.as_str().ok().map(str::to_owned))
                    },
                    |values| values.word_contents(id, word),
                )?;
                Some((value, host))
            }
            WordAddress::Substituted { host, call, word } => {
                let (at, calls, span) = self.hosts.get(host)?;
                let value = logical.map_or_else(
                    || {
                        proven_substituted_word_value(fu, *at, calls, (call, word), config)
                            .and_then(|(value, _)| value.as_str().ok().map(str::to_owned))
                    },
                    |values| values.substituted_word_contents(*at, calls, (call, word)),
                )?;
                Some((value, *span))
            }
            WordAddress::Return { at, word } => {
                let (block, tokens, span) = self.returns.get(at)?;
                let value = logical.map_or_else(
                    || {
                        proven_return_word_value(fu, *block, tokens, word, config)
                            .and_then(|(value, _)| value.as_str().ok().map(str::to_owned))
                    },
                    |values| values.return_word_contents(*block, tokens, word),
                )?;
                Some((value, Some(*span)))
            }
        }
    }
}

/// Whether the walk could not read `text` and the lattice may prove it: a
/// variable read, or a word with substitutions and no command substitution
/// (which the lattice never answers for).
fn may_be_proven(text: &str, token: &Token, single: bool, expanded: bool) -> bool {
    !expanded
        && !text.contains('[')
        && text.contains('$')
        && (token.kind == TokenType::Var || !single)
}

impl ProvenSite {
    /// Where each word the lattice may prove starts.
    fn candidates(&self) -> impl Iterator<Item = u32> + '_ {
        self.arg_tokens
            .iter()
            .enumerate()
            .filter(|&(at, token)| {
                self.args.get(at).is_some_and(|text| {
                    may_be_proven(
                        text,
                        token,
                        self.arg_single.get(at).copied().unwrap_or(false),
                        self.arg_expand_in.get(at + 1).copied().unwrap_or(false),
                    )
                })
            })
            .map(|(_, token)| token.span.start())
    }

    /// The site's words with each word the lattice proves at its statement
    /// substituted, or `None` when it proves none.
    fn proven(&self, index: &WordIndex<'_>, config: tcl_lexer::LexerConfig) -> Option<ProvenWords> {
        let mut words = ProvenWords {
            args: self.args.clone(),
            proven: Vec::new(),
            at: Vec::new(),
            host: None,
        };
        let mut site = None;
        for (at, token) in self.arg_tokens.iter().enumerate() {
            let expanded = self.arg_expand_in.get(at + 1).copied().unwrap_or(false);
            let single = self.arg_single.get(at).copied().unwrap_or(false);
            let Some(text) = self.args.get(at) else {
                continue;
            };
            if !may_be_proven(text, token, single, expanded) {
                continue;
            }
            let Some(&(fu, address)) = index.words.get(&(token.span.start(), token.span.end()))
            else {
                continue;
            };
            // The words must be this call's own: the argument at `at` is
            // word `at + 1` of one statement or one command substitution.
            let (word, call) = match address {
                WordAddress::Statement(id, word) => (word, WordAddress::Statement(id, 0)),
                WordAddress::Substituted { host, call, word } => (
                    word,
                    WordAddress::Substituted {
                        host,
                        call,
                        word: 0,
                    },
                ),
                WordAddress::Return { at, word } => (word, WordAddress::Return { at, word: 0 }),
            };
            if word != at + 1 || site.is_some_and(|seen| seen != call) {
                continue;
            }
            let Some((value, host)) = index.proven(fu, address, config) else {
                continue;
            };
            let text = value;
            site = Some(call);
            words.host = host;
            words.args[at] = text;
            words.proven.push(token.span);
            words.at.push(at);
        }
        (!words.at.is_empty()).then_some(words)
    }
}

impl Analyser {
    /// Keep `call` for [`Self::emit_proven_word_diagnostics`] when a word
    /// a literal-only check reads may have a proven value.
    pub(in crate::analyser) fn record_proven_site(&mut self, call: &CallWords<'_>) {
        let candidate = |at: usize| {
            let (Some(text), Some(token)) = (call.args.get(at), call.arg_tokens.get(at)) else {
                return false;
            };
            may_be_proven(
                text,
                token,
                call.arg_single.get(at).copied().unwrap_or(false),
                call.arg_expand_in.get(at + 1).copied().unwrap_or(false),
            )
        };
        if !(0..call.args.len()).any(candidate) {
            return;
        }
        self.proven_sites.push(ProvenSite {
            cmd_tok: call.cmd_tok,
            args: call.args.to_vec(),
            arg_tokens: call.arg_tokens.to_vec(),
            arg_single: call.arg_single.to_vec(),
            arg_expand_in: call.arg_expand_in.to_vec(),
        });
    }

    /// Run the literal-only checks again over the words `cu`'s lattice
    /// proves at each call the walk kept.
    pub(super) fn emit_proven_word_diagnostics(&mut self, cu: &CompilationUnit) {
        let sites = std::mem::take(&mut self.proven_sites);
        if sites.is_empty() {
            return;
        }
        let config = self.file_lexer_config();
        let generation = self.analysis_context();
        let registry = generation.commands();
        let surface = self.command_surface(registry);
        let mut wanted: Vec<u32> = sites.iter().flat_map(ProvenSite::candidates).collect();
        wanted.sort_unstable();
        let index = WordIndex::build(cu, &wanted, (&self.source, config), &surface);
        let proven: Vec<(&ProvenSite, ProvenWords)> = sites
            .iter()
            .filter_map(|site| site.proven(&index, config).map(|words| (site, words)))
            .collect();
        if proven.is_empty() {
            return;
        }
        let facts = super::validity::UserResolutionFacts::build(self);
        for (site, words) in &proven {
            self.rerun_literal_checks(site, words, &facts);
        }
    }

    /// The checks over one site's proven words.
    fn rerun_literal_checks(
        &mut self,
        site: &ProvenSite,
        words: &ProvenWords,
        facts: &super::validity::UserResolutionFacts,
    ) {
        let Some(written) = self.original_diagnostic_call_at(site.cmd_tok, &site.arg_tokens) else {
            return;
        };
        let literals = words
            .at
            .iter()
            .filter_map(|&index| words.args.get(index).map(|value| (index, value.clone())))
            .collect::<Vec<_>>();
        let Some(original) = written.with_supplemental_literals(&literals) else {
            return;
        };
        self.refine_original_path_arguments(&written, &original);
        self.refine_original_pattern_arguments(&written, &original);
        let marks = (
            self.result.diagnostics.len(),
            self.dsl_gate_sites.len(),
            self.pending_arity.len(),
        );
        let cmd_name = original.command();
        let Some((arguments, argument_tokens, _single)) = original.source_arguments() else {
            return;
        };
        let formats = Self::original_format_templates(Some(&original));
        self.emit_binary_field_version_gates(&formats);
        self.record_dsl_format_sites(cmd_name, &formats);
        self.emit_w121_invalid_subnet_mask(&arguments, &argument_tokens);
        self.emit_w127_closed_value_args(Some(&original));
        self.emit_w127_closed_option_values(Some(&original));
        self.emit_w146_literal_argument_validation(Some(&original));
        if words.at.first() == Some(&0) {
            self.emit_w001_unknown_subcommand(Some(&original));
        }
        self.emit_w004_dialect_invalid_option(Some(&original), cmd_name);
        self.emit_proven_source_option_relationships(&original, cmd_name);
        let original_proven_spans = original
            .supplemental_literals()
            .iter()
            .filter_map(|(ordinal, _)| original.word(*ordinal).map(|word| word.span()))
            .collect::<Vec<_>>();
        let at_proven = |code: DiagCode, span: Span| {
            PROVEN_CODES.contains(&code)
                && (words.proven.contains(&span) || original_proven_spans.contains(&span))
        };
        let found = self.result.diagnostics.split_off(marks.0);
        self.result.diagnostics.extend(
            found
                .into_iter()
                .filter(|diagnostic| at_proven(diagnostic.code, diagnostic.span))
                .map(|diagnostic| diagnostic.with_fixes(Vec::new())),
        );
        let gated = self.dsl_gate_sites.split_off(marks.1);
        self.dsl_gate_sites.extend(
            gated
                .into_iter()
                .filter(|gate| at_proven(gate.code, gate.span)),
        );
        let verdicts: Vec<_> = self
            .pending_arity
            .split_off(marks.2)
            .into_iter()
            .filter(|(_, _, _, diagnostic)| at_proven(diagnostic.code, diagnostic.span))
            .map(|(name, ns, enforce_order, diagnostic)| {
                (name, ns, enforce_order, diagnostic.with_fixes(Vec::new()))
            })
            .collect();
        self.settle_builtin_verdicts(facts, verdicts);
        self.rerun_index_checks(site, words, &written, &original);
    }

    /// W230 and W232 over the proven words: an index the written list or
    /// string could not be checked against is checked against the proven
    /// one, and what the written words or the interval checks already drew
    /// is not drawn again.
    fn rerun_index_checks(
        &mut self,
        site: &ProvenSite,
        words: &ProvenWords,
        written: &super::super::diagnostic_registry::OriginalDiagnosticInvocation,
        proven: &super::super::diagnostic_registry::OriginalDiagnosticInvocation,
    ) {
        let written_diagnostics = super::super::bounds_checks::original_index_diagnostics(written);
        for diagnostic in super::super::bounds_checks::original_index_diagnostics(proven) {
            let drawn = written_diagnostics.iter().any(|seen| {
                seen.code == diagnostic.code
                    && seen.span == diagnostic.span
                    && seen.message == diagnostic.message
            });
            if !drawn && !self.interval_reported(site, words.host, &diagnostic) {
                self.result.diagnostics.push(diagnostic);
            }
        }
    }

    /// Whether the interval checks reported the access `diagnostic` is
    /// anchored at: its index is a written `$var`, and they reported that
    /// variable's index under the same code at the site's host.
    fn interval_reported(
        &self,
        site: &ProvenSite,
        host: Option<Span>,
        diagnostic: &crate::analyser::types::Diagnostic,
    ) -> bool {
        let Some(host) = host else {
            return false;
        };
        site.arg_tokens
            .iter()
            .zip(&site.args)
            .find(|(token, _)| token.span == diagnostic.span)
            .and_then(|(_, text)| crate::interval_bounds::plain_var_name(text))
            .is_some_and(|name| {
                self.interval_index_sites
                    .contains(&(diagnostic.code, host, name))
            })
    }
}

#[cfg(test)]
mod source_value_projection_tests {
    use super::*;

    #[test]
    fn supplemental_values_preserve_original_alias_operands_and_source_subjects() {
        // naming.diagnostic.registry-source-ownership
        // docs/design/analysis/name-resolution-proofs/diagnostic-registry-source-ownership.md
        // Source/API correspondence only: no native argv or completion follows.
        let source = "interp alias {} classify {} string is; classify $kind value";
        let mut analyser = Analyser::new();
        let result = analyser
            .analyse_and_retain_result_for_test(source, "tcl")
            .clone();
        assert!(result.analysis_context_unavailable.is_none());
        let command = crate::segmenter::segment_commands_with_offset_and_config(
            source,
            0,
            analyser.lexer_config(),
        )
        .pop()
        .expect("the original alias call");
        let original = analyser
            .original_diagnostic_call_at(command.argv[0], command.arg_tokens())
            .expect("the retained source alias selection");
        assert_eq!(original.command(), "string");
        assert_eq!(original.literal(0), Some("is"));
        assert_eq!(original.literal(1), None);
        let projection = original
            .with_supplemental_literals(&[(0, "integer".to_owned())])
            .expect("a value on the original written operand");
        assert_eq!(projection.words(), original.words());
        assert_eq!(projection.literal(0), Some("is"));
        assert_eq!(projection.literal(1), Some("integer"));
        assert!(projection.word(0).unwrap().span().end() < projection.head().span().start());
        let subject = projection
            .subject(
                crate::analyser::RegistrySourceDiagnosticKind::LiteralArgument,
                Some(1),
            )
            .expect("the original written operand subject");
        let crate::analyser::DiagnosticSubject::RegistrySource(subject) = subject else {
            panic!("a structured registry source subject");
        };
        assert_eq!(subject.written_argument(), Some(0));
        assert_eq!(
            subject.supplemental_literals(),
            &[(1, "integer".to_owned())]
        );
        assert_eq!(subject.span(), original.word(1).unwrap().span());
        assert!(
            original
                .with_supplemental_literals(&[(8, "integer".to_owned())])
                .is_none()
        );
        assert!(
            original
                .with_supplemental_literals(&[(0, "integer".to_owned()), (0, "dict".to_owned())])
                .is_none()
        );
    }

    #[test]
    fn supplemental_values_cannot_resurrect_a_source_replaced_command() {
        // naming.diagnostic.registry-source-ownership
        // docs/design/analysis/name-resolution-proofs/diagnostic-registry-source-ownership.md
        let source = "proc string args {return CUSTOM}; string is $kind value";
        let mut analyser = Analyser::new();
        analyser.analyse_and_retain_result_for_test(source, "tcl");
        let command = crate::segmenter::segment_commands_with_offset_and_config(
            source,
            0,
            analyser.lexer_config(),
        )
        .pop()
        .unwrap();
        assert!(
            analyser
                .original_diagnostic_call_at(command.argv[0], command.arg_tokens())
                .is_none()
        );
    }
}
