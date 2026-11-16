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

//! Which structured control commands the registry's release compiles, and the `errorInfo` body sites that follow from it.

use super::{CfgBuilder, cfg_lower};
use crate::ir::{CommandTokens, Statement};
use tcl_lexer::{Span, TokenType};
use tcl_registry::CommandRegistry;

/// Whether a `switch` lowers to a dispatch chain of `StrEq` branches, one
/// per arm, rather than to one opaque statement: only an exact,
/// case-sensitive one with no fall-through arm, whose subject no option scan
/// may read, does — the registry's release and whether the arms are one list
/// word or pattern and body words decide that. Its selection is then the
/// chain's decided branches; the opaque forms' is the selection record.
pub(crate) fn switch_is_flattened(
    stmt: &Statement,
    registry: &CommandRegistry,
    config: &tcl_lexer::LexerConfig,
) -> bool {
    let Statement::Switch {
        mode, nocase, arms, ..
    } = stmt
    else {
        return false;
    };
    *mode == crate::ir::SwitchMode::Exact
        && !*nocase
        && !arms.iter().any(|arm| arm.fallthrough)
        && !cfg_lower::subject_may_scan_as_option(stmt, registry, config)
}

/// Where the registry's release compiles the control command `command` of
/// `args`, the word texts after it, each braced where `tokens` says so
/// ([`tcl_registry::release_compilation::compiles_control`]); compiled
/// everywhere where the release or the command is not modelled.
pub(crate) fn release_compiles_control(
    registry: &CommandRegistry,
    command: &str,
    args: &[String],
    tokens: Option<&CommandTokens>,
) -> crate::cfg::CompiledAt {
    let Some(release) =
        tcl_registry::value_transfer::TargetSemantics::of(registry.profile()).release
    else {
        return crate::cfg::CompiledAt::ALWAYS;
    };
    let words: Vec<tcl_registry::release_compilation::CompileWord<'_>> = args
        .iter()
        .enumerate()
        .map(|(index, text)| {
            let braced = tokens.is_some_and(|tokens| {
                tokens.argv_kinds.get(index + 1) == Some(&TokenType::Str)
                    && tokens
                        .single_token_word
                        .get(index + 1)
                        .copied()
                        .unwrap_or(false)
            });
            tcl_registry::release_compilation::CompileWord {
                text,
                literal: !tcl_syntax::naming::word_is_dynamic(text, braced),
            }
        })
        .collect();
    let compiles = |in_procedure| {
        tcl_registry::release_compilation::compiles_control(release, command, &words, in_procedure)
            .unwrap_or(true)
    };
    crate::cfg::CompiledAt {
        in_procedure: compiles(true),
        outside_procedure: compiles(false),
    }
}

/// Whether the registry's release compiles the `switch` `stmt` with its
/// compile procedure ([`tcl_registry::release_compilation::compiles_control`]),
/// where its release and command are known.
pub(crate) fn release_compiles_switch(
    stmt: &Statement,
    registry: &CommandRegistry,
) -> Option<bool> {
    let Statement::Switch {
        raw_args,
        raw_arg_braced,
        command,
        ..
    } = stmt
    else {
        return None;
    };
    let release = tcl_registry::value_transfer::TargetSemantics::of(registry.profile()).release?;
    let words: Vec<tcl_registry::release_compilation::CompileWord<'_>> = raw_args
        .iter()
        .enumerate()
        .map(
            |(index, text)| tcl_registry::release_compilation::CompileWord {
                text,
                literal: !tcl_syntax::naming::word_is_dynamic(
                    text,
                    raw_arg_braced.get(index).copied().unwrap_or(false),
                ),
            },
        )
        .collect();
    tcl_registry::release_compilation::compiles_control(release, command, &words, true)
}

impl CfgBuilder<'_> {
    /// Whether this build lowers the `switch` `stmt` to a chain of branches
    /// ([`switch_is_flattened`]); for bytecode, only a form the release
    /// compiles, any other running as the command.
    pub(super) fn flattens_switch(&self, stmt: &Statement) -> bool {
        switch_is_flattened(stmt, self.registry, &self.config)
            && (self.faithful_exceptions
                || release_compiles_switch(stmt, self.registry) != Some(false))
    }

    /// Record the `errorInfo` frame each body of the structured control
    /// command `stmt` adds where the release runs the command rather than
    /// compiling it, and where it compiles it ([`InlineBodyErrorSite`]).
    pub(super) fn record_body_error_sites(&mut self, stmt: &Statement) {
        use crate::cfg::{BodyErrorContext, CompiledAt};
        let mut sites: Vec<(BodyErrorContext, Span)> = Vec::new();
        let compiled = match stmt {
            Statement::Foreach {
                body_span,
                is_lmap,
                is_dict_iteration: false,
                is_array_iteration: false,
                raw_args,
                raw_tokens,
                ..
            } => {
                let command = if *is_lmap { "lmap" } else { "foreach" };
                sites.push((BodyErrorContext::LoopBody(command), *body_span));
                release_compiles_control(self.registry, command, raw_args, raw_tokens.as_ref())
            }
            Statement::While {
                body_span,
                raw_args,
                raw_tokens,
                ..
            } => {
                sites.push((BodyErrorContext::LoopBody("while"), *body_span));
                release_compiles_control(self.registry, "while", raw_args, raw_tokens.as_ref())
            }
            Statement::For {
                init_span,
                next_span,
                body_span,
                raw_args,
                raw_tokens,
                ..
            } => {
                sites.push((
                    BodyErrorContext::Clause("\"for\" initial command"),
                    *init_span,
                ));
                sites.push((
                    BodyErrorContext::Clause("\"for\" loop-end command"),
                    *next_span,
                ));
                sites.push((BodyErrorContext::LoopBody("for"), *body_span));
                release_compiles_control(self.registry, "for", raw_args, raw_tokens.as_deref())
            }
            Statement::If {
                clauses, else_span, ..
            } => {
                sites.extend(
                    clauses
                        .iter()
                        .map(|clause| (BodyErrorContext::Script, clause.body_span)),
                );
                sites.extend(else_span.map(|span| (BodyErrorContext::Script, span)));
                CompiledAt::ALWAYS
            }
            Statement::Switch { arms, .. } if self.flattens_switch(stmt) => {
                sites.extend(arms.iter().filter_map(|arm| {
                    arm.body_span
                        .map(|body| (BodyErrorContext::Arm(arm.pattern_span), body))
                }));
                CompiledAt::ALWAYS
            }
            _ => return,
        };
        let span = stmt.span();
        self.inline_body_error_sites
            .extend(
                sites
                    .into_iter()
                    .map(|(context, region)| crate::cfg::InlineBodyErrorSite {
                        span,
                        context,
                        region,
                        compiled,
                    }),
            );
    }
}
