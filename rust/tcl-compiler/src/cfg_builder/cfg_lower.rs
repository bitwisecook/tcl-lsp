//! Per-construct CFG lowering methods.
//!
//! Each method flattens one structured IR construct (`If`, `For`,
//! `While`, `Foreach`, `Switch`, `Catch`, `Try`) into basic blocks
//! with terminators, returning the name of the "continuation" block
//! (or `None` if control doesn't fall through).

use crate::cfg::{LoopNode, Terminator};
use crate::expr_ast::{BinOp, ExprNode};
use crate::ir::{Statement, SwitchMode};

use super::CfgBuilder;

impl CfgBuilder {
    // ── if ────────────────────────────────────────────────────────

    /// Flatten `Statement::If` into cascaded branch blocks.
    pub(super) fn lower_if(&mut self, stmt: &Statement, block_name: &str) -> String {
        let Statement::If {
            span,
            clauses,
            else_body,
            else_span,
            ..
        } = stmt
        else {
            unreachable!("lower_if called with non-If");
        };

        let end_block = self.new_block("if_end");
        let mut dispatch = block_name.to_owned();

        for clause in clauses {
            let then_block = self.new_block("if_then");
            let next_dispatch = self.new_block("if_next");

            self.block_mut(&dispatch).terminator = Some(Terminator::Branch {
                condition: clause.condition.clone(),
                true_target: then_block.clone(),
                false_target: next_dispatch.clone(),
                span: Some(clause.condition_span),
            });

            if let Some(tail) = self.lower_script(&clause.body, &then_block) {
                self.ensure_goto(&tail, &end_block, Some(clause.body_span));
            }

            dispatch = next_dispatch;
        }

        if let Some(eb) = else_body {
            if let Some(tail) = self.lower_script(eb, &dispatch) {
                self.ensure_goto(&tail, &end_block, *else_span);
            }
        } else {
            self.ensure_goto(&dispatch, &end_block, Some(*span));
        }

        end_block
    }

    // ── for ───────────────────────────────────────────────────────

    /// Flatten `Statement::For` into init → header → body → step → header loop.
    pub(super) fn lower_for(&mut self, stmt: &Statement, block_name: &str) -> Option<String> {
        let Statement::For {
            span,
            init,
            init_span,
            condition,
            condition_span,
            next,
            next_span,
            body,
            body_span,
            ..
        } = stmt
        else {
            unreachable!("lower_for called with non-For");
        };

        // Placeholder for empty init clause.
        if init.statements.is_empty() {
            self.block_mut(block_name).statements.push(Statement::Call {
                span: *init_span,
                command: "<empty_clause>".into(),
                args: vec![],
                defs: vec![],
                reads: vec![],
                reads_own_defs: false,
                safe_on_uninit: false,
                tokens: None,
            });
        }
        let init_tail = self.lower_script(init, block_name)?;

        let header = self.new_block("for_header");
        let body_block = self.new_block("for_body");
        let step_block = self.new_block("for_step");
        let end_block = self.new_block("for_end");

        self.ensure_goto(&init_tail, &header, Some(*init_span));

        self.block_mut(&header).terminator = Some(Terminator::Branch {
            condition: condition.clone(),
            true_target: body_block.clone(),
            false_target: end_block.clone(),
            span: Some(*condition_span),
        });

        if let Some(tail) = self.lower_script(body, &body_block) {
            self.ensure_goto(&tail, &step_block, Some(*body_span));
        }

        // Placeholder for empty next clause.
        if next.statements.is_empty() {
            self.block_mut(&step_block)
                .statements
                .push(Statement::Call {
                    span: *next_span,
                    command: "<empty_clause>".into(),
                    args: vec![],
                    defs: vec![],
                    reads: vec![],
                    reads_own_defs: false,
                    safe_on_uninit: false,
                    tokens: None,
                });
        }
        if let Some(step_tail) = self.lower_script(next, &step_block) {
            self.ensure_goto(&step_tail, &header, Some(*next_span));
        }

        self.loop_nodes.insert(
            end_block.clone(),
            LoopNode {
                entry_block: block_name.to_owned(),
                span: *span,
            },
        );

        Some(end_block)
    }

