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

//! Inline variable — replace a single-use variable with its value.

use tcl_compiler::analyser::AnalysisResult;
use tcl_compiler::analyser::types::{Scope, VarDef};
use tcl_compiler::segmenter::{SegmentedCommand, segment_commands_with_offset_and_config};
use tcl_lexer::{LexerConfig, LineIndex, Token, TokenType};
use tcl_registry::CommandRegistry;

use super::{
    MAX_COMMAND_SEARCH_DEPTH, RefactorEdit, Refactoring, find_command_at, token_end_offset,
};
use crate::code_actions::ActionKind;

/// Inline the variable defined by the `set` command at byte offset
/// `cursor`.
///
/// Only inlines when the cursor is on a `set var value` command and the
/// variable has exactly one reference after the definition; returns
/// `None` otherwise.
#[must_use]
pub fn inline_variable(
    source: &str,
    cursor: u32,
    analysis: &AnalysisResult,
    registry: &CommandRegistry,
    line_index: &LineIndex,
) -> Option<Refactoring> {
    // The document's own lexing grammar — `analysis.dialect` carries the
    // name the host analysed this document under (issue: dialect-drift).
    let config =
        LexerConfig::from_grammar(crate::environment_for_dialect(&analysis.dialect).grammar());
    let cmd = find_command_at(source, cursor, Some("set"), registry, config)?;
    if cmd.texts.len() < 3 {
        return None; // `set var` read form — nothing to inline
    }
    let var_name = cmd.texts[1].clone();

    // The value word, verbatim from source (widened to include any
    // closing quote / brace / bracket).
    let value_tok = cmd.argv[2];
    let value_start = value_tok.span.start() as usize;
    let value_end = token_end_offset(source, value_tok) as usize;
    let value_text = source.get(value_start..value_end)?;

    // Locate the var definition whose defining line matches the `set`
    // command's line.
    let cmd_line = line_index.line_at(cmd.span.start());
    let var_def = walk_scopes(&analysis.global_scope).into_iter().find(|vd| {
        vd.name == var_name && line_index.line_at(vd.definition_span.start()) == cmd_line
    })?;

    // Only inline when used exactly once.
    if var_def.references.len() != 1 {
        return None;
    }
    let ref_span = var_def.references[0];

    // Delete the `set` command, and with it the whole line when the
    // command is alone on one: the leading indentation belongs to the
    // deleted line, so leaving it behind would push the following line
    // out by that much.
    let (mut cmd_start, mut cmd_end) = super::command_span_offsets(source, &cmd);
    let line_start = source[..cmd_start as usize]
        .rfind('\n')
        .map_or(0, |nl| nl + 1);
    if source
        .get(line_start..cmd_start as usize)?
        .trim()
        .is_empty()
    {
        cmd_start = u32::try_from(line_start).ok()?;
    }
    if source.as_bytes().get(cmd_end as usize) == Some(&b'\n') {
        cmd_end += 1;
    }
    let delete = RefactorEdit {
        start: cmd_start,
        end: cmd_end,
        new_text: String::new(),
    };

    // Resolve the reference's VAR token + enclosing word so we know,
    // from the tokens alone, whether the reference is a standalone word
    // or interpolated inside a larger word.
    let ctx = reference_token(source, ref_span.start(), registry, config)?;

    // The `$var` / `${var}` span to replace.  The VAR token starts at
    // `$`; its end omits the braced form's `}`, so re-add it.
    let ref_start = ctx.var_tok.span.start();
    let mut ref_end = token_end_offset(source, ctx.var_tok);
    if source
        .get(ref_start as usize..ref_start as usize + 2)
        .is_some_and(|s| s == "${")
        && source.as_bytes().get(ref_end as usize) == Some(&b'}')
    {
        ref_end += 1;
    }

    // A standalone word keeps the value verbatim (preserving quotes /
    // braces so it stays one word).  Interpolated inside a larger word,
    // splice the unquoted content instead — keeping the delimiters would
    // yield `"hello "world""`.  A bare (unquoted) concatenation can only
    // take a value with no whitespace.
    let new_text = if ctx.word_is_single {
        value_text.to_owned()
    } else {
        let inner = dequote(value_text);
        // Splicing a brace-quoted (literal) value into an interpolated context
        // (a bare concatenation or inside `"…"`) would ACTIVATE any `$`/`[`/`\`
        // that were literal inside the braces: `set x {a $b}` inlined into
        // `puts "v: $x"` must not turn `$b` into a live substitution.
        // Decline the refactor rather than change the value.
        let was_braced = value_text.trim_start().starts_with('{');
        if was_braced && inner.bytes().any(|b| matches!(b, b'$' | b'[' | b'\\')) {
            return None;
        }
        let in_quoted_string = source.as_bytes().get(ctx.word_start as usize) == Some(&b'"');
        if !in_quoted_string && inner.chars().any(char::is_whitespace) {
            return None;
        }
        inner.to_owned()
    };

    let replace = RefactorEdit {
        start: ref_start,
        end: ref_end,
        new_text,
    };

    Some(Refactoring {
        title: format!("Inline variable '${var_name}'"),
        edits: vec![delete, replace],
        kind: ActionKind::RefactorInline,
        data_group: None,
        disabled: None,
    })
}

