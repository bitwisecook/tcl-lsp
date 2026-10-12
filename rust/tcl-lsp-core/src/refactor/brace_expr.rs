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

use tcl_compiler::analyser::AnalysisResult;

use super::source_rewrite::{RewriteObligation, select};
use super::{RefactorEdit, Refactoring, token_end_offset};
use crate::code_actions::ActionKind;

/// Convert the `expr` command at `cursor` to braced form, or `None` when the
/// cursor is not on an `expr` command, the expr has no arguments, or it is
/// already braced.
///
/// The complete current analysis supplies the actual source grammar and
/// Registry. Original handler selection is independent of the permission to
/// change expression evaluation; an unavailable permission returns no edits.
#[must_use]
pub fn brace_expr(source: &str, cursor: u32, analysis: &AnalysisResult) -> Option<Refactoring> {
    let (cmd, obligation) = select(
        source,
        cursor,
        analysis,
        tcl_registry::hooks::LoweringHookId::Expr,
        RewriteObligation::ExpressionEvaluation,
    )?;

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

    if let Some(obligation) = obligation {
        let walk = super::FrameWalk::new(source, analysis)?;
        let tokens = walk.tokens(source, &cmd);
        let permission = tokens.source_binding.as_ref().and_then(|binding| {
            binding.original_literal_expression_bracing(&tokens, analysis.resolved_registry()?)
        });
        let Some(permission) = permission else {
            return Some(obligation.refusal("Brace expr for safety and performance"));
        };
        if !permission.matches_source(
            &tcl_lexer::SourceImage::document(source),
            analysis.body_lexer_config?,
        ) || !permission.matches_registry(analysis.resolved_registry()?)
        {
            return None;
        }
        let span = permission.original_operand().span();
        return Some(Refactoring {
            title: "Brace expr for safety and performance".to_owned(),
            edits: vec![RefactorEdit {
                start: span.start(),
                end: span.end(),
                new_text: permission.replacement_source_word().to_owned(),
            }],
            kind: ActionKind::RefactorRewrite,
            data_group: None,
            disabled: None,
        });
    }

    // Unwrap a quoted expression argument (`expr "$a + $b"` → `expr {$a + $b}`)
    // only when the whole expression is a *single* quoted word (`argv` is
    // `expr` + one arg).  A multi-word expression that merely starts and ends
    // with `"` (`expr "1" + "2"`) must be braced verbatim — stripping the outer
    // quotes there would leave stray inner quotes (`{1" + "2}`) that fail to
    // parse.
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

    fn run(source: &str, cursor: u32) -> Option<Refactoring> {
        let registry = tcl_registry::CommandRegistry::build_default();
        let analysis = super::super::source_rewrite::lexical_analysis(source, &registry);
        brace_expr(source, cursor, &analysis)
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
        let r = run(src, cursor).expect("expr at cursor");
        assert_eq!(r.title, "Brace expr for safety and performance");
        assert_eq!(r.apply(src), "expr {$a + $b}\n");
        assert_eq!(r.edits.len(), 1);
    }

    #[test]
    fn braces_a_bare_expr() {
        let src = "expr $a + $b\n";
        let r = run(src, offset(src, 0, 0)).expect("expr at cursor");
        assert_eq!(r.apply(src), "expr {$a + $b}\n");
    }

    #[test]
    fn multi_arg_expr_bounded_by_quotes_is_braced_verbatim() {
        // `expr "1" + "2"` starts and ends with `"` but is three words, not one
        // quoted expression. It must be braced whole, not quote-unwrapped into
        // the invalid `{1" + "2}`.
        let src = "expr \"1\" + \"2\"\n";
        let r = run(src, offset(src, 0, 0)).expect("expr at cursor");
        assert_eq!(r.apply(src), "expr {\"1\" + \"2\"}\n");
    }

    #[test]
    fn already_braced_is_none() {
        let src = "expr {$a + $b}\n";
        assert!(run(src, offset(src, 0, 0)).is_none());
    }

    #[test]
    fn off_target_is_none() {
        let src = "set x 5\n";
        assert!(run(src, offset(src, 0, 0)).is_none());
    }
}

#[cfg(test)]
mod original_tests {
    use super::*;

    #[test]
    fn original_brace_expr_keeps_exact_operands_and_declines_missing_equivalence() {
        // Implementation contract: naming.refactor.original-literal-expression-bracing
        // docs/design/analysis/name-resolution-proofs/original-literal-expression-bracing.md
        for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jimtcl"] {
            let source = "expr \"1 + 2\"";
            let mut analysis = tcl_compiler::analyser::Analyser::new().analyse(source, profile);
            analysis.command_invocations.clear();
            analysis.all_procs.clear();
            let action = brace_expr(source, 0, &analysis).unwrap_or_else(|| panic!("{profile}"));
            assert!(
                action.disabled.is_none(),
                "{profile}: {:?}",
                action.disabled
            );
            assert_eq!(action.apply(source), "expr {1 + 2}");
            assert!(brace_expr("# changed\nexpr \"1 + 2\"", 0, &analysis).is_none());
        }
        for source in ["expr \"$value + 2\"", "expr 1 + 2", "expr \"abs(1)\""] {
            let analysis = tcl_compiler::analyser::Analyser::new().analyse(source, "tcl8.6");
            let action = brace_expr(source, 0, &analysis).expect("current expression shape");
            assert!(action.edits.is_empty());
            assert!(
                action
                    .disabled
                    .unwrap()
                    .starts_with("missing-expression-evaluation-equivalence:")
            );
        }
        let source = "proc expr {args} {return SHADOW}; expr \"1 + 2\"";
        let analysis = tcl_compiler::analyser::Analyser::new().analyse(source, "tcl8.6");
        assert!(
            brace_expr(
                source,
                u32::try_from(source.rfind("expr").unwrap()).unwrap(),
                &analysis
            )
            .is_none()
        );
    }
}
