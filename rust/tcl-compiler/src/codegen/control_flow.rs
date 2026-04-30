//! Catch/try/error handling control flow emission.
//!
//! Extends [`CodegenCtx`] with methods for emitting `beginCatch4`/`endCatch`
//! bytecodes for `catch` and `try` commands.
//! Ported from `core/compiler/codegen/_control_flow.py`.

#![allow(clippy::too_many_lines, clippy::similar_names, clippy::doc_markdown)]

use crate::cfg::Function as CfgFunction;
use crate::expr_ast::{BinOp, ExprNode};
use crate::ir::Statement;

use super::cmd_subst::parse_cmd_parts;
use super::values::is_qualified;
use super::{bytecode_imm, CodegenCtx, Op, Operand};

// ---------------------------------------------------------------------------
// Constant expression error detection
// ---------------------------------------------------------------------------

/// Detect compile-time expression errors (e.g. divide by zero).
///
/// Returns `(error_message, error_options_string)` when the AST is a
/// fully constant expression that would raise a known error at runtime.
#[must_use]
pub fn detect_const_expr_error(node: &ExprNode) -> Option<(String, String)> {
    if let ExprNode::Binary { op, left, right } = node {
        if matches!(op, BinOp::Div | BinOp::Mod) {
            if let (ExprNode::Literal { .. }, ExprNode::Literal { text: rv, .. }) =
                (left.as_ref(), right.as_ref())
            {
                if rv == "0" {
                    return Some((
                        "divide by zero".to_owned(),
                        "-code 1 -level 0 -errorcode {ARITH DIVZERO {divide by zero}}".to_owned(),
                    ));
                }
            }
        }
    }
    None
}

// ---------------------------------------------------------------------------
// CodegenCtx methods
// ---------------------------------------------------------------------------

