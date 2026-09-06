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

//! Offset rebasing for a memoised [`FunctionUnit`] (slice 4 offset-invariance).
//!
//! The per-procedure lattice cache keys on a procedure's **body source** (not
//! its position), so a body that is unchanged but *shifted* (lines inserted
//! above it) is a cache hit.  The cached unit's CFG/SSA/SCCP spans are
//! absolute, though, so on such a hit we must shift every span by the
//! difference between the procedure's current definition offset and the offset
//! the cached unit was built at.  The result is then byte-identical to a
//! freshly-built unit at the new position.
//!
//! Only the span-carrying parts of a [`FunctionUnit`] are traversed (every
//! other lattice field — `types`/`taints`/`rendered_props`/`def_use`/
//! `memory_ssa`/SSA phis — is span-free): the CFG block statements +
//! terminators + loop-node spans + inlined-`eval` body spans, the SSA blocks'
//! cloned statements (read for positions by some emitters), and the SCCP
//! constant-branch spans.
//! `ExprNode` carries *relative* offsets anchored to a statement span we shift,
//! so it needs no rebasing — but the absolute `expr_base` / `condition_base`
//! anchors those offsets map through do.

use tcl_lexer::Span;

use crate::cfg::Terminator;
use crate::compilation_unit::FunctionUnit;
use crate::ir::{CommandTokens, Script, Statement, WordExpr, WordPart};

/// Shift every absolute span in `fu` by `delta` bytes (signed).  A no-op for
/// `delta == 0`.
pub(crate) fn rebase_function_unit(fu: &mut FunctionUnit, delta: i64) {
    if delta == 0 {
        return;
    }
    for block in fu.cfg.blocks.values_mut() {
        for stmt in &mut block.statements {
            rebase_statement(stmt, delta);
        }
        if let Some(term) = &mut block.terminator {
            rebase_terminator(term, delta);
        }
    }
    for loop_node in fu.cfg.loop_nodes.values_mut() {
        shift(&mut loop_node.span, delta);
        rebase_statement(&mut loop_node.for_stmt, delta);
    }
    // Inlined-body error sites carry absolute spans too; without shifting them
    // a cache-hit, offset-rebased unit keeps stale offsets for error-region
    // mapping / explorer views — issue 149.
    for site in &mut fu.cfg.inline_body_error_sites {
        shift(&mut site.span, delta);
    }
    for site in &mut fu.cfg.command_binding_sites {
        shift(&mut site.span, delta);
    }
    for site in fu.cfg.command_boundary_sites.values_mut() {
        shift(&mut site.span, delta);
    }
    // SSA holds its own clones of the IR statements; some emitters read spans
    // from them (`stmt.statement.span()`), so rebase those too.
    for block in fu.ssa.blocks.values_mut() {
        for ssa_stmt in &mut block.statements {
            rebase_statement(&mut ssa_stmt.statement, delta);
        }
    }
    for cb in &mut fu.sccp.constant_branches {
        shift_opt(&mut cb.span, delta);
    }
}

fn shift(span: &mut Span, delta: i64) {
    let start = (i64::from(span.start()) + delta).max(0);
    let end = (i64::from(span.end()) + delta).max(0);
    // `start`/`end` are clamped to `>= 0`; spans are `u32` offsets, so a
    // shifted offset past `u32::MAX` is degenerate and clamps to the max.
    let start = u32::try_from(start).unwrap_or(u32::MAX);
    let end = u32::try_from(end).unwrap_or(u32::MAX);
    *span = Span::new(start, end);
}

fn shift_opt(span: &mut Option<Span>, delta: i64) {
    if let Some(span) = span {
        shift(span, delta);
    }
}

/// Shift an absolute expression-text anchor (`expr_base` / `condition_base`)
/// — same clamping as [`shift`].
fn shift_base(base: &mut Option<u32>, delta: i64) {
    if let Some(b) = base {
        let shifted = (i64::from(*b) + delta).max(0);
        *b = u32::try_from(shifted).unwrap_or(u32::MAX);
    }
}

