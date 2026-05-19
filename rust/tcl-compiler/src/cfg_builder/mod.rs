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

use self::upvar_info::{collect_upvar_targets, UpvarInfo};

mod cfg_lower;
pub mod upvar_info;

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
    /// Map from command name to upvar summary, used to pre-populate
    /// caller-side `defs` on calls to procs that use `upvar`.  Empty
    /// when the builder is constructed without an upvar context
    /// (e.g. for one-off CFGs that don't have a Module to scan).
    /// Mirrors Python `_CFGBuilder._upvar_procs`.
    upvar_procs: HashMap<String, UpvarInfo>,
    /// Map from command name to parameter list, used by the upvar
    /// wiring to resolve param-based upvar sources (`upvar 1 $param
    /// local`) against the actual call-site argument.  Mirrors Python
    /// `_CFGBuilder._proc_params`.
    proc_params: HashMap<String, Vec<String>>,
}

impl CfgBuilder {
    fn new(inline_loops: bool) -> Self {
        Self::new_with_upvars(inline_loops, HashMap::new(), HashMap::new())
    }

    fn new_with_upvars(
        inline_loops: bool,
        upvar_procs: HashMap<String, UpvarInfo>,
        proc_params: HashMap<String, Vec<String>>,
    ) -> Self {
        Self {
            counter: 0,
            blocks: HashMap::new(),
            loop_nodes: HashMap::new(),
            inline_loops,
            upvar_procs,
            proc_params,
        }
    }

    /// Augment a `Statement::Call`'s `defs` with caller-side
    /// variable names that the callee proc will modify via `upvar`.
    /// Returns the statement unchanged for non-`Call` shapes, or when
    /// the command is not a registered upvar proc.
    ///
    /// Mirrors the direct-call branch of Python's
    /// `_apply_upvar_invalidation` in `core/compiler/cfg.py`.  The
    /// embedded-substitution branch (lifting `[upvar_proc ...]` out
    /// of `AssignValue`'s text or `Call`'s args) is a follow-up — the
    /// direct-call form is the dominant pattern and lands first.
    fn apply_upvar_invalidation(&self, mut stmt: Statement) -> Statement {
        if self.upvar_procs.is_empty() {
            return stmt;
        }
        // First borrow immutably to compute the extra defs.
        let extra: Vec<String> = match &stmt {
            Statement::Call { command, args, .. } => self
                .upvar_procs
                .get(command.as_str())
                .map(|info| {
                    let params: &[String] = self
                        .proc_params
                        .get(command.as_str())
                        .map_or(&[][..], Vec::as_slice);
                    info.caller_side_defs(args, params)
                })
                .unwrap_or_default(),
            _ => Vec::new(),
        };
        if extra.is_empty() {
            return stmt;
        }
        // Then merge mutably.
        if let Statement::Call { defs, .. } = &mut stmt {
            for d in extra {
                if !defs.contains(&d) {
                    defs.push(d);
                }
            }
        }
        stmt
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
                                canonical_command: None,
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
                                canonical_command: None,
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

                // Inline block (C34d): flatten the body's statements
                // into the current control-flow stream so SSA / codegen
                // see them as plain inline statements.
                Statement::Block { body, .. } => {
                    if let Some(next_current) = self.lower_script(body, &current) {
                        current = next_current;
                    } else {
                        return None;
                    }
                }

                // All other statements (assignments, calls, barriers,
                // expr-evals) go straight into the current block —
                // after the upvar-invalidation pass augments
                // `Statement::Call`'s `defs` for calls to procs that
                // use `upvar`.
                other => {
                    let augmented = self.apply_upvar_invalidation(other.clone());
                    self.block_mut(&current).statements.push(augmented);
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
                canonical_command: None,
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
                canonical_command: None,
                args: raw_args.clone(),
                defs: loop_vars,
                reads: vec![],
                reads_own_defs: false,
                safe_on_uninit: false,
                tokens: None,
                foreach_groups: None,
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
            tokens,
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
        // Preserve ``tokens`` on the synthetic ``Statement::Call``
        // so the codegen's eval-fallback can detect the braced
        // body word and re-wrap it in ``{…}`` when reconstructing
        // the script for ``tcl_eval``.  Without this,
        // ``catch {$undef} msg`` would lower to ``catch $undef
        // msg`` and the var-read trap would fire before catch
        // could intercept it.  Mirrors upstream commit
        // ``31f5357f`` (PR #341).
        self.block_mut(current).statements.push(Statement::Call {
            span: *span,
            command: "catch".into(),
            canonical_command: None,
            args: raw_args.clone(),
            defs: catch_defs,
            reads: vec![],
            reads_own_defs: false,
            safe_on_uninit: false,
            tokens: tokens.clone(),
            foreach_groups: None,
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
                canonical_command: None,
                args: raw_args.clone(),
                defs: try_defs,
                reads: vec![],
                reads_own_defs: false,
                safe_on_uninit: false,
                tokens: None,
                foreach_groups: None,
            });
            return current.to_owned();
        }

        self.lower_try(stmt, current)
    }
}

