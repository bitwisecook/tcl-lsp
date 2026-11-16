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

//! The command a statement runs when its head is static, dynamic or a `$var` resolved from reaching definitions.

use super::super::{
    CommandHeadContext, CommandTokens, DiagCode, HashMap, SinkWarningContext, Span, SsaFunction,
    SsaStatement, Statement, TaintLattice, TaintWarning, ValueKey, WordExpr, WordPart,
    decoded_text_fragment,
};
use super::isolated_scripts::emit_isolated_script_warnings;
use super::source_reads::{original_scalar_reference_name, reference_root_with_config};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in super::super) struct StaticCommandHead {
    pub(in super::super) command: String,
    prepended_args: Vec<String>,
    /// Number of source argument words consumed while leading empty `{*}`
    /// expansions were skipped before finding the runtime command word.
    consumed_source_args: usize,
    expanded: bool,
}

impl StaticCommandHead {
    pub(super) fn affects_args(&self) -> bool {
        self.expanded || self.consumed_source_args != 0 || !self.prepended_args.is_empty()
    }

    pub(in super::super) fn effective_args(&self, call_args: &[String]) -> Option<Vec<String>> {
        self.affects_args().then(|| {
            let mut args = self.prepended_args.clone();
            args.extend_from_slice(
                call_args
                    .get(self.consumed_source_args..)
                    .unwrap_or_default(),
            );
            args
        })
    }
}

#[derive(Debug)]
pub(super) struct StaticInvocation {
    pub(super) head: StaticCommandHead,
    pub(in super::super) effective_args: Option<Vec<String>>,
}

impl StaticInvocation {
    pub(super) fn sink_tokens<'a>(
        &self,
        tokens: Option<&'a CommandTokens>,
    ) -> Option<&'a CommandTokens> {
        self.effective_args.is_none().then_some(tokens).flatten()
    }
}

pub(super) fn static_invocation(
    tokens: &CommandTokens,
    call_args: &[String],
    dialect: Option<&'static tcl_dialect::DialectProfile>,
) -> Option<StaticInvocation> {
    let head = static_command_head(
        tokens,
        tcl_dialect::EscapeSyntax::of_profile(dialect),
        tcl_syntax::word_rules::WordValueRules::of_profile(dialect),
    )?;
    let effective_args = head.effective_args(call_args);
    Some(StaticInvocation {
        head,
        effective_args,
    })
}

/// Resolve the first runtime argv element, accounting for static leading
/// `{*}` list expansions. Expansion can both prepend arguments
/// (`{*}{{eval} safe}`) and disappear entirely (`{*}{}`), in which case a
/// later source word becomes the command.
fn static_command_head(
    tokens: &CommandTokens,
    escapes: tcl_dialect::EscapeSyntax,
    rules: tcl_syntax::word_rules::WordValueRules,
) -> Option<StaticCommandHead> {
    static_command_head_from(tokens, escapes, rules, 0)
}

fn static_command_head_from(
    tokens: &CommandTokens,
    escapes: tcl_dialect::EscapeSyntax,
    rules: tcl_syntax::word_rules::WordValueRules,
    start: usize,
) -> Option<StaticCommandHead> {
    for (source_index, word) in tokens.word_exprs.iter().enumerate().skip(start) {
        match word {
            WordExpr::Expand { word, .. } => {
                let value = static_word_expr_value(word, escapes, rules)?;
                let elements = rules.split_list(&value).ok()?;
                let Some((command, rest)) = elements.split_first() else {
                    continue;
                };
                return Some(StaticCommandHead {
                    command: command.to_string(),
                    prepended_args: rest.iter().map(ToString::to_string).collect(),
                    consumed_source_args: source_index,
                    expanded: true,
                });
            }
            _ => {
                return Some(StaticCommandHead {
                    command: static_word_expr_value(word, escapes, rules)?,
                    prepended_args: Vec::new(),
                    consumed_source_args: source_index,
                    expanded: false,
                });
            }
        }
    }
    None
}

