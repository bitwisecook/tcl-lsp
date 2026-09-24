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
//! substitutes each proven word — its text as a braced literal at the
//! word's own span — and runs each check the site's words could have
//! drawn. What a check reports is kept only at a word the lattice proved,
//! so nothing the walk reported is reported twice and nothing lands at a
//! span the user did not write; the index checks, which report at the
//! literal index a proven list or string makes checkable, keep what they
//! report over the proven words and did not report over the written ones.
//! A finding over a proven word carries no fix: its word is a substitution
//! the user wrote, which a fix would replace with a constant.

use rustc_hash::FxHashMap;
use tcl_core_types::DiagCode;
use tcl_lexer::{Span, Token, TokenType};

use crate::analyser::state::Analyser;
use crate::compilation_unit::{CompilationUnit, FunctionUnit};
use crate::value_transfer::{StatementId, proven_word_value};

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
    pub cmd_name: &'a str,
    pub cmd_tok: Token,
    pub args: &'a [String],
    pub arg_tokens: &'a [Token],
    pub arg_single: &'a [bool],
    /// Parallel to the whole argv, the command word at `0`.
    pub arg_expand_in: &'a [bool],
    pub scope_path: &'a [usize],
}

/// A call the walk dispatched with a word a literal-only check could not
/// read, owned until the pass reads the words' proven values.
#[derive(Debug)]
pub(in crate::analyser) struct ProvenSite {
    cmd_name: String,
    cmd_tok: Token,
    args: Vec<String>,
    arg_tokens: Vec<Token>,
    arg_single: Vec<bool>,
    arg_expand_in: Vec<bool>,
    scope_path: Vec<usize>,
}

/// A site's words with every proven word substituted.
struct ProvenWords {
    args: Vec<String>,
    arg_tokens: Vec<Token>,
    arg_single: Vec<bool>,
    /// The spans of the substituted words: the only places a finding over
    /// the proven words is kept.
    proven: Vec<Span>,
    /// The substituted argument indices.
    at: Vec<usize>,
}

/// Every call word of the unit's reached statements by its absolute span:
/// the unit, the statement and the word's index in its argv.
struct WordIndex<'u> {
    words: FxHashMap<(u32, u32), (&'u FunctionUnit, StatementId, usize)>,
}

