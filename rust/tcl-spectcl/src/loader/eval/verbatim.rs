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

//! The verbatim skeleton of the file: the `speclib` body, each declaration's body block and the rows a captured invocation matches.

use super::super::{Stmt, Word, block, pack_statements};
use super::{Skeleton, VerbatimBody, VerbatimIndex};
use std::rc::Rc;

impl VerbatimBody {
    pub(super) fn original_for_capture(
        &self,
        head: &str,
        args: &[String],
        actual_body: &str,
    ) -> Option<tcl_compiler::parsing::spectcl_source::OriginalPackSourceWord> {
        // The evaluated body is the braced word's value, in which each
        // backslash-newline has folded to one space; the verbatim text keeps
        // it, and with it the line count (C keeps the lines of a folded
        // continuation for nested bodies, TIP 280's `ContLineLoc`). The two
        // correspond when folding the verbatim text gives the value.
        let verbatim = self.original.as_ref()?.captured_text();
        let folded = tcl_syntax::backslash::collapse_brace_continuations_str_for(
            verbatim,
            tcl_dialect::BraceBackslashNewline::Folds,
        );
        if folded != actual_body {
            return None;
        }
        let args: Vec<String> = args
            .iter()
            .map(|arg| {
                if arg == actual_body {
                    verbatim.to_owned()
                } else {
                    arg.clone()
                }
            })
            .collect();
        self.original
            .as_ref()
            .filter(|original| original.matches_literal_statement_capture(head, &args))
            .cloned()
    }
}

impl VerbatimIndex {
    /// The cheap layer: the first `speclib` block's body.
    pub(super) fn of(source: &str) -> Self {
        let mut index = Self {
            source: Rc::from(source),
            ..Self::default()
        };
        let top = pack_statements(source);
        let Some(statement) = top.iter().find(|stmt| stmt.word_text(0) == "speclib") else {
            return index;
        };
        let Some(body) = statement.arg(3).filter(|word| word.braced) else {
            return index;
        };
        index.speclib = Some(VerbatimBody {
            stmt_line: statement.line,
            text: body.text.clone(),
            body_line: body.line,
            original: (top
                .iter()
                .filter(|row| row.word_text(0) == "speclib" && row.line == statement.line)
                .count()
                == 1)
                .then(|| body.original_source.clone())
                .flatten(),
        });
        index
    }

    /// The declaration skeleton, built on first ask.
    pub(super) fn skeleton(&mut self) -> &mut Skeleton {
        if self.skeleton.is_none() {
            self.skeleton = Some(Skeleton::of(&self.source));
        }
        self.skeleton
            .as_mut()
            .unwrap_or_else(|| unreachable!("just built"))
    }

    /// Take the verbatim body for a `word NAME …` invocation captured at
    /// `line`, when the file has exactly that statement there.
    pub(super) fn take(
        &mut self,
        word: &'static str,
        name: &str,
        line: u32,
    ) -> Option<VerbatimBody> {
        let entries = self.skeleton().bodies.get_mut(&(word, name.to_owned()))?;
        let at = entries.iter().position(|body| body.stmt_line == line)?;
        let unambiguous = entries.iter().filter(|body| body.stmt_line == line).count() == 1;
        if !unambiguous {
            return None;
        }
        Some(entries.remove(at))
    }

    /// The verbatim statement matching a captured `word arg…` invocation at
    /// `line`, when the file has one of the same shape there. A templated
    /// invocation (a loop body, a helper proc) attributes to a line whose
    /// source statement has a different head word or width, so it misses
    /// and keeps its evaluated capture.
    pub(super) fn row(&mut self, word: &str, args: &[String], line: u32) -> Option<Stmt> {
        let entries = self.skeleton().rows.get(&line)?;
        let mut matching = entries
            .iter()
            .filter(|stmt| stmt.word_text(0) == word && stmt.words.len() == args.len() + 1);
        let selected = matching.next()?.clone();
        // Evaluated values cannot borrow a merely equal-width source row.
        // Unavailable/ambiguous correspondence keeps the genuine capture.
        if matching.next().is_some()
            || !selected
                .words
                .iter()
                .skip(1)
                .zip(args)
                .all(|(word, value)| word.text == *value)
        {
            return None;
        }
        Some(selected)
    }
}

impl Skeleton {
    pub(super) fn of(source: &str) -> Self {
        let mut index = Self::default();
        let top = pack_statements(source);
        index.record_rows(&top);
        let Some(speclib) = top.into_iter().find(|stmt| stmt.word_text(0) == "speclib") else {
            return index;
        };
        let Some(body) = speclib.arg(3).filter(|word| word.braced) else {
            return index;
        };
        let body_stmts = block(body);
        index.record_rows(&body_stmts);
        for stmt in &body_stmts {
            // `environment` / `dialect` bodies are evaluated like a
            // `command` body, so they need the same verbatim record: the
            // physical lines their rows report, and the rows themselves for
            // the ones that substitute nothing.
            if let word @ ("environment" | "dialect") = stmt.word_text(0) {
                let word = if word == "environment" {
                    "environment"
                } else {
                    "dialect"
                };
                if let Some(block_body) = stmt.words.iter().skip(2).find(|word| word.braced) {
                    index.record_rows(&block(block_body));
                    index.record(word, stmt.word_text(1), stmt, block_body);
                }
                continue;
            }
            if stmt.word_text(0) != "command" {
                continue;
            }
            let Some(command_body) = stmt.words.iter().skip(2).find(|word| word.braced) else {
                continue;
            };
            let command_stmts = block(command_body);
            index.record_rows(&command_stmts);
            for sub in &command_stmts {
                if sub.word_text(0) != "subcommand" {
                    continue;
                }
                let Some(sub_body) = sub.arg(2).filter(|word| word.braced) else {
                    continue;
                };
                index.record_rows(&block(sub_body));
                index.record("subcommand", sub.word_text(1), sub, sub_body);
            }
            index.record("command", stmt.word_text(1), stmt, command_body);
        }
        index
    }

    fn record_rows(&mut self, stmts: &[Stmt]) {
        for stmt in stmts {
            self.rows.entry(stmt.line).or_default().push(stmt.clone());
        }
    }

    pub(super) fn record(&mut self, word: &'static str, name: &str, stmt: &Stmt, body: &Word) {
        self.bodies
            .entry((word, name.to_owned()))
            .or_default()
            .push(VerbatimBody {
                stmt_line: stmt.line,
                text: body.text.clone(),
                body_line: body.line,
                original: body.original_source.clone(),
            });
    }
}