    // ── while ─────────────────────────────────────────────────────

    /// Flatten `Statement::While` into header → body → header loop.
    pub(super) fn lower_while(&mut self, stmt: &Statement, block_name: &str) -> String {
        let Statement::While {
            condition,
            condition_span,
            body,
            body_span,
            ..
        } = stmt
        else {
            unreachable!("lower_while called with non-While");
        };

        let header = self.new_block("while_header");
        let body_block = self.new_block("while_body");
        let end_block = self.new_block("while_end");

        self.ensure_goto(block_name, &header, Some(*condition_span));

        self.block_mut(&header).terminator = Some(Terminator::Branch {
            condition: condition.clone(),
            true_target: body_block.clone(),
            false_target: end_block.clone(),
            span: Some(*condition_span),
        });

        if let Some(tail) = self.lower_script(body, &body_block) {
            self.ensure_goto(&tail, &header, Some(*body_span));
        }

        end_block
    }

    // ── foreach / lmap ────────────────────────────────────────────

    /// Flatten `Statement::Foreach` into a header → body → header loop
    /// with a synthetic variable-definition node at the header.
    pub(super) fn lower_foreach(&mut self, stmt: &Statement, block_name: &str) -> String {
        let Statement::Foreach {
            span,
            iterators,
            body,
            body_span,
            is_lmap,
            is_dict_iteration,
            ..
        } = stmt
        else {
            unreachable!("lower_foreach called with non-Foreach");
        };

        let header = self.new_block("foreach_header");
        let body_block = self.new_block("foreach_body");
        let end_block = self.new_block("foreach_end");

        self.ensure_goto(block_name, &header, Some(*span));

        // Collect all iteration variable names.
        let all_vars: Vec<String> = iterators.iter().flat_map(|it| it.vars.clone()).collect();
        let list_args: Vec<String> = iterators.iter().map(|it| it.list_arg.clone()).collect();

        let fe_cmd = match (*is_dict_iteration, *is_lmap) {
            (true, false) => "dict for",
            (true, true) => "dict map",
            (false, true) => "lmap",
            (false, false) => "foreach",
        };

        // Synthetic def node for iteration variables.
        self.block_mut(&header).statements.push(Statement::Call {
            span: *span,
            command: fe_cmd.into(),
            args: list_args,
            defs: all_vars,
            reads: vec![],
            reads_own_defs: false,
            safe_on_uninit: false,
            tokens: None,
        });

        // Opaque condition: non-deterministic branch.
        self.block_mut(&header).terminator = Some(Terminator::Branch {
            condition: ExprNode::Raw {
                text: "<foreach_has_next>".into(),
            },
            true_target: body_block.clone(),
            false_target: end_block.clone(),
            span: Some(*span),
        });

        if let Some(tail) = self.lower_script(body, &body_block) {
            self.ensure_goto(&tail, &header, Some(*body_span));
        }

        end_block
    }

    // ── switch ────────────────────────────────────────────────────

