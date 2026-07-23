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

//! Expression AST compilation to bytecode.
//!
//! Extends [`CodegenCtx`] with [`CodegenCtx::emit_expr`] which walks an
//! [`ExprNode`] tree and produces the corresponding bytecode
//! instructions.

use tcl_lexer::backslash_subst;

use super::values::{parse_braced_scalar_ref, parse_simple_var_ref};
use super::{CodegenCtx, Op, Operand, bytecode_imm};
use crate::depth_guard::MAX_EXPR_NODE_DEPTH;
use crate::expr_ast::{BinOp, ExprNode, UnaryOp, render_expr};
use crate::tcl_expr_eval::{Env, TclValue, eval_tcl_expr_with_octal};

/// Operators whose constant integer value `eval_tcl_expr` computes soundly and
/// which C Tcl folds at compile time: arithmetic, shift, bitwise, logical, and
/// **numeric** comparison. Deliberately excludes string comparisons (`eq`/`lt`/
/// …) and list membership (`in`/`ni`) — the evaluator has no string value type,
/// so those cannot be folded soundly here — and every iRules dialect operator
/// (`contains`, `and`/`or`, `matches_*`, …), which has no C-Tcl equivalent and
/// must keep its dedicated opcode.
const fn binop_folds(op: BinOp) -> bool {
    matches!(
        op,
        BinOp::Add
            | BinOp::Sub
            | BinOp::Mul
            | BinOp::Div
            | BinOp::Mod
            | BinOp::Pow
            | BinOp::LShift
            | BinOp::RShift
            | BinOp::BitAnd
            | BinOp::BitOr
            | BinOp::BitXor
            | BinOp::And
            | BinOp::Or
            | BinOp::Eq
            | BinOp::Ne
            | BinOp::Lt
            | BinOp::Le
            | BinOp::Gt
            | BinOp::Ge
    )
}

/// Unary operators eligible for constant integer folding (the standard numeric
/// and logical ones; not the iRules word-`!`).
const fn unaryop_folds(op: UnaryOp) -> bool {
    matches!(
        op,
        UnaryOp::Neg | UnaryOp::Pos | UnaryOp::BitNot | UnaryOp::Not
    )
}