/// Exact static value of a command-head word retained by semantic IR.
/// Variable/command substitutions abstain; literal, braced, quoted, and
/// backslash-built heads reduce under Tcl's shared decoder.
pub(in super::super) fn static_word_expr_value(
    word: &WordExpr,
    escapes: tcl_dialect::EscapeSyntax,
    rules: tcl_syntax::word_rules::WordValueRules,
) -> Option<String> {
    match word {
        WordExpr::Literal { text, .. } => Some(text.clone()),
        WordExpr::BracedLiteral { text, .. } => Some(rules.collapse_braced_word(text).into_owned()),
        WordExpr::Template { parts, .. } => {
            let mut value = String::new();
            for part in parts {
                let WordPart::Text { text, .. } = part else {
                    return None;
                };
                value.push_str(&decoded_text_fragment(text, escapes)?);
            }
            Some(value)
        }
        WordExpr::Opaque { text, .. } if !text.contains(['$', '[']) => {
            decoded_text_fragment(text, escapes)
        }
        _ => None,
    }
}

/// A tainted value used directly as a command name is itself code injection,
/// even when no finite constant target can be proven. This also covers the
/// canonical IR produced when a static `eval` body reduces to a dynamic head.
pub(in super::super) fn emit_dynamic_command_head_warning<S: std::hash::BuildHasher>(
    original: Option<(&WordExpr, tcl_lexer::LexerConfig)>,
    statement: &SsaStatement,
    taints: &HashMap<ValueKey, TaintLattice, S>,
    warnings: &mut Vec<TaintWarning>,
    ssa: &SsaFunction,
) {
    let Some((word, config)) = original else {
        return;
    };
    let Some((reference, _)) = word.sole_variable_substitution() else {
        return;
    };
    let Some(name) = reference_root_with_config(reference, config) else {
        return;
    };
    let Some(symbol) = ssa.var_symbol(name) else {
        return;
    };
    let Some(&version) = statement.uses.get(&symbol) else {
        return;
    };
    if !taints
        .get(&(symbol, version))
        .copied()
        .is_some_and(TaintLattice::is_tainted)
    {
        return;
    }
    let span = word.source().span;
    warnings.push(TaintWarning {
        source_sink: None,
        span,
        variable: name.to_owned(),
        sink_command: "dynamic command dispatch".to_owned(),
        code: DiagCode::T100,
        message: format!(
            "Tainted variable ${name} is used as a dynamic command name; possible code injection"
        ),
        replacement: None,
        fixes: Vec::new(),
    });
}

/// Classify commands executed while substituting a dynamic command head.
pub(super) fn emit_command_head_substitution_warnings<
    S: std::hash::BuildHasher,
    H: std::hash::BuildHasher,
>(
    tokens: Option<&CommandTokens>,
    ssa_stmt: &SsaStatement,
    taints: &HashMap<ValueKey, TaintLattice, S>,
    warnings: &mut Vec<TaintWarning>,
    ssa: &SsaFunction,
    context: &SinkWarningContext<'_, H>,
    command_head_ctx: Option<CommandHeadContext<'_>>,
) {
    let mut substitutions = Vec::new();
    if let Some(word) = tokens.and_then(|tokens| tokens.word_exprs.first()) {
        command_substitutions_in_word(word, &mut substitutions);
    }
    for (substitution, source_span) in substitutions {
        let Some(inner) = substitution
            .trim()
            .strip_prefix('[')
            .and_then(|text| text.strip_suffix(']'))
        else {
            continue;
        };
        emit_isolated_script_warnings(
            inner,
            source_span,
            ssa_stmt,
            taints,
            warnings,
            ssa,
            context,
            command_head_ctx,
        );
    }
}

/// Collect every command substitution executed while constructing a command
/// head. Literal prefixes/suffixes affect only the eventual outer command;
/// they do not suppress the nested commands. A [`WordExpr::BracedLiteral`]
/// has no executable parts and therefore contributes nothing.
pub(in super::super) fn command_substitutions_in_word<'a>(
    word: &'a WordExpr,
    out: &mut Vec<(&'a str, Span)>,
) {
    match word {
        WordExpr::CommandSubstitution { spelling, source } => {
            out.push((spelling, source.span));
        }
        WordExpr::Template { parts, .. } => {
            for part in parts {
                if let WordPart::CommandSubstitution { spelling, source } = part {
                    out.push((spelling, source.span));
                }
            }
        }
        WordExpr::Expand { word, .. } => command_substitutions_in_word(word, out),
        _ => {}
    }
}