fn rebase_tokens(tokens: &mut Option<CommandTokens>, delta: i64) {
    if let Some(tokens) = tokens {
        for span in &mut tokens.argv {
            shift(span, delta);
        }
        for span in &mut tokens.all_tokens {
            shift(span, delta);
        }
        for word in &mut tokens.word_exprs {
            rebase_word_expr(word, delta);
        }
    }
}

/// Rebase the source sites retained by the structured word compatibility view.
fn rebase_word_expr(word: &mut WordExpr, delta: i64) {
    match word {
        WordExpr::Literal { source, .. }
        | WordExpr::BracedLiteral { source, .. }
        | WordExpr::Variable { source, .. }
        | WordExpr::CommandSubstitution { source, .. }
        | WordExpr::Opaque { source, .. } => shift(&mut source.span, delta),
        WordExpr::Template { parts, source } => {
            shift(&mut source.span, delta);
            for part in parts {
                rebase_word_part(part, delta);
            }
        }
        WordExpr::Expand { source, word } => {
            shift(&mut source.span, delta);
            rebase_word_expr(word, delta);
        }
    }
}

/// Rebase one lexical template part.
fn rebase_word_part(part: &mut WordPart, delta: i64) {
    match part {
        WordPart::Text { source, .. }
        | WordPart::Variable { source, .. }
        | WordPart::CommandSubstitution { source, .. }
        | WordPart::Opaque { source, .. } => shift(&mut source.span, delta),
    }
}

/// Shift every absolute span in `script`'s statements by `delta` bytes.  Used
/// to normalise a procedure body to offset 0 before interning it as a
/// salsa-native [`crate::compilation_unit`] lattice key (and reused for nested
/// `Try` bodies during unit rebasing).
pub fn rebase_script(script: &mut Script, delta: i64) {
    if delta == 0 {
        return;
    }
    for stmt in &mut script.statements {
        rebase_statement(stmt, delta);
    }
    for site in script.command_binding_sites.iter_mut() {
        shift(&mut site.span, delta);
    }
}

fn rebase_terminator(term: &mut Terminator, delta: i64) {
    match term {
        Terminator::Goto { span, .. } | Terminator::Return { span, .. } => shift_opt(span, delta),
        Terminator::Branch {
            span,
            condition_base,
            ..
        } => {
            shift_opt(span, delta);
            shift_base(condition_base, delta);
        }
    }
}

fn rebase_statement(stmt: &mut Statement, delta: i64) {
    match stmt {
        Statement::AssignConst {
            span, value_span, ..
        } => {
            shift(span, delta);
            shift_opt(value_span, delta);
        }
        Statement::Incr { span, .. } | Statement::Return { span, .. } => shift(span, delta),
        Statement::AssignExpr {
            span, expr_base, ..
        }
        | Statement::ExprEval {
            span, expr_base, ..
        } => {
            shift(span, delta);
            shift_base(expr_base, delta);
        }
        Statement::AssignValue { span, tokens, .. }
        | Statement::Call { span, tokens, .. }
        | Statement::Barrier { span, tokens, .. } => {
            shift(span, delta);
            rebase_tokens(tokens, delta);
        }
        Statement::Block {
            span, body, tokens, ..
        }
        | Statement::UpFrame {
            span, body, tokens, ..
        } => {
            shift(span, delta);
            rebase_script(body, delta);
            rebase_tokens(tokens, delta);
        }
        Statement::If { .. } | Statement::Try { .. } | Statement::Switch { .. } => {
            rebase_branching_statement(stmt, delta);
        }
        Statement::For { .. }
        | Statement::While { .. }
        | Statement::Foreach { .. }
        | Statement::Catch { .. } => {
            rebase_loop_statement(stmt, delta);
        }
    }
}