/// Resolved reference context — the VAR token, whether it is its own
/// word, and the enclosing word's start offset.
struct RefContext {
    var_tok: Token,
    word_is_single: bool,
    word_start: u32,
}

/// Resolve `ref_off` through the segmenter
/// to its VAR token and enclosing word.
fn reference_token(
    source: &str,
    ref_off: u32,
    registry: &CommandRegistry,
    config: LexerConfig,
) -> Option<RefContext> {
    let cmd = find_command_at(source, ref_off, None, registry, config)?;
    resolve_in_command(source, &cmd, ref_off, config, 0)
}

/// Resolve `ref_off` against `cmd`'s own tokens, descending into the
/// command substitution that holds it when it is not one of them.
///
/// `set r [f $x]` reaches `$x` only through the `[f $x]` word, and it is
/// the *inner* command's words that decide how the value may be spliced:
/// `$x` is a whole word of `f`, so the value keeps its own quoting.
fn resolve_in_command(
    source: &str,
    cmd: &SegmentedCommand,
    ref_off: u32,
    config: LexerConfig,
    depth: u32,
) -> Option<RefContext> {
    let var_tok = cmd.all_tokens.iter().copied().find(|tok| {
        tok.kind == TokenType::Var
            && tok.span.start() <= ref_off
            && ref_off <= token_end_offset(source, *tok)
    });

    let Some(var_tok) = var_tok else {
        if MAX_COMMAND_SEARCH_DEPTH.exceeded(depth) {
            return None;
        }
        let (start, end) = substitution_interior(cmd, ref_off)?;
        let interior = source.get(start as usize..end as usize)?;
        let inner = segment_commands_with_offset_and_config(interior, start, config)
            .into_iter()
            .find(|c| c.span.start() <= ref_off && ref_off <= c.span.end())?;
        return resolve_in_command(source, &inner, ref_off, config, depth + 1);
    };

    for (word_tok, &is_single) in cmd.argv.iter().zip(cmd.single_token_word.iter()) {
        let word_start = word_tok.span.start();
        let word_end = token_end_offset(source, *word_tok);
        if word_start <= var_tok.span.start() && var_tok.span.start() < word_end {
            return Some(RefContext {
                var_tok,
                word_is_single: is_single,
                word_start,
            });
        }
    }
    Some(RefContext {
        var_tok,
        word_is_single: true,
        word_start: var_tok.span.start(),
    })
}

/// Byte range of the interior (brackets excluded) of `cmd`'s command
/// substitution containing `ref_off`.
fn substitution_interior(cmd: &SegmentedCommand, ref_off: u32) -> Option<(u32, u32)> {
    cmd.all_tokens
        .iter()
        .filter(|tok| tok.kind == TokenType::Cmd)
        .map(|tok| {
            (
                tok.span.start() + u32::from(tok.content_offset),
                tok.span.end(),
            )
        })
        .find(|&(start, end)| start <= ref_off && ref_off < end)
}

/// Strip one layer of matching `"…"` or `{…}` delimiters.
fn dequote(value: &str) -> &str {
    let bytes = value.as_bytes();
    if bytes.len() >= 2 {
        if bytes[0] == b'"' && bytes[bytes.len() - 1] == b'"' {
            return &value[1..value.len() - 1];
        }
        if bytes[0] == b'{' && bytes[bytes.len() - 1] == b'}' {
            return &value[1..value.len() - 1];
        }
    }
    value
}

/// Yield every `VarDef` in `scope` and its descendants.
fn walk_scopes(scope: &Scope) -> Vec<&VarDef> {
    walk_scopes_at_depth(scope, 0)
}