impl CodegenCtx<'_> {
    /// Compile an expression AST node; leaves the result on TOS.
    ///
    /// Returns `true` when the result is *guaranteed numeric*
    /// (arithmetic, comparison, logical, bitwise, or literal number),
    /// `false` when it might be a string (variable, ternary, function
    /// call, command subst).
    ///
    /// `tryCvtToNumeric` is **never** emitted by this method — the
    /// caller is responsible for emitting it when the return value is
    /// Emit a `Literal` expression — falls back to exprStk for
    /// invalid prefix literals (`0o289`, `0xGG`) so the runtime
    /// reports the parse error.  Returns `true` (numeric coercion
    /// guaranteed by Tcl literal handling).
    fn emit_expr_literal(&mut self, text: &str) -> bool {
        let clean = text.trim().trim_start_matches(['+', '-']);
        if clean.len() > 1
            && clean.starts_with('0')
            && matches!(
                clean.as_bytes().get(1),
                Some(b'o' | b'O' | b'x' | b'X' | b'b' | b'B')
            )
            && i64::from_str_radix(
                &clean[2..],
                match clean.as_bytes()[1] {
                    b'o' | b'O' => 8,
                    b'x' | b'X' => 16,
                    b'b' | b'B' => 2,
                    _ => 10,
                },
            )
            .is_err()
        {
            self.push_lit(text);
            self.emit(Op::EXPR_STK, vec![]);
            return true;
        }
        // Push the literal as written and report it as *not* guaranteed-canonical
        // so a top-level `expr`/`set =` result runs `tryCvtToNumeric`, which
        // regenerates the number's canonical string (`expr {1e3}` → `1000.0`,
        // `expr {0xff}` → `255`). As an operand of an arithmetic op the raw form
        // is fine — the op coerces — so this only adds a conversion where the
        // literal is itself the whole expression.
        self.push_lit(text);
        false
    }

    /// Emit a `String` expression — strips surrounding `"..."` or
    /// `{...}` delimiters and processes backslash escapes.  Returns
    /// `false` (string isn't necessarily numeric).
    fn emit_expr_string(&mut self, text: &str) -> bool {
        // A brace-delimited operand `{…}` is a literal — no substitution.
        if text.len() >= 2 && text.starts_with('{') && text.ends_with('}') {
            self.push_lit(&text[1..text.len() - 1]);
            return false;
        }
        // A quote-delimited operand `"…"` is a double-quoted word: its `$var` /
        // `[cmd]` / backslash escapes are substituted, so `expr {"item $i"}` is
        // `"item 0"`, not the literal `item $i` (append-5.1, the `$x eq "…$y"`
        // idiom). Decompose it into literal / variable / command parts (the expr
        // text is not pre-normalised to `${name}`, so go through the template
        // parser, which handles bare `$var`) and concat them.
        if text.len() >= 2 && text.starts_with('"') && text.ends_with('"') {
            let inner = &text[1..text.len() - 1];
            // No `$`/`[`: a pure literal. Decode with the *full* backslash decoder
            // (`\xNN`, `\NNN`, `\uNNNN`, …), which the template parser's simplified
            // literal decoding does not cover (expr-8.13's `"\374"`).
            if !inner.contains('$') && !inner.contains('[') {
                self.push_lit(&backslash_subst(inner));
                return false;
            }
            match super::helpers::parse_subst_template(inner) {
                Some(parts) => self.emit_subst_parts(&parts),
                // Unparseable template (e.g. a bare `$` with no name): literal.
                None => self.push_lit(&backslash_subst(inner)),
            }
            return false;
        }
        // Bare text (no delimiters): a literal, backslash-decoded.
        if text.contains('\\') {
            let processed = backslash_subst(text);
            self.push_lit(&processed);
        } else {
            self.push_lit(text);
        }
        false
    }

    /// Emit the parts of a substituted double-quoted word, leaving one value on
    /// the stack: each part pushes its value (literal / variable load / command
    /// substitution), and >1 part is joined with `strConcat`. Mirrors the
    /// command-argument interpolation path, reused for `"…"` expr operands.
    fn emit_subst_parts(&mut self, parts: &[super::helpers::SubstPart]) {
        use super::helpers::SubstPart;
        if parts.is_empty() {
            self.push_lit("");
            return;
        }
        for part in parts {
            match part {
                SubstPart::Lit(t) => self.push_lit(t),
                SubstPart::Var(name) => self.load_var(name),
                SubstPart::Scalar(name) => {
                    self.push_lit(name);
                    self.emit(Op::LOAD_STK, vec![]);
                }
                SubstPart::Cmd(cmd_text) => self.emit_inline_cmd_subst(cmd_text),
            }
        }
        if parts.len() > 1 {
            self.emit(
                Op::STR_CONCAT1,
                vec![Operand::Imm(i32::try_from(parts.len()).unwrap_or(i32::MAX))],
            );
        }
    }

    /// Emit a `Binary` expression — short-circuits `&&` / `||`
    /// (with synthetic labels) and falls back to exprStk for
    /// operators without a direct opcode.
    ///
    /// `depth` is the `Binary` node's own `ExprNode` nesting level; the
    /// operands are emitted one level deeper (issue #996 — `emit_expr`
    /// guards the cap).
    fn emit_expr_binary(
        &mut self,
        node: &ExprNode,
        op: BinOp,
        left: &ExprNode,
        right: &ExprNode,
        depth: u32,
    ) {
        match op {
            BinOp::And => {
                let false_lbl = self.fresh_label("and_f");
                let end_lbl = self.fresh_label("and_end");
                self.emit_expr_at(left, depth + 1);
                self.emit(Op::JUMP_FALSE4, vec![Operand::Label(false_lbl.clone())]);
                self.emit_expr_at(right, depth + 1);
                self.emit(Op::JUMP_FALSE4, vec![Operand::Label(false_lbl.clone())]);
                self.push_lit("1");
                self.emit(Op::JUMP4, vec![Operand::Label(end_lbl.clone())]);
                self.place_label(&false_lbl);
                self.push_lit("0");
                self.place_label(&end_lbl);
            }
            BinOp::Or => {
                let true_lbl = self.fresh_label("or_t");
                let end_lbl = self.fresh_label("or_end");
                self.emit_expr_at(left, depth + 1);
                self.emit(Op::JUMP_TRUE4, vec![Operand::Label(true_lbl.clone())]);
                self.emit_expr_at(right, depth + 1);
                self.emit(Op::JUMP_TRUE4, vec![Operand::Label(true_lbl.clone())]);
                self.push_lit("0");
                self.emit(Op::JUMP4, vec![Operand::Label(end_lbl.clone())]);
                self.place_label(&true_lbl);
                self.push_lit("1");
                self.place_label(&end_lbl);
            }
            _ => {
                if let Some(bc) = Op::from_binop(op) {
                    self.emit_expr_at(left, depth + 1);
                    self.emit_expr_at(right, depth + 1);
                    self.emit(bc, vec![]);
                } else {
                    self.push_lit(&render_expr(node));
                    self.emit(Op::EXPR_STK, vec![]);
                }
            }
        }
    }

    /// `false` and the context requires numeric coercion.
    ///
    /// Public entry point: the top of an expression tree is nesting depth
    /// `0` (issue #996 — the recursion cap lives in [`Self::emit_expr_at`]).
    pub fn emit_expr(&mut self, node: &ExprNode) -> bool {
        self.emit_expr_at(node, 0)
    }

    /// Depth-carrying core of [`Self::emit_expr`] — see that method's
    /// contract. `depth` is this node's `ExprNode` nesting level.
    fn emit_expr_at(&mut self, node: &ExprNode, depth: u32) -> bool {
        // Native-stack safety net (issue #996). Past the cap, stop recursing
        // (and stop the const-folding walk below, which itself descends the
        // whole subtree): push a placeholder so the stack contract — exactly
        // one value left on TOS — still holds, and report a non-canonical
        // (possibly-string) result so any caller stays conservative. Only
        // reachable from an `ExprNode` tree nested past 256 levels, which the
        // Pratt parser never produces (it caps at `MAX_EXPR_DEPTH`, degrading
        // to `Raw`); this guards any test-only or future direct-tree caller.
        if MAX_EXPR_NODE_DEPTH.exceeded(depth) {
            self.push_lit("0");
            return false;
        }
        // Codegen-time constant folding, mirroring C Tcl's compile-time expr
        // folding: a compound subexpression whose value is a constant integer
        // collapses to a single `push`. Recursion makes this fire on constant
        // *subexpressions* too (`$x + (1+2)` → `loadScalar; push "3"; add`),
        // exactly as tclsh does. Only `Binary`/`Unary` integer results are
        // folded — `eval_tcl_expr` over an empty env returns `None` for any
        // node that reads a variable or runs a command, so substitutions are
        // never mis-folded; float results are left to the normal path
        // (their string formatting is the runtime's job). Literals keep their
        // own arm (the caller's `tryCvtToNumeric` / invalid-prefix handling).
        let foldable = match node {
            ExprNode::Binary { op, .. } => binop_folds(*op),
            ExprNode::Unary { op, .. } => unaryop_folds(*op),
            _ => false,
        };
        if foldable
            && let Some(TclValue::Int(i)) = eval_tcl_expr_with_octal(
                node,
                &Env::new(),
                // Profile-built registries answer from the dialect profile
                // (octal in 8.x, decimal in 9.x/bpf, abstain with no Tcl
                // runtime); hand-built ones keep the loaded-packs rule.
                self.registry.octal_fold_policy(),
            )
        {
            self.push_lit(&i.to_string());
            return true;
        }
        match node {
            ExprNode::Literal { text, .. } => self.emit_expr_literal(text),

            ExprNode::String { text, .. } => self.emit_expr_string(text),

            ExprNode::Var { text, name, .. } => {
                let var_ref = if text.contains('(') {
                    text.trim_start_matches('$')
                } else {
                    name.as_str()
                };
                self.load_var(var_ref);
                false
            }

            ExprNode::Binary { op, left, right } => {
                self.emit_expr_binary(node, *op, left, right, depth);
                true
            }

            ExprNode::Unary { op, operand } => {
                if let Some(bc) = Op::from_unaryop(*op) {
                    self.emit_expr_at(operand, depth + 1);
                    self.emit(bc, vec![]);
                } else {
                    self.push_lit(&render_expr(node));
                    self.emit(Op::EXPR_STK, vec![]);
                }
                true
            }

            ExprNode::Ternary {
                condition,
                true_branch,
                false_branch,
            } => {
                let false_lbl = self.fresh_label("tern_f");
                let end_lbl = self.fresh_label("tern_end");
                self.emit_expr_at(condition, depth + 1);
                self.emit(Op::JUMP_FALSE4, vec![Operand::Label(false_lbl.clone())]);
                self.emit_expr_at(true_branch, depth + 1);
                self.emit(Op::JUMP4, vec![Operand::Label(end_lbl.clone())]);
                self.place_label(&false_lbl);
                self.emit_expr_at(false_branch, depth + 1);
                self.place_label(&end_lbl);
                false
            }

            ExprNode::Raw { text } => {
                // Braced scalar: $={name} → push name + loadStk
                if let Some(name) = parse_braced_scalar_ref(text) {
                    self.push_lit(name);
                    self.emit(Op::LOAD_STK, vec![]);
                } else if let Some(var_name) = parse_simple_var_ref(text) {
                    self.load_var(var_name);
                } else {
                    self.push_lit(text);
                    self.emit(Op::EXPR_STK, vec![]);
                    return false;
                }
                false
            }

            ExprNode::Call { function, args, .. } => {
                self.push_lit(&format!("tcl::mathfunc::{function}"));
                for arg in args {
                    self.emit_expr_at(arg, depth + 1);
                }
                // invokeStk1 has a 1-byte operand; switch to invokeStk4 when
                // arg_count exceeds u8 to avoid truncation.
                let arg_count = bytecode_imm(1 + args.len());
                let op = if arg_count < 256 {
                    Op::INVOKE_STK1
                } else {
                    Op::INVOKE_STK4
                };
                self.emit_comment(op, vec![Operand::Imm(arg_count)], "");
                false
            }

            ExprNode::Command { text, .. } => {
                // Command substitution requires the main emitter pipeline.
                // Emit as exprStk fallback.
                self.push_lit(text);
                self.emit(Op::EXPR_STK, vec![]);
                // Place the deferred `<cond>` startCommand
                // end label after the command-substitution instructions
                // so the startCommand covers the ExprCommand body.
                if let Some(label) = self.pending_cond_end_label.take() {
                    self.place_label(&label);
                }
                false
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use tcl_registry::CommandRegistry;

    use crate::codegen::{CodegenCtx, Op, Operand};
    use crate::expr_ast::{BinOp, ExprNode, UnaryOp};

    /// Helper: collect opcodes from a context's instruction stream.
    fn opcodes(ctx: &CodegenCtx) -> Vec<Op> {
        ctx.instructions.iter().map(|i| i.op).collect()
    }

    /// A literal expression node.
    fn lit(text: &str) -> ExprNode {
        ExprNode::Literal {
            text: text.into(),
            start: 0,
            end: 0,
        }
    }

    /// A scalar variable expression node (non-constant — defeats const folding,
    /// so operator opcodes are still emitted).
    fn var(name: &str) -> ExprNode {
        ExprNode::Var {
            text: format!("${name}"),
            name: name.into(),
            start: 0,
            end: 0,
        }
    }

    // -- Literal --

    #[test]
    fn emit_literal() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(false, &[], &registry);
        let node = ExprNode::Literal {
            text: "42".into(),
            start: 0,
            end: 2,
        };
        // A plain literal pushes its text but is reported as *not* canonical, so
        // a top-level `expr` result regenerates the number's string via
        // `tryCvtToNumeric` (`1e3` → `1000.0`). The push itself is the same.
        let numeric = ctx.emit_expr(&node);
        assert!(!numeric);
        assert_eq!(opcodes(&ctx), vec![Op::PUSH1]);
    }

    #[test]
    fn emit_literal_invalid_prefix() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(false, &[], &registry);
        let node = ExprNode::Literal {
            text: "0o289".into(),
            start: 0,
            end: 5,
        };
        let numeric = ctx.emit_expr(&node);
        assert!(numeric);
        // Invalid octal → push + exprStk for runtime error
        assert_eq!(opcodes(&ctx), vec![Op::PUSH1, Op::EXPR_STK]);
    }

    #[test]
    fn emit_literal_valid_hex() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(false, &[], &registry);
        let node = ExprNode::Literal {
            text: "0xFF".into(),
            start: 0,
            end: 4,
        };
        let numeric = ctx.emit_expr(&node);
        // Not reported canonical: a top-level `expr {0xFF}` normalises to `255`
        // through `tryCvtToNumeric`.
        assert!(!numeric);
        // Valid hex → just push
        assert_eq!(opcodes(&ctx), vec![Op::PUSH1]);
    }

    // -- String --

    #[test]
    fn emit_string_quoted() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(false, &[], &registry);
        let node = ExprNode::String {
            text: "\"hello\"".into(),
            start: 0,
            end: 7,
        };
        let numeric = ctx.emit_expr(&node);
        assert!(!numeric);
        assert_eq!(opcodes(&ctx), vec![Op::PUSH1]);
        // The literal should be "hello" (without quotes)
        assert_eq!(ctx.literals.entries()[0], "hello");
    }

    #[test]
    fn emit_string_braced() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(false, &[], &registry);
        let node = ExprNode::String {
            text: "{world}".into(),
            start: 0,
            end: 7,
        };
        let numeric = ctx.emit_expr(&node);
        assert!(!numeric);
        assert_eq!(ctx.literals.entries()[0], "world");
    }

    #[test]
    fn emit_string_backslash_escape() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(false, &[], &registry);
        let node = ExprNode::String {
            text: "\"a\\nb\"".into(),
            start: 0,
            end: 6,
        };
        ctx.emit_expr(&node);
        assert_eq!(ctx.literals.entries()[0], "a\nb");
    }

    // -- Variable --

    #[test]
    fn emit_var_scalar() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(true, &["x"], &registry);
        let node = ExprNode::Var {
            text: "$x".into(),
            name: "x".into(),
            start: 0,
            end: 2,
        };
        let numeric = ctx.emit_expr(&node);
        assert!(!numeric);
        assert_eq!(opcodes(&ctx), vec![Op::LOAD_SCALAR1]);
    }

    #[test]
    fn emit_var_array() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(true, &[], &registry);
        let node = ExprNode::Var {
            text: "$arr(key)".into(),
            name: "arr".into(),
            start: 0,
            end: 9,
        };
        let numeric = ctx.emit_expr(&node);
        assert!(!numeric);
        // Should end with LOAD_ARRAY1
        assert_eq!(ctx.instructions.last().unwrap().op, Op::LOAD_ARRAY1);
    }

    // -- Binary ops --

    #[test]
    fn emit_binary_add() {
        // A non-constant operand keeps the ADD opcode (constant folding of an
        // all-literal `1+2` is covered by `emit_binary_const_folds`).
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(true, &["x"], &registry);
        let node = ExprNode::Binary {
            op: BinOp::Add,
            left: Box::new(var("x")),
            right: Box::new(lit("2")),
        };
        let numeric = ctx.emit_expr(&node);
        assert!(numeric);
        assert_eq!(opcodes(&ctx), vec![Op::LOAD_SCALAR1, Op::PUSH1, Op::ADD]);
    }

    #[test]
    fn emit_binary_const_folds() {
        // `1 + 2` collapses to a single push of "3" (C-Tcl compile-time fold).
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(false, &[], &registry);
        let node = ExprNode::Binary {
            op: BinOp::Add,
            left: Box::new(lit("1")),
            right: Box::new(lit("2")),
        };
        assert!(ctx.emit_expr(&node));
        assert_eq!(opcodes(&ctx), vec![Op::PUSH1]);
        assert_eq!(ctx.literals.entries()[0], "3");
    }

    #[test]
    fn emit_irules_op_not_folded() {
        // iRules operators have no C-Tcl fold and must keep their opcode even
        // with constant operands.
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(false, &[], &registry);
        let node = ExprNode::Binary {
            op: BinOp::Contains,
            left: Box::new(lit("ab")),
            right: Box::new(lit("a")),
        };
        ctx.emit_expr(&node);
        assert!(opcodes(&ctx).contains(&Op::IRULE_CONTAINS));
    }

    #[test]
    fn emit_binary_streq() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(false, &[], &registry);
        let node = ExprNode::Binary {
            op: BinOp::StrEq,
            left: Box::new(ExprNode::Literal {
                text: "1".into(),
                start: 0,
                end: 1,
            }),
            right: Box::new(ExprNode::Literal {
                text: "2".into(),
                start: 5,
                end: 6,
            }),
        };
        let numeric = ctx.emit_expr(&node);
        assert!(numeric);
        assert_eq!(opcodes(&ctx), vec![Op::PUSH1, Op::PUSH1, Op::STR_EQ]);
    }

    #[test]
    fn emit_short_circuit_and() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(false, &[], &registry);
        // A variable operand keeps the short-circuit codegen (a constant
        // `1 && 0` would fold to a single push).
        let node = ExprNode::Binary {
            op: BinOp::And,
            left: Box::new(var("a")),
            right: Box::new(var("b")),
        };
        let numeric = ctx.emit_expr(&node);
        assert!(numeric);
        // Should contain JUMP_FALSE4 for short-circuit
        assert!(opcodes(&ctx).contains(&Op::JUMP_FALSE4));
        // Should push "1" and "0" for the result
        assert!(opcodes(&ctx).contains(&Op::JUMP4));
    }

    #[test]
    fn emit_short_circuit_or() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(false, &[], &registry);
        let node = ExprNode::Binary {
            op: BinOp::Or,
            left: Box::new(var("a")),
            right: Box::new(var("b")),
        };
        let numeric = ctx.emit_expr(&node);
        assert!(numeric);
        assert!(opcodes(&ctx).contains(&Op::JUMP_TRUE4));
    }

    #[test]
    fn emit_binary_in() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(false, &[], &registry);
        let node = ExprNode::Binary {
            op: BinOp::In,
            left: Box::new(ExprNode::Literal {
                text: "1".into(),
                start: 0,
                end: 1,
            }),
            right: Box::new(ExprNode::Literal {
                text: "{1 2 3}".into(),
                start: 5,
                end: 12,
            }),
        };
        let numeric = ctx.emit_expr(&node);
        assert!(numeric);
        assert!(opcodes(&ctx).contains(&Op::LIST_IN));
    }

    // -- Unary ops --

    #[test]
    fn emit_unary_neg() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(false, &[], &registry);
        let node = ExprNode::Unary {
            op: UnaryOp::Neg,
            operand: Box::new(var("x")),
        };
        let numeric = ctx.emit_expr(&node);
        assert!(numeric);
        assert!(opcodes(&ctx).contains(&Op::UMINUS));
    }

    #[test]
    fn emit_unary_const_folds() {
        // `-5` and `~0` are constant → fold to a push.
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(false, &[], &registry);
        let node = ExprNode::Unary {
            op: UnaryOp::Neg,
            operand: Box::new(lit("5")),
        };
        assert!(ctx.emit_expr(&node));
        assert_eq!(opcodes(&ctx), vec![Op::PUSH1]);
        assert_eq!(ctx.literals.entries()[0], "-5");
    }

    #[test]
    fn emit_unary_not() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(false, &[], &registry);
        let node = ExprNode::Unary {
            op: UnaryOp::Not,
            operand: Box::new(var("x")),
        };
        let numeric = ctx.emit_expr(&node);
        assert!(numeric);
        assert!(opcodes(&ctx).contains(&Op::NOT));
    }

    #[test]
    fn emit_unary_bitnot() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(false, &[], &registry);
        let node = ExprNode::Unary {
            op: UnaryOp::BitNot,
            operand: Box::new(var("x")),
        };
        let numeric = ctx.emit_expr(&node);
        assert!(numeric);
        assert!(opcodes(&ctx).contains(&Op::BITNOT));
    }

    // -- Ternary --

    #[test]
    fn emit_ternary() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(true, &["x"], &registry);
        let node = ExprNode::Ternary {
            condition: Box::new(ExprNode::Var {
                text: "$x".into(),
                name: "x".into(),
                start: 0,
                end: 2,
            }),
            true_branch: Box::new(ExprNode::Literal {
                text: "1".into(),
                start: 5,
                end: 6,
            }),
            false_branch: Box::new(ExprNode::Literal {
                text: "0".into(),
                start: 9,
                end: 10,
            }),
        };
        let numeric = ctx.emit_expr(&node);
        assert!(!numeric); // ternary might be string
        let ops = opcodes(&ctx);
        // Pattern: load cond, JUMP_FALSE, true branch, JUMP, false branch
        assert!(ops.contains(&Op::LOAD_SCALAR1));
        assert!(ops.contains(&Op::JUMP_FALSE4));
        assert!(ops.contains(&Op::JUMP4));
    }

    // -- Raw --

    #[test]
    fn emit_raw_var_ref() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(true, &["x"], &registry);
        let node = ExprNode::Raw {
            text: "${x}".into(),
        };
        let numeric = ctx.emit_expr(&node);
        assert!(!numeric);
        assert_eq!(opcodes(&ctx), vec![Op::LOAD_SCALAR1]);
    }

    #[test]
    fn emit_raw_braced_scalar() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(false, &[], &registry);
        let node = ExprNode::Raw {
            text: "$={a(1)}".into(),
        };
        let numeric = ctx.emit_expr(&node);
        assert!(!numeric);
        // push name + loadStk
        assert_eq!(opcodes(&ctx), vec![Op::PUSH1, Op::LOAD_STK]);
    }

    #[test]
    fn emit_raw_unknown() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(false, &[], &registry);
        let node = ExprNode::Raw {
            text: "some complex thing".into(),
        };
        let numeric = ctx.emit_expr(&node);
        assert!(!numeric);
        // Fallback: push + exprStk
        assert_eq!(opcodes(&ctx), vec![Op::PUSH1, Op::EXPR_STK]);
    }

    // -- Call (math functions) --

    #[test]
    fn emit_call_sin() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(false, &[], &registry);
        let node = ExprNode::Call {
            function: "sin".into(),
            args: vec![ExprNode::Literal {
                text: "2.0".into(),
                start: 4,
                end: 7,
            }],
            start: 0,
            end: 8,
        };
        let numeric = ctx.emit_expr(&node);
        assert!(!numeric); // function call — not guaranteed numeric
        let ops = opcodes(&ctx);
        // push "tcl::mathfunc::sin", push arg, invokeStk1 2
        assert_eq!(ops[0], Op::PUSH1); // function name
        assert_eq!(ops[1], Op::PUSH1); // arg
        assert_eq!(ops[2], Op::INVOKE_STK1);
        assert_eq!(ctx.instructions[2].operands[0], Operand::Imm(2));
        // Literal pool should contain "tcl::mathfunc::sin"
        assert!(
            ctx.literals
                .entries()
                .iter()
                .any(|e| e == "tcl::mathfunc::sin")
        );
    }

    #[test]
    fn emit_call_max_two_args() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(false, &[], &registry);
        let node = ExprNode::Call {
            function: "max".into(),
            args: vec![
                ExprNode::Literal {
                    text: "1".into(),
                    start: 4,
                    end: 5,
                },
                ExprNode::Literal {
                    text: "2".into(),
                    start: 7,
                    end: 8,
                },
            ],
            start: 0,
            end: 9,
        };
        ctx.emit_expr(&node);
        // invokeStk1 with count 3 (function + 2 args)
        let invoke = ctx.instructions.last().unwrap();
        assert_eq!(invoke.op, Op::INVOKE_STK1);
        assert_eq!(invoke.operands[0], Operand::Imm(3));
    }

    // -- Command substitution --

    #[test]
    fn emit_command_fallback() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(false, &[], &registry);
        let node = ExprNode::Command {
            text: "[info exists x]".into(),
            start: 0,
            end: 16,
        };
        let numeric = ctx.emit_expr(&node);
        assert!(!numeric);
        // Fallback: push + exprStk
        assert_eq!(opcodes(&ctx), vec![Op::PUSH1, Op::EXPR_STK]);
    }

    // -- iRules operators --

    #[test]
    fn emit_irules_contains() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(false, &[], &registry);
        let node = ExprNode::Binary {
            op: BinOp::Contains,
            left: Box::new(ExprNode::Literal {
                text: "1".into(),
                start: 0,
                end: 1,
            }),
            right: Box::new(ExprNode::Literal {
                text: "2".into(),
                start: 12,
                end: 13,
            }),
        };
        let numeric = ctx.emit_expr(&node);
        assert!(numeric);
        assert!(opcodes(&ctx).contains(&Op::IRULE_CONTAINS));
    }

    #[test]
    fn emit_irules_word_not() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(false, &[], &registry);
        let node = ExprNode::Unary {
            op: UnaryOp::WordNot,
            operand: Box::new(ExprNode::Literal {
                text: "1".into(),
                start: 4,
                end: 5,
            }),
        };
        let numeric = ctx.emit_expr(&node);
        assert!(numeric);
        assert!(opcodes(&ctx).contains(&Op::IRULE_WORD_NOT));
    }

    // -- Nested expressions --

    #[test]
    fn emit_nested_binary() {
        // (1 + 2) * 3
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(false, &[], &registry);
        // `(x + 2) * 3` — the var defeats whole-expression folding, so the
        // ADD and MULT opcodes are emitted (the constant `(1+2)*3` fold is
        // covered by `emit_nested_const_folds`).
        let node = ExprNode::Binary {
            op: BinOp::Mul,
            left: Box::new(ExprNode::Binary {
                op: BinOp::Add,
                left: Box::new(var("x")),
                right: Box::new(lit("2")),
            }),
            right: Box::new(lit("3")),
        };
        ctx.emit_expr(&node);
        let ops = opcodes(&ctx);
        assert!(ops.contains(&Op::ADD));
        assert!(ops.contains(&Op::MULT));
    }

    /// Regression coverage for issue #996: `emit_expr` / `emit_expr_binary`
    /// recurse once per `ExprNode` operator-tree level, with no depth cap
    /// before this fix. The Pratt parser caps *its* output at 256 levels, but
    /// a tree built directly (as every unit test in this module does) is
    /// unbounded, and empirically this walker overflowed the native stack
    /// (SIGABRT) in the low thousands of levels on a 2 MiB thread
    /// (`cargo test`'s per-test default). 3000 is comfortably past that crash
    /// range and past `MAX_EXPR_NODE_DEPTH` (256); the assertion is that
    /// emission returns at all. Uses nested `Ternary` (never constant-
    /// foldable) so this exercises `emit_expr`'s *own* recursion in
    /// isolation, not the separate whole-subtree const-fold walk.
    #[test]
    fn deeply_nested_expr_emission_survives() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(true, &["x"], &registry);
        let mut node = lit("0");
        for _ in 0..3000 {
            node = ExprNode::Ternary {
                condition: Box::new(var("x")),
                true_branch: Box::new(node),
                false_branch: Box::new(lit("0")),
            };
        }
        // Returns without overflowing the native stack.
        let _ = ctx.emit_expr(&node);
    }

    /// Companion to `deeply_nested_expr_emission_survives`: a moderately
    /// nested tree (well under `MAX_EXPR_NODE_DEPTH`) must emit exactly the
    /// same bytecode as before — the cap changes nothing for realistic
    /// input. A 100-deep nested `-` chain still emits 100 `UMINUS` ops.
    #[test]
    fn moderate_depth_expr_emission_unchanged() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(true, &["x"], &registry);
        let mut node = var("x");
        for _ in 0..100 {
            node = ExprNode::Unary {
                op: UnaryOp::Neg,
                operand: Box::new(node),
            };
        }
        ctx.emit_expr(&node);
        let uminus = opcodes(&ctx).iter().filter(|&&op| op == Op::UMINUS).count();
        assert_eq!(uminus, 100);
    }

    #[test]
    fn emit_nested_const_folds() {
        // `(1 + 2) * 3` is fully constant → folds to push "9".
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(false, &[], &registry);
        let node = ExprNode::Binary {
            op: BinOp::Mul,
            left: Box::new(ExprNode::Binary {
                op: BinOp::Add,
                left: Box::new(lit("1")),
                right: Box::new(lit("2")),
            }),
            right: Box::new(lit("3")),
        };
        ctx.emit_expr(&node);
        assert_eq!(opcodes(&ctx), vec![Op::PUSH1]);
        assert_eq!(ctx.literals.entries()[0], "9");
    }
}
