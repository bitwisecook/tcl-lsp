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

use tcl_lexer::backslash_subst_in;
use tcl_registry::expr_surface::RuntimeExprSurface;

use super::values::{parse_braced_scalar_ref, parse_simple_var_ref};
use super::{CodegenCtx, Op, Operand, bytecode_imm};
use crate::depth_guard::MAX_EXPR_NODE_DEPTH;
use crate::expr_ast::{BinOp, ExprNode, UnaryOp, render_expr};
use crate::tcl_expr_eval::{Env, FoldPolicy, TclValue, eval_tcl_expr_with_policy};

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

/// Whether `text` begins the way a numeral does — an optional sign then an
/// ASCII digit — which is what makes a failed parse a *bad number* rather than
/// simply a non-numeric word. C draws the same line in `ParseLexeme`: a word
/// starting with a digit is scanned as a NUMBER and diagnosed if it does not
/// parse, while a word starting with a letter is a bareword (a function name,
/// or an error) and never a numeral diagnosis.
fn starts_like_a_numeral(text: &str) -> bool {
    let bytes = text.as_bytes();
    let rest = match bytes.first() {
        Some(b'+' | b'-') => &bytes[1..],
        _ => bytes,
    };
    rest.first().is_some_and(u8::is_ascii_digit)
}

