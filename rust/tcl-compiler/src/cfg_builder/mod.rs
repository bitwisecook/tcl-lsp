//! CFG construction from structured IR.
//!
//! Flattens structured IR (`If`, `For`, `While`, `Switch`, `Catch`,
//! `Try`) into a graph of basic blocks connected by terminators.
//! The per-construct lowering methods live in [`cfg_lower`].
//!
//! Public API:
//! - [`build_cfg`] — build CFGs for a whole module (top-level + procs).
//! - [`build_cfg_function`] — build a CFG for a single script body.

use std::collections::HashMap;

use tcl_lexer::Span;

use crate::cfg::{Block, CfgModule, Function, LoopNode, Terminator};
use crate::expr_ast::ExprNode;
use crate::ir::{Module, Script, Statement};
use crate::ir_helpers::defs_from_ir_script;

mod cfg_lower;

/// Mutable block used during construction, frozen into [`Block`] at the end.
struct MutableBlock {
    name: String,
    statements: Vec<Statement>,
    terminator: Option<Terminator>,
}

/// Builder that accumulates blocks and loop metadata for one function.
pub(crate) struct CfgBuilder {
    counter: u32,
    blocks: HashMap<String, MutableBlock>,
    loop_nodes: HashMap<String, LoopNode>,
    inline_loops: bool,
}

impl CfgBuilder {
    fn new(inline_loops: bool) -> Self {
        Self {
            counter: 0,
            blocks: HashMap::new(),
            loop_nodes: HashMap::new(),
            inline_loops,
        }
    }

    /// Allocate a new empty block with a unique name.
    fn new_block(&mut self, prefix: &str) -> String {
        self.counter += 1;
        let name = format!("{prefix}_{}", self.counter);
        self.blocks.insert(
            name.clone(),
            MutableBlock {
                name: name.clone(),
                statements: Vec::new(),
                terminator: None,
            },
        );
        name
    }

    /// Borrow a mutable block by name. Panics if missing.
    fn block_mut(&mut self, name: &str) -> &mut MutableBlock {
        self.blocks
            .get_mut(name)
            .unwrap_or_else(|| panic!("block {name} not found"))
    }

    /// Set a `Goto` terminator on a block only if it doesn't already
    /// have one.
    fn ensure_goto(&mut self, block_name: &str, target: &str, span: Option<Span>) {
        let block = self.block_mut(block_name);
        if block.terminator.is_none() {
            block.terminator = Some(Terminator::Goto {
                target: target.to_owned(),
                span,
            });
        }
    }

    /// Build a [`Function`] by lowering a script starting at a fresh
    /// entry block, then freezing all mutable blocks.
    fn build_function(&mut self, name: &str, script: &Script) -> Function {
        let entry = self.new_block("entry");
        let tail = self.lower_script(script, &entry);
        if let Some(tail) = tail {
            let exit = self.new_block("exit");
            self.ensure_goto(&tail, &exit, None);
        }

        let frozen: HashMap<String, Block> = self
            .blocks
            .drain()
            .map(|(k, mb)| {
                (
                    k,
                    Block {
                        name: mb.name,
                        statements: mb.statements,
                        terminator: mb.terminator,
                    },
                )
            })
            .collect();

        let loop_nodes = std::mem::take(&mut self.loop_nodes);

        Function {
            name: name.to_owned(),
            entry,
            blocks: frozen,
            loop_nodes,
        }
    }

    /// Lower a script (sequence of IR statements) into CFG blocks.
    ///
    /// `block_name` is the block where the first statement lands.
    /// Returns `Some(tail_block)` — the block where subsequent code
    /// should go — or `None` if control doesn't fall through (e.g.
    /// the script ends with a `return`).
    fn lower_script(&mut self, script: &Script, block_name: &str) -> Option<String> {
        let mut current = block_name.to_owned();

        for stmt in &script.statements {
            // If the current block is already terminated, subsequent
            // statements are dead code.
            if self.block_mut(&current).terminator.is_some() {
                return None;
            }

            match stmt {
                Statement::If { .. } => {
                    current = self.lower_if(stmt, &current);
                }

                Statement::For {
                    condition,
                    raw_args,
                    span,
                    ..
                } => {
                    // Frozen for: condition is a command substitution.
                    if matches!(condition, ExprNode::Command { .. }) && !raw_args.is_empty() {
                        self.block_mut(&current)
                            .statements
                            .push(Statement::Barrier {
                                span: *span,
                                reason: "frozen for (cmd-subst condition)".into(),
                                command: "for".into(),
                                args: raw_args.clone(),
                                tokens: None,
                            });
                    } else {
                        current = self.lower_for(stmt, &current)?;
                    }
                }

                Statement::While {
                    condition,
                    raw_args,
                    span,
                    ..
                } => {
                    if matches!(condition, ExprNode::Command { .. }) && !raw_args.is_empty() {
                        self.block_mut(&current)
                            .statements
                            .push(Statement::Barrier {
                                span: *span,
                                reason: "frozen while (cmd-subst condition)".into(),
                                command: "while".into(),
                                args: raw_args.clone(),
                                tokens: None,
                            });
                    } else {
                        current = self.lower_while(stmt, &current);
                    }
                }

                Statement::Foreach { .. } => {
                    current = self.lower_foreach_dispatch(stmt, &current);
                }

                Statement::Catch { .. } => {
                    self.emit_opaque_catch(stmt, &current);
                }

                Statement::Try { .. } => {
                    current = self.lower_try_dispatch(stmt, &current);
                }

                Statement::Switch { .. } => {
                    current = self.lower_switch(stmt, &current);
                }

                Statement::Return {
                    span,
                    value,
                    expr,
                    braced,
                } => {
                    self.block_mut(&current).terminator = Some(Terminator::Return {
                        value: value.clone(),
                        span: Some(*span),
                        expr: expr.clone(),
                        braced: *braced,
                    });
                    return None;
                }

                // All other statements (assignments, calls, barriers,
                // expr-evals) go straight into the current block.
                other => {
                    self.block_mut(&current).statements.push(other.clone());
                }
            }
        }

        Some(current)
    }

