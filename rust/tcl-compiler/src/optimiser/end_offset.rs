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

//! O128 — end-offset index rewrite.
//!
//! Rewrites length arithmetic used as a list/string index to Tcl's
//! `end` / `end-N` form: `lindex $L [expr {[llength $L] - 1}]` →
//! `lindex $L end`, `string range $s 0 [expr {[string length $s] - 2}]`
//! → `string range $s 0 end-1`, and so on. The rewrite only fires when
//! the length command's argument is *textually identical* to the
//! command's own list/string operand — otherwise `[llength $A]` used to
//! index `$B` would change semantics.
//!
//! Each statement's source slice is re-segmented (the IR `Statement`
//! carries only argv *texts*, not per-word spans, so the index
//! argument's command-substitution span is recovered by re-tokenising),
//! and nested `[…]` command substitutions are walked recursively.

use tcl_core_types::DiagCode;
use tcl_lexer::{Span, TokenType};

use crate::compilation_unit::CompilationUnit;
use crate::expr_ast::{BinOp, ExprNode};
use crate::expr_parser::parse_expr;
use crate::ir::{Script, Statement};
use crate::segmenter::{SegmentedCommand, segment_commands_with_offset};

use super::{Optimisation, PassContext};

/// Which length builtin produced an end-offset candidate.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum LengthKind {
    /// `llength` — list length.
    Llength,
    /// `string length` — string length.
    Strlen,
}

/// Run the O128 end-offset detection over the whole compilation unit.
pub fn run(ctx: &mut PassContext<'_>, cu: &CompilationUnit) {
    // Copy the source reference so the immutable slice borrow does not
    // conflict with the `&mut ctx` report calls below.
    let source = ctx.source;
    let mut spans: Vec<Span> = Vec::new();
    collect_statement_spans(&cu.ir_module.top_level, &mut spans);
    for proc in cu.ir_module.procedures.values() {
        collect_statement_spans(&proc.body, &mut spans);
    }
    for span in spans {
        let Some(slice) = source.get(span.start() as usize..span.end() as usize) else {
            continue;
        };
        for cmd in segment_commands_with_offset(slice, span.start()) {
            apply_to_command(ctx, &cmd);
        }
    }
}

/// Collect every statement's span, recursing through control-flow bodies
/// so commands nested in `if` / `for` / `while` / `switch` / `try`
/// bodies are reached.
fn collect_statement_spans(script: &Script, out: &mut Vec<Span>) {
    for stmt in &script.statements {
        out.push(stmt.span());
        match stmt {
            Statement::If {
                clauses, else_body, ..
            } => {
                for c in clauses {
                    collect_statement_spans(&c.body, out);
                }
                if let Some(b) = else_body {
                    collect_statement_spans(b, out);
                }
            }
            Statement::For {
                init, next, body, ..
            } => {
                collect_statement_spans(init, out);
                collect_statement_spans(next, out);
                collect_statement_spans(body, out);
            }
            Statement::While { body, .. }
            | Statement::Catch { body, .. }
            | Statement::Foreach { body, .. } => collect_statement_spans(body, out),
            Statement::Try {
                body,
                handlers,
                finally_body,
                ..
            } => {
                collect_statement_spans(body, out);
                for h in handlers {
                    collect_statement_spans(&h.body, out);
                }
                if let Some(fb) = finally_body {
                    collect_statement_spans(fb, out);
                }
            }
            Statement::Switch {
                arms, default_body, ..
            } => {
                for a in arms {
                    if let Some(b) = &a.body {
                        collect_statement_spans(b, out);
                    }
                }
                if let Some(b) = default_body {
                    collect_statement_spans(b, out);
                }
            }
            _ => {}
        }
    }
}

/// Emit O128 for any end-offset index in `cmd`, then recurse into the
/// command's nested `[…]` substitutions.
fn apply_to_command(ctx: &mut PassContext<'_>, cmd: &SegmentedCommand) {
    emit_for_command(ctx, cmd);
    for (idx, tok) in cmd.argv.iter().enumerate() {
        if cmd.single_token_word.get(idx).copied() != Some(true) {
            continue;
        }
        if tok.kind != TokenType::Cmd {
            continue;
        }
        // The word text is the verbatim `[inner]`; re-segment `inner`
        // anchored one byte past the opening `[` so the recovered spans
        // stay absolute.
        let full = &cmd.texts[idx];
        let Some(inner) = full.strip_prefix('[').and_then(|s| s.strip_suffix(']')) else {
            continue;
        };
        let base = tok.span.start() + 1;
        for nested in segment_commands_with_offset(inner, base) {
            apply_to_command(ctx, &nested);
        }
    }
}

