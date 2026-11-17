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

//! The text of a body spliced on the live compile with the splice's renames
//! written into it, so a native selection's replay reads the variables the
//! splice reads, as the same text inlined by hand would.
//!
//! A read is renamed at the span its invocation recorded it at; a name a
//! command writes, at the one literal word of the command that spells it. A
//! name with no such word, or two, leaves the text unwritten and the site a
//! call. The caller checks the text against the IR rename before using it.

use std::collections::HashMap;

use tcl_compiler::ir::{CommandTokens, Script, Statement, WordExpr};
use tcl_lexer::{LexerConfig, Span};

/// `body` with every variable `script` (its lowering, offsets relative to the
/// body) reads or writes renamed by `names`.
pub(super) fn renamed_text(
    body: &str,
    script: &Script,
    names: &HashMap<String, String>,
    config: LexerConfig,
) -> Option<String> {
    let mut edits: Vec<(Span, String)> = Vec::new();
    let mut complete = true;
    walk(script, &mut |script, statement| {
        let tokens = statement.and_then(|statement| tokens_of(script, statement));
        if let Some(statement) = statement {
            for name in written_names(statement) {
                let renamed = super::rename::rename_written(name, names, config);
                if renamed == name {
                    continue;
                }
                match tokens.and_then(|tokens| written_word(tokens, name)) {
                    Some(span) => edits.push((span, renamed)),
                    None => complete = false,
                }
            }
        }
        let reads = statement.and_then(Statement::tokens).into_iter().chain(
            script
                .command_binding_sites
                .iter()
                .filter_map(|site| site.source_tokens.as_deref()),
        );
        for tokens in reads {
            for access in &tokens.variable_accesses {
                let spelling = &access.original_spelling;
                let renamed = super::rename::rename_reference(spelling, names, config);
                if renamed == *spelling {
                    continue;
                }
                let span = access.source.span;
                if body.get(span.start() as usize..span.end() as usize) != Some(spelling.as_str()) {
                    complete = false;
                    continue;
                }
                edits.push((span, renamed));
            }
        }
    });
    if !complete {
        return None;
    }
    edits.sort_by_key(|(span, _)| (span.start(), span.end()));
    edits.dedup();
    let mut text = String::with_capacity(body.len());
    let mut at = 0usize;
    for (span, renamed) in edits {
        let start = span.start() as usize;
        if start < at {
            return None;
        }
        text.push_str(body.get(at..start)?);
        text.push_str(&renamed);
        at = span.end() as usize;
    }
    text.push_str(body.get(at..)?);
    Some(text)
}

/// Visit each script once with no statement, for its binding sites, then each
/// of its statements, nested scripts included.
fn walk<'a>(script: &'a Script, visit: &mut impl FnMut(&'a Script, Option<&'a Statement>)) {
    let mut pending = vec![script];
    while let Some(script) = pending.pop() {
        visit(script, None);
        for statement in &script.statements {
            visit(script, Some(statement));
            pending.extend(statement.child_scripts());
        }
    }
}

/// The invocation a statement was lowered from: its own tokens, or a typed
/// operation's binding site at the same span.
fn tokens_of<'a>(script: &'a Script, statement: &'a Statement) -> Option<&'a CommandTokens> {
    statement.tokens().or_else(|| {
        script
            .command_binding_sites
            .iter()
            .find(|site| site.span == statement.span())
            .and_then(|site| site.source_tokens.as_deref())
    })
}

/// The names a statement writes in its own frame.
fn written_names(statement: &Statement) -> Vec<&str> {
    match statement {
        Statement::AssignConst { name, .. }
        | Statement::AssignValue { name, .. }
        | Statement::AssignExpr { name, .. }
        | Statement::Incr { name, .. } => vec![name.as_str()],
        Statement::Call { defs, .. } => defs.iter().map(String::as_str).collect(),
        Statement::Foreach { iterators, .. } => iterators
            .iter()
            .flat_map(|iterator| iterator.vars.iter().map(String::as_str))
            .collect(),
        Statement::Catch {
            result_var,
            options_var,
            ..
        } => result_var
            .iter()
            .chain(options_var)
            .map(String::as_str)
            .collect(),
        Statement::Try { handlers, .. } => handlers
            .iter()
            .flat_map(|handler| handler.var_name.iter().chain(&handler.options_var))
            .map(String::as_str)
            .collect(),
        _ => Vec::new(),
    }
}

/// The span of the one bare literal argument word of `tokens` that spells
/// `name`.
fn written_word(tokens: &CommandTokens, name: &str) -> Option<Span> {
    let mut spans = tokens.words().iter().skip(1).filter_map(|word| match word {
        WordExpr::Literal { text, source } if text == name => Some(source.span),
        _ => None,
    });
    let span = spans.next()?;
    spans.next().is_none().then_some(span)
}