    /// Dispatch `Foreach` — dict for/map, opaque top-level, or inlined.
    fn lower_foreach_dispatch(&mut self, stmt: &Statement, current: &str) -> String {
        let Statement::Foreach {
            is_dict_iteration,
            raw_args,
            iterators,
            is_lmap,
            span,
            ..
        } = stmt
        else {
            unreachable!();
        };

        if *is_dict_iteration && !raw_args.is_empty() {
            let sub = &raw_args[0];
            let qual_cmd = format!("::tcl::dict::{sub}");
            self.block_mut(current).statements.push(Statement::Barrier {
                span: *span,
                reason: "dict for/map".into(),
                command: qual_cmd,
                args: raw_args[1..].to_vec(),
                tokens: None,
            });
            return current.to_owned();
        }

        let has_qualified_vars = iterators
            .iter()
            .any(|it| it.vars.iter().any(|v| v.starts_with("::")));

        if (!self.inline_loops && !raw_args.is_empty()) || has_qualified_vars {
            let cmd = if *is_lmap { "lmap" } else { "foreach" };
            let loop_vars: Vec<String> = iterators.iter().flat_map(|it| it.vars.clone()).collect();
            self.block_mut(current).statements.push(Statement::Call {
                span: *span,
                command: cmd.into(),
                args: raw_args.clone(),
                defs: loop_vars,
                reads: vec![],
                reads_own_defs: false,
                safe_on_uninit: false,
                tokens: None,
            });
            return current.to_owned();
        }

        self.lower_foreach(stmt, current)
    }

    /// Emit an opaque `catch` call with defs for modified variables.
    fn emit_opaque_catch(&mut self, stmt: &Statement, current: &str) {
        let Statement::Catch {
            body,
            result_var,
            options_var,
            raw_args,
            span,
            ..
        } = stmt
        else {
            unreachable!();
        };

        let mut catch_defs = defs_from_ir_script(body);
        if let Some(rv) = result_var {
            catch_defs.push(rv.clone());
        }
        if let Some(ov) = options_var {
            catch_defs.push(ov.clone());
        }
        dedup_preserve_order(&mut catch_defs);
        self.block_mut(current).statements.push(Statement::Call {
            span: *span,
            command: "catch".into(),
            args: raw_args.clone(),
            defs: catch_defs,
            reads: vec![],
            reads_own_defs: false,
            safe_on_uninit: false,
            tokens: None,
        });
    }

    /// Dispatch `Try` — deferred opaque or inlined.
    fn lower_try_dispatch(&mut self, stmt: &Statement, current: &str) -> String {
        let Statement::Try {
            handlers,
            finally_body,
            raw_args,
            body,
            span,
            ..
        } = stmt
        else {
            unreachable!();
        };

        let defer = !self.inline_loops
            && !raw_args.is_empty()
            && (!handlers.is_empty() || finally_body.is_none());

        if defer {
            let mut try_defs = defs_from_ir_script(body);
            for handler in handlers {
                if let Some(vn) = &handler.var_name {
                    try_defs.push(vn.clone());
                }
                if let Some(ov) = &handler.options_var {
                    try_defs.push(ov.clone());
                }
                try_defs.extend(defs_from_ir_script(&handler.body));
            }
            if let Some(fb) = finally_body {
                try_defs.extend(defs_from_ir_script(fb));
            }
            dedup_preserve_order(&mut try_defs);
            self.block_mut(current).statements.push(Statement::Call {
                span: *span,
                command: "try".into(),
                args: raw_args.clone(),
                defs: try_defs,
                reads: vec![],
                reads_own_defs: false,
                safe_on_uninit: false,
                tokens: None,
            });
            return current.to_owned();
        }

        self.lower_try(stmt, current)
    }
}

// Public API