/// A string-only taint projection requires actual captured prefix bytes.
/// Unknown prefix values retain the conservative unresolved-call path.
fn retained_static_taint_head(
    binding: &crate::command_binding::SourceInvocationBinding,
) -> Option<StaticCommandHead> {
    let target = binding
        .proved_execution_target()
        .filter(|target| target.registry_backed)?;
    let prepended_args = target
        .prepended
        .iter()
        .map(|word| match word {
            crate::registry_invocation::EffectiveInvocationWord::Literal(value) => {
                Some(value.clone())
            }
            _ => None,
        })
        .collect::<Option<Vec<_>>>()?;
    Some(StaticCommandHead {
        command: target.command.clone(),
        prepended_args,
        consumed_source_args: 0,
        expanded: false,
    })
}

/// Resolve a whole-word `$var` command head from the definitions reaching this
/// exact call site. Literal heads take the allocation-free path. A traced or
/// otherwise unprovable variable head yields no candidate rather than guessing.
pub(in super::super) fn resolve_taint_command_heads(
    written_command: &str,
    stmt: &Statement,
    tokens: Option<&CommandTokens>,
    ctx: Option<CommandHeadContext<'_>>,
) -> Vec<StaticCommandHead> {
    // The lowered invocation owns the point-specific binding. A matching
    // source spelling cannot restore semantics explicitly declined by it.
    if let Some(binding) = tokens.and_then(|tokens| tokens.source_binding.as_ref()) {
        return retained_static_taint_head(binding).into_iter().collect();
    }
    let Some(ctx) = ctx else {
        return vec![StaticCommandHead {
            command: written_command.to_owned(),
            prepended_args: Vec::new(),
            consumed_source_args: 0,
            expanded: false,
        }];
    };
    let Some(actual_tokens) = tokens.or_else(|| stmt.tokens()) else {
        return Vec::new();
    };
    let config = ctx.fu.source_lexer_config();
    if let Some(head) = static_command_head(
        actual_tokens,
        config.escapes,
        tcl_syntax::word_rules::WordValueRules::from_config(&config),
    ) {
        return vec![head];
    }
    let Some(var_name) = actual_tokens
        .word_exprs
        .first()
        .and_then(|word| original_scalar_reference_name(word, config))
    else {
        return Vec::new();
    };
    if ctx.module_traces.has_dynamic_variable_trace
        || ctx.module_traces.traced_variables.contains(var_name)
    {
        return Vec::new();
    }
    let absolute_offset = ctx.fu.abs_span(stmt.span()).start();
    let contributors = crate::value_provenance::known_const_contributors(
        ctx.fu,
        absolute_offset,
        var_name,
        ctx.fu.source_lexer_config(),
    );
    if contributors.is_empty() {
        return Vec::new();
    }
    let expanded = tokens
        .and_then(|tokens| tokens.word_exprs.first())
        .is_some_and(|word| matches!(word, WordExpr::Expand { .. }));
    let escapes = ctx.fu.source_lexer_config().escapes;
    let rules = tcl_syntax::word_rules::WordValueRules::from_config(&ctx.fu.source_lexer_config());
    let mut heads = Vec::with_capacity(contributors.len());
    for contributor in contributors {
        let head = if expanded {
            let Ok(elements) = rules.split_list(&contributor.value) else {
                // This reaching arm raises while expanding and never invokes a
                // command. It must not erase other arms that do reach a sink.
                continue;
            };
            if let Some((command, rest)) = elements.split_first() {
                StaticCommandHead {
                    command: command.to_string(),
                    prepended_args: rest.iter().map(ToString::to_string).collect(),
                    consumed_source_args: 0,
                    expanded: true,
                }
            } else {
                let Some(tokens) = tokens else {
                    continue;
                };
                let Some(mut following) = static_command_head_from(tokens, escapes, rules, 1)
                else {
                    // This arm's command remains dynamic. Retain any other
                    // reaching arm already proven to invoke a concrete sink.
                    continue;
                };
                following.expanded = true;
                following
            }
        } else if contributor.value.is_empty() {
            continue;
        } else {
            StaticCommandHead {
                command: contributor.value,
                prepended_args: Vec::new(),
                consumed_source_args: 0,
                expanded: false,
            }
        };
        if !heads.contains(&head) {
            heads.push(head);
        }
    }
    heads
}
