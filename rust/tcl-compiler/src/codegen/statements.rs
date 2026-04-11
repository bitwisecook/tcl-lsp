//! Statement-level bytecode emission.
//!
//! Extends [`CodegenCtx`] with methods for emitting IR statements
//! (assignments, calls, returns, barriers) with `startCommand`
//! wrapping.  Ported from `core/compiler/codegen/_statements.py`.

use crate::ir::Statement;

use super::values::{is_array_ref, is_qualified, parse_simple_var_ref, split_array_ref};
use super::{CodegenCtx, Op, Operand};

/// Tag used to identify `startCommand` instructions wrapping generic
/// invokes so the peephole pass can selectively remove them.
pub(crate) const SC_GENERIC_TAG: &str = "sc:generic";

/// Tag appended to no-dedup literal comments.
pub(crate) const NO_DEDUP_TAG: &str = " #nodedup";

#[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
impl CodegenCtx {
    /// Emit a statement, wrapping with `startCommand` if needed.
    ///
    /// `count_override` overrides the default count of 1 (e.g. 2 when
    /// a compound command and its init share the same bytecode offset).
    /// `deferred_end_label` uses a caller-provided label instead of
    /// creating and placing one automatically.
    pub fn emit_stmt_with_start_cmd(
        &mut self,
        stmt: &Statement,
        count_override: Option<u32>,
        deferred_end_label: Option<&str>,
    ) {
        let count = count_override.unwrap_or(1);
        let emit_sc = self.cmd_index > 0;
        let mut end_label: Option<String> = None;
        let mut sc_idx: Option<usize> = None;

        if emit_sc {
            let label = match deferred_end_label {
                Some(l) => l.to_owned(),
                None => self.fresh_label("cmd_end"),
            };
            sc_idx = Some(self.emit_comment(
                Op::START_CMD,
                vec![Operand::Label(label.clone()), Operand::Imm(count as i32)],
                "",
            ));
            end_label = Some(label);
        }

        let mut used_generic_invoke = false;
        self.emit_stmt(stmt, &mut used_generic_invoke);

        if used_generic_invoke {
            // Tag this startCommand as wrapping a generic invoke
            if let Some(idx) = sc_idx {
                if !self.is_proc {
                    SC_GENERIC_TAG.clone_into(&mut self.instructions[idx].comment);
                }
            }
        }

        if let Some(ref label) = end_label {
            if deferred_end_label.is_none() {
                // Place end label before the trailing pop (or at end)
                let pos = if self.instructions.last().is_some_and(|i| i.op == Op::POP) {
                    self.instructions.len() - 1
                } else {
                    self.instructions.len()
                };
                self.label_positions.insert(label.clone(), pos);
            }
        }

        self.cmd_index += 1;
    }