/// Build CFGs for a whole module: top-level script + each procedure.
///
/// When `defer_top_level` is `true`, `foreach`/`catch`/`try` at the
/// top level are compiled as opaque calls (matching tclsh bytecode
/// output). Analysis passes should leave this `false` to get full
/// inlining of loop bodies.
#[must_use]
pub fn build_cfg(module: &Module, defer_top_level: bool) -> CfgModule {
    let top_cfg = build_cfg_function("::top", &module.top_level, !defer_top_level);

    let mut proc_cfgs = HashMap::new();
    for (qname, proc) in &module.procedures {
        proc_cfgs.insert(qname.clone(), build_cfg_function(qname, &proc.body, true));
    }

    CfgModule {
        top_level: top_cfg,
        procedures: proc_cfgs,
    }
}

/// Build a CFG for a single script body.
#[must_use]
pub fn build_cfg_function(name: &str, script: &Script, inline_loops: bool) -> Function {
    let mut builder = CfgBuilder::new(inline_loops);
    builder.build_function(name, script)
}

/// Deduplicate a `Vec` while preserving first-occurrence order.
fn dedup_preserve_order(v: &mut Vec<String>) {
    let mut seen = std::collections::HashSet::new();
    v.retain(|item| seen.insert(item.clone()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{IfClause, Script};
    use tcl_lexer::Span;

    #[test]
    fn empty_script_produces_entry_exit() {
        let func = build_cfg_function("::test", &Script::new(), true);
        assert_eq!(func.name, "::test");
        assert!(func.blocks.contains_key(&func.entry));
        // entry → exit
        assert!(func.blocks.len() >= 2);
    }

    #[test]
    fn linear_script() {
        let script = Script::from_statements(vec![
            Statement::AssignConst {
                span: Span::new(0, 7),
                name: "x".into(),
                value: "1".into(),
            },
            Statement::AssignConst {
                span: Span::new(8, 15),
                name: "y".into(),
                value: "2".into(),
            },
        ]);
        let func = build_cfg_function("::test", &script, true);
        // Entry block should have both statements.
        let entry = &func.blocks[&func.entry];
        assert_eq!(entry.statements.len(), 2);
    }

    #[test]
    fn return_terminates() {
        let script = Script::from_statements(vec![
            Statement::AssignConst {
                span: Span::new(0, 7),
                name: "x".into(),
                value: "1".into(),
            },
            Statement::Return {
                span: Span::new(8, 16),
                value: Some("$x".into()),
                expr: None,
                braced: false,
            },
            Statement::AssignConst {
                span: Span::new(17, 24),
                name: "y".into(),
                value: "2".into(), // dead code
            },
        ]);
        let func = build_cfg_function("::test", &script, true);
        let entry = &func.blocks[&func.entry];
        // Only one statement before the return terminator.
        assert_eq!(entry.statements.len(), 1);
        assert!(matches!(entry.terminator, Some(Terminator::Return { .. })));
    }

    #[test]
    fn if_creates_branches() {
        let script = Script::from_statements(vec![Statement::If {
            span: Span::new(0, 30),
            clauses: vec![IfClause {
                condition: ExprNode::Var {
                    text: "$x".into(),
                    name: "x".into(),
                    start: 0,
                    end: 2,
                },
                condition_span: Span::new(3, 5),
                body: Script::from_statements(vec![Statement::AssignConst {
                    span: Span::new(7, 14),
                    name: "y".into(),
                    value: "1".into(),
                }]),
                body_span: Span::new(6, 15),
            }],
            else_body: None,
            else_span: None,
        }]);
        let func = build_cfg_function("::test", &script, true);
        // Should have at least: entry, if_then, if_next, if_end, exit
        assert!(func.blocks.len() >= 4);
        // Entry block should have a Branch terminator.
        let entry = &func.blocks[&func.entry];
        assert!(
            matches!(entry.terminator, Some(Terminator::Branch { .. })),
            "entry should branch; got {:?}",
            entry.terminator
        );
    }

    #[test]
    fn build_cfg_module() {
        let module = Module::default();
        let cfg = build_cfg(&module, false);
        assert_eq!(cfg.top_level.name, "::top");
        assert!(cfg.procedures.is_empty());
    }

    #[test]
    fn catch_emits_opaque_call() {
        let script = Script::from_statements(vec![Statement::Catch {
            span: Span::new(0, 30),
            body: Script::from_statements(vec![Statement::AssignConst {
                span: Span::new(7, 14),
                name: "inner".into(),
                value: "1".into(),
            }]),
            body_span: Span::new(6, 15),
            result_var: Some("result".into()),
            options_var: None,
            raw_args: vec!["{set inner 1}".into(), "result".into()],
        }]);
        let func = build_cfg_function("::test", &script, true);
        let entry = &func.blocks[&func.entry];
        // Should have a Call to "catch" with defs.
        assert!(entry.statements.iter().any(|s| matches!(
            s,
            Statement::Call {
                command, defs, ..
            } if command == "catch" && !defs.is_empty()
        )));
    }

    #[test]
    fn dedup_preserves_order() {
        let mut v = vec!["a".into(), "b".into(), "a".into(), "c".into(), "b".into()];
        dedup_preserve_order(&mut v);
        assert_eq!(v, vec!["a", "b", "c"]);
    }
}
