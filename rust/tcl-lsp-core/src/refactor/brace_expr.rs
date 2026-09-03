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

//! Brace an `expr` command's arguments — convert `expr "..."` or
//! `expr $a + $b` to the braced `expr {$a + $b}` form for safety and
//! performance.

use tcl_lexer::LexerConfig;
use tcl_registry::CommandRegistry;

use super::{RefactorEdit, Refactoring, find_command_at, token_end_offset};
use crate::code_actions::ActionKind;

/// Convert the `expr` command at `cursor` to braced form, or `None` when the
/// cursor is not on an `expr` command, the expr has no arguments, or it is
/// already braced.
///
/// `config` is the document's [`LexerConfig`] — the command search re-lexes
/// nested bodies under it.
#[must_use]
pub fn brace_expr(
    source: &str,
    cursor: u32,
    registry: &CommandRegistry,
    config: LexerConfig,
) -> Option<Refactoring> {
    let cmd = find_command_at(source, cursor, Some("expr"), registry, config)?;

    // Need at least the command word plus one argument.
    if cmd.argv.len() < 2 {
        return None;
    }

    // The raw source span of every argument after `expr`. The first argument
    // starts the span; the closing delimiter belongs to the final argument, so
    // widen its end via `token_end_offset` (the authoritative word-closer) so an
    // empty trailing `{}` / `[]` / `""` is not overshot.
    let first_arg = cmd.argv.get(1)?;
    let last_arg = *cmd.argv.last()?;
    let raw_start = first_arg.span.start();
    let raw_end = token_end_offset(source, last_arg);
    let raw = source.get(raw_start as usize..raw_end as usize)?;

    // Already braced — nothing to do.
    if raw.starts_with('{') && raw.ends_with('}') {
        return None;
    }

    // Unwrap a quoted expression argument (`expr "$a + $b"` → `expr {$a + $b}`)
    // only when the whole expression is a *single* quoted word (`argv` is
    // `expr` + one arg).  A multi-word expression that merely starts and ends
    // with `"` (`expr "1" + "2"`) must be braced verbatim — stripping the outer
    // quotes there would leave stray inner quotes (`{1" + "2}`) that fail to
    // parse (issue 180).
    let single_quoted_arg =
        cmd.argv.len() == 2 && raw.starts_with('"') && raw.ends_with('"') && raw.len() >= 2;
    let braced = if single_quoted_arg {
        format!("{{{}}}", &raw[1..raw.len() - 1])
    } else {
        format!("{{{raw}}}")
    };

    Some(Refactoring {
        title: "Brace expr for safety and performance".to_owned(),
        edits: vec![RefactorEdit {
            start: raw_start,
            end: raw_end,
            new_text: braced,
        }],
        kind: ActionKind::RefactorRewrite,
        data_group: None,
        disabled: None,
    })
}

#[cfg(test)]
mod tests {
    use tcl_lexer::LineIndex;

    use super::*;

    fn reg() -> CommandRegistry {
        CommandRegistry::build_default()
    }

    /// Byte offset of `(line, character)` for the test cursors below.
    fn offset(source: &str, line: u32, character: u32) -> u32 {
        let li = LineIndex::new(source);
        li.offset_at_utf16(line, tcl_lexer::Utf16Col::new(character), source)
    }

    #[test]
    fn braces_a_quoted_expr() {
        let src = "expr \"$a + $b\"\n";
        let cursor = offset(src, 0, 0);
        let r = brace_expr(src, cursor, &reg(), LexerConfig::default()).expect("expr at cursor");
        assert_eq!(r.title, "Brace expr for safety and performance");
        assert_eq!(r.apply(src), "expr {$a + $b}\n");
        assert_eq!(r.edits.len(), 1);
    }

    #[test]
    fn braces_a_bare_expr() {
        let src = "expr $a + $b\n";
        let r = brace_expr(src, offset(src, 0, 0), &reg(), LexerConfig::default())
            .expect("expr at cursor");
        assert_eq!(r.apply(src), "expr {$a + $b}\n");
    }

    #[test]
    fn multi_arg_expr_bounded_by_quotes_is_braced_verbatim() {
        // `expr "1" + "2"` starts and ends with `"` but is three words, not one
        // quoted expression. It must be braced whole, not quote-unwrapped into
        // the invalid `{1" + "2}` (issue 180).
        let src = "expr \"1\" + \"2\"\n";
        let r = brace_expr(src, offset(src, 0, 0), &reg(), LexerConfig::default())
            .expect("expr at cursor");
        assert_eq!(r.apply(src), "expr {\"1\" + \"2\"}\n");
    }

    #[test]
    fn already_braced_is_none() {
        let src = "expr {$a + $b}\n";
        assert!(brace_expr(src, offset(src, 0, 0), &reg(), LexerConfig::default()).is_none());
    }

    #[test]
    fn off_target_is_none() {
        let src = "set x 5\n";
        assert!(brace_expr(src, offset(src, 0, 0), &reg(), LexerConfig::default()).is_none());
    }
}