    /// Dispatch a statement to the appropriate emission handler.
    #[allow(clippy::too_many_lines)]
    pub fn emit_stmt(&mut self, stmt: &Statement, used_generic_invoke: &mut bool) {
        match stmt {
            Statement::AssignConst { name, value, .. } => {
                if needs_stk_push(name, self.is_proc) {
                    self.push_var_ref(name);
                }
                self.push_lit(value);
                self.store_var(name);
                self.emit(Op::POP, vec![]);
            }

            Statement::AssignValue {
                name,
                value,
                value_needs_backsubst,
                ..
            } => {
                let value = if *value_needs_backsubst {
                    tcl_lexer::backslash_subst(value).into_owned()
                } else {
                    value.clone()
                };

                if needs_stk_push(name, self.is_proc) {
                    self.push_var_ref(name);
                }
                self.emit_value_interpolated(&value);
                self.store_var(name);
                self.emit(Op::POP, vec![]);
            }

            Statement::AssignExpr { name, expr, .. } => {
                if needs_stk_push(name, self.is_proc) {
                    self.push_var_ref(name);
                }
                let inner_end = self.fresh_label("cmd_end");
                self.emit_comment(
                    Op::START_CMD,
                    vec![Operand::Label(inner_end.clone()), Operand::Imm(1)],
                    "",
                );
                let guaranteed_numeric = self.emit_expr(expr);
                if !guaranteed_numeric {
                    self.emit(Op::TRY_CVT_TO_NUMERIC, vec![]);
                }
                self.place_label(&inner_end);
                self.store_var(name);
                self.emit(Op::POP, vec![]);
            }

            Statement::Incr { name, amount, .. } => {
                self.emit_incr(name, amount.as_deref());
                self.emit(Op::POP, vec![]);
            }

            Statement::ExprEval { expr, .. } => {
                let guaranteed_numeric = self.emit_expr(expr);
                if !guaranteed_numeric {
                    self.emit(Op::TRY_CVT_TO_NUMERIC, vec![]);
                }
                self.emit(Op::POP, vec![]);
            }

            Statement::Call {
                command,
                args,
                tokens,
                ..
            } => {
                if command == "<cond>" {
                    // CFG placeholder — condition handled by block terminator
                    return;
                }
                if command == "<empty_clause>" {
                    // Empty for-loop clause — 3 nops (tclsh constant-fold)
                    self.literals.intern("");
                    self.emit(Op::NOP, vec![]);
                    self.emit(Op::NOP, vec![]);
                    self.emit(Op::NOP, vec![]);
                    return;
                }

                // Check for {*} expansion
                let has_expand = tokens
                    .as_ref()
                    .and_then(|t| t.expand_word.as_ref())
                    .is_some_and(|ew| ew.iter().any(|&e| e));

                if has_expand {
                    let ew = tokens
                        .as_ref()
                        .and_then(|t| t.expand_word.as_ref())
                        .unwrap();
                    self.emit_expanded_call(command, args, ew);
                    *used_generic_invoke = true;
                } else {
                    self.emit_call(command, args, used_generic_invoke);
                }
            }

            Statement::Barrier {
                command,
                args,
                reason,
                tokens,
                ..
            } => {
                if command.is_empty() {
                    self.emit_comment(Op::NOP, vec![], &format!("barrier: {reason}"));
                } else {
                    let has_expand = tokens
                        .as_ref()
                        .and_then(|t| t.expand_word.as_ref())
                        .is_some_and(|ew| ew.iter().any(|&e| e));

                    if has_expand {
                        let ew = tokens
                            .as_ref()
                            .and_then(|t| t.expand_word.as_ref())
                            .unwrap();
                        self.emit_expanded_call(command, args, ew);
                        *used_generic_invoke = true;
                    } else {
                        self.emit_call(command, args, used_generic_invoke);
                    }
                }
            }

            Statement::Return { value, .. } => {
                let val = value.as_deref().unwrap_or("");
                self.emit_value_interpolated(val);
                self.emit(Op::RETURN_IMM, vec![Operand::Imm(0), Operand::Imm(0)]);
            }

            // Structured control flow is handled by the main emitter loop;
            // if we reach these here, emit a NOP placeholder.
            _ => {
                self.emit_comment(Op::NOP, vec![], "unhandled statement");
            }
        }
    }

    /// Emit a simple value — handles `${var}` references and literals.
    ///
    /// This is a simplified `_emit_value` that handles the common cases.
    /// Full interpolation with command substitution requires the main
    /// emitter pipeline (deferred to later chunks).
    pub fn emit_value_interpolated(&mut self, value: &str) {
        // Braced scalar marker: $={name} → push + loadStk
        if let Some(name) = super::values::parse_braced_scalar_ref(value) {
            self.push_lit(name);
            self.emit(Op::LOAD_STK, vec![]);
            return;
        }
        // Variable reference: ${var} → load
        if let Some(var_name) = parse_simple_var_ref(value) {
            self.load_var(var_name);
            return;
        }
        // Constant-fold [list arg1 arg2 ...]
        if let Some(folded) = super::helpers::fold_list_cmd(value) {
            self.push_lit_no_dedup(&folded);
            return;
        }
        // Constant-fold [dict create k v ...]
        if let Some(folded) = super::helpers::fold_dict_create_cmd(value) {
            self.push_lit(&folded);
            self.emit(Op::DUP, vec![]);
            self.emit(Op::VERIFY_DICT, vec![]);
            return;
        }
        // Default: push as literal
        self.push_lit(value);
    }

    /// Push variable reference for store operations (name/key on stack).
    pub fn push_var_ref(&mut self, name: &str) {
        if let Some((base, elem)) = split_array_ref(name) {
            if self.is_proc && !is_qualified(name) {
                self.push_array_key(elem);
            } else {
                self.push_lit(base);
                self.push_array_key(elem);
            }
        } else {
            self.push_lit(name);
        }
    }