/// Rebase the span-bearing looping/catch statements (`For` / `While` /
/// `Foreach` / `Catch`), extracted from [`rebase_statement`] to keep each
/// function small.
fn rebase_loop_statement(stmt: &mut Statement, delta: i64) {
    match stmt {
        Statement::For {
            span,
            init,
            init_span,
            next,
            next_span,
            condition_span,
            condition_base,
            body,
            body_span,
            raw_tokens,
            ..
        } => {
            shift(span, delta);
            shift(init_span, delta);
            shift(condition_span, delta);
            shift_base(condition_base, delta);
            shift(next_span, delta);
            shift(body_span, delta);
            rebase_script(init, delta);
            rebase_script(next, delta);
            rebase_script(body, delta);
            rebase_tokens(raw_tokens, delta);
        }
        Statement::While {
            span,
            condition_span,
            condition_base,
            body,
            body_span,
            raw_tokens,
            ..
        } => {
            shift(span, delta);
            shift(condition_span, delta);
            shift_base(condition_base, delta);
            shift(body_span, delta);
            rebase_script(body, delta);
            rebase_tokens(raw_tokens, delta);
        }
        Statement::Foreach {
            span,
            body,
            body_span,
            raw_tokens,
            ..
        } => {
            shift(span, delta);
            shift(body_span, delta);
            rebase_script(body, delta);
            rebase_tokens(raw_tokens, delta);
        }
        Statement::Catch {
            span,
            body,
            body_span,
            tokens,
            ..
        } => {
            shift(span, delta);
            shift(body_span, delta);
            rebase_script(body, delta);
            rebase_tokens(tokens, delta);
        }
        _ => unreachable!("rebase_loop_statement called on non-loop statement"),
    }
}

