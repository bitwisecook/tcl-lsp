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

//! An invocation's arguments as source words: operand sources, call and case-list words, and the resolver's view over them.

use super::ArgWord;
use crate::ir::CommandTokens;
use std::borrow::Cow;
use tcl_lexer::{LexerConfig, Span, TokenType};
use tcl_registry::hooks::LoweringHookId;
use tcl_registry::value_transfer::{
    AnalysisInputs, CaseArms, FactDomain, FactView, InvocationLayout, OperandView,
    ResolvedInvocationView,
};
use tcl_registry::{
    ArgRole, CommandRegistry, InvocationWord, InvocationWordKind, ResolvedInvocation,
    SemanticOperationId,
};
use tcl_syntax::word_rules::WordValueRules;

/// The commands a typed statement stands for: every registry command whose
/// lowering `hook` produces it, shortest spelling first so the head a
/// consumer resolves through is stable. A typed node records no spelling,
/// so these are what binding validity is asked about.
pub(super) fn typed_node_commands(registry: &CommandRegistry, hook: LoweringHookId) -> Vec<&str> {
    let mut names: Vec<&str> = registry
        .command_names_for_semantic_operation(SemanticOperationId::StructuredLowering(hook))
        .collect();
    names.sort_unstable_by(|a, b| a.len().cmp(&b.len()).then_with(|| a.cmp(b)));
    names
}

/// The source-word classification of a typed `Incr`'s amount: a braced
/// amount is a literal whatever it spells, any other is classified by its
/// text ([`word_of`]).
pub(super) fn amount_word((text, braced): (&str, bool)) -> InvocationWord<'_> {
    if braced {
        InvocationWord::Literal(text)
    } else {
        word_of(text)
    }
}

/// The source-word classification of one operand text: a literal unless
/// it substitutes.
pub(super) fn word_of(text: &str) -> InvocationWord<'_> {
    if text.contains('$') || text.contains('[') {
        InvocationWord::Dynamic
    } else {
        InvocationWord::Literal(text)
    }
}

/// A single-token literal word's value as Tcl substitutes it: a bare or
/// quoted word (`Esc`) under the document's escape grammar, a braced word
/// (`Str`) with its backslash-newlines collapsed. The const-fold engine
/// (`const_subst.rs`) cooks a literal word by this same rule, so a
/// declared route and the engine read one value for one word; `None` for
/// any other token kind. A word's raw spelling is not its value: `"a\tb"`
/// is three characters and `{$x}` is not a read of `x`.
pub(crate) fn literal_token_value<'t>(
    text: &'t str,
    kind: TokenType,
    config: &LexerConfig,
) -> Option<Cow<'t, str>> {
    match kind {
        TokenType::Esc => Some(tcl_lexer::backslash_subst_in(text, config.escapes)),
        TokenType::Str => Some(WordValueRules::from_config(config).collapse_braced_word(text)),
        _ => None,
    }
}

/// The value of a word a case-list statement recorded as `text`, when the
/// source states one: a braced word is its content with backslash-newlines
/// collapsed; a bare or quoted word with nothing to substitute is its escapes
/// decoded under the document's grammar; `None` for a word that substitutes.
/// The one decoder of a recorded word, read by the selection's arguments
/// ([`switch_arguments`]) and by the flattened `switch` chain's operands, so
/// the two cannot read one word two ways: `a\nb` is three characters and
/// `{a\b}` is three characters, whichever way the statement is analysed.
pub(crate) fn recorded_word_value<'t>(
    text: &'t str,
    braced: bool,
    config: &LexerConfig,
) -> Option<Cow<'t, str>> {
    if braced {
        return Some(WordValueRules::from_config(config).collapse_braced_word(text));
    }
    (!substitutes(text)).then(|| tcl_lexer::backslash_subst_in(text, config.escapes))
}

/// Whether a bare or quoted word still has a substitution to make: a `$` or
/// `[` that no backslash escapes, so `a\$b` is the literal `a$b`. A backslash
/// escapes the byte after it, and all three bytes are ASCII, so walking bytes
/// never splits a character.
pub(super) fn substitutes(text: &str) -> bool {
    let mut bytes = text.bytes();
    while let Some(byte) = bytes.next() {
        match byte {
            b'\\' => {
                bytes.next();
            }
            b'$' | b'[' => return true,
            _ => {}
        }
    }
    false
}

/// What the source says about how an operand's word substitutes, for the
/// inputs that answer its structure ([`AnalysisInputs::word_structure`])
/// and a substituted word's value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum OperandSource {
    /// A single braced word: its content is its value.
    BracedLiteral,
    /// A bare word with nothing to substitute — or, in a statement with no
    /// token snapshot, a word taken as spelled: its cooked text is its
    /// value.
    Literal,
    /// A double-quoted word with nothing to substitute: its cooked text is
    /// its value. Apart from [`Self::Literal`] because 9.1b0's byte-compiled
    /// `switch` reads a quoted fall-through body as a command.
    QuotedLiteral,
    /// A bare or quoted word with substitutions, spelled as the source
    /// wrote its content: the value is the parts' values concatenated.
    Substituted,
    /// Nothing is known of the word's quoting: an expansion, a braced
    /// compound, a word respelled after lowering, or no source at all.
    Unknown,
}