fn walk_scopes_at_depth(scope: &Scope, depth: u32) -> Vec<&VarDef> {
    if crate::MAX_SCOPE_WALK_DEPTH.exceeded(depth) {
        return Vec::new();
    }
    let mut out: Vec<&VarDef> = scope.variables.values().collect();
    for child in &scope.children {
        out.extend(walk_scopes_at_depth(child, depth + 1));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fmt::Write as _;
    use tcl_compiler::analyser::Analyser;

    fn run(source: &str, cursor: u32) -> Option<String> {
        let reg = super::super::test_registry();
        let analysis = Analyser::new().analyse(source, "tcl8.6").clone();
        let li = LineIndex::new(source);
        inline_variable(source, cursor, &analysis, &reg, &li).map(|r| r.apply(source))
    }

    /// `walk_scopes_at_depth` recurses once per nested namespace / proc
    /// scope, and `MAX_SCOPE_WALK_DEPTH` (`crate::lib`) is what stops that
    /// recursion from overflowing `cargo test`'s bare ~2 MiB per-test
    /// stack: unguarded, it goes over at around 100 levels. The assertion
    /// is that 80 levels return at all, not what they return.
    #[test]
    fn deeply_nested_namespaces_survive_scope_walk() {
        const DEPTH: usize = 80;
        let mut source = String::new();
        for i in 0..DEPTH {
            let _ = writeln!(source, "namespace eval ns{i} {{");
        }
        source.push_str("set x 1\nputs $x\n");
        for _ in 0..DEPTH {
            source.push_str("}\n");
        }
        let analysis = Analyser::new().analyse(&source, "tcl8.6").clone();
        let _ = walk_scopes(&analysis.global_scope);
    }

    #[test]
    fn inline_single_use() {
        let source = "set name \"hello\"\nputs $name";
        let applied = run(source, 0).expect("result");
        assert!(!applied.contains("set name"), "{applied:?}");
        assert!(applied.contains("\"hello\""), "{applied:?}");
    }

    #[test]
    fn inline_braced_reference() {
        let source = "set x 1\nputs ${x}";
        assert_eq!(run(source, 0).as_deref(), Some("puts 1"));
    }

    #[test]
    fn does_not_inline_braced_literal_with_subst_into_quotes() {
        // `set x {a $b}` holds the LITERAL `a $b`. Inlining it
        // into `"v: $x"` would turn `$b` into a live substitution, so the
        // refactor must decline rather than corrupt the value.
        let source = "set x {a $b}\nputs \"v: $x\"";
        assert_eq!(run(source, 0), None);
        // A `[cmd]` in a braced literal is equally unsafe.
        let source2 = "set x {a [cmd]}\nputs \"v: $x\"";
        assert_eq!(run(source2, 0), None);
        // FP-guard: a braced literal WITHOUT substitution chars still inlines.
        let source3 = "set x {a b}\nputs \"v: $x\"";
        assert_eq!(run(source3, 0).as_deref(), Some("puts \"v: a b\""));
    }

    #[test]
    fn inline_preserves_following_line_without_trailing_newline() {
        let source = "set x 1\nset y $x";
        assert_eq!(run(source, 0).as_deref(), Some("set y 1"));
    }

    #[test]
    fn inline_into_command_substitution() {
        // The only use sits inside `[…]`, so the reference resolves through
        // the substitution's own command.
        let source = "set timeout 30\nset result [http::geturl $url -timeout $timeout]";
        assert_eq!(
            run(source, 0).as_deref(),
            Some("set result [http::geturl $url -timeout 30]")
        );
    }

    #[test]
    fn inline_into_nested_command_substitution() {
        let source = "set n 2\nputs [format %d [string repeat x $n]]";
        assert_eq!(
            run(source, 0).as_deref(),
            Some("puts [format %d [string repeat x 2]]")
        );
    }

    #[test]
    fn no_inline_of_a_reference_inside_a_braced_word() {
        // `expr {$n + 1}` holds `$n` in a braced literal, which `expr`
        // substitutes itself; the segmenter has no VAR token there, so
        // there is no reference span to rewrite.
        let source = "set n 2\nputs [expr {$n + 1}]";
        assert_eq!(run(source, 0), None);
    }

    #[test]
    fn inline_keeps_the_following_line_indentation() {
        // Deleting the `set` line must take its indentation with it.
        let source = "proc p {} {\n    set timeout 30\n    puts $timeout\n}";
        assert_eq!(
            run(source, 16).as_deref(),
            Some("proc p {} {\n    puts 30\n}")
        );
    }

    #[test]
    fn no_inline_multiple_uses() {
        assert!(run("set x 42\nputs $x\nputs $x", 0).is_none());
    }

    #[test]
    fn no_inline_read_form() {
        assert!(run("set x", 0).is_none());
    }

    #[test]
    fn no_inline_non_set() {
        assert!(run("puts \"hello\"", 0).is_none());
    }
}