impl CodegenCtx<'_> {
    /// Read a Tcl source word as a wide integer under this module's compile
    /// target, for an operand baked into the bytecode at compile time.
    ///
    /// `str::parse::<i64>` is wrong here even though it looks harmless: it is
    /// Rust's decimal grammar, not Tcl's, so it reads `010` as 10 on every
    /// release (8.6 says 8), rejects `0x10`/`0o17`/`1_0` that Tcl accepts, and
    /// accepts nothing Tcl's radix forms need. An operand folded into an
    /// instruction has to mean what the target release says it means.
    pub(super) fn parse_int_operand(&self, text: &str) -> Option<i64> {
        let flags = tcl_syntax::number::ParseFlags {
            integer_only: true,
            ..tcl_syntax::number::ParseFlags::for_syntax(self.numbers)
        };
        match tcl_syntax::number::parse_whole_with(text, flags)? {
            tcl_syntax::number::Number::Int(v) => Some(v),
            _ => None,
        }
    }

    /// The constant-folding policy for this module's compile target.
    ///
    /// The registry answers the iRules-operator and character-model facts, but
    /// its *numeric* answers are re-derived from whatever profile it happens to
    /// carry — and a hand-built registry has none, so `octal_fold_policy` falls
    /// back to a loaded-packs heuristic that reads a 9.0 target as 8.x octal.
    /// `CodegenCtx::numbers` comes from `Module.dialect`, which is the compile
    /// target itself, so it wins.
    ///
    /// Two sources for one fact is exactly the drift the single number facility
    /// exists to prevent: before this, `puts "x: [expr {0755 + 1}]"` folded to
    /// 494 for a 9.0 target while the bare `puts [expr {0755 + 1}]` gave the
    /// correct 756, because only one of the two paths reached a profile-built
    /// registry.
    fn fold_policy(&self) -> FoldPolicy {
        FoldPolicy {
            octal: Some(self.numbers.leading_zero_is_octal()),
            numbers: Some(self.numbers),
            ..FoldPolicy::from_registry(self.registry)
        }
    }

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
        // What the *target release* decides here is whether the text is a
        // numeral at all — not what it evaluates to. `0o8` is never one, `0d99`
        // is one only from 9.0, `0o17` only from 8.5. Everything about the
        // spelling that is release-dependent (is `0755` octal 493 or decimal
        // 755?) is settled later, by the `tryCvtToNumeric` the caller emits,
        // running in a runtime built for this same dialect.
        //
        // A valid numeral is *not* folded, because C does not fold one:
        //
        //     % tcl::unsupported::disassemble script {expr {0xFF}}
        //     (0) push1 0    # "0xFF"
        //     (2) tryCvtToNumeric
        //
        // `CompileExprTree`'s `OT_LITERAL` case pushes the source spelling and
        // leaves `convert` set, so the root emits `INST_TRY_CVT_TO_NUMERIC`.
        // Only a *compound* constant subtree folds (`expr {1+1}` pushes `"2"`
        // and clears `convert`, via `ExecConstantExprTree`) — that path is
        // `tcl_expr_eval`'s, and it is dialect-aware in its own right. Folding
        // here instead would put the value where C puts the spelling: the same
        // answer, a different literal pool and one instruction fewer.
        let trimmed = text.trim();
        if tcl_syntax::number::is_whole_number(trimmed, self.numbers) {
            self.push_lit(trimmed);
            // Not canonical: the caller emits `tryCvtToNumeric`, which converts
            // under the runtime's grammar (`1e3` → `1000.0`, `0xFF` → `255`).
            return false;
        }
        // A numeral this release cannot read — `0o8` anywhere, `0d99`/`1_0`/`08`
        // before 9.0. C classifies it as a bareword and compiles a syntax error;
        // `exprStk` raises the same diagnosis at run time with the full
        // `ParseExpr` message.
        //
        // Only for text that *tries* to be a numeral, i.e. starts with a digit
        // after an optional sign. `Literal` is not exclusively an expression
        // operand: `cfg_lower` models an exact-`switch` arm pattern as one so
        // the `STR_EQ(subject, pattern)` condition stays foldable into a jump
        // table, and those patterns are arbitrary strings. Diverting every
        // non-numeral here would compile `switch zz { a {…} }` into an `expr`
        // evaluation of the bareword `a`.
        if starts_like_a_numeral(trimmed) {
            self.push_lit(text);
            self.emit(Op::EXPR_STK, vec![]);
            return true;
        }
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
                self.push_lit(&backslash_subst_in(inner, self.escapes));
                return false;
            }
            match super::helpers::parse_subst_template(inner, self.escapes) {
                Some(parts) => self.emit_subst_parts(&parts),
                // Unparseable template (e.g. a bare `$` with no name): literal.
                None => self.push_lit(&backslash_subst_in(inner, self.escapes)),
            }
            return false;
        }
        // Bare text (no delimiters): a literal, backslash-decoded.
        if text.contains('\\') {
            let processed = backslash_subst_in(text, self.escapes);
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

    /// The `expr` operator surface of the release being compiled for, or
    /// `None` when the compile named no dialect — or named one the catalog
    /// does not know.
    ///
    /// Resolved from [`CodegenCtx::dialect`] — the compile's own dialect, the
    /// same fact `numbers`/`escapes` come from — rather than from the
    /// registry's profile, which a caller may not have set (the VM's compile
    /// service passes a default-built registry beside a pinned dialect).
    ///
    /// The `None` is an abstention, not a permissive default resolved to
    /// plain `tcl`. That fallback profile is *not* silent on the question: its
    /// availability mask is `ALL_TCL`, which excludes the iRules bit, so
    /// asking it would answer "this dialect has no `contains`" for a compile
    /// that never said which dialect it was. With no dialect in hand the
    /// operator set is unknown, so nothing is gated and a dialect-less compile
    /// keeps exactly the surface it had.
    fn expr_surface(&self) -> Option<RuntimeExprSurface> {
        self.dialect.map(RuntimeExprSurface::for_profile)
    }

    /// Emit an expression tree, reporting whether the value it leaves on the
    /// stack is already canonically numeric — the caller emits
    /// `tryCvtToNumeric` when this is `false` and the context requires numeric
    /// coercion.
    ///
    /// Public entry point: the top of an expression tree is nesting depth
    /// `0` (issue #996 — the recursion cap lives in [`Self::emit_expr_at`]).
    ///
    /// An operator the target release's `expr` grammar does not have is not
    /// specialised at all (issue #1435). The compiled and interpreted paths
    /// otherwise apply opposite dialect discipline: `Op::from_binop` is total
    /// over `BinOp`, so `expr {2 ** 3}` compiled *for* 8.4 would emit `expon`
    /// and the emulating VM would execute it, while the same source reached
    /// through `exprStk` is rejected by [`RuntimeExprSurface::validate`] as C
    /// Tcl rejects it. Refusing to specialise routes the whole expression back
    /// through `exprStk`, so the one gate — the registry's, on the release's
    /// own operator table — decides both, and the failure is the interpreter's
    /// (`invalid bareword "**"`, `TCL PARSE EXPR BAREWORD`) rather than a
    /// second diagnostic invented here. It is the lowering fallback pattern:
    /// codegen inlines only what the target release can run. A compile that
    /// named no dialect has no release to gate against and is left alone —
    /// see [`Self::expr_surface`].
    pub fn emit_expr(&mut self, node: &ExprNode) -> bool {
        if self
            .expr_surface()
            .is_some_and(|surface| surface.validate(node).is_err())
        {
            self.push_lit(&render_expr(node));
            self.emit(Op::EXPR_STK, vec![]);
            return false;
        }
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
            && let Some(TclValue::Int(i)) =
                eval_tcl_expr_with_policy(node, &Env::new(), self.fold_policy())
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
                // A `[…]` operand is a *word*: C compiles it with
                // `TclCompileTokens` and takes the command's result as the
                // operand value (`tclCompExpr.c`'s `OT_TOKENS` leaf) — it never
                // re-parses that result. The non-verbatim push below performs the
                // substitution at runtime, so the value is already the operand;
                // an `exprStk` after it used to evaluate the *result* a second
                // time as an expression (`expr {[set x]}` with `x` = `1+2` gave
                // `3`, and `expr {"set" in [info commands]}` only worked while
                // `exprStk` handed an unparsable expression back unchanged).
                self.push_lit(text);
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

    /// `$x <op> $y`, whose variable operands keep the operator's opcode out of
    /// the constant folder.
    fn binary(op: BinOp) -> ExprNode {
        ExprNode::Binary {
            op,
            left: Box::new(var("x")),
            right: Box::new(var("y")),
        }
    }

    /// A context compiling *for* `dialect`, as `codegen_module` builds one from
    /// the IR module's dialect.
    fn ctx_for<'r>(registry: &'r CommandRegistry, dialect: &'static tcl_dialect::DialectProfile) -> CodegenCtx<'r> {
        let mut ctx = CodegenCtx::new(true, &["x", "y"], registry);
        ctx.dialect = Some(dialect);
        ctx
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

    /// A valid numeral goes into the literal pool as its **source spelling**,
    /// never as a folded value — C's `CompileExprTree` `OT_LITERAL` case pushes
    /// the spelling and lets the root's `tryCvtToNumeric` convert it. Verified
    /// against tclsh 9.0:
    ///
    /// ```text
    /// % tcl::unsupported::disassemble script {expr {0xFF}}
    ///     (0) push1 0    # "0xFF"
    ///     (2) tryCvtToNumeric
    /// ```
    ///
    /// Folding here would agree on the answer and disagree on the bytecode.
    /// Surrounding whitespace *is* trimmed (`expr { 42 }` pushes `"42"`).
    #[test]
    fn a_valid_numeral_is_pushed_as_its_spelling_not_its_value() {
        let registry = CommandRegistry::build_default();
        for (text, want) in [
            ("0xFF", "0xFF"),
            ("1e3", "1e3"),
            ("42", "42"),
            (" 42 ", "42"),
            ("0755", "0755"),
            ("NaN", "NaN"),
            ("99999999999999999999999", "99999999999999999999999"),
        ] {
            let mut ctx = CodegenCtx::new(false, &[], &registry);
            assert!(!ctx.emit_expr(&lit(text)), "{text}: reported canonical");
            assert_eq!(opcodes(&ctx), vec![Op::PUSH1], "{text}: opcodes");
            assert_eq!(ctx.literals.entries()[0], want, "{text}: literal pool");
        }
    }

    /// `ExprNode::Literal` is not exclusively an expression operand, so a
    /// non-numeral must be pushed verbatim rather than diagnosed.
    ///
    /// `cfg_lower` models an exact-`switch` arm pattern as a `Literal` so the
    /// `STR_EQ(subject, pattern)` condition stays foldable into a jump table,
    /// and an arm pattern is an arbitrary string. Routing every non-numeral to
    /// `exprStk` compiled `switch zz { a {puts a} default {puts def} }` into an
    /// `expr` evaluation of the bareword `a`, which errors — caught by
    /// `tcl-vm`'s `switch_glob_and_default`. Only text that *starts* like a
    /// numeral can be a bad numeral; C's `ParseLexeme` draws the same line.
    #[test]
    fn a_non_numeral_literal_is_pushed_verbatim_not_diagnosed() {
        let registry = CommandRegistry::build_default();
        for text in ["a", "zz", "default", "some pattern", "", "x1"] {
            let mut ctx = CodegenCtx::new(false, &[], &registry);
            let numeric = ctx.emit_expr(&lit(text));
            assert_eq!(opcodes(&ctx), vec![Op::PUSH1], "{text:?}: opcodes");
            assert!(!numeric, "{text:?}: must not claim numeric");
            assert_eq!(ctx.literals.entries()[0], text, "{text:?}: literal pool");
        }
        // Sign-led words are words too, not numerals.
        for text in ["-foo", "+bar"] {
            let mut ctx = CodegenCtx::new(false, &[], &registry);
            ctx.emit_expr(&lit(text));
            assert_eq!(opcodes(&ctx), vec![Op::PUSH1], "{text:?}");
        }
    }

    /// The *validity* of a numeral is release-dependent, and that is the one
    /// thing codegen decides about it: a spelling its target release does not
    /// have is a bareword, routed to `exprStk` for the runtime diagnosis.
    #[test]
    fn numeral_validity_follows_the_target_release() {
        use tcl_dialect::NumberSyntax;
        let registry = CommandRegistry::build_default();
        // (text, syntax, is a numeral here)
        for (text, numbers, valid) in [
            // `0o` arrived in 8.5, `0d` in 9.0, `_` in 9.0.
            ("0o17", NumberSyntax::Tcl84, false),
            ("0o17", NumberSyntax::Tcl85, true),
            ("0d99", NumberSyntax::Tcl85, false),
            ("0d99", NumberSyntax::Tcl90, true),
            ("1_000", NumberSyntax::Tcl85, false),
            ("1_000", NumberSyntax::Tcl90, true),
            // Bad digits for the radix are never a numeral.
            ("0o8", NumberSyntax::Tcl85, false),
            ("0o8", NumberSyntax::Tcl90, false),
            // A leading zero is octal up to 8.6 and decimal from 9.0, but it is
            // a numeral either way — only `tryCvtToNumeric` sees the difference.
            ("0755", NumberSyntax::Tcl85, true),
            ("0755", NumberSyntax::Tcl90, true),
        ] {
            let mut ctx = CodegenCtx::new(false, &[], &registry);
            ctx.numbers = numbers;
            let numeric = ctx.emit_expr(&lit(text));
            let want = if valid {
                vec![Op::PUSH1]
            } else {
                vec![Op::PUSH1, Op::EXPR_STK]
            };
            assert_eq!(opcodes(&ctx), want, "{text} under {numbers:?}");
            assert_eq!(numeric, !valid, "{text} under {numbers:?}: canonical flag");
        }
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

    /// A `[…]` operand is a word: the (non-verbatim) push *is* the substitution,
    /// so the value it leaves is already the operand. No `exprStk` — that used to
    /// follow, and re-evaluated the command's result a second time as an
    /// expression.
    #[test]
    fn emit_command_pushes_the_word_without_re_evaluating_it() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(false, &[], &registry);
        let node = ExprNode::Command {
            text: "[info exists x]".into(),
            start: 0,
            end: 16,
        };
        let numeric = ctx.emit_expr(&node);
        assert!(!numeric);
        assert_eq!(opcodes(&ctx), vec![Op::PUSH1]);
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

    // -- The versioned `expr` operator surface (issue #1435) --

    /// The refusal shape: the whole expression is pushed as source text and
    /// handed to `exprStk`, so the emulating VM's own
    /// `RuntimeExprSurface::validate` raises C Tcl's diagnostic. Nothing is
    /// specialised and nothing is folded.
    fn assert_refused(ctx: &CodegenCtx, rendered: &str) {
        assert_eq!(opcodes(ctx), vec![Op::PUSH1, Op::EXPR_STK], "{rendered}");
        assert_eq!(ctx.literals.entries()[0], rendered);
    }

    /// Operators whose *release* floor the compile target must respect.
    ///
    /// Verified against the C sources: 8.4's `operatorTable`
    /// (`tmp/tcl8.4.20/generic/tclCompExpr.c:112-136`) has `eq`/`ne` and
    /// neither `**` nor `in`/`ni`; `EXPON` and `INST_LIST_IN` arrive with
    /// 8.5's rewritten parser (`tmp/tcl8.5.19/generic/tclCompExpr.c:260,417`,
    /// TIP 123/201); `STR_LT` and its three companions arrive in 9.0
    /// (`tmp/tcl9.0.4/generic/tclCompExpr.c:273,413`, TIP 461) and are absent
    /// from 8.6's table. The registry's own tables say the same
    /// (`BinOp::expr_grammar_min_version`,
    /// `tcl_dialect::EXPR_WORD_OPERATORS`), which is what the gate reads.
    #[test]
    fn expr_operators_are_specialised_only_from_their_release_floor() {
        let registry = CommandRegistry::build_default();
        // (operator, rendered form, the newest release that lacks it, the
        // oldest that has it, the opcode that release specialises it to).
        let vectors = [
            (BinOp::Pow, "$x ** $y", "tcl8.4", "tcl8.5", Op::EXPON),
            (BinOp::In, "$x in $y", "tcl8.4", "tcl8.5", Op::LIST_IN),
            (BinOp::Ni, "$x ni $y", "tcl8.4", "tcl8.5", Op::LIST_NOT_IN),
            (BinOp::StrLt, "$x lt $y", "tcl8.6", "tcl9.0", Op::STR_LT),
            (BinOp::StrLe, "$x le $y", "tcl8.6", "tcl9.0", Op::STR_LE),
            (BinOp::StrGt, "$x gt $y", "tcl8.6", "tcl9.0", Op::STR_GT),
            (BinOp::StrGe, "$x ge $y", "tcl8.6", "tcl9.0", Op::STR_GE),
        ];
        for (op, rendered, before, since, opcode) in vectors {
            // FN: before the floor the operator is not specialised.
            let mut old = ctx_for(&registry, tcl_dialect::DialectProfile::by_name(before));
            old.emit_expr(&binary(op));
            assert_refused(&old, rendered);

            // TP: from the floor it compiles to its own opcode.
            let mut modern = ctx_for(&registry, tcl_dialect::DialectProfile::by_name(since));
            modern.emit_expr(&binary(op));
            assert_eq!(
                opcodes(&modern),
                vec![Op::LOAD_SCALAR1, Op::LOAD_SCALAR1, opcode],
                "{rendered} at {since}"
            );
        }

        // TN: `eq`/`ne` are in 8.4's operator table, so the gate must not
        // sweep them up with the operators that are not.
        for (op, opcode) in [(BinOp::StrEq, Op::STR_EQ), (BinOp::StrNe, Op::STR_NEQ)] {
            let mut ctx = ctx_for(&registry, tcl_dialect::DialectProfile::by_name("tcl8.4"));
            ctx.emit_expr(&binary(op));
            assert_eq!(
                opcodes(&ctx),
                vec![Op::LOAD_SCALAR1, Op::LOAD_SCALAR1, opcode]
            );
        }

        // FP guard: a dialect-less compile keeps the permissive surface, so
        // every Tcl operator still compiles inline.
        let mut plain = CodegenCtx::new(true, &["x", "y"], &registry);
        plain.emit_expr(&binary(BinOp::Pow));
        assert_eq!(
            opcodes(&plain),
            vec![Op::LOAD_SCALAR1, Op::LOAD_SCALAR1, Op::EXPON]
        );
    }

    /// The constant folder runs before the operand walk, so the gate has to
    /// sit in front of it: `expr {2 ** 3}` compiled for 8.4 folded to a push
    /// of `8` — a valid answer from an operator the release cannot parse.
    #[test]
    fn a_pre_floor_operator_is_not_constant_folded_either() {
        let registry = CommandRegistry::build_default();
        let node = ExprNode::Binary {
            op: BinOp::Pow,
            left: Box::new(lit("2")),
            right: Box::new(lit("3")),
        };

        let mut old = ctx_for(&registry, tcl_dialect::DialectProfile::by_name("tcl8.4"));
        old.emit_expr(&node);
        assert_refused(&old, "2 ** 3");

        let mut modern = ctx_for(&registry, tcl_dialect::DialectProfile::by_name("tcl8.5"));
        modern.emit_expr(&node);
        assert_eq!(opcodes(&modern), vec![Op::PUSH1]);
        assert_eq!(modern.literals.entries()[0], "8");
    }

    /// The other axis: the iRules word operators are gated by dialect
    /// *identity*, not by a version floor, and iRules is a Tcl 8.4 runtime —
    /// so a release-only gate would either lose them or admit 8.5 operators
    /// alongside them.
    #[test]
    fn irules_word_operators_need_the_irules_dialect() {
        let registry = CommandRegistry::build_default();
        let contains = ExprNode::Binary {
            op: BinOp::Contains,
            left: Box::new(var("x")),
            right: Box::new(var("y")),
        };

        // TP: available under the dialect that defines them.
        let mut irules = ctx_for(&registry, tcl_dialect::DialectProfile::by_name("f5-irules"));
        irules.emit_expr(&contains);
        assert!(opcodes(&irules).contains(&Op::IRULE_CONTAINS));

        // FN: not part of any plain-Tcl release's grammar.
        let mut plain = ctx_for(&registry, tcl_dialect::DialectProfile::by_name("tcl8.6"));
        plain.emit_expr(&contains);
        assert_refused(&plain, "$x contains $y");

        // TN: iRules is an 8.4 runtime, so the 8.5 membership operator is
        // absent there too, dialect bit notwithstanding.
        let mut membership = ctx_for(&registry, tcl_dialect::DialectProfile::by_name("f5-irules"));
        membership.emit_expr(&binary(BinOp::In));
        assert_refused(&membership, "$x in $y");

        // FP guard: a compile that named no dialect abstains rather than
        // answering from the permissive `tcl` profile, whose `ALL_TCL` mask
        // would reject the iRules bit it never had an opinion about.
        let mut unnamed = CodegenCtx::new(true, &["x", "y"], &registry);
        unnamed.emit_expr(&contains);
        assert!(opcodes(&unnamed).contains(&Op::IRULE_CONTAINS));
    }
}