/// Emit O128 for each index argument of `cmd` that is length arithmetic
/// over the command's own list/string operand.
fn emit_for_command(ctx: &mut PassContext<'_>, cmd: &SegmentedCommand) {
    let Some((index_positions, container_pos, expected)) = end_offset_command_shape(&cmd.texts)
    else {
        return;
    };
    if container_pos >= cmd.texts.len() || container_pos >= cmd.argv.len() {
        return;
    }
    if cmd.single_token_word.get(container_pos).copied() != Some(true) {
        return;
    }
    if cmd.argv[container_pos].kind != TokenType::Var {
        return;
    }
    // Compare full variable references (`${L}`, `$a(1)`), not normalised
    // base names — `$a(1)` and `$a(2)` are different containers.
    let container_repr = cmd.texts[container_pos].trim();

    for pos in index_positions {
        if pos >= cmd.texts.len() || pos >= cmd.argv.len() {
            continue;
        }
        if cmd.single_token_word.get(pos).copied() != Some(true) {
            continue;
        }
        let idx_tok = &cmd.argv[pos];
        if idx_tok.kind != TokenType::Cmd {
            continue;
        }
        let Some(expr_arg) = expr_arg_from_command_word(&cmd.texts[pos], ctx.registry) else {
            continue;
        };
        let Some((kind, length_arg, offset)) = try_end_offset_from_length_expr(&expr_arg) else {
            continue;
        };
        if kind != expected || length_arg.trim() != container_repr {
            continue;
        }
        let replacement = if offset == 0 {
            "end".to_owned()
        } else {
            format!("end-{offset}")
        };
        // The `Cmd` token span follows the lexer's inner-end convention:
        // `span.end()` is the exclusive offset of the closing `]`, so the
        // full `[…]` substitution is `start..end + 1`.
        let span = Span::new(idx_tok.span.start(), idx_tok.span.end() + 1);
        ctx.report(Optimisation::new(
            DiagCode::O128,
            "Use end-offset index instead of length arithmetic",
            span,
            replacement,
        ));
    }
}

/// Identify list/string commands that accept `end` / `end-N` index args.
/// Returns `(index_positions, container_position, expected_kind)`.
///
/// `linsert` is intentionally excluded (`linsert $L end x` appends, but
/// `linsert $L [expr {[llength $L] - 1}] x` inserts before the last
/// element — no `end`/`end-N` rewrite preserves that). `lindex` with
/// multiple indices resolves later indices against sub-lists, so only the
/// first index (position 2) is safe.
fn end_offset_command_shape(texts: &[String]) -> Option<(Vec<usize>, usize, LengthKind)> {
    let cmd = texts.first()?.as_str();
    let nargs = texts.len();
    match cmd {
        "lindex" if nargs >= 3 => Some((vec![2], 1, LengthKind::Llength)),
        "lrange" if nargs == 4 => Some((vec![2, 3], 1, LengthKind::Llength)),
        "lreplace" if nargs >= 4 => Some((vec![2, 3], 1, LengthKind::Llength)),
        "string" if nargs >= 2 => match texts[1].as_str() {
            "index" if nargs == 4 => Some((vec![3], 2, LengthKind::Strlen)),
            "range" if nargs == 5 => Some((vec![3, 4], 2, LengthKind::Strlen)),
            "replace" if nargs >= 5 => Some((vec![3, 4], 2, LengthKind::Strlen)),
            _ => None,
        },
        _ => None,
    }
}

/// Return the single `expr` argument of a `[expr <arg>]` command word, or
/// `None` for anything else. The word text is the verbatim `[…]`
/// substitution; the head is recognised via the registry's
/// `EXPR_CONCATENATES_ARGS` trait, not a name match.
fn expr_arg_from_command_word(
    word: &str,
    registry: Option<&tcl_registry::CommandRegistry>,
) -> Option<String> {
    let inner = word.strip_prefix('[').and_then(|s| s.strip_suffix(']'))?;
    let cmds = segment_commands_with_offset(inner, 0);
    let [cmd] = cmds.as_slice() else {
        return None;
    };
    if cmd.texts.len() != 2 {
        return None;
    }
    let head_is_expr = registry
        .and_then(|r| r.get(&cmd.texts[0]))
        .is_some_and(|s| {
            s.traits
                .contains(tcl_registry::Traits::EXPR_CONCATENATES_ARGS)
        });
    if !head_is_expr {
        return None;
    }
    Some(cmd.texts[1].clone())
}

/// Parse `[llength $L] - N` / `[string length $s] - N` (with `N >= 1`)
/// into `(kind, container_word, end_offset)` where the Tcl end-offset is
/// `N - 1` (`[llength $L] - 1` → `end`). A bare `[llength $L]` (no
/// subtraction) is one past the last index and is rejected.
fn try_end_offset_from_length_expr(expr_text: &str) -> Option<(LengthKind, String, i64)> {
    let node = parse_expr(expr_text.trim(), None);
    let ExprNode::Binary {
        op: BinOp::Sub,
        left,
        right,
    } = node
    else {
        return None;
    };
    let ExprNode::Command { text: cmd_text, .. } = left.as_ref() else {
        return None;
    };
    let ExprNode::Literal { text: rhs, .. } = right.as_ref() else {
        return None;
    };
    let n = rhs.trim().parse::<i64>().ok()?;
    if n < 1 {
        return None;
    }
    if let Some(arg) = parse_length_arg(cmd_text, &["llength"]) {
        return Some((LengthKind::Llength, arg, n - 1));
    }
    if let Some(arg) = parse_length_arg(cmd_text, &["string", "length"]) {
        return Some((LengthKind::Strlen, arg, n - 1));
    }
    None
}