    /// Emit a command call with `{*}` expansion on marked arguments.
    pub fn emit_expanded_call(&mut self, cmd: &str, args: &[String], expand_word: &[bool]) {
        self.emit_comment(Op::EXPAND_START, vec![], &format!("{cmd} (expanded)"));
        // Build full word list: [cmd, *args]
        self.emit_value_interpolated(cmd);
        let mut word_count: u32 = 1;
        for (i, arg) in args.iter().enumerate() {
            self.emit_value_interpolated(arg);
            word_count += 1;
            // expand_word[0] is the command itself, args start at [1]
            if expand_word.get(i + 1).copied().unwrap_or(false) {
                self.emit(Op::EXPAND_STKTOP, vec![Operand::Imm(word_count as i32)]);
            }
        }
        self.emit_comment(Op::INVOKE_EXPANDED, vec![], cmd);
        self.emit(Op::POP, vec![]);
    }

    /// Emit a regular command call.
    #[allow(clippy::similar_names)]
    pub fn emit_call(&mut self, cmd: &str, args: &[String], used_generic_invoke: &mut bool) {
        // break/continue inside loops: emit jump instead of invokeStk
        if cmd == "continue" {
            if let Some(cont_lbl) = self.continue_target.clone() {
                self.emit_comment(Op::JUMP4, vec![Operand::Label(cont_lbl)], "continue");
                return;
            }
        }
        if cmd == "break" {
            if let Some(brk_lbl) = self.break_target.clone() {
                self.emit_comment(Op::JUMP4, vec![Operand::Label(brk_lbl)], "break");
                return;
            }
        }

        self.push_lit(cmd);
        for a in args {
            self.emit_value_interpolated(a);
        }
        #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
        let argc = (1 + args.len()) as i32;
        let op = if argc < 256 {
            Op::INVOKE_STK1
        } else {
            Op::INVOKE_STK4
        };
        self.emit_comment(op, vec![Operand::Imm(argc)], cmd);
        self.emit(Op::POP, vec![]);
        *used_generic_invoke = true;
    }
}