    /// Flatten `Statement::Switch` into a chain of string-equality branches.
    ///
    /// Glob/regexp switches are emitted as opaque barriers (matching
    /// tclsh's generic `invokeStk` codegen for those modes).
    pub(super) fn lower_switch(&mut self, stmt: &Statement, block_name: &str) -> String {
        let Statement::Switch {
            span,
            subject,
            arms,
            default_body,
            default_span,
            mode,
            raw_args,
            ..
        } = stmt
        else {
            unreachable!("lower_switch called with non-Switch");
        };

        // Glob/regexp → barrier.
        if *mode != SwitchMode::Exact {
            self.block_mut(block_name)
                .statements
                .push(Statement::Barrier {
                    span: *span,
                    reason: format!("switch -{}", mode.as_str()),
                    command: "switch".into(),
                    args: raw_args.clone(),
                    tokens: None,
                });
            return block_name.to_owned();
        }

        let end_block = self.new_block("switch_end");
        let default_block = self.new_block("switch_default");

        // Create body blocks for each arm.
        let body_targets: Vec<String> = arms.iter().map(|_| self.new_block("switch_arm")).collect();

        // Resolve fallthrough: fallthrough arms jump to the next
        // non-fallthrough body, or to default.
        let final_targets: Vec<String> = arms
            .iter()
            .enumerate()
            .map(|(i, arm)| {
                if arm.fallthrough {
                    let mut j = i + 1;
                    while j < arms.len() {
                        if !arms[j].fallthrough {
                            return body_targets[j].clone();
                        }
                        j += 1;
                    }
                    default_block.clone()
                } else {
                    body_targets[i].clone()
                }
            })
            .collect();

        // Chain of equality tests.
        let mut dispatch = block_name.to_owned();
        for (i, arm) in arms.iter().enumerate() {
            let next_dispatch = self.new_block("switch_next");
            let cond = ExprNode::Binary {
                op: BinOp::StrEq,
                left: Box::new(ExprNode::Raw {
                    text: subject.clone(),
                }),
                right: Box::new(ExprNode::Literal {
                    text: arm.pattern.clone(),
                    start: 0,
                    end: 0,
                }),
            };
            self.block_mut(&dispatch).terminator = Some(Terminator::Branch {
                condition: cond,
                true_target: final_targets[i].clone(),
                false_target: next_dispatch.clone(),
                span: arm.pattern_span.into(),
            });
            dispatch = next_dispatch;
        }

        // Last dispatch falls through to default.
        self.ensure_goto(&dispatch, &default_block, default_span.or(Some(*span)));

        // Lower arm bodies.
        for (i, arm) in arms.iter().enumerate() {
            if arm.fallthrough || arm.body.is_none() {
                self.ensure_goto(
                    &body_targets[i],
                    &final_targets[i],
                    arm.body_span.or(Some(arm.pattern_span)),
                );
                continue;
            }
            if let Some(ref body) = arm.body {
                if let Some(tail) = self.lower_script(body, &body_targets[i]) {
                    self.ensure_goto(&tail, &end_block, arm.body_span);
                }
            }
        }

        // Default body.
        if let Some(db) = default_body {
            if let Some(tail) = self.lower_script(db, &default_block) {
                self.ensure_goto(&tail, &end_block, *default_span);
            }
        } else {
            self.ensure_goto(&default_block, &end_block, Some(*span));
        }

        end_block
    }

    // ── try ───────────────────────────────────────────────────────