impl<'u> WordIndex<'u> {
    fn build(cu: &'u CompilationUnit) -> Self {
        let mut words = FxHashMap::default();
        let units = std::iter::once(&cu.top_level)
            .chain(cu.procedures.values())
            .chain(cu.methods.values())
            .chain(cu.body_units.values());
        for fu in units {
            for &block in &fu.sccp.executable_blocks {
                let Some(cfg_block) = fu.cfg.blocks.get(&block) else {
                    continue;
                };
                for (index, statement) in cfg_block.statements.iter().enumerate() {
                    let crate::ir::Statement::Call {
                        tokens: Some(tokens),
                        ..
                    } = statement
                    else {
                        continue;
                    };
                    if tokens.synthetic.is_some() {
                        continue;
                    }
                    for (word, &span) in tokens.argv.iter().enumerate() {
                        let span = fu.abs_span(span);
                        words.entry((span.start(), span.end())).or_insert((
                            fu,
                            StatementId { block, index },
                            word,
                        ));
                    }
                }
            }
        }
        Self { words }
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
    /// The site's words with each word the lattice proves at its statement
    /// substituted, or `None` when it proves none.
    fn proven(&self, index: &WordIndex<'_>, config: tcl_lexer::LexerConfig) -> Option<ProvenWords> {
        let mut words = ProvenWords {
            args: self.args.clone(),
            arg_tokens: self.arg_tokens.clone(),
            arg_single: self.arg_single.clone(),
            proven: Vec::new(),
            at: Vec::new(),
        };
        let mut statement = None;
        for (at, token) in self.arg_tokens.iter().enumerate() {
            let expanded = self.arg_expand_in.get(at + 1).copied().unwrap_or(false);
            let single = self.arg_single.get(at).copied().unwrap_or(false);
            let Some(text) = self.args.get(at) else {
                continue;
            };
            if !may_be_proven(text, token, single, expanded) {
                continue;
            }
            let Some(&(fu, id, word)) = index.words.get(&(token.span.start(), token.span.end()))
            else {
                continue;
            };
            // The words must be this call's own: the argument at `at` is
            // argv word `at + 1` of one statement.
            if word != at + 1 || statement.is_some_and(|seen| seen != id) {
                continue;
            }
            let Some((value, _)) = proven_word_value(fu, id, word, config) else {
                continue;
            };
            let Ok(text) = String::from_utf8(value.bytes) else {
                continue;
            };
            statement = Some(id);
            words.args[at] = text;
            words.arg_tokens[at] = Token::new(TokenType::Str, token.span);
            if let Some(single) = words.arg_single.get_mut(at) {
                *single = true;
            }
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
            cmd_name: call.cmd_name.to_owned(),
            cmd_tok: call.cmd_tok,
            args: call.args.to_vec(),
            arg_tokens: call.arg_tokens.to_vec(),
            arg_single: call.arg_single.to_vec(),
            arg_expand_in: call.arg_expand_in.to_vec(),
            scope_path: call.scope_path.to_vec(),
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
        let index = WordIndex::build(cu);
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
        let marks = (
            self.result.diagnostics.len(),
            self.dsl_gate_sites.len(),
            self.pending_arity.len(),
        );
        let cmd_name = site.cmd_name.as_str();
        self.emit_binary_field_version_gates(
            cmd_name,
            site.cmd_tok,
            &words.args,
            &words.arg_tokens,
            &words.arg_single,
        );
        self.record_dsl_format_sites(cmd_name, site.cmd_tok, &words.args, &words.arg_tokens);
        self.emit_w121_invalid_subnet_mask(&words.args, &words.arg_tokens);
        self.emit_w303_redos(cmd_name, &words.args, &words.arg_tokens);
        self.emit_w127_closed_value_args(cmd_name, &words.args, &words.arg_tokens, site.cmd_tok);
        self.emit_w127_closed_option_values(cmd_name, &words.args, &words.arg_tokens, site.cmd_tok);
        self.emit_w146_literal_argument_validation(
            cmd_name,
            &words.args,
            &words.arg_tokens,
            &words.arg_single,
            site.arg_expand_in.get(1..).unwrap_or(&[]),
            &site.scope_path,
        );
        if words.at.first() == Some(&0) {
            self.emit_w145_for_proven_word(
                cmd_name,
                &words.args[0],
                site.cmd_tok,
                &words.arg_tokens,
            );
        }
        // W145 at an option word, from the option walk W004 shares.
        self.emit_w004_dialect_invalid_option(
            cmd_name,
            &words.args,
            &words.arg_tokens,
            site.arg_expand_in.get(1..).unwrap_or(&[]),
            &site.scope_path,
        );
        let at_proven = |code: DiagCode, span: Span| {
            PROVEN_CODES.contains(&code) && words.proven.contains(&span)
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
        let relations = self.proven_option_relations(&super::validity::ProvenCall {
            cmd_name,
            cmd_tok: site.cmd_tok,
            written: (&site.args, &site.arg_tokens),
            proven: (&words.args, &words.arg_tokens),
            arg_expand_in: &site.arg_expand_in,
            scope_path: &site.scope_path,
        });
        self.settle_builtin_verdicts(facts, relations);
        self.rerun_index_checks(site, words);
    }

    /// W230 and W232 over the proven words: an index the written list or
    /// string could not be checked against is checked against the proven
    /// one, and what the written words already drew is not drawn again.
    fn rerun_index_checks(&mut self, site: &ProvenSite, words: &ProvenWords) {
        let numbers = self.grammar().numbers;
        let rules = self.word_rules();
        let run = |args: &[String], tokens: &[Token]| {
            let mut found = super::super::bounds_checks::list_index_diagnostics(
                &site.cmd_name,
                args,
                tokens,
                numbers,
                rules,
            );
            found.extend(super::super::bounds_checks::string_index_diagnostics(
                &site.cmd_name,
                args,
                tokens,
                numbers,
            ));
            found
        };
        let written = run(&site.args, &site.arg_tokens);
        for diagnostic in run(&words.args, &words.arg_tokens) {
            if !written.iter().any(|seen| {
                seen.code == diagnostic.code
                    && seen.span == diagnostic.span
                    && seen.message == diagnostic.message
            }) {
                self.result.diagnostics.push(diagnostic);
            }
        }
    }
}
