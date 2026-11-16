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

//! The static fast path: running a body, or the whole file, through capture without the interpreter when every statement is a static row.

use super::super::{Stmt, Word, pack_statements};
use super::capture_handlers::{
    capture_stmt, stage_block, stage_command, stage_include, stage_speclib, stage_subcommand,
    track_declared_support,
};
use super::{
    BodyText, Commands, Drive, NEEDS_INTERPRETER, Node, PackNode, ScopeKind, StagedBlock,
    StagedCommand, State, new_state,
};
use std::cell::RefCell;
use std::rc::Rc;
use tcl_spec_hooks::pack_eval;

/// Run one body: when it is the file's verbatim text and every statement at
/// this level is static vocabulary, capture the statements directly —
/// exactly what evaluating them would do, minus the interpreter — and hand
/// anything else to the interpreter whole. The fast path is what keeps a
/// 20k-line declarative pack from paying a 20k-line bytecode compilation
/// for an evaluation that could only ever capture rows.
pub(super) fn run_body(
    drive: Drive<'_, '_>,
    state: &Rc<RefCell<State>>,
    body: &BodyText,
) -> Result<(), String> {
    let original = {
        let state = state.borrow();
        body.original
            .as_ref()
            .filter(|original| match state.original_body_sources.last() {
                Some(None) => false,
                Some(Some(parent)) => original.matches_original_parent(Some(parent)),
                None => original.matches_original_parent(None),
            })
            .cloned()
    };
    state
        .borrow_mut()
        .original_body_sources
        .push(original.clone());
    let outcome = (|| {
        if body.verbatim && drive.fast_path {
            let stmts = super::super::body_statements(&body.text, body.base, original.as_ref());
            if stmts.iter().all(static_stmt) {
                return drive_static(drive, state, &stmts);
            }
        }
        match drive.ctx {
            Some(ctx) => ctx.eval_body(&body.text),
            None => Err(NEEDS_INTERPRETER.to_owned()),
        }
    })();
    state.borrow_mut().original_body_sources.pop();
    outcome
}

/// General Tcl whose dispatch reads or writes interpreter state — a
/// statement headed by one of these must really evaluate.
pub(super) fn is_general_tcl(word: &str) -> bool {
    let parts = tcl_syntax::naming::written_command_parts(word.as_bytes());
    tcl_spec_hooks::SANDBOX_COMMANDS.contains(&word)
        || pack_eval::PACK_EVAL_EXTRA_COMMANDS.contains(&word)
        || parts.absolute
        || parts.qualifiers.first().is_some_and(|part| *part == b"tcl")
}

/// Whether `stmt` is a **static** vocabulary row at its own level: its
/// evaluation could only capture it verbatim. Substitution in any unbraced
/// word, a general-Tcl or denied head word, and `available?` all force the
/// interpreter path.
///
/// The one owner of "this word substitutes": the fast path asks it to skip
/// the interpreter, and `tcl spec upgrade --restyle` asks it (over every
/// statement, nested bodies included) before it may write a snapshot back
/// over its source (E-R12) — a computed word is a program either way.
pub(crate) fn static_stmt(stmt: &Stmt) -> bool {
    let head = stmt.word_text(0);
    if head.is_empty()
        || head == "available?"
        || is_general_tcl(head)
        || pack_eval::denied_axis(head).is_some()
    {
        return false;
    }
    substitution_free(stmt)
}

/// Whether every word of `stmt` means, as written, exactly what evaluating
/// it would yield: a braced word is its own value, and an unbraced one is
/// only when it carries no substitution.
///
/// Two readers share it. The static fast path uses it to decide a statement
/// can be captured without an interpreter at all, and [`State::captured`]
/// uses it to decide the file's own bytes may stand in for the evaluated
/// invocation. Both are the same question — "would replaying the source
/// text reproduce the values?" — and answering it in one place is what
/// stops the fast path and the interpreter path from disagreeing about a
/// row that substitutes.
pub(super) fn substitution_free(stmt: &Stmt) -> bool {
    stmt.words.iter().all(|word| {
        tcl_compiler::parsing::spectcl_source::pack_source_word_is_literal(&word.text, word.braced)
    })
}