    /// Flatten `Statement::Try` into body → handlers → finally → end CFG.
    pub(super) fn lower_try(&mut self, stmt: &Statement, block_name: &str) -> String {
        let Statement::Try {
            span,
            body,
            body_span,
            handlers,
            finally_body,
            finally_span,
            ..
        } = stmt
        else {
            unreachable!("lower_try called with non-Try");
        };

        let body_block = self.new_block("try_body");
        let end_block = self.new_block("try_end");

        self.ensure_goto(block_name, &body_block, Some(*span));

        // Where does control go after body succeeds?
        let post_body = if handlers.is_empty() {
            end_block.clone()
        } else {
            self.new_block("try_ok")
        };

        if let Some(tail) = self.lower_script(body, &body_block) {
            self.ensure_goto(&tail, &post_body, Some(*body_span));
        }

        // Each handler reachable from body failure.
        for handler in handlers {
            let handler_block = self.new_block("try_handler");
            self.ensure_goto(block_name, &handler_block, Some(*span));

            let mut var_defs = Vec::new();
            if let Some(vn) = &handler.var_name {
                var_defs.push(vn.clone());
            }
            if let Some(ov) = &handler.options_var {
                var_defs.push(ov.clone());
            }
            if !var_defs.is_empty() {
                self.block_mut(&handler_block)
                    .statements
                    .push(Statement::Call {
                        span: *span,
                        command: "try".into(),
                        args: vec![],
                        defs: var_defs,
                        reads: vec![],
                        reads_own_defs: false,
                        safe_on_uninit: false,
                        tokens: None,
                    });
            }

            if let Some(tail) = self.lower_script(&handler.body, &handler_block) {
                self.ensure_goto(&tail, &end_block, Some(handler.body_span));
            }
        }

        // Success path reaches end.
        if !handlers.is_empty() {
            self.ensure_goto(&post_body, &end_block, Some(*span));
        }

        // Finally block.
        if let Some(fb) = finally_body {
            let finally_block = self.new_block("try_finally");
            let fin_span = finally_span.or(Some(*span));
            self.ensure_goto(&end_block, &finally_block, fin_span);

            let after_finally = self.new_block("try_after_finally");
            if let Some(tail) = self.lower_script(fb, &finally_block) {
                self.ensure_goto(&tail, &after_finally, fin_span);
            }
            return after_finally;
        }

        end_block
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cfg_builder::build_cfg_function;
    use crate::ir::{ForeachIterator, Script, SwitchArm, TryHandler};
    use tcl_lexer::Span;

    #[test]
    fn for_loop_creates_header_body_step() {
        let script = Script::from_statements(vec![Statement::For {
            span: Span::new(0, 40),
            init: Script::from_statements(vec![Statement::AssignConst {
                span: Span::new(5, 12),
                name: "i".into(),
                value: "0".into(),
            }]),
            init_span: Span::new(4, 13),
            condition: ExprNode::Binary {
                op: BinOp::Lt,
                left: Box::new(ExprNode::Var {
                    text: "$i".into(),
                    name: "i".into(),
                    start: 0,
                    end: 2,
                }),
                right: Box::new(ExprNode::Literal {
                    text: "10".into(),
                    start: 5,
                    end: 7,
                }),
            },
            condition_span: Span::new(14, 22),
            next: Script::from_statements(vec![Statement::Incr {
                span: Span::new(24, 30),
                name: "i".into(),
                amount: None,
                safe_on_uninit: false,
            }]),
            next_span: Span::new(23, 31),
            body: Script::new(),
            body_span: Span::new(32, 34),
            raw_args: vec![],
        }]);

        let func = build_cfg_function("::test", &script, true);
        // Should have blocks for: entry, for_header, for_body, for_step, for_end, exit
        assert!(func.blocks.len() >= 5, "got {} blocks", func.blocks.len());
        // A for_header block should have a Branch terminator.
        let header = func
            .blocks
            .values()
            .find(|b| b.name.starts_with("for_header"))
            .expect("should have for_header block");
        assert!(matches!(header.terminator, Some(Terminator::Branch { .. })));
    }

    #[test]
    fn while_loop_creates_header_body() {
        let script = Script::from_statements(vec![Statement::While {
            span: Span::new(0, 20),
            condition: ExprNode::Literal {
                text: "1".into(),
                start: 0,
                end: 1,
            },
            condition_span: Span::new(6, 7),
            body: Script::new(),
            body_span: Span::new(9, 11),
            raw_args: vec![],
        }]);

        let func = build_cfg_function("::test", &script, true);
        let header = func
            .blocks
            .values()
            .find(|b| b.name.starts_with("while_header"))
            .expect("should have while_header");
        assert!(matches!(header.terminator, Some(Terminator::Branch { .. })));
    }

    #[test]
    fn foreach_creates_loop() {
        let script = Script::from_statements(vec![Statement::Foreach {
            span: Span::new(0, 30),
            iterators: vec![ForeachIterator {
                vars: vec!["x".into()],
                list_arg: "$list".into(),
            }],
            body: Script::new(),
            body_span: Span::new(20, 25),
            is_lmap: false,
            raw_args: vec![],
            is_dict_iteration: false,
        }]);

        let func = build_cfg_function("::test", &script, true);
        let header = func
            .blocks
            .values()
            .find(|b| b.name.starts_with("foreach_header"))
            .expect("should have foreach_header");
        // Header should have a synthetic Call for iteration vars.
        assert!(header.statements.iter().any(|s| matches!(
            s,
            Statement::Call { command, .. } if command == "foreach"
        )));
    }

    #[test]
    fn switch_exact_creates_branches() {
        let script = Script::from_statements(vec![Statement::Switch {
            span: Span::new(0, 50),
            subject: "$x".into(),
            subject_span: Span::new(7, 9),
            arms: vec![
                SwitchArm {
                    pattern: "a".into(),
                    pattern_span: Span::new(11, 12),
                    body: Some(Script::new()),
                    body_span: Some(Span::new(13, 15)),
                    fallthrough: false,
                },
                SwitchArm {
                    pattern: "b".into(),
                    pattern_span: Span::new(16, 17),
                    body: Some(Script::new()),
                    body_span: Some(Span::new(18, 20)),
                    fallthrough: false,
                },
            ],
            default_body: None,
            default_span: None,
            mode: SwitchMode::Exact,
            nocase: false,
            raw_args: vec![],
        }]);

        let func = build_cfg_function("::test", &script, true);
        // Entry should have a Branch (first arm test).
        let entry = &func.blocks[&func.entry];
        assert!(matches!(entry.terminator, Some(Terminator::Branch { .. })));
    }

    #[test]
    fn switch_glob_emits_barrier() {
        let script = Script::from_statements(vec![Statement::Switch {
            span: Span::new(0, 30),
            subject: "$x".into(),
            subject_span: Span::new(7, 9),
            arms: vec![],
            default_body: None,
            default_span: None,
            mode: SwitchMode::Glob,
            nocase: false,
            raw_args: vec!["$x".into(), "a".into(), "{}".into()],
        }]);

        let func = build_cfg_function("::test", &script, true);
        let entry = &func.blocks[&func.entry];
        assert!(entry.statements.iter().any(|s| matches!(
            s,
            Statement::Barrier { reason, .. } if reason.contains("glob")
        )));
    }

    #[test]
    fn try_finally_creates_finally_block() {
        let script = Script::from_statements(vec![Statement::Try {
            span: Span::new(0, 40),
            body: Script::new(),
            body_span: Span::new(4, 6),
            handlers: vec![],
            finally_body: Some(Script::from_statements(vec![Statement::Call {
                span: Span::new(20, 35),
                command: "cleanup".into(),
                args: vec![],
                defs: vec![],
                reads: vec![],
                reads_own_defs: false,
                safe_on_uninit: false,
                tokens: None,
            }])),
            finally_span: Some(Span::new(18, 36)),
            raw_args: vec![],
        }]);

        let func = build_cfg_function("::test", &script, true);
        // Should have a try_finally block.
        assert!(
            func.blocks
                .values()
                .any(|b| b.name.starts_with("try_finally")),
            "should have try_finally block"
        );
    }

    #[test]
    fn try_with_handler() {
        let script = Script::from_statements(vec![Statement::Try {
            span: Span::new(0, 60),
            body: Script::new(),
            body_span: Span::new(4, 6),
            handlers: vec![TryHandler {
                kind: "on".into(),
                match_arg: "error".into(),
                var_name: Some("e".into()),
                options_var: None,
                body: Script::new(),
                body_span: Span::new(30, 35),
            }],
            finally_body: None,
            finally_span: None,
            raw_args: vec![],
        }]);

        let func = build_cfg_function("::test", &script, true);
        assert!(
            func.blocks
                .values()
                .any(|b| b.name.starts_with("try_handler")),
            "should have try_handler block"
        );
    }
}