// Public API

/// Scan a module for procedures whose bodies contain `upvar`
/// declarations, returning a map from command name to
/// [`UpvarInfo`].  Both the fully qualified name (`::ns::foo`) and
/// the short name (`foo`) are registered so call sites using either
/// spelling resolve to the same info.
///
/// Mirrors Python `_detect_upvar_procs` in `core/compiler/cfg.py`.
#[must_use]
pub fn detect_upvar_procs(module: &Module) -> HashMap<String, UpvarInfo> {
    let mut result: HashMap<String, UpvarInfo> = HashMap::new();
    for (qname, proc) in &module.procedures {
        let info = collect_upvar_targets(&proc.body, &proc.params);
        if info.is_empty() {
            continue;
        }
        if let Some((_, short)) = qname.rsplit_once("::") {
            if !short.is_empty() {
                result.insert(short.to_owned(), info.clone());
            }
        }
        result.insert(qname.clone(), info);
    }
    result
}

/// Return the upvar-procs map and the parameter-list map used by
/// the CFG builder's upvar-invalidation pass.  Both the qualified
/// and short forms are registered for every proc.
///
/// Mirrors Python `prepare_cfg_context` in `core/compiler/cfg.py`.
#[must_use]
pub fn prepare_cfg_context(
    module: &Module,
) -> (HashMap<String, UpvarInfo>, HashMap<String, Vec<String>>) {
    let upvar_procs = detect_upvar_procs(module);
    let mut proc_params: HashMap<String, Vec<String>> = HashMap::new();
    for (qname, proc) in &module.procedures {
        if let Some((_, short)) = qname.rsplit_once("::") {
            if !short.is_empty() {
                proc_params.insert(short.to_owned(), proc.params.clone());
            }
        }
        proc_params.insert(qname.clone(), proc.params.clone());
    }
    (upvar_procs, proc_params)
}

/// Build CFGs for a whole module: top-level script + each procedure.
///
/// When `defer_top_level` is `true`, `foreach`/`catch`/`try` at the
/// top level are compiled as opaque calls (matching tclsh bytecode
/// output). Analysis passes should leave this `false` to get full
/// inlining of loop bodies.
///
/// The builder also applies the upvar-invalidation pass — calls to
/// procedures whose bodies use `upvar` have their `defs` augmented
/// with the caller-side variable names the callee will modify.  See
/// [`prepare_cfg_context`] for the per-module scan.
#[must_use]
pub fn build_cfg(module: &Module, defer_top_level: bool) -> CfgModule {
    let (upvar_procs, proc_params) = prepare_cfg_context(module);

    let mut top_builder =
        CfgBuilder::new_with_upvars(!defer_top_level, upvar_procs.clone(), proc_params.clone());
    let top_cfg = top_builder.build_function("::top", &module.top_level);

    let mut proc_cfgs = HashMap::new();
    for (qname, proc) in &module.procedures {
        let mut builder =
            CfgBuilder::new_with_upvars(true, upvar_procs.clone(), proc_params.clone());
        proc_cfgs.insert(qname.clone(), builder.build_function(qname, &proc.body));
    }

    CfgModule {
        top_level: top_cfg,
        procedures: proc_cfgs,
    }
}