/// Rebase the span-bearing branching statements (`If` / `Try` / `Switch`),
/// extracted from [`rebase_statement`] to keep each function small.
fn rebase_branching_statement(stmt: &mut Statement, delta: i64) {
    match stmt {
        Statement::If {
            span,
            clauses,
            else_body,
            else_span,
        } => {
            shift(span, delta);
            shift_opt(else_span, delta);
            for clause in clauses {
                shift(&mut clause.condition_span, delta);
                shift_base(&mut clause.condition_base, delta);
                shift(&mut clause.body_span, delta);
                rebase_script(&mut clause.body, delta);
            }
            if let Some(body) = else_body {
                rebase_script(body, delta);
            }
        }
        Statement::Try {
            span,
            body,
            body_span,
            handlers,
            finally_body,
            finally_span,
            ..
        } => {
            shift(span, delta);
            shift(body_span, delta);
            shift_opt(finally_span, delta);
            rebase_script(body, delta);
            for handler in handlers {
                shift(&mut handler.body_span, delta);
                rebase_script(&mut handler.body, delta);
            }
            if let Some(finally) = finally_body {
                rebase_script(finally, delta);
            }
        }
        Statement::Switch {
            span,
            subject_span,
            arms,
            default_body,
            default_span,
            ..
        } => {
            shift(span, delta);
            shift(subject_span, delta);
            shift_opt(default_span, delta);
            for arm in arms {
                shift(&mut arm.pattern_span, delta);
                shift_opt(&mut arm.body_span, delta);
                if let Some(body) = &mut arm.body {
                    rebase_script(body, delta);
                }
            }
            if let Some(body) = default_body {
                rebase_script(body, delta);
            }
        }
        _ => unreachable!("rebase_branching_statement called on non-branching statement"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compilation_unit::CompilationUnit;
    use crate::ir::WordExpr;
    use crate::lowering::lower_to_ir;
    use tcl_registry::CommandRegistry;

    /// A cache-hit, offset-rebased unit must shift its inlined-`eval` body
    /// spans along with every other absolute span, or error-region / explorer
    /// consumers see stale offsets (issue 149).
    #[test]
    fn rebase_shifts_inline_body_error_sites() {
        let reg = CommandRegistry::build_default();
        // A static `eval {…}` body is flattened inline and its command span
        // recorded with its registry-described error context.
        let cu = CompilationUnit::build_for("proc f {} { eval { set x 1 } }", &reg, false);
        let mut fu = cu.function("::f").expect("::f built").clone();
        assert!(
            !fu.cfg.inline_body_error_sites.is_empty(),
            "an inlined eval body should record an error-context site",
        );
        assert!(
            !fu.cfg.command_binding_sites.is_empty(),
            "an inlined eval body should retain its structured command binding",
        );
        assert!(
            !fu.cfg.command_boundary_sites.is_empty(),
            "an inlined eval body should retain its runtime replay boundary",
        );
        let before_errors = fu.cfg.inline_body_error_sites.clone();
        let before_bindings = fu.cfg.command_binding_sites.clone();
        let before_boundaries = fu.cfg.command_boundary_sites.clone();
        rebase_function_unit(&mut fu, 100);
        for (before, after) in before_errors.iter().zip(&fu.cfg.inline_body_error_sites) {
            assert_eq!(after.span.start(), before.span.start() + 100);
            assert_eq!(after.span.end(), before.span.end() + 100);
            assert_eq!(after.context, before.context);
        }
        for (before, after) in before_bindings.iter().zip(&fu.cfg.command_binding_sites) {
            assert_eq!(after.span.start(), before.span.start() + 100);
            assert_eq!(after.span.end(), before.span.end() + 100);
            assert_eq!(after.binding, before.binding);
        }
        for (block, before) in before_boundaries {
            let after = &fu.cfg.command_boundary_sites[&block];
            assert_eq!(after.span.start(), before.span.start() + 100);
            assert_eq!(after.span.end(), before.span.end() + 100);
            assert_eq!(after.binding, before.binding);
        }
    }

    #[test]
    fn rebase_shifts_structured_word_sites() {
        let registry = CommandRegistry::build_default();
        let mut module = lower_to_ir("puts prefix-$name-[clock seconds]", &registry);
        let Statement::Call {
            tokens: Some(tokens),
            ..
        } = &module.top_level.statements[0]
        else {
            panic!("lowered puts call should retain command tokens");
        };
        let before_word = tokens.words()[1].source().span;
        let WordExpr::Template { parts, .. } = &tokens.words()[1] else {
            panic!("compound word should retain its template shape");
        };
        let before_part = match &parts[1] {
            WordPart::Variable { source, .. } => source.span,
            _ => panic!("second part should be the variable substitution"),
        };

        rebase_script(&mut module.top_level, 37);
        let Statement::Call {
            tokens: Some(tokens),
            ..
        } = &module.top_level.statements[0]
        else {
            panic!("rebased call should retain command tokens");
        };
        assert_eq!(
            tokens.words()[1].source().span.start(),
            before_word.start() + 37
        );
        let WordExpr::Template { parts, .. } = &tokens.words()[1] else {
            panic!("rebased word should retain template shape");
        };
        let WordPart::Variable { source, .. } = &parts[1] else {
            panic!("rebased second part should remain a variable substitution");
        };
        assert_eq!(source.span.start(), before_part.start() + 37);
    }

    /// The load-bearing invariant behind the per-procedure lattice memo: a
    /// procedure's body, normalised to offset 0, must be **byte-identical**
    /// whether the procedure sits at offset X or at X+delta.
    ///
    /// `compilation_unit::build_for_memoized` normalises with exactly this call
    /// before interning `FnLatticeKey`, so if this does not hold then inserting
    /// a line anywhere above a procedure re-keys it and rebuilds its lattice,
    /// its checks, and its taint cascade — the whole point of the memo is that
    /// a *shift* is free and only a *content* change costs.
    #[test]
    fn offset_zero_body_is_identical_under_a_pure_shift() {
        let reg = CommandRegistry::build_default();
        let src = "proc a {x} {\n    set y $x\n    if {$y > 1} { puts hi } else { puts lo }\n    \
                   foreach i {1 2 3} { incr y $i }\n    return $y\n}\n\
                   proc b {} {\n    set z [expr {1 + 2}]\n    return $z\n}\n";
        let shifted = format!("# a comment line that shifts everything below it\n{src}");
        let base = CompilationUnit::build_for(src, &reg, false);
        let moved = CompilationUnit::build_for(&shifted, &reg, false);

        let at_zero = |cu: &CompilationUnit, qname: &str| {
            let p = cu
                .ir_module
                .procedures
                .get(qname)
                .unwrap_or_else(|| panic!("{qname} lowered"));
            let mut body = p.body.clone();
            rebase_script(&mut body, -i64::from(p.span.start()));
            body
        };
        for qname in ["::a", "::b"] {
            assert_eq!(
                at_zero(&base, qname),
                at_zero(&moved, qname),
                "offset-0 body for {qname} changed under a pure shift — every \
                 procedure below an edit will miss its lattice memo",
            );
        }
    }
}