/// Capture a run of static statements without the interpreter, recursing
/// into `command`/`subcommand` bodies through the same fast-or-evaluate
/// decision.
pub(super) fn drive_static(
    mut drive: Drive<'_, '_>,
    state: &Rc<RefCell<State>>,
    stmts: &[Stmt],
) -> Result<(), String> {
    for stmt in stmts {
        let at_pack = state.borrow().scopes.is_empty();
        let in_command = matches!(state.borrow().scopes.last(), Some((ScopeKind::Command, _)));
        match stmt.word_text(0) {
            "speclib" => {
                let row = static_row(stmt);
                let args: Vec<String> = row
                    .words
                    .iter()
                    .skip(1)
                    .map(|word| word.text.clone())
                    .collect();
                stage_speclib(drive.reborrow(), state, &args, stmt.line)?;
            }
            "command" if at_pack => {
                if state.borrow().commands == Commands::Skipped {
                    continue;
                }
                let name = stmt.word_text(1).to_owned();
                let overrides = stmt.words.iter().any(|w| w.text == "-override");
                let Some(body) = stmt.words.iter().skip(2).find(|w| w.braced) else {
                    state
                        .borrow_mut()
                        .pack_nodes
                        .push(PackNode::Command(StagedCommand {
                            name,
                            overrides,
                            line: stmt.line,
                            body: None,
                        }));
                    continue;
                };
                let block = BodyText::static_word(body);
                stage_command(drive.reborrow(), state, name, overrides, stmt.line, &block)?;
            }
            "subcommand" if in_command => {
                let name = stmt.word_text(1).to_owned();
                let Some(body) = stmt.arg(2) else {
                    // A `subcommand` with no body word is silently
                    // dropped, on this route as on the interpreter's.
                    continue;
                };
                let block = BodyText::static_word(body);
                stage_subcommand(drive.reborrow(), state, name, stmt.line, &block)?;
            }
            "default" => {
                let row = static_row(stmt);
                let args: Vec<String> = row.words.iter().skip(1).map(|w| w.text.clone()).collect();
                let mut st = state.borrow_mut();
                track_declared_support(&mut st, &args, row.line);
                st.push_node(Node::Row(row));
            }
            "include" if at_pack => {
                let words: Vec<&str> = stmt.words.iter().skip(1).map(|w| w.text.as_str()).collect();
                stage_include(drive.reborrow(), state, &words, stmt.line)?;
            }
            head @ ("environment" | "dialect") if at_pack => {
                let word = if head == "environment" {
                    "environment"
                } else {
                    "dialect"
                };
                // The body is the first braced word past the name, which is
                // index 2 for a declaration and 3 for `-extend`.
                let body_at = stmt
                    .words
                    .iter()
                    .enumerate()
                    .skip(2)
                    .find(|(_, word)| word.braced)
                    .map(|(index, _)| index);
                let head_words: Vec<Word> = stmt
                    .words
                    .iter()
                    .enumerate()
                    .filter(|(index, _)| Some(*index) != body_at)
                    .map(|(_, word)| word.clone())
                    .collect();
                let Some(body_at) = body_at else {
                    state
                        .borrow_mut()
                        .pack_nodes
                        .push(PackNode::Block(StagedBlock {
                            word,
                            head: head_words,
                            line: stmt.line,
                            body: None,
                        }));
                    continue;
                };
                let body = &stmt.words[body_at];
                let block = BodyText::static_word(body);
                stage_block(drive.reborrow(), state, word, head_words, stmt.line, &block)?;
            }
            _ => state.borrow_mut().push_node(Node::Row(static_row(stmt))),
        }
    }
    Ok(())
}

/// A static row as evaluating it captures it.
///
/// A braced word's value is its text with each backslash-newline (and the
/// whitespace after it) folded to one space — Tcl's one substitution inside
/// braces, which every release from 8.4 to 9.1 applies at any brace depth. The
/// CST keeps the continuation in the word's text, so a row whose braced word
/// holds one is captured from its values, exactly as the interpreter captures
/// it: [`VerbatimIndex::row`] matches no source statement whose text differs
/// from the value, so that route falls back to [`capture_stmt`] too. Bodies
/// are still re-read from the verbatim text, which is what keeps their lines.
fn static_row(stmt: &Stmt) -> Stmt {
    fn value(word: &Word) -> std::borrow::Cow<'_, str> {
        if word.braced {
            tcl_syntax::backslash::collapse_brace_continuations_str_for(
                &word.text,
                tcl_dialect::BraceBackslashNewline::Folds,
            )
        } else {
            std::borrow::Cow::Borrowed(word.text.as_str())
        }
    }
    if stmt
        .words
        .iter()
        .all(|word| matches!(value(word), std::borrow::Cow::Borrowed(_)))
    {
        return stmt.clone();
    }
    let values: Vec<String> = stmt
        .words
        .iter()
        .map(|word| value(word).into_owned())
        .collect();
    let Some((head, args)) = values.split_first() else {
        return stmt.clone();
    };
    capture_stmt(head, args, stmt.line)
}

/// Drive the whole file through the static capture layer, with no
/// interpreter behind it.
///
/// `None` means some body — at any depth — was not static vocabulary, so the
/// load must run for real. The attempt is abandoned whole: the partially
/// built state is dropped and [`evaluate_pack_in`] starts again on the
/// interpreter path, which is why this cannot leave half a pack behind. The
/// cost of a failed attempt is one `static_stmt` pass over statements the
/// segmentation memo then hands straight back to the retry (and, for a pack
/// carrying `include` rows, one repeat of the resolver's read).
///
/// Any error abandons the attempt, not only [`NEEDS_INTERPRETER`]: the
/// interpreter path re-runs the whole file and produces the real diagnosis,
/// so falling back is always the safe answer to "this did not work".
pub(super) fn drive_file_statically(
    source: &str,
    include: Option<Rc<super::super::IncludeContext>>,
    commands: bool,
) -> Option<State> {
    // The capture layer segments leniently; an incomplete script (an unclosed
    // brace, bracket or quote) is a real evaluation failure the interpreter
    // path diagnoses.
    if !matches!(
        tcl_lexer::script_image_is_complete(
            &tcl_lexer::SourceImage::native(source.as_bytes()),
            super::super::source_config(super::super::FileBom::Skip),
        ),
        Ok(true)
    ) {
        return None;
    }
    let state = new_state(source, include, commands);
    // `FileBom::Skip`: this is the file entry point, so a leading mark is a
    // prologue rather than part of the first word.
    let top = pack_statements(source);
    if !top.iter().all(static_stmt) {
        return None;
    }
    drive_static(
        Drive {
            ctx: None,
            fast_path: true,
        },
        &state,
        &top,
    )
    .ok()?;
    Rc::try_unwrap(state).ok().map(RefCell::into_inner)
}
