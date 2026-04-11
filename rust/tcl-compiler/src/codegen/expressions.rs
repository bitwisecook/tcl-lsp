//! Expression AST compilation to bytecode.
//!
//! Extends [`CodegenCtx`] with [`emit_expr`] which walks an
//! [`ExprNode`] tree and produces the corresponding bytecode
//! instructions.  Ported from `core/compiler/codegen/_expressions.py`.

use tcl_lexer::backslash_subst;

use super::values::{parse_braced_scalar_ref, parse_simple_var_ref};
use super::{CodegenCtx, Op, Operand};
use crate::expr_ast::{render_expr, BinOp, ExprNode};

#[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
impl CodegenCtx {
    /// Compile an expression AST node; leaves the result on TOS.
    ///
    /// Returns `true` when the result is *guaranteed numeric*
    /// (arithmetic, comparison, logical, bitwise, or literal number),
    /// `false` when it might be a string (variable, ternary, function
    /// call, command subst).
    ///
    /// `tryCvtToNumeric` is **never** emitted by this method — the
    /// caller is responsible for emitting it when the return value is
    /// `false` and the context requires numeric coercion.
    #[allow(clippy::too_many_lines)]
    pub fn emit_expr(&mut self, node: &ExprNode) -> bool {
        match node {
            ExprNode::Literal { text, .. } => {
                // Validate prefix literals at compile time — invalid
                // prefixed numbers (0o289, 0xGG) must error at runtime.
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
                self.push_lit(text);
                true
            }

            ExprNode::String { text, .. } => {
                let mut inner = text.as_str();
                // Strip surrounding delimiters
                if inner.len() >= 2
                    && ((inner.starts_with('"') && inner.ends_with('"'))
                        || (inner.starts_with('{') && inner.ends_with('}')))
                {
                    inner = &inner[1..inner.len() - 1];
                }
                // Process Tcl backslash escapes
                if inner.contains('\\') {
                    let processed = backslash_subst(inner);
                    self.push_lit(&processed);
                } else {
                    self.push_lit(inner);
                }
                false
            }

            ExprNode::Var { text, name, .. } => {
                // text includes $ and optional array index: $arr(key)
                let var_ref = if text.contains('(') {
                    text.trim_start_matches('$')
                } else {
                    name.as_str()
                };
                self.load_var(var_ref);
                false
            }

            ExprNode::Binary { op, left, right } => {
                match op {
                    // Short-circuit evaluation for &&
                    BinOp::And => {
                        let false_lbl = self.fresh_label("and_f");
                        let end_lbl = self.fresh_label("and_end");
                        self.emit_expr(left);
                        self.emit(Op::JUMP_FALSE4, vec![Operand::Label(false_lbl.clone())]);
                        self.emit_expr(right);
                        self.emit(Op::JUMP_FALSE4, vec![Operand::Label(false_lbl.clone())]);
                        self.push_lit("1");
                        self.emit(Op::JUMP4, vec![Operand::Label(end_lbl.clone())]);
                        self.place_label(&false_lbl);
                        self.push_lit("0");
                        self.place_label(&end_lbl);
                    }
                    // Short-circuit evaluation for ||
                    BinOp::Or => {
                        let true_lbl = self.fresh_label("or_t");
                        let end_lbl = self.fresh_label("or_end");
                        self.emit_expr(left);
                        self.emit(Op::JUMP_TRUE4, vec![Operand::Label(true_lbl.clone())]);
                        self.emit_expr(right);
                        self.emit(Op::JUMP_TRUE4, vec![Operand::Label(true_lbl.clone())]);
                        self.push_lit("0");
                        self.emit(Op::JUMP4, vec![Operand::Label(end_lbl.clone())]);
                        self.place_label(&true_lbl);
                        self.push_lit("1");
                        self.place_label(&end_lbl);
                    }
                    _ => {
                        if let Some(bc) = Op::from_binop(*op) {
                            self.emit_expr(left);
                            self.emit_expr(right);
                            self.emit(bc, vec![]);
                        } else {
                            // Fallback to exprStk
                            self.push_lit(&render_expr(node));
                            self.emit(Op::EXPR_STK, vec![]);
                        }
                    }
                }
                true
            }

            ExprNode::Unary { op, operand } => {
                if let Some(bc) = Op::from_unaryop(*op) {
                    self.emit_expr(operand);
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
                self.emit_expr(condition);
                self.emit(Op::JUMP_FALSE4, vec![Operand::Label(false_lbl.clone())]);
                self.emit_expr(true_branch);
                self.emit(Op::JUMP4, vec![Operand::Label(end_lbl.clone())]);
                self.place_label(&false_lbl);
                self.emit_expr(false_branch);
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
                    self.emit_expr(arg);
                }
                self.emit_comment(
                    Op::INVOKE_STK1,
                    vec![Operand::Imm(1 + args.len() as i32)],
                    "",
                );
                false
            }

            ExprNode::Command { text, .. } => {
                // Command substitution requires the main emitter pipeline.
                // Emit as exprStk fallback — the full emitter will replace
                // this with inline compilation in a later chunk.
                self.push_lit(text);
                self.emit(Op::EXPR_STK, vec![]);
                false
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codegen::{CodegenCtx, Op, Operand};
    use crate::expr_ast::{BinOp, ExprNode, UnaryOp};

    /// Helper: collect opcodes from a context's instruction stream.
    fn opcodes(ctx: &CodegenCtx) -> Vec<Op> {
        ctx.instructions.iter().map(|i| i.op).collect()
    }

    // -- Literal --

    #[test]
    fn emit_literal() {
        let mut ctx = CodegenCtx::new(false, &[]);
        let node = ExprNode::Literal {
            text: "42".into(),
            start: 0,
            end: 2,
        };
        let numeric = ctx.emit_expr(&node);
        assert!(numeric);
        assert_eq!(opcodes(&ctx), vec![Op::PUSH1]);
    }

    #[test]
    fn emit_literal_invalid_prefix() {
        let mut ctx = CodegenCtx::new(false, &[]);
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
        let mut ctx = CodegenCtx::new(false, &[]);
        let node = ExprNode::Literal {
            text: "0xFF".into(),
            start: 0,
            end: 4,
        };
        let numeric = ctx.emit_expr(&node);
        assert!(numeric);
        // Valid hex → just push
        assert_eq!(opcodes(&ctx), vec![Op::PUSH1]);
    }

    // -- String --

    #[test]
    fn emit_string_quoted() {
        let mut ctx = CodegenCtx::new(false, &[]);
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
        let mut ctx = CodegenCtx::new(false, &[]);
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
        let mut ctx = CodegenCtx::new(false, &[]);
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
        let mut ctx = CodegenCtx::new(true, &["x"]);
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
        let mut ctx = CodegenCtx::new(true, &[]);
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
        let mut ctx = CodegenCtx::new(false, &[]);
        let node = ExprNode::Binary {
            op: BinOp::Add,
            left: Box::new(ExprNode::Literal {
                text: "1".into(),
                start: 0,
                end: 1,
            }),
            right: Box::new(ExprNode::Literal {
                text: "2".into(),
                start: 4,
                end: 5,
            }),
        };
        let numeric = ctx.emit_expr(&node);
        assert!(numeric);
        assert_eq!(opcodes(&ctx), vec![Op::PUSH1, Op::PUSH1, Op::ADD]);
    }

    #[test]
    fn emit_binary_streq() {
        let mut ctx = CodegenCtx::new(false, &[]);
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
        let mut ctx = CodegenCtx::new(false, &[]);
        let node = ExprNode::Binary {
            op: BinOp::And,
            left: Box::new(ExprNode::Literal {
                text: "1".into(),
                start: 0,
                end: 1,
            }),
            right: Box::new(ExprNode::Literal {
                text: "0".into(),
                start: 5,
                end: 6,
            }),
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
        let mut ctx = CodegenCtx::new(false, &[]);
        let node = ExprNode::Binary {
            op: BinOp::Or,
            left: Box::new(ExprNode::Literal {
                text: "0".into(),
                start: 0,
                end: 1,
            }),
            right: Box::new(ExprNode::Literal {
                text: "1".into(),
                start: 5,
                end: 6,
            }),
        };
        let numeric = ctx.emit_expr(&node);
        assert!(numeric);
        assert!(opcodes(&ctx).contains(&Op::JUMP_TRUE4));
    }

    #[test]
    fn emit_binary_in() {
        let mut ctx = CodegenCtx::new(false, &[]);
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
        let mut ctx = CodegenCtx::new(false, &[]);
        let node = ExprNode::Unary {
            op: UnaryOp::Neg,
            operand: Box::new(ExprNode::Literal {
                text: "5".into(),
                start: 1,
                end: 2,
            }),
        };
        let numeric = ctx.emit_expr(&node);
        assert!(numeric);
        assert_eq!(opcodes(&ctx), vec![Op::PUSH1, Op::UMINUS]);
    }

    #[test]
    fn emit_unary_not() {
        let mut ctx = CodegenCtx::new(false, &[]);
        let node = ExprNode::Unary {
            op: UnaryOp::Not,
            operand: Box::new(ExprNode::Literal {
                text: "1".into(),
                start: 1,
                end: 2,
            }),
        };
        let numeric = ctx.emit_expr(&node);
        assert!(numeric);
        assert_eq!(opcodes(&ctx), vec![Op::PUSH1, Op::NOT]);
    }

    #[test]
    fn emit_unary_bitnot() {
        let mut ctx = CodegenCtx::new(false, &[]);
        let node = ExprNode::Unary {
            op: UnaryOp::BitNot,
            operand: Box::new(ExprNode::Literal {
                text: "0xFF".into(),
                start: 1,
                end: 5,
            }),
        };
        let numeric = ctx.emit_expr(&node);
        assert!(numeric);
        assert_eq!(opcodes(&ctx), vec![Op::PUSH1, Op::BITNOT]);
    }

    // -- Ternary --

    #[test]
    fn emit_ternary() {
        let mut ctx = CodegenCtx::new(true, &["x"]);
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
        let mut ctx = CodegenCtx::new(true, &["x"]);
        let node = ExprNode::Raw {
            text: "${x}".into(),
        };
        let numeric = ctx.emit_expr(&node);
        assert!(!numeric);
        assert_eq!(opcodes(&ctx), vec![Op::LOAD_SCALAR1]);
    }

    #[test]
    fn emit_raw_braced_scalar() {
        let mut ctx = CodegenCtx::new(false, &[]);
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
        let mut ctx = CodegenCtx::new(false, &[]);
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
        let mut ctx = CodegenCtx::new(false, &[]);
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
        assert!(ctx
            .literals
            .entries()
            .iter()
            .any(|e| e == "tcl::mathfunc::sin"));
    }

    #[test]
    fn emit_call_max_two_args() {
        let mut ctx = CodegenCtx::new(false, &[]);
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
        let mut ctx = CodegenCtx::new(false, &[]);
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
        let mut ctx = CodegenCtx::new(false, &[]);
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
        let mut ctx = CodegenCtx::new(false, &[]);
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
        let mut ctx = CodegenCtx::new(false, &[]);
        let node = ExprNode::Binary {
            op: BinOp::Mul,
            left: Box::new(ExprNode::Binary {
                op: BinOp::Add,
                left: Box::new(ExprNode::Literal {
                    text: "1".into(),
                    start: 0,
                    end: 1,
                }),
                right: Box::new(ExprNode::Literal {
                    text: "2".into(),
                    start: 4,
                    end: 5,
                }),
            }),
            right: Box::new(ExprNode::Literal {
                text: "3".into(),
                start: 9,
                end: 10,
            }),
        };
        ctx.emit_expr(&node);
        assert_eq!(
            opcodes(&ctx),
            vec![Op::PUSH1, Op::PUSH1, Op::ADD, Op::PUSH1, Op::MULT]
        );
    }
}