/// Build a CFG for a single script body.  Does not apply the upvar-
/// invalidation pass — use [`build_cfg`] when a whole module is
/// available and call-site def invalidation matters.
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
    use crate::ir::{ForeachIterator, IfClause, Script};
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
            tokens: None,
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
    fn foreach_synthetic_call_records_iterator_group_sizes() {
        // ``foreach {a b} L1 c L2 { … }`` has two iterator groups
        // — the first binds two vars, the second binds one.  The
        // synthesised header `Statement::Call` must record the
        // group sizes via ``foreach_groups`` so codegen can
        // reconstruct the original pairing (mirrors upstream
        // commit ``342d4c7a`` / PR #331).
        let body = Script::from_statements(vec![Statement::AssignConst {
            span: Span::new(0, 0),
            name: "x".into(),
            value: "1".into(),
        }]);
        let script = Script::from_statements(vec![Statement::Foreach {
            span: Span::new(0, 0),
            iterators: vec![
                ForeachIterator {
                    vars: vec!["a".into(), "b".into()],
                    list_arg: "L1".into(),
                },
                ForeachIterator {
                    vars: vec!["c".into()],
                    list_arg: "L2".into(),
                },
            ],
            body,
            body_span: Span::new(0, 0),
            is_lmap: false,
            raw_args: vec![],
            is_dict_iteration: false,
        }]);
        let func = build_cfg_function("::test", &script, true);
        let mut found_groups: Option<Vec<usize>> = None;
        for block in func.blocks.values() {
            for stmt in &block.statements {
                if let Statement::Call {
                    command,
                    foreach_groups,
                    ..
                } = stmt
                {
                    if command == "foreach" {
                        found_groups = foreach_groups.clone();
                    }
                }
            }
        }
        assert_eq!(found_groups, Some(vec![2, 1]));
    }

    #[test]
    fn dedup_preserves_order() {
        let mut v = vec!["a".into(), "b".into(), "a".into(), "c".into(), "b".into()];
        dedup_preserve_order(&mut v);
        assert_eq!(v, vec!["a", "b", "c"]);
    }

    // SYNC-JUN-CFG-uplevel-literal-set wiring tests.
    //
    // Each test drives the full pipeline:
    // `lower_to_ir` → `build_cfg` (which calls `prepare_cfg_context`).
    // The assertions inspect the resulting CFG to confirm that calls
    // to upvar-using procs carry the expected caller-side defs.

    fn lower_module(src: &str) -> Module {
        use tcl_registry::CommandRegistry;
        crate::lowering::lower_to_ir(src, &CommandRegistry::build_default())
    }

    fn find_call_defs<'a>(func: &'a Function, command: &str) -> Option<&'a [String]> {
        for block in func.blocks.values() {
            for stmt in &block.statements {
                if let Statement::Call {
                    command: c, defs, ..
                } = stmt
                {
                    if c == command {
                        return Some(defs);
                    }
                }
            }
        }
        None
    }

    #[test]
    fn detect_upvar_procs_registers_short_and_qualified() {
        let module = lower_module("proc ::ns::p {} { upvar 1 caller_x x }\nproc ::ns::p2 {} {}");
        let upvar_procs = detect_upvar_procs(&module);
        assert!(upvar_procs.contains_key("::ns::p"));
        assert!(upvar_procs.contains_key("p"));
        // p2 has no upvar — not registered.
        assert!(!upvar_procs.contains_key("::ns::p2"));
        assert!(!upvar_procs.contains_key("p2"));
    }

    #[test]
    fn prepare_cfg_context_registers_params_for_all_procs() {
        let module = lower_module("proc ::ns::p {a b} { upvar 1 $a x }\nproc q {c} {}");
        let (_upvar_procs, proc_params) = prepare_cfg_context(&module);
        assert_eq!(
            proc_params.get("::ns::p"),
            Some(&vec!["a".to_string(), "b".to_string()]),
        );
        assert_eq!(
            proc_params.get("p"),
            Some(&vec!["a".to_string(), "b".to_string()]),
        );
        // q has no upvar but its params should still be in proc_params.
        assert_eq!(proc_params.get("q"), Some(&vec!["c".to_string()]));
    }

    #[test]
    fn literal_upvar_proc_call_augments_defs() {
        // Caller invokes a proc whose body has `upvar 1 caller_x x`.
        // The call should land with `caller_x` in its defs.
        let module = lower_module(
            "proc setter {} { upvar 1 caller_x x; set x 1 }\n\
             setter",
        );
        let cfg = build_cfg(&module, false);
        let defs = find_call_defs(&cfg.top_level, "setter")
            .expect("setter call should be in top-level CFG");
        assert!(
            defs.contains(&"caller_x".to_string()),
            "expected caller_x in defs, got {defs:?}",
        );
    }

    #[test]
    fn param_upvar_proc_call_resolves_call_site_arg() {
        // `proc setter {name} { upvar 1 $name x }` aliased to whatever
        // the caller passes for `name`.  Call `setter my_var` should
        // augment the call with `my_var` in defs.
        let module = lower_module(
            "proc setter {name} { upvar 1 $name x; set x 1 }\n\
             setter my_var",
        );
        let cfg = build_cfg(&module, false);
        let defs = find_call_defs(&cfg.top_level, "setter")
            .expect("setter call should be in top-level CFG");
        assert!(
            defs.contains(&"my_var".to_string()),
            "expected my_var in defs, got {defs:?}",
        );
    }

    #[test]
    fn param_upvar_normalises_dollar_call_arg() {
        // `setter $caller_var` — the call passes a `$`-prefixed name;
        // the wiring normalises it to `caller_var` for the def list.
        let module = lower_module(
            "proc setter {name} { upvar 1 $name x }\n\
             setter $caller_var",
        );
        let cfg = build_cfg(&module, false);
        let defs = find_call_defs(&cfg.top_level, "setter")
            .expect("setter call should be in top-level CFG");
        assert!(
            defs.contains(&"caller_var".to_string()),
            "expected caller_var (normalised from $caller_var) in defs, got {defs:?}",
        );
    }

    #[test]
    fn non_upvar_proc_call_unchanged() {
        // No upvar in the callee — the call's defs should be empty.
        let module = lower_module("proc no_upvar {} { set x 1 }\nno_upvar");
        let cfg = build_cfg(&module, false);
        let defs = find_call_defs(&cfg.top_level, "no_upvar")
            .expect("no_upvar call should be in top-level CFG");
        assert!(defs.is_empty(), "expected no augmented defs, got {defs:?}",);
    }

    #[test]
    fn qualified_call_resolves_via_qualified_key() {
        // `proc ::ns::setter` is registered under both `::ns::setter`
        // and `setter`.  A qualified call site `::ns::setter` should
        // resolve via the qualified key.
        let module = lower_module(
            "proc ::ns::setter {} { upvar 1 caller_x x }\n\
             ::ns::setter",
        );
        let cfg = build_cfg(&module, false);
        let defs = find_call_defs(&cfg.top_level, "::ns::setter")
            .expect("::ns::setter call should be in top-level CFG");
        assert!(
            defs.contains(&"caller_x".to_string()),
            "expected caller_x in defs (qualified call), got {defs:?}",
        );
    }

    #[test]
    fn cross_proc_call_inside_proc_body_augments_defs() {
        // Outer proc calls an inner upvar-using proc.  The outer
        // proc's CFG should reflect the augmented defs.
        let module = lower_module(
            "proc inner {} { upvar 1 caller_x x }\n\
             proc outer {} { inner }",
        );
        let cfg = build_cfg(&module, false);
        let outer = cfg
            .procedures
            .get("::outer")
            .expect("outer proc CFG should exist");
        let defs = find_call_defs(outer, "inner").expect("inner call should be in outer CFG");
        assert!(
            defs.contains(&"caller_x".to_string()),
            "expected caller_x in inner's defs (called from outer), got {defs:?}",
        );
    }

    #[test]
    fn upvar_call_inside_if_branch_augments_defs() {
        // Calls inside structured constructs (if branches, while
        // bodies, ...) must also be augmented — the wiring runs in
        // `lower_script`, which is invoked recursively for every
        // body.
        let module = lower_module(
            "proc setter {} { upvar 1 caller_y y }\n\
             if {1} { setter }",
        );
        let cfg = build_cfg(&module, false);
        let defs = find_call_defs(&cfg.top_level, "setter")
            .expect("setter call in if-branch should be in top-level CFG");
        assert!(
            defs.contains(&"caller_y".to_string()),
            "expected caller_y in defs (call inside if branch), got {defs:?}",
        );
    }

    #[test]
    fn empty_module_no_upvar_context_no_panic() {
        // Empty module → empty upvar_procs and proc_params.  Building
        // a CFG should still succeed.
        let module = Module::default();
        let cfg = build_cfg(&module, false);
        assert_eq!(cfg.top_level.name, "::top");
        assert!(cfg.procedures.is_empty());
    }

    #[test]
    fn build_cfg_function_does_not_apply_upvar_wiring() {
        // `build_cfg_function` is the no-context variant used by
        // tests and one-off CFG construction.  It MUST NOT augment
        // defs even when the script calls a known upvar proc —
        // the function only sees the script, not a module.
        let module = lower_module("proc setter {} { upvar 1 caller_x x }\nsetter");
        // Build only the top-level CFG via the no-context API.
        let func = build_cfg_function("::top", &module.top_level, true);
        let defs = find_call_defs(&func, "setter").expect("setter call should be in top-level CFG");
        assert!(
            defs.is_empty(),
            "build_cfg_function without context should leave defs empty, got {defs:?}",
        );
    }
}