/// Whether a store needs the name/key pushed first (stk-based ops).
fn needs_stk_push(name: &str, is_proc: bool) -> bool {
    if !is_proc {
        return true;
    }
    if is_qualified(name) {
        return true;
    }
    is_array_ref(name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codegen::CodegenCtx;
    use crate::expr_ast::ExprNode;
    use tcl_lexer::Span;

    fn opcodes(ctx: &CodegenCtx) -> Vec<Op> {
        ctx.instructions.iter().map(|i| i.op).collect()
    }

    fn sp() -> Span {
        Span::new(0, 0)
    }

    #[test]
    fn emit_assign_const_proc() {
        let mut ctx = CodegenCtx::new(true, &[]);
        let stmt = Statement::AssignConst {
            span: sp(),
            name: "x".into(),
            value: "42".into(),
        };
        let mut ugi = false;
        ctx.emit_stmt(&stmt, &mut ugi);
        assert!(!ugi);
        assert_eq!(opcodes(&ctx), vec![Op::PUSH1, Op::STORE_SCALAR1, Op::POP]);
    }

    #[test]
    fn emit_assign_const_toplevel() {
        let mut ctx = CodegenCtx::new(false, &[]);
        let stmt = Statement::AssignConst {
            span: sp(),
            name: "x".into(),
            value: "42".into(),
        };
        let mut ugi = false;
        ctx.emit_stmt(&stmt, &mut ugi);
        assert_eq!(
            opcodes(&ctx),
            vec![Op::PUSH1, Op::PUSH1, Op::STORE_STK, Op::POP]
        );
    }

    #[test]
    fn emit_incr_stmt() {
        let mut ctx = CodegenCtx::new(true, &["x"]);
        let stmt = Statement::Incr {
            span: sp(),
            name: "x".into(),
            amount: None,
            safe_on_uninit: false,
        };
        let mut ugi = false;
        ctx.emit_stmt(&stmt, &mut ugi);
        assert_eq!(opcodes(&ctx), vec![Op::INCR_SCALAR1_IMM, Op::POP]);
    }

    #[test]
    fn emit_call_generic() {
        let mut ctx = CodegenCtx::new(false, &[]);
        let stmt = Statement::Call {
            span: sp(),
            command: "puts".into(),
            args: vec!["hello".into()],
            defs: vec![],
            reads: vec![],
            reads_own_defs: false,
            safe_on_uninit: false,
            tokens: None,
        };
        let mut ugi = false;
        ctx.emit_stmt(&stmt, &mut ugi);
        assert!(ugi);
        assert!(opcodes(&ctx).contains(&Op::INVOKE_STK1));
    }

    #[test]
    fn emit_call_break_in_loop() {
        let mut ctx = CodegenCtx::new(true, &[]);
        ctx.break_target = Some("loop_end_0".into());
        let stmt = Statement::Call {
            span: sp(),
            command: "break".into(),
            args: vec![],
            defs: vec![],
            reads: vec![],
            reads_own_defs: false,
            safe_on_uninit: false,
            tokens: None,
        };
        let mut ugi = false;
        ctx.emit_stmt(&stmt, &mut ugi);
        assert!(!ugi);
        assert_eq!(opcodes(&ctx), vec![Op::JUMP4]);
    }

    #[test]
    fn emit_return_empty() {
        let mut ctx = CodegenCtx::new(false, &[]);
        let stmt = Statement::Return {
            span: sp(),
            value: None,
            expr: None,
            braced: false,
        };
        let mut ugi = false;
        ctx.emit_stmt(&stmt, &mut ugi);
        assert!(opcodes(&ctx).contains(&Op::RETURN_IMM));
    }

    #[test]
    fn emit_stmt_with_start_cmd_numbering() {
        let mut ctx = CodegenCtx::new(false, &[]);
        let stmt = Statement::AssignConst {
            span: sp(),
            name: "x".into(),
            value: "1".into(),
        };
        ctx.emit_stmt_with_start_cmd(&stmt, None, None);
        assert!(!opcodes(&ctx).contains(&Op::START_CMD));
        assert_eq!(ctx.cmd_index, 1);

        let stmt2 = Statement::AssignConst {
            span: sp(),
            name: "y".into(),
            value: "2".into(),
        };
        ctx.emit_stmt_with_start_cmd(&stmt2, None, None);
        assert!(opcodes(&ctx).contains(&Op::START_CMD));
        assert_eq!(ctx.cmd_index, 2);
    }

    #[test]
    fn emit_empty_clause() {
        let mut ctx = CodegenCtx::new(true, &[]);
        let stmt = Statement::Call {
            span: sp(),
            command: "<empty_clause>".into(),
            args: vec![],
            defs: vec![],
            reads: vec![],
            reads_own_defs: false,
            safe_on_uninit: false,
            tokens: None,
        };
        let mut ugi = false;
        ctx.emit_stmt(&stmt, &mut ugi);
        assert_eq!(opcodes(&ctx), vec![Op::NOP, Op::NOP, Op::NOP]);
    }

    #[test]
    fn emit_barrier_with_command() {
        let mut ctx = CodegenCtx::new(false, &[]);
        let stmt = Statement::Barrier {
            span: sp(),
            reason: "test".into(),
            command: "eval".into(),
            args: vec!["script".into()],
            tokens: None,
        };
        let mut ugi = false;
        ctx.emit_stmt(&stmt, &mut ugi);
        assert!(ugi);
        assert!(opcodes(&ctx).contains(&Op::INVOKE_STK1));
    }

    #[test]
    fn emit_barrier_without_command() {
        let mut ctx = CodegenCtx::new(false, &[]);
        let stmt = Statement::Barrier {
            span: sp(),
            reason: "side-effect".into(),
            command: String::new(),
            args: vec![],
            tokens: None,
        };
        let mut ugi = false;
        ctx.emit_stmt(&stmt, &mut ugi);
        assert!(!ugi);
        assert_eq!(opcodes(&ctx), vec![Op::NOP]);
    }

    #[test]
    fn emit_assign_expr() {
        let mut ctx = CodegenCtx::new(true, &["x"]);
        let stmt = Statement::AssignExpr {
            span: sp(),
            name: "x".into(),
            expr: ExprNode::Literal {
                text: "42".into(),
                start: 0,
                end: 2,
            },
        };
        let mut ugi = false;
        ctx.emit_stmt(&stmt, &mut ugi);
        assert!(opcodes(&ctx).contains(&Op::START_CMD));
        assert!(opcodes(&ctx).contains(&Op::STORE_SCALAR1));
    }

    #[test]
    fn emit_expr_eval() {
        let mut ctx = CodegenCtx::new(false, &[]);
        let stmt = Statement::ExprEval {
            span: sp(),
            expr: ExprNode::Literal {
                text: "1".into(),
                start: 0,
                end: 1,
            },
        };
        let mut ugi = false;
        ctx.emit_stmt(&stmt, &mut ugi);
        assert!(opcodes(&ctx).contains(&Op::POP));
    }

    #[test]
    fn emit_expanded_call_basic() {
        let mut ctx = CodegenCtx::new(false, &[]);
        ctx.emit_expanded_call("puts", &["hello".into()], &[false, true]);
        let ops = opcodes(&ctx);
        assert!(ops.contains(&Op::EXPAND_START));
        assert!(ops.contains(&Op::EXPAND_STKTOP));
        assert!(ops.contains(&Op::INVOKE_EXPANDED));
    }
}