impl<'t> ArgWord<'t> {
    /// The word one segmented token stands for: a single-token literal is
    /// its cooked value, a `{*}` word expands, and any other word is
    /// dynamic — substituted from its spelling unless it is a braced
    /// compound, whose spelling is not its source.
    pub(super) fn of_token(
        text: &'t str,
        (kind, quoted): (TokenType, bool),
        single: bool,
        config: &LexerConfig,
    ) -> Self {
        if kind == TokenType::Expand {
            return Self::spelled(text, InvocationWordKind::Expanded, OperandSource::Unknown);
        }
        match literal_token_value(text, kind, config).filter(|_| single) {
            Some(value) => Self {
                text: value,
                kind: InvocationWordKind::Literal,
                source: if kind == TokenType::Str {
                    OperandSource::BracedLiteral
                } else if quoted {
                    OperandSource::QuotedLiteral
                } else {
                    OperandSource::Literal
                },
            },
            None if kind == TokenType::Str => {
                Self::spelled(text, InvocationWordKind::Dynamic, OperandSource::Unknown)
            }
            None => Self::spelled(
                text,
                InvocationWordKind::Dynamic,
                OperandSource::Substituted,
            ),
        }
    }

    pub(super) const fn spelled(
        text: &'t str,
        kind: InvocationWordKind,
        source: OperandSource,
    ) -> Self {
        Self {
            text: Cow::Borrowed(text),
            kind,
            source,
        }
    }

    /// The resolver's view of the word.
    pub(super) fn word(&self) -> InvocationWord<'_> {
        match self.kind {
            InvocationWordKind::Literal => InvocationWord::Literal(&self.text),
            InvocationWordKind::KnownBytes => InvocationWord::KnownBytes(self.text.as_bytes()),
            InvocationWordKind::Dynamic => InvocationWord::Dynamic,
            InvocationWordKind::Expanded => InvocationWord::Expanded,
            InvocationWordKind::Opaque => InvocationWord::Opaque,
        }
    }
}

/// A call statement's arguments as source words. The call's token snapshot
/// says which argument was a braced, bare, or quoted literal; an argument
/// whose spelling differs from its source word (rewritten after lowering)
/// is dynamic. With no source words to consult, a spelling that
/// substitutes is dynamic and one holding a backslash is too: its quoting,
/// and so its value, is unknown.
pub(super) fn call_arguments<'t>(
    args: &'t [String],
    tokens: Option<&CommandTokens>,
    config: &LexerConfig,
) -> Vec<ArgWord<'t>> {
    let source = tokens.filter(|tokens| {
        tokens.synthetic.is_none()
            && tokens.argv_kinds.len() == args.len() + 1
            && tokens.single_token_word.len() == args.len() + 1
            && tokens.argv_texts.len() == args.len() + 1
    });
    args.iter()
        .enumerate()
        .map(|(index, text)| {
            let Some(tokens) = source else {
                return match word_of(text) {
                    InvocationWord::Literal(_) if !text.contains('\\') => {
                        ArgWord::spelled(text, InvocationWordKind::Literal, OperandSource::Literal)
                    }
                    _ => {
                        ArgWord::spelled(text, InvocationWordKind::Dynamic, OperandSource::Unknown)
                    }
                };
            };
            let at = index + 1;
            let expanded = tokens
                .expand_word
                .as_ref()
                .and_then(|flags| flags.get(at))
                .copied()
                .unwrap_or(false);
            if expanded {
                ArgWord::spelled(text, InvocationWordKind::Expanded, OperandSource::Unknown)
            } else if tokens.argv_texts[at] == *text {
                // A quoted word's span opens at its `"`, so it runs past its
                // content; a bare word's is its spelling.
                let span = tokens.argv[at];
                let quoted = tokens.argv_kinds[at] == TokenType::Esc
                    && (span.end() - span.start()) as usize > text.len();
                ArgWord::of_token(
                    text,
                    (tokens.argv_kinds[at], quoted),
                    tokens.single_token_word[at],
                    config,
                )
            } else {
                ArgWord::spelled(text, InvocationWordKind::Dynamic, OperandSource::Unknown)
            }
        })
        .collect()
}