impl CodegenCtx<'_> {
    /// Emit inline `beginCatch4`/`endCatch` bytecodes for `catch`.
    ///
    /// Compiles the body as a single command inline, then emits the
    /// normal/handler paths and stores result/options variables.
    /// The catch return code is left on the stack.
    pub fn emit_catch_inline(
        &mut self,
        body_text: &str,
        result_var: Option<&str>,
        options_var: Option<&str>,
    ) {
        // Strip outer braces from body text.
        let body = body_text.trim();
        let body = if body.starts_with('{') && body.ends_with('}') {
            &body[1..body.len() - 1]
        } else {
            body
        };

        // Pre-intern result_var so it gets a lower LVT slot
        if let Some(rv) = result_var {
            if self.is_proc && !is_qualified(rv) {
                self.lvt.intern(rv);
            }
        }

        // beginCatch4 with current nesting depth.
        self.emit(
            Op::BEGIN_CATCH4,
            vec![Operand::Imm(
                i32::try_from(self.catch_depth).expect("catch_depth fits in i32"),
            )],
        );
        self.catch_depth += 1;

        // Compile body command with startCommand wrapping.
        self.emit_catch_body(body);

        // Normal completion: push code "0".
        self.push_lit("0");

        if options_var.is_some() {
            // 3-arg catch: normal path jumps to shared pushReturnOpts.
            let opts_label = self.fresh_label("catch_opts");
            self.emit(Op::JUMP1, vec![Operand::Label(opts_label.clone())]);

            // Handler entry: push caught result and return code.
            self.emit(Op::PUSH_RESULT, vec![]);
            self.emit(Op::PUSH_RETURN_CODE, vec![]);

            // pushReturnOpts shared by both paths.
            self.place_label(&opts_label);
            self.emit(Op::PUSH_RETURN_OPTS, vec![]);
        } else {
            // 2-arg catch: jump over handler to endCatch.
            let end_label = self.fresh_label("catch_end");
            self.emit(Op::JUMP1, vec![Operand::Label(end_label.clone())]);

            // Handler entry: push caught result and return code.
            self.emit(Op::PUSH_RESULT, vec![]);
            self.emit(Op::PUSH_RETURN_CODE, vec![]);

            self.place_label(&end_label);
        }

        // endCatch.
        self.catch_depth -= 1;
        self.emit(Op::END_CATCH, vec![]);

        // Stack: [result, code] (or [result, code, opts] for 3-arg).
        if let Some(ov) = options_var {
            self.store_var(ov);
            self.emit(Op::POP, vec![]);
        }

        self.emit(Op::REVERSE, vec![Operand::Imm(2)]);

        // Store result in result_var.
        if let Some(rv) = result_var {
            self.store_var(rv);
        }
        self.emit(Op::POP, vec![]);
        // Return code stays on the stack as value of catch.
    }

    /// Compile a single-command catch body inline.
    pub fn emit_catch_body(&mut self, body: &str) {
        let body_parts = parse_cmd_parts(body);
        if body_parts.is_empty() {
            self.push_lit("");
            return;
        }

        let body_cmd = &body_parts[0].0;
        let body_args = &body_parts[1..];

        // Commands that need startCommand wrapping
        let needs_start_cmd = matches!(
            body_cmd.as_str(),
            "return" | "error" | "break" | "continue" | "expr"
        );

        let sc_label = if needs_start_cmd {
            let label = self.fresh_label("catch_body_end");
            self.emit_comment(
                Op::START_CMD,
                vec![Operand::Label(label.clone()), Operand::Imm(1)],
                "",
            );
            self.cmd_index += 1;
            Some(label)
        } else {
            None
        };

        match body_cmd.as_str() {
            "return" => self.emit_catch_return(body_args),
            "error" => self.emit_catch_error(body_args),
            "break" => {
                self.emit(Op::BREAK, vec![]);
            }
            "continue" => {
                self.emit(Op::CONTINUE, vec![]);
            }
            "expr" if body_args.len() == 1 => {
                let expr_text = &body_args[0].0;
                let node = crate::expr_parser::parse_expr(expr_text, None);
                if let Some((msg, opts)) = detect_const_expr_error(&node) {
                    self.push_lit(&msg);
                    self.push_lit(&opts);
                    self.emit(Op::SYNTAX, vec![Operand::Imm(1), Operand::Imm(0)]);
                } else {
                    self.emit_expr(&node);
                }
            }
            "try"
                if self.is_proc
                    && body_args.len() == 5
                    && body_args[1].0 == "on"
                    && body_args[2].0 == "error" =>
            {
                let try_sc = self.fresh_label("catch_body_end");
                self.emit_comment(
                    Op::START_CMD,
                    vec![Operand::Label(try_sc.clone()), Operand::Imm(1)],
                    "",
                );
                self.cmd_index += 1;
                self.emit_try_on_error_inline(body_args, &try_sc);
                self.place_label(&try_sc);
                self.seen_generic_invoke = true;
            }
            _ => {
                // Generic command call
                self.push_lit(body_cmd);
                for (arg, braced) in body_args {
                    self.emit_cmd_subst_arg(arg, *braced);
                }
                let argc = bytecode_imm(1 + body_args.len());
                let invoke_op = if argc < 256 {
                    Op::INVOKE_STK1
                } else {
                    Op::INVOKE_STK4
                };
                self.emit(invoke_op, vec![Operand::Imm(argc)]);
                self.seen_generic_invoke = true;
            }
        }

        if let Some(label) = sc_label {
            self.place_label(&label);
        }
    }

    /// Compile `return ?-code C? ?-level L? ?value?` inside a catch body.
    pub fn emit_catch_return(&mut self, args: &[(String, bool)]) {
        let code_names: &[(&str, i32)] = &[
            ("ok", 0),
            ("error", 1),
            ("return", 2),
            ("break", 3),
            ("continue", 4),
        ];
        let mut i = 0;
        let mut code: Option<i32> = None;
        let mut level: Option<i32> = None;

        while i < args.len() {
            let flag = &args[i].0;
            if flag == "-code" && i + 1 < args.len() {
                let val = &args[i + 1].0;
                code = code_names
                    .iter()
                    .find(|(n, _)| *n == val)
                    .map(|(_, c)| *c)
                    .or_else(|| val.parse::<i32>().ok());
                if code.is_none() {
                    break;
                }
                i += 2;
            } else if flag == "-level" && i + 1 < args.len() {
                level = args[i + 1].0.parse::<i32>().ok();
                if level.is_none() {
                    break;
                }
                i += 2;
            } else if flag == "--" {
                i += 1;
                break;
            } else {
                // Non-flag or unknown flag: stop option parsing
                break;
            }
        }

        let remaining = &args[i..];
        let value = if remaining.is_empty() {
            ""
        } else {
            &remaining[0].0
        };

        let (ret_code, ret_level) = match code {
            None => (0, 1),                               // Simple return
            Some(1) => (1, level.unwrap_or(1)),           // error
            Some(c) if c >= 2 => (0, level.unwrap_or(c)), // return/break/continue
            _ => (0, level.unwrap_or(1)),
        };

        self.emit_value(value, true);
        self.push_lit("");
        self.emit(
            Op::RETURN_IMM,
            vec![Operand::Imm(ret_code), Operand::Imm(ret_level)],
        );
    }

    /// Compile `error msg ?info? ?code?` inside a catch body.
    pub fn emit_catch_error(&mut self, args: &[(String, bool)]) {
        if let Some(first) = args.first() {
            self.emit_cmd_subst_arg(&first.0, first.1);
        } else {
            self.push_lit("");
        }
        self.push_lit(""); // options
        self.emit(
            Op::RETURN_IMM,
            vec![Operand::Imm(1), Operand::Imm(0)], // code=error, level=0
        );
    }

    // -- inline try/on error compilation --

    /// Emit inline `try { body } on error {var} { handler }` bytecodes.
    pub fn emit_try_on_error_inline(&mut self, args: &[(String, bool)], normal_exit: &str) {
        let try_body_text = &args[0].0;
        let handler_var = args[3].0.trim().to_owned();
        let handler_body_text = &args[4].0;

        // Allocate LVT slots in tclsh order
        let msg_slot = bytecode_imm(self.lvt.intern(&handler_var));
        let temp_result_name = format!("#temp{}", self.catch_depth);
        let temp_opts_name = format!("#temp{}", self.catch_depth + 1);
        let temp_result_slot = bytecode_imm(self.lvt.intern(&temp_result_name));
        let temp_opts_slot = bytecode_imm(self.lvt.intern(&temp_opts_name));

        let initial_depth = self.catch_depth;

        // Try body exception range
        self.emit(
            Op::BEGIN_CATCH4,
            vec![Operand::Imm(
                i32::try_from(self.catch_depth).expect("catch_depth fits in i32"),
            )],
        );
        self.catch_depth += 1;

        // Compile try body
        self.emit_catch_body(try_body_text);

        // Normal exit from try body
        self.emit(Op::END_CATCH, vec![]);
        self.emit_comment(
            Op::JUMP4,
            vec![Operand::Label(normal_exit.to_owned())],
            "try_on",
        );

        // Exception handler for try body
        self.emit(Op::PUSH_RETURN_CODE, vec![]);
        self.emit(Op::PUSH_RESULT, vec![]);
        self.emit(Op::PUSH_RETURN_OPTS, vec![]);
        self.emit(Op::END_CATCH, vec![]);

        // Store return opts and result to temps
        self.emit_comment(
            Op::STORE_SCALAR1,
            vec![Operand::Imm(temp_opts_slot)],
            &format!("temp var {temp_opts_slot}"),
        );
        self.emit(Op::POP, vec![]);
        self.emit_comment(
            Op::STORE_SCALAR1,
            vec![Operand::Imm(temp_result_slot)],
            &format!("temp var {temp_result_slot}"),
        );
        self.emit(Op::POP, vec![]);

        // Return code dispatch: check if code == 1 (TCL_ERROR)
        self.emit(Op::DUP, vec![]);
        self.push_lit("1");
        self.emit(Op::EQ, vec![]);
        let no_match = self.fresh_label("try_on_nomatch");
        self.emit_comment(
            Op::JUMP_FALSE4,
            vec![Operand::Label(no_match.clone())],
            "try_on",
        );

        // Matched error handler (code == 1)
        self.emit(Op::POP, vec![]);
        self.emit_comment(
            Op::LOAD_SCALAR1,
            vec![Operand::Imm(temp_result_slot)],
            &format!("temp var {temp_result_slot}"),
        );
        self.emit_comment(
            Op::STORE_SCALAR1,
            vec![Operand::Imm(msg_slot)],
            &format!("var \"{handler_var}\""),
        );
        self.emit(Op::POP, vec![]);

        // Handler body exception range
        self.emit(
            Op::BEGIN_CATCH4,
            vec![Operand::Imm(
                i32::try_from(self.catch_depth).expect("catch_depth fits in i32"),
            )],
        );
        self.catch_depth += 1;

        // Compile handler body
        self.emit_try_handler_body(handler_body_text);

        // Normal exit from handler body
        self.emit(Op::END_CATCH, vec![]);
        self.emit_comment(
            Op::JUMP4,
            vec![Operand::Label(normal_exit.to_owned())],
            "try_on",
        );

        // Exception handler for handler body
        self.emit(Op::PUSH_RESULT, vec![]);
        self.emit(Op::PUSH_RETURN_OPTS, vec![]);
        self.emit(Op::PUSH_RETURN_CODE, vec![]);
        self.emit(Op::END_CATCH, vec![]);

        // Check if handler exception code == 1 for -during merge
        self.push_lit("1");
        self.emit(Op::EQ, vec![]);
        let shared_cleanup = self.fresh_label("try_on_cleanup");
        self.emit(
            Op::JUMP_FALSE1,
            vec![Operand::Label(shared_cleanup.clone())],
        );

        // -during merge
        self.emit_comment(
            Op::LOAD_SCALAR1,
            vec![Operand::Imm(temp_opts_slot)],
            &format!("temp var {temp_opts_slot}"),
        );
        self.emit(Op::REVERSE, vec![Operand::Imm(2)]);
        self.emit_comment(
            Op::STORE_SCALAR1,
            vec![Operand::Imm(temp_opts_slot)],
            &format!("temp var {temp_opts_slot}"),
        );
        self.emit(Op::POP, vec![]);
        self.push_lit("-during");
        self.emit(Op::REVERSE, vec![Operand::Imm(2)]);
        self.emit_comment(
            Op::DICT_SET,
            vec![Operand::Imm(1), Operand::Imm(temp_opts_slot)],
            &format!("temp var {temp_opts_slot}"),
        );

        // Shared cleanup
        self.place_label(&shared_cleanup);
        self.emit(Op::REVERSE, vec![Operand::Imm(2)]);
        self.emit(Op::RETURN_STK, vec![]);
        self.emit_comment(
            Op::JUMP4,
            vec![Operand::Label(normal_exit.to_owned())],
            "try_on",
        );

        // No match (code != 1): re-raise
        self.place_label(&no_match);
        self.emit(Op::POP, vec![]);
        self.emit_comment(
            Op::LOAD_SCALAR1,
            vec![Operand::Imm(temp_opts_slot)],
            &format!("temp var {temp_opts_slot}"),
        );
        self.emit_comment(
            Op::LOAD_SCALAR1,
            vec![Operand::Imm(temp_result_slot)],
            &format!("temp var {temp_result_slot}"),
        );
        self.emit(Op::RETURN_STK, vec![]);

        // Restore catch depth
        self.catch_depth = initial_depth;
    }

    /// Compile a try/on handler body inline with `startCommand`.
    pub fn emit_try_handler_body(&mut self, body_text: &str) {
        let parts = parse_cmd_parts(body_text);
        if parts.is_empty() {
            self.push_lit("");
            return;
        }

        let cmd = &parts[0].0;
        let cmd_args = &parts[1..];

        let sc_label = self.fresh_label("handler_body_end");
        self.emit_comment(
            Op::START_CMD,
            vec![Operand::Label(sc_label.clone()), Operand::Imm(1)],
            "",
        );
        self.cmd_index += 1;

        match cmd.as_str() {
            "set" if cmd_args.len() == 2 => {
                self.emit_cmd_subst_arg(&cmd_args[1].0, cmd_args[1].1);
                self.store_var(&cmd_args[0].0);
            }
            _ => {
                self.push_lit(cmd);
                for (a, b) in cmd_args {
                    self.emit_cmd_subst_arg(a, *b);
                }
                let argc = bytecode_imm(1 + cmd_args.len());
                let invoke_op = if argc < 256 {
                    Op::INVOKE_STK1
                } else {
                    Op::INVOKE_STK4
                };
                self.emit(invoke_op, vec![Operand::Imm(argc)]);
            }
        }

        self.place_label(&sc_label);
    }

    // -- inline try/finally compilation --

    /// Emit inline `try { body } finally { cleanup }` bytecodes.
    pub fn emit_try_finally_inline(
        &mut self,
        cfg: &CfgFunction,
        try_body_name: &str,
        try_finally_name: &str,
    ) {
        let body_blk = &cfg.blocks[try_body_name];
        let finally_blk = &cfg.blocks[try_finally_name];

        // try body
        self.emit(
            Op::BEGIN_CATCH4,
            vec![Operand::Imm(
                i32::try_from(self.catch_depth).expect("catch_depth fits in i32"),
            )],
        );
        self.catch_depth += 1;

        for stmt in &body_blk.statements {
            self.emit_try_body_stmt(stmt);
        }

        // Transition: jump over pushResult, converge at pushReturnOpts + endCatch
        let conv = self.fresh_label("try_conv");
        self.emit(Op::JUMP1, vec![Operand::Label(conv.clone())]);
        self.emit(Op::PUSH_RESULT, vec![]);
        self.place_label(&conv);
        self.emit(Op::PUSH_RETURN_OPTS, vec![]);
        self.emit(Op::END_CATCH, vec![]);

        // finally body
        self.emit(
            Op::BEGIN_CATCH4,
            vec![Operand::Imm(
                i32::try_from(self.catch_depth).expect("catch_depth fits in i32"),
            )],
        );
        self.catch_depth += 1;

        for stmt in &finally_blk.statements {
            self.emit_try_finally_stmt(stmt);
        }

        // Normal exit from finally
        self.emit(Op::END_CATCH, vec![]);
        self.emit(Op::POP, vec![]);
        let normal_exit = self.fresh_label("try_normal");
        self.emit(Op::JUMP1, vec![Operand::Label(normal_exit.clone())]);

        // Exception handler for the finally body
        self.emit(Op::PUSH_RESULT, vec![]);
        self.emit(Op::PUSH_RETURN_OPTS, vec![]);
        self.emit(Op::PUSH_RETURN_CODE, vec![]);
        self.emit(Op::END_CATCH, vec![]);

        // Check return code == 1 for -during merge
        self.push_lit("1");
        self.emit(Op::EQ, vec![]);
        let shared_cleanup = self.fresh_label("try_cleanup");
        self.emit(
            Op::JUMP_FALSE1,
            vec![Operand::Label(shared_cleanup.clone())],
        );

        // -during option merging
        self.push_lit("-during");
        self.emit(Op::OVER, vec![Operand::Imm(3)]);
        self.emit(Op::LIST, vec![Operand::Imm(2)]);
        self.emit(Op::LIST_CONCAT, vec![]);

        // Shared cleanup
        self.place_label(&shared_cleanup);
        self.emit(Op::REVERSE, vec![Operand::Imm(4)]);
        self.emit(Op::POP, vec![]);
        self.emit(Op::POP, vec![]);
        let return_target = self.fresh_label("try_return");
        self.emit(Op::JUMP1, vec![Operand::Label(return_target.clone())]);

        // Normal exit: swap result and return opts
        self.place_label(&normal_exit);
        self.emit(Op::REVERSE, vec![Operand::Imm(2)]);

        // Return / re-raise
        self.place_label(&return_target);
        self.emit(Op::RETURN_STK, vec![]);

        // Restore catch depth
        self.catch_depth -= 2;
    }

    /// Emit a statement in try-body context.
    pub fn emit_try_body_stmt(&mut self, stmt: &Statement) {
        if let Statement::Call { command, args, .. } = stmt {
            if command == "error" {
                if let Some(arg) = args.first() {
                    self.emit_value(arg, false);
                } else {
                    self.push_lit("");
                }
                self.push_lit("");
                self.emit(Op::RETURN_IMM, vec![Operand::Imm(1), Operand::Imm(0)]);
                self.cmd_index += 1;
                return;
            }
        }
        let mut ugi = false;
        self.emit_stmt(stmt, &mut ugi);
        // Remove trailing pop — result stays on stack
        if self.instructions.last().is_some_and(|i| i.op == Op::POP) {
            self.instructions.pop();
        }
        self.cmd_index += 1;
    }

    /// Emit a statement in finally-body context.
    pub fn emit_try_finally_stmt(&mut self, stmt: &Statement) {
        let mut ugi = false;
        self.emit_stmt(stmt, &mut ugi);
        // Remove trailing pop — result stays on stack
        if self.instructions.last().is_some_and(|i| i.op == Op::POP) {
            self.instructions.pop();
        }
        self.cmd_index += 1;
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_registry::CommandRegistry;

    #[test]
    fn catch_inline_emits_begin_end_catch() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(true, &[], &registry);
        ctx.emit_catch_inline("set x 1", Some("result"), None);
        let ops: Vec<Op> = ctx.instructions.iter().map(|i| i.op).collect();
        assert!(ops.contains(&Op::BEGIN_CATCH4));
        assert!(ops.contains(&Op::END_CATCH));
    }

    #[test]
    fn catch_inline_stores_result_var() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(true, &[], &registry);
        ctx.emit_catch_inline("set x 1", Some("res"), None);
        let ops: Vec<Op> = ctx.instructions.iter().map(|i| i.op).collect();
        assert!(ops.contains(&Op::STORE_SCALAR1));
    }

    #[test]
    fn catch_inline_with_options_var() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(true, &[], &registry);
        ctx.emit_catch_inline("set x 1", Some("res"), Some("opts"));
        let ops: Vec<Op> = ctx.instructions.iter().map(|i| i.op).collect();
        assert!(ops.contains(&Op::PUSH_RETURN_OPTS));
        // Two stores: one for opts, one for result
        let store_count = ops.iter().filter(|&&o| o == Op::STORE_SCALAR1).count();
        assert!(store_count >= 2);
    }

    #[test]
    fn catch_return_simple() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(true, &[], &registry);
        ctx.emit_catch_return(&[("hello".into(), false)]);
        let ops: Vec<Op> = ctx.instructions.iter().map(|i| i.op).collect();
        assert!(ops.contains(&Op::RETURN_IMM));
    }

    #[test]
    fn catch_return_with_code() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(true, &[], &registry);
        ctx.emit_catch_return(&[
            ("-code".into(), false),
            ("error".into(), false),
            ("oops".into(), false),
        ]);
        let ops: Vec<Op> = ctx.instructions.iter().map(|i| i.op).collect();
        assert!(ops.contains(&Op::RETURN_IMM));
    }

    #[test]
    fn catch_error_emits_return_imm() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(true, &[], &registry);
        ctx.emit_catch_error(&[("oops".into(), false)]);
        let ops: Vec<Op> = ctx.instructions.iter().map(|i| i.op).collect();
        assert!(ops.contains(&Op::RETURN_IMM));
    }

    #[test]
    fn detect_div_by_zero() {
        let node = ExprNode::Binary {
            op: BinOp::Div,
            left: Box::new(ExprNode::Literal {
                text: "1".into(),
                start: 0,
                end: 1,
            }),
            right: Box::new(ExprNode::Literal {
                text: "0".into(),
                start: 4,
                end: 5,
            }),
        };
        let result = detect_const_expr_error(&node);
        assert!(result.is_some());
        assert_eq!(result.unwrap().0, "divide by zero");
    }

    #[test]
    fn no_error_for_non_zero() {
        let node = ExprNode::Binary {
            op: BinOp::Div,
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
        assert!(detect_const_expr_error(&node).is_none());
    }
}