/// If `cmd_text` is `[<head…> <arg>]` (or the same without the brackets)
/// where the leading words equal `head`, return the trailing `<arg>` as
/// it segments (so the spelling matches the container word). Covers both
/// `llength $L` (one head word) and `string length $s` (two).
fn parse_length_arg(cmd_text: &str, head: &[&str]) -> Option<String> {
    let inner = cmd_text
        .strip_prefix('[')
        .and_then(|s| s.strip_suffix(']'))
        .unwrap_or(cmd_text)
        .trim();
    let cmds = segment_commands_with_offset(inner, 0);
    let [cmd] = cmds.as_slice() else {
        return None;
    };
    if cmd.texts.len() != head.len() + 1 {
        return None;
    }
    if cmd.texts[..head.len()]
        .iter()
        .zip(head)
        .any(|(t, h)| t != h)
    {
        return None;
    }
    cmd.texts.last().cloned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interprocedural::InterproceduralAnalysis;
    use tcl_registry::CommandRegistry;

    fn registry() -> CommandRegistry {
        CommandRegistry::build_default()
    }

    fn run_pass(source: &str) -> Vec<Optimisation> {
        let reg = registry();
        let cu = CompilationUnit::build_for(source, &reg, false);
        let mut ctx = PassContext::new(&cu.source, InterproceduralAnalysis::default());
        ctx.registry = Some(&reg);
        run(&mut ctx, &cu);
        ctx.optimisations
    }

    /// The replacement and the exact source slice the rewrite targets.
    fn o128_rewrite(source: &str) -> Option<(String, String)> {
        let reg = registry();
        let cu = CompilationUnit::build_for(source, &reg, false);
        let mut ctx = PassContext::new(&cu.source, InterproceduralAnalysis::default());
        ctx.registry = Some(&reg);
        run(&mut ctx, &cu);
        let opt = ctx
            .optimisations
            .into_iter()
            .find(|o| o.code == DiagCode::O128)?;
        let slice = source
            .get(opt.span.start() as usize..opt.span.end() as usize)?
            .to_owned();
        Some((slice, opt.replacement))
    }

    #[test]
    fn lindex_last_element_rewrites_to_end() {
        let (slice, repl) = o128_rewrite("lindex $L [expr {[llength $L] - 1}]").unwrap();
        assert_eq!(slice, "[expr {[llength $L] - 1}]");
        assert_eq!(repl, "end");
    }

    #[test]
    fn lindex_offset_two_rewrites_to_end_minus_one() {
        let (_slice, repl) = o128_rewrite("lindex $L [expr {[llength $L] - 2}]").unwrap();
        assert_eq!(repl, "end-1");
    }

    #[test]
    fn string_range_last_char_rewrites_to_end() {
        let (slice, repl) =
            o128_rewrite("string range $s 0 [expr {[string length $s] - 1}]").unwrap();
        assert_eq!(slice, "[expr {[string length $s] - 1}]");
        assert_eq!(repl, "end");
    }

    #[test]
    fn lrange_rewrites_both_index_positions() {
        let opts = run_pass("lrange $L [expr {[llength $L] - 3}] [expr {[llength $L] - 1}]");
        let repls: Vec<&str> = opts
            .iter()
            .filter(|o| o.code == DiagCode::O128)
            .map(|o| o.replacement.as_str())
            .collect();
        assert!(repls.contains(&"end-2"), "got {repls:?}");
        assert!(repls.contains(&"end"), "got {repls:?}");
    }

    #[test]
    fn mismatched_container_does_not_rewrite() {
        // The length is of `$A` but the indexed list is `$L` — rewriting
        // to `end` would change semantics.
        let opts = run_pass("lindex $L [expr {[llength $A] - 1}]");
        assert!(
            opts.iter().all(|o| o.code != DiagCode::O128),
            "mismatched container must not rewrite, got {opts:?}",
        );
    }

    #[test]
    fn bare_llength_index_is_not_end_offset() {
        // `[llength $L]` (no subtraction) is one past the last index.
        let opts = run_pass("lindex $L [expr {[llength $L]}]");
        assert!(
            opts.iter().all(|o| o.code != DiagCode::O128),
            "got {opts:?}"
        );
    }

    #[test]
    fn linsert_is_excluded() {
        let opts = run_pass("linsert $L [expr {[llength $L] - 1}] x");
        assert!(
            opts.iter().all(|o| o.code != DiagCode::O128),
            "got {opts:?}"
        );
    }

    #[test]
    fn rewrites_inside_proc_body() {
        let (_slice, repl) =
            o128_rewrite("proc ::f {L} { return [lindex $L [expr {[llength $L] - 1}]] }").unwrap();
        assert_eq!(repl, "end");
    }
}