/// A case-list statement's words as its lowering recorded them
/// (`Statement::Switch`'s `raw_args` with the per-word braced and quoted
/// flags, which the caller has checked cover every word), each read by
/// [`recorded_word_value`]: a word with a value is that value, its source
/// saying whether it was braced, quoted or bare; any other word is
/// substituted from its spelling.
pub(super) fn switch_arguments<'t>(
    raw_args: &'t [String],
    (braced, quoted): (&[bool], &[bool]),
    config: &LexerConfig,
) -> Vec<ArgWord<'t>> {
    let flag = |flags: &[bool], index: usize| flags.get(index).copied().unwrap_or(false);
    raw_args
        .iter()
        .enumerate()
        .map(|(index, text)| {
            let braced = flag(braced, index);
            let Some(value) = recorded_word_value(text, braced, config) else {
                return ArgWord::spelled(
                    text,
                    InvocationWordKind::Dynamic,
                    OperandSource::Substituted,
                );
            };
            let source = match (braced, flag(quoted, index)) {
                (true, _) => OperandSource::BracedLiteral,
                (false, true) => OperandSource::QuotedLiteral,
                (false, false) => OperandSource::Literal,
            };
            ArgWord {
                text: value,
                kind: InvocationWordKind::Literal,
                source,
            }
        })
        .collect()
}

/// The command a statement's binding site names: the registry
/// identity the lowering recorded at the statement's own span, when every
/// site there names the same one — the canonical command a
/// `Statement::Switch`, which keeps no resolved name of its own, was lowered
/// for.
pub(super) fn binding_at(cfg: &crate::cfg::Function, span: Span) -> Option<&str> {
    let mut identities = cfg
        .command_binding_sites
        .iter()
        .filter(|site| site.span == span)
        .map(|site| site.binding.identity.as_str());
    let first = identities.next()?;
    identities.all(|other| other == first).then_some(first)
}

/// How many pattern and body pairs a case-list plan reads: the inline
/// form's pairs, or the clause-list word's exact value split under the
/// document's list rules — `None` where that word has no exact value or is
/// not a list of pairs, which the transfer declines too.
pub(super) fn plan_clause_count(
    inputs: &dyn AnalysisInputs,
    arms: &CaseArms,
    rules: WordValueRules,
) -> Option<usize> {
    match arms {
        CaseArms::Words(pairs) => Some(pairs.len()),
        CaseArms::List(list) => {
            let FactView::Exact(value, _) = inputs.operand(*list, FactDomain::ExactValue) else {
                return None;
            };
            let text = String::from_utf8(value.bytes).ok()?;
            let elements = rules.split_list(&text).ok()?.len();
            elements.is_multiple_of(2).then_some(elements / 2)
        }
    }
}

/// The resolver's projection of `resolved` over `texts`: the canonical
/// names, the layout, and the operands with their kinds and roles. Roles
/// come from the effective descriptor — the resolver over literal words,
/// else the static roles — as the resolver's owned facts compute them.
pub(crate) fn view_of<'a>(
    resolved: &ResolvedInvocation<'_, '_>,
    texts: &[&'a str],
    words: &[InvocationWord<'a>],
    layout: InvocationLayout<'a>,
) -> ResolvedInvocationView<'a> {
    let semantics = &resolved.semantics;
    let offset = semantics.argument_offset;
    let literals: Option<Vec<&str>> = words.iter().map(|w| w.literal()).collect();
    let roles: Vec<(u8, ArgRole)> = if semantics.arg_role_layout_resolver.is_some()
        || semantics.arg_role_count_resolver.is_some()
    {
        // A layout resolver reads the structured operands themselves
        // (`unset ?-nocomplain? ?--? name…`), and a count resolver only how
        // many there are (`scan string format ?varName …?`), so the roles
        // are the registry's own projection of this invocation.
        resolved.argument_roles().0
    } else {
        match (semantics.arg_role_resolver, literals) {
            (Some(role_resolver), Some(literals)) => literals
                .get(offset..)
                .map_or_else(|| semantics.arg_roles.to_vec(), role_resolver),
            _ => semantics.arg_roles.to_vec(),
        }
    };
    let role_at = |index: usize| {
        index
            .checked_sub(offset)
            .and_then(|i| u8::try_from(i).ok())
            .and_then(|i| roles.iter().find(|(at, _)| *at == i).map(|(_, role)| *role))
    };
    let operands = texts
        .iter()
        .zip(words)
        .enumerate()
        .map(|(index, (text, word))| OperandView {
            text,
            kind: word.kind(),
            role: role_at(index),
        })
        .collect();
    ResolvedInvocationView {
        canonical_command: resolved.canonical_command,
        subcommand: match resolved.subcommand {
            tcl_registry::SubcommandResolution::Exact(sub)
            | tcl_registry::SubcommandResolution::UniquePrefix(sub) => Some(sub.canonical_name),
            _ => None,
        },
        form: resolved.form.as_ref().map(|form| form.name),
        layout,
        operands,
        argument_offset: offset,
        arity: Some(semantics.arity),
    }
}
