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

//! Catch/try/error handling control flow emission.
//!
//! Extends [`CodegenCtx`] with methods for emitting `beginCatch4`/`endCatch`
//! bytecodes for `catch` and `try` commands.

use tcl_registry::hooks::{InlineCodegenHookId, LoweringHookId};
use tcl_registry::{CommandRegistry, Traits, TryClauseKind, TryCompletionSelector};

use crate::cfg::Function as CfgFunction;
use crate::expr_ast::{BinOp, ExprNode};
use crate::ir::Statement;

use super::cmd_subst::{parse_cmd_parts, parse_cmd_parts_expand};
use super::values::is_qualified;
use super::{CodegenCtx, Op, Operand, bytecode_imm};

// Constant expression error detection

/// Detect compile-time expression errors (e.g. divide by zero).
///
/// Returns `(error_message, error_options_string)` when the AST is a
/// fully constant expression that would raise a known error at runtime.
#[must_use]
pub fn detect_const_expr_error(node: &ExprNode) -> Option<(String, String)> {
    if let ExprNode::Binary { op, left, right } = node
        && matches!(op, BinOp::Div | BinOp::Mod)
        && let (ExprNode::Literal { .. }, ExprNode::Literal { text: rv, .. }) =
            (left.as_ref(), right.as_ref())
        && rv == "0"
    {
        return Some((
            "divide by zero".to_owned(),
            "-code 1 -level 0 -errorcode {ARITH DIVZERO {divide by zero}}".to_owned(),
        ));
    }
    None
}

/// Whether a body script is a straight-line sequence of simple commands the
/// inline `dict for` emitter can compile — no nested control flow (which needs
/// its own blocks), no loop jumps (which need loop-exception routing we
/// don't emit yet), and no nested definitions.
///
/// The loop-jump set is the registry's [`Traits::BREAKS_LOOP`] /
/// [`Traits::CONTINUES_LOOP`] classification, matched against the raw
/// head word (a `::`-qualified spelling never matched the retired
/// hardcoded names, so it stays straight-line).
fn is_straight_line_body(script: &crate::ir::Script, registry: &CommandRegistry) -> bool {
    script.statements.iter().all(|s| {
        // Reject nested control flow (needs its own blocks), opaque body
        // commands, frame shifts, and `break`/`continue` (need loop-exception
        // routing we don't emit). Everything else — assignments, plain calls,
        // `return` — is straight-line and emits inline.
        match s {
            Statement::If { .. }
            | Statement::For { .. }
            | Statement::While { .. }
            | Statement::Foreach { .. }
            | Statement::Catch { .. }
            | Statement::Try { .. }
            | Statement::Switch { .. }
            | Statement::Block { .. }
            | Statement::UpFrame { .. }
            | Statement::Barrier { .. } => false,
            Statement::Call { command, .. } => !registry.get(command).is_some_and(|spec| {
                spec.name == command.as_str()
                    && spec
                        .traits
                        .intersects(Traits::BREAKS_LOOP | Traits::CONTINUES_LOOP)
            }),
            _ => true,
        }
    })
}

#[derive(Clone, Copy)]
enum InlineBodyEmitter {
    Catch,
    TryHandler,
}

#[derive(Debug, Clone)]
struct TryHandlerBindings {
    body_index: usize,
    handler_body_index: usize,
    result_var: Option<String>,
    options_var: Option<String>,
}

/// Whether a value already resolved by Tcl's word/list grammar can safely use
/// the procedure-local scalar bytecodes emitted by the narrow catch/try
/// specialisers. Other valid Tcl variable names (qualified names, arrays, and
/// names requiring quoting) stay on the generic runtime path.
fn is_inline_local_scalar_name(name: &str) -> bool {
    crate::value_shapes::is_static_var_word(name) && !is_qualified(name)
}

/// Parse a `dict for`/`dict map` variable-list word by the Tcl list grammar
/// and return the two loop-variable names when it is exactly a two-element
/// list of *plain* names — not qualified (`::`), not an array element (`(`),
/// and not needing list quoting (whitespace/empty). Returns `None` (inline
/// emitter bails to the runtime invoke) otherwise, so a 1-element list like
/// `{{a b}}` errors at runtime instead of being miscompiled as two vars, and a
/// spaced name like `{{a b} v}` keeps its runtime semantics.
fn is_two_plain_names(vars_text: &str) -> Option<[String; 2]> {
    let elems = super::helpers::split_list_simple(vars_text);
    let [a, b] = <[String; 2]>::try_from(elems).ok()?;
    for v in [&a, &b] {
        if v.is_empty() || is_qualified(v) || v.contains('(') || v.chars().any(char::is_whitespace)
        {
            return None;
        }
    }
    Some([a, b])
}

/// Normalise a parsed `return -code C -level L` into the `(code, level)`
/// immediates `returnImm` carries, exactly as C's `TclMergeReturnOptions`
/// hands them to `CompileReturnInternal`.
///
/// Two rules do all the work (`tclResult.c:866-974`):
///
/// 1. `-level` defaults to **1**, not 0 — a plain `return` is `(0, 1)`, and
///    `(0, 0)` means something else entirely (`TclProcessReturn` at level 0
///    with code OK falls through to the next instruction).
/// 2. `-code return` is rewritten to `-code ok -level L+1`, so `TCL_RETURN`
///    never reaches the operand: `return -code return` is `(0, 2)`, and
///    `return -code return -level 3` is `(0, 4)`.
///
/// Every other code rides its own operand with the level untouched:
/// `-code error` → `(1, 1)`, `-code break` → `(3, 1)`, `-code continue` →
/// `(4, 1)`, `-code N` → `(N, 1)`.
fn merge_return_operands(code: Option<i32>, level: Option<i32>) -> (i32, i32) {
    let mut ret_code = code.unwrap_or(0);
    let mut ret_level = level.unwrap_or(1);
    if ret_code == 2 {
        ret_level = ret_level.saturating_add(1);
        ret_code = 0;
    }
    (ret_code, ret_level)
}

// CodegenCtx methods

/// Instruction indices in [`CodegenCtx::emit_dict_map`] whose slot operands are
/// emitted with a placeholder and back-patched once the result and iterator
/// temps are interned (they take the highest slots, after the body's locals).
struct DictMapPatch {
    res_store: usize,
    dict_first: usize,
    dict_set: usize,
    dict_next: usize,
    unset_it_err: usize,
    unset_res_err: usize,
    unset_it_exit: usize,
    res_load: usize,
    unset_res_exit: usize,
}

impl CodegenCtx<'_> {
    /// Whether the literal-body `catch` specialiser can preserve the exact
    /// source-word semantics of every argument.
    ///
    /// A braced body is already the script value which `catch` will evaluate.
    /// An unbraced/quoted body must first undergo Tcl substitution and escape
    /// decoding, so it belongs to the generic, explicit-stack runtime command.
    /// Result/options variable names may be bare literals, but only the shared
    /// Tcl word classifier and the local-scalar shape acceptor may prove them.
    pub(crate) fn catch_inline_args_are_static(args: &[(String, bool)]) -> bool {
        args.first().is_some_and(|(_, braced)| *braced)
            && args.iter().skip(1).all(|(name, braced)| {
                !tcl_syntax::naming::word_is_dynamic(name, *braced)
                    && !name.contains('\\')
                    && is_inline_local_scalar_name(name)
            })
    }

    /// Resolve the one `try body on error variableList handler` shape this
    /// inline emitter implements.
    ///
    /// The registry owns the typed clause grammar and Tcl's shared list codec
    /// owns `variableList`. This consumer only adds its code-generation proof:
    /// body and handler must be closed braced script values, and bound names
    /// must fit procedure-local scalar bytecodes. Anything else declines to
    /// the generic, yield-aware runtime `try` implementation.
    fn try_on_error_inline_bindings(
        &self,
        command: &str,
        args: &[(String, bool)],
    ) -> Option<TryHandlerBindings> {
        let arg_refs: Vec<&str> = args.iter().map(|(arg, _)| arg.as_str()).collect();
        let invocation = self.registry.try_control_invocation(
            command,
            &arg_refs,
            self.registry.own_surface_query(),
        )?;
        let [clause] = invocation.clauses.as_slice() else {
            return None;
        };
        if clause.kind != TryClauseKind::On(TryCompletionSelector::Error)
            || clause.fallthrough
            || !args[invocation.body_index].1
            || !args[clause.body_index].1
        {
            return None;
        }
        let variable_list_index = clause.variable_list_index?;
        if tcl_syntax::naming::word_is_dynamic(
            &args[variable_list_index].0,
            args[variable_list_index].1,
        ) || (!args[variable_list_index].1 && args[variable_list_index].0.contains('\\'))
        {
            return None;
        }

        let names = tcl_syntax::list::split_list(&args[variable_list_index].0).ok()?;
        let result_var = names
            .first()
            .filter(|name| !name.is_empty())
            .map(ToString::to_string);
        // Tcl suppresses an empty first result name, but a present empty
        // second name is still a real options-variable name.
        let options_var = names.get(1).map(ToString::to_string);
        if result_var
            .iter()
            .chain(options_var.iter().filter(|name| !name.is_empty()))
            .any(|name| !is_inline_local_scalar_name(name))
        {
            return None;
        }

        Some(TryHandlerBindings {
            body_index: invocation.body_index,
            handler_body_index: clause.body_index,
            result_var,
            options_var,
        })
    }

    /// Emit one complete Tcl body without losing its command boundaries.
    ///
    /// Both inline `catch` and its nested `try` phases use this owner. Each
    /// command is delegated to the phase's existing one-command emitter, and
    /// every non-final result is discarded exactly as script evaluation does.
    /// Fatal/incomplete input takes the runtime evaluator path so the live
    /// surrounding exception range receives the parse error.
    fn emit_segmented_inline_body(&mut self, body: &str, emitter: InlineBodyEmitter) {
        let config = self.lexer_config();
        let segmented = crate::lowering::command_at_time_script_with_config(body, config);
        if segmented.commands.is_empty() && segmented.fatal_tail.is_none() {
            self.push_lit("");
            return;
        }

        let last_complete = segmented.commands.len().saturating_sub(1);
        for (index, command) in segmented.commands.iter().enumerate() {
            let execution_span = command.execution_span(body);
            let command_text = body
                .get(execution_span.start() as usize..execution_span.end() as usize)
                .expect("execution spans index their source body");
            let emitted_start = self.instructions.len();
            match emitter {
                InlineBodyEmitter::Catch => self.emit_catch_body(command_text),
                InlineBodyEmitter::TryHandler => {
                    self.emit_try_handler_command(command_text);
                }
            }
            let body_prefix = body
                .get(..command.span.start() as usize)
                .expect("segment start indexes its source body");
            let line = self.span_line().saturating_add(
                u32::try_from(body_prefix.bytes().filter(|byte| *byte == b'\n').count())
                    .unwrap_or(u32::MAX),
            );
            self.restamp_emitted_inline_command_boundaries(emitted_start, command_text, line);
            if segmented.fatal_tail.is_some() || index != last_complete {
                self.emit(Op::POP, vec![]);
            }
        }

        if let Some((tail_start, _)) = segmented.fatal_tail {
            // The enclosing body word was braced and is already a resolved Tcl
            // value. Push the malformed command suffix verbatim so none of its
            // substitutions occur before EVAL_STK reports the parse error.
            let tail = body
                .get(tail_start..)
                .expect("fatal command start indexes its source body");
            self.push_lit_verbatim(tail);
            self.emit(Op::EVAL_STK, vec![]);
        }
    }

    /// Emit `dict map`'s catch error epilogue (dead in our VM; present for
    /// C-Tcl byte fidelity). Returns the placeholder `unsetScalar` indices for
    /// the iterator and result temps, to be back-patched by the caller.
    fn emit_dict_map_err_epilogue(&mut self, iter_name: &str, result_name: &str) -> (usize, usize) {
        self.emit(Op::PUSH_RETURN_OPTS, vec![]);
        self.emit(Op::PUSH_RESULT, vec![]);
        self.emit(Op::END_CATCH, vec![]);
        let unset_it = self.emit_comment(
            Op::UNSET_SCALAR,
            vec![Operand::Imm(0), Operand::Imm(0)],
            iter_name,
        );
        let unset_res = self.emit_comment(
            Op::UNSET_SCALAR,
            vec![Operand::Imm(0), Operand::Imm(0)],
            result_name,
        );
        self.emit(Op::RETURN_STK, vec![]);
        (unset_it, unset_res)
    }

    /// Emit `dict map`'s normal-exit tail: drop the leftover key/value, close
    /// the catch, unset the iterator, load the accumulated dict as the result,
    /// and unset the result temp. Returns the placeholder indices (iterator
    /// unset, result load, result unset) for the caller to back-patch.
    fn emit_dict_map_normal_exit(
        &mut self,
        iter_name: &str,
        result_name: &str,
    ) -> (usize, usize, usize) {
        self.emit(Op::POP, vec![]);
        self.emit(Op::POP, vec![]);
        self.emit(Op::END_CATCH, vec![]);
        let unset_it = self.emit_comment(
            Op::UNSET_SCALAR,
            vec![Operand::Imm(0), Operand::Imm(0)],
            iter_name,
        );
        let res_load = self.emit_comment(Op::LOAD_SCALAR1, vec![Operand::Imm(0)], result_name);
        let unset_res = self.emit_comment(
            Op::UNSET_SCALAR,
            vec![Operand::Imm(0), Operand::Imm(0)],
            result_name,
        );
        (unset_it, res_load, unset_res)
    }

    /// Back-patch the placeholder slot operands recorded in `p` with the now-known
    /// `res_slot` (result accumulator) and `iter_slot` (dict iterator).
    fn backpatch_dict_map(&mut self, p: &DictMapPatch, res_slot: i32, iter_slot: i32) {
        self.instructions[p.res_store].operands = vec![Operand::Imm(res_slot)];
        self.instructions[p.dict_first].operands = vec![Operand::Imm(iter_slot)];
        self.instructions[p.dict_set].operands = vec![Operand::Imm(1), Operand::Imm(res_slot)];
        self.instructions[p.dict_next].operands = vec![Operand::Imm(iter_slot)];
        self.instructions[p.unset_it_err].operands = vec![Operand::Imm(0), Operand::Imm(iter_slot)];
        self.instructions[p.unset_res_err].operands = vec![Operand::Imm(0), Operand::Imm(res_slot)];
        self.instructions[p.unset_it_exit].operands =
            vec![Operand::Imm(0), Operand::Imm(iter_slot)];
        self.instructions[p.res_load].operands = vec![Operand::Imm(res_slot)];
        self.instructions[p.unset_res_exit].operands =
            vec![Operand::Imm(0), Operand::Imm(res_slot)];
    }

    /// Emit a compiled `dict for {k v} DICT { body }` inline, matching C Tcl's
    /// low-level bytecode: `beginCatch4` → `dictFirst <iter>` → `jumpTrue`
    /// (skip empty) → store k/v → inline body → `dictNext` → `jumpFalse` (loop)
    /// → `jump` (normal exit) → catch epilogue → `pop pop`. Returns `false`
    /// (caller falls back to the runtime invoke) unless this is a proc context
    /// with exactly two loop vars and a straight-line body.
    ///
    /// `vars_text` is the `{k v}` word, `dict_text` the dict expression, and
    /// `body_text` the braced body. The body is re-lowered and each statement
    /// emitted inline. The `beginCatch`/epilogue give C Tcl's iterator cleanup
    /// on error; our VM frees the iterator with the frame, so the epilogue is
    /// present for byte-fidelity but only reached via C Tcl's exception ranges.
    pub fn emit_dict_for(&mut self, vars_text: &str, dict_text: &str, body_text: &str) -> bool {
        // Exactly two loop variables, both plain (compilable-local) names. The
        // `{k v}` word is a Tcl list, so split by the list grammar (not
        // whitespace) — a 1-element list like `{{a b}}` must error at runtime,
        // not be miscompiled as two vars — and bail to the runtime invoke for
        // any name that is qualified, an array element, or needs list quoting.
        let vnames = is_two_plain_names(vars_text);
        let Some(vnames) = vnames else {
            return false;
        };
        if !self.is_proc {
            return false;
        }
        // The body must be a straight-line sequence of simple commands.
        let body_ir =
            crate::lowering::lower_to_ir_with_config(body_text, self.registry, self.lexer_config());
        if !body_ir.procedures.is_empty()
            || !body_ir.methods.is_empty()
            || !is_straight_line_body(&body_ir.top_level, self.registry)
        {
            return false;
        }

        // Slot allocation mirrors C Tcl's `TclCompileDictForCmd`: the two loop
        // variables, then a spare temp, then the body's locals, and finally the
        // iterator temp — so the iterator gets the *highest* slot. Because the
        // body (which interns its own locals) is emitted between `dictFirst` and
        // the iterator's allocation, the `dictFirst`/`dictNext`/`unsetScalar`
        // operands are emitted with a placeholder and back-patched once the
        // iterator slot is known.
        let k_slot = bytecode_imm(self.lvt.intern(&vnames[0]));
        let v_slot = bytecode_imm(self.lvt.intern(&vnames[1]));
        // C Tcl allocates an unused companion temp right after the loop vars.
        let _spare = self
            .lvt
            .intern(&format!("#dictfor_spare{}", self.catch_depth));
        let iter_name = format!("#dictfor{}", self.catch_depth);
        let loop_lbl = self.fresh_label("dict_for_loop");
        let end_lbl = self.fresh_label("dict_for_end");

        // Load the dict value, then begin the iterator under a catch range.
        self.emit_value(dict_text, true);
        self.emit(
            Op::BEGIN_CATCH4,
            vec![Operand::Imm(
                i32::try_from(self.catch_depth).expect("catch_depth fits in i32"),
            )],
        );
        self.catch_depth += 1;
        let dict_first_idx = self.emit(Op::DICT_FIRST, vec![Operand::Imm(0)]);
        // Tag the loop-control jumps `dict_for` so the layout pass keeps them
        // 4-byte (matching C Tcl, which never narrows a dict-for's `jumpTrue4`/
        // `jumpFalse4`); the normal-exit `jump` below is left untagged so it
        // narrows to `jump1` as C Tcl's does.
        self.emit_comment(
            Op::JUMP_TRUE4,
            vec![Operand::Label(end_lbl.clone())],
            "dict_for",
        );

        // Loop body: bind key/value, run the body, advance.
        self.place_label(&loop_lbl);
        self.emit_comment(Op::STORE_SCALAR1, vec![Operand::Imm(k_slot)], &vnames[0]);
        self.emit(Op::POP, vec![]);
        self.emit_comment(Op::STORE_SCALAR1, vec![Operand::Imm(v_slot)], &vnames[1]);
        self.emit(Op::POP, vec![]);
        let mut ugi = false;
        for stmt in &body_ir.top_level.statements {
            self.emit_stmt(stmt, &mut ugi);
        }
        let dict_next_idx = self.emit(Op::DICT_NEXT, vec![Operand::Imm(0)]);
        self.emit_comment(Op::JUMP_FALSE4, vec![Operand::Label(loop_lbl)], "dict_for");
        self.emit_comment(Op::JUMP4, vec![Operand::Label(end_lbl.clone())], "");

        // Catch epilogue (C Tcl reaches this via its iterator-cleanup exception
        // range; our VM frees the iterator with the frame, so it is dead here).
        self.emit(Op::PUSH_RETURN_OPTS, vec![]);
        self.emit(Op::PUSH_RESULT, vec![]);
        self.emit(Op::END_CATCH, vec![]);
        let unset_idx = self.emit_comment(
            Op::UNSET_SCALAR,
            vec![Operand::Imm(0), Operand::Imm(0)],
            &iter_name,
        );
        self.emit(Op::RETURN_STK, vec![]);
        self.catch_depth -= 1;

        // Now the body's locals are interned; the iterator gets the next
        // (highest) slot. Back-patch the placeholders.
        let iter_slot = bytecode_imm(self.lvt.intern(&iter_name));
        self.instructions[dict_first_idx].operands = vec![Operand::Imm(iter_slot)];
        self.instructions[dict_next_idx].operands = vec![Operand::Imm(iter_slot)];
        self.instructions[unset_idx].operands = vec![Operand::Imm(0), Operand::Imm(iter_slot)];

        // Normal exit: drop the leftover key/value from the final dictNext.
        self.place_label(&end_lbl);
        self.emit(Op::POP, vec![]);
        self.emit(Op::POP, vec![]);
        // `dict for` yields the empty string.
        self.push_lit("");
        self.seen_generic_invoke = true;
        true
    }

    /// Emit a compiled `dict map {k v} DICT { body }` inline, matching C Tcl:
    /// like `dict for` but accumulating each iteration's body result into a new
    /// dictionary (`dictSet result[k] = body-result`) which becomes the value.
    /// Returns `false` (runtime-invoke fallback) unless proc context, two loop
    /// vars, and a straight-line body whose final statement yields a value.
    pub fn emit_dict_map(&mut self, vars_text: &str, dict_text: &str, body_text: &str) -> bool {
        // Parse the `{k v}` word by the Tcl list grammar and require exactly two
        // plain names (see `emit_dict_for`); anything else bails to the runtime
        // invoke so malformed / non-simple var lists keep C Tcl's semantics.
        let Some(vnames) = is_two_plain_names(vars_text) else {
            return false;
        };
        if !self.is_proc {
            return false;
        }
        let body_ir =
            crate::lowering::lower_to_ir_with_config(body_text, self.registry, self.lexer_config());
        if !body_ir.procedures.is_empty()
            || !body_ir.methods.is_empty()
            || body_ir.top_level.statements.is_empty()
            || !is_straight_line_body(&body_ir.top_level, self.registry)
        {
            return false;
        }

        // Slots: the result accumulator and iterator temps are allocated after
        // the loop vars and the body's locals (interned during body emission),
        // so both are back-patched once known.
        let k_slot = bytecode_imm(self.lvt.intern(&vnames[0]));
        let v_slot = bytecode_imm(self.lvt.intern(&vnames[1]));
        let result_name = format!("#dictmap_res{}", self.catch_depth);
        let iter_name = format!("#dictmap_it{}", self.catch_depth);
        let loop_lbl = self.fresh_label("dict_map_loop");
        let end_lbl = self.fresh_label("dict_map_end");

        // Snapshot the emit state so a mid-emission bail-out (a body that does
        // not leave a trailing `pop`, e.g. one ending in `return`) rolls back to
        // a pristine context for the caller's runtime-invoke fallback.
        let insns_mark = self.instructions.len();
        let catch_mark = self.catch_depth;

        // Initialise the accumulator to the empty dict.
        self.push_lit("");
        let res_store_idx =
            self.emit_comment(Op::STORE_SCALAR1, vec![Operand::Imm(0)], &result_name);
        self.emit(Op::POP, vec![]);

        self.emit_value(dict_text, true);
        self.emit(
            Op::BEGIN_CATCH4,
            vec![Operand::Imm(
                i32::try_from(self.catch_depth).expect("catch_depth fits in i32"),
            )],
        );
        self.catch_depth += 1;
        let dict_first_idx = self.emit(Op::DICT_FIRST, vec![Operand::Imm(0)]);
        self.emit_comment(
            Op::JUMP_TRUE4,
            vec![Operand::Label(end_lbl.clone())],
            "dict_for",
        );

        self.place_label(&loop_lbl);
        self.emit_comment(Op::STORE_SCALAR1, vec![Operand::Imm(k_slot)], &vnames[0]);
        self.emit(Op::POP, vec![]);
        self.emit_comment(Op::STORE_SCALAR1, vec![Operand::Imm(v_slot)], &vnames[1]);
        self.emit(Op::POP, vec![]);
        // Emit the body; strip the final statement's trailing `pop` so its
        // result (the mapped value) stays on the stack. If the last statement
        // left no `pop`, bail out — an unusual body shape we don't model.
        let mut ugi = false;
        for stmt in &body_ir.top_level.statements {
            self.emit_stmt(stmt, &mut ugi);
        }
        if self.instructions.last().map(|i| i.op) != Some(Op::POP) {
            self.instructions.truncate(insns_mark);
            self.catch_depth = catch_mark;
            return false;
        }
        self.instructions.pop();
        // Accumulate: result[key] = mapped value.
        self.emit_comment(Op::LOAD_SCALAR1, vec![Operand::Imm(k_slot)], &vnames[0]);
        self.emit(Op::OVER, vec![Operand::Imm(1)]);
        let dict_set_idx = self.emit_comment(
            Op::DICT_SET,
            vec![Operand::Imm(1), Operand::Imm(0)],
            &result_name,
        );
        self.emit(Op::POP, vec![]);
        self.emit(Op::POP, vec![]);

        let dict_next_idx = self.emit(Op::DICT_NEXT, vec![Operand::Imm(0)]);
        self.emit_comment(Op::JUMP_FALSE4, vec![Operand::Label(loop_lbl)], "dict_for");
        self.emit_comment(Op::JUMP4, vec![Operand::Label(end_lbl.clone())], "");

        // Error epilogue (dead in our VM; present for C-Tcl fidelity).
        let (unset_it_err_idx, unset_res_err_idx) =
            self.emit_dict_map_err_epilogue(&iter_name, &result_name);
        self.catch_depth -= 1;

        // Normal exit: drop the leftover key/value, close the catch, unset the
        // iterator, load the accumulated dict as the result, unset the temp.
        self.place_label(&end_lbl);
        let (unset_it_exit_idx, res_load_idx, unset_res_exit_idx) =
            self.emit_dict_map_normal_exit(&iter_name, &result_name);

        // Now intern the result + iterator temps (highest slots) and back-patch.
        let res_slot = bytecode_imm(self.lvt.intern(&result_name));
        let iter_slot = bytecode_imm(self.lvt.intern(&iter_name));
        self.backpatch_dict_map(
            &DictMapPatch {
                res_store: res_store_idx,
                dict_first: dict_first_idx,
                dict_set: dict_set_idx,
                dict_next: dict_next_idx,
                unset_it_err: unset_it_err_idx,
                unset_res_err: unset_res_err_idx,
                unset_it_exit: unset_it_exit_idx,
                res_load: res_load_idx,
                unset_res_exit: unset_res_exit_idx,
            },
            res_slot,
            iter_slot,
        );

        self.seen_generic_invoke = true;
        true
    }

    /// Emit a compiled `dict update DICTVAR key1 var1 ?key2 var2 …? { body }`
    /// inline, matching C Tcl: bind each keyed value into its target local under
    /// a (VM-dead) catch range, run the straight-line body, then write the
    /// locals back into the dict (`dictUpdateStart`/`dictUpdateEnd`). `rest` is
    /// `[dictvar, k1, v1, …, body]`. Returns `false` (runtime-invoke fallback)
    /// unless proc context, a plain-local dict var and targets, and a
    /// straight-line body whose final statement yields a value.
    pub fn emit_dict_update(&mut self, rest: &[String]) -> bool {
        if !self.compiles_locals() || rest.len() < 4 || !rest.len().is_multiple_of(2) {
            return false;
        }
        let dict_var = &rest[0];
        if is_qualified(dict_var) || dict_var.contains('(') {
            return false;
        }
        let body_text = rest.last().expect("rest is non-empty");
        let mid = &rest[1..rest.len() - 1]; // (key, targetvar) pairs, even length
        let keys: Vec<String> = mid.iter().step_by(2).cloned().collect();
        let vars: Vec<String> = mid.iter().skip(1).step_by(2).cloned().collect();
        if vars.iter().any(|v| is_qualified(v) || v.contains('(')) {
            return false;
        }
        let body_ir =
            crate::lowering::lower_to_ir_with_config(body_text, self.registry, self.lexer_config());
        if !body_ir.procedures.is_empty()
            || !body_ir.methods.is_empty()
            || body_ir.top_level.statements.is_empty()
            || !is_straight_line_body(&body_ir.top_level, self.registry)
        {
            return false;
        }

        let dict_slot = bytecode_imm(self.lvt.intern(dict_var));
        // Pre-intern the target locals so their slots exist; the names travel
        // out-of-band in `dict_vars` (the VM stores/reads them by name).
        for v in &vars {
            self.lvt.intern(v);
        }
        let end_lbl = self.fresh_label("dict_update_end");

        // Snapshot the emit state so a mid-emission bail-out (a body that does
        // not leave a trailing `pop`, e.g. one ending in `return`) rolls back to
        // a pristine context for the caller's runtime-invoke fallback.
        let insns_mark = self.instructions.len();
        let catch_mark = self.catch_depth;

        // Push the key list.
        for k in &keys {
            self.emit_value_interpolated(k);
        }
        self.emit(Op::LIST, vec![Operand::Imm(bytecode_imm(keys.len()))]);

        // Prologue: read the dict, bind each keyed value to its target local.
        let start_idx = self.emit_comment(
            Op::DICT_UPDATE_START,
            vec![Operand::Imm(dict_slot), Operand::Imm(0)],
            &format!("var \"{dict_var}\""),
        );
        self.instructions[start_idx].dict_vars = Some(vars.clone());

        self.emit(
            Op::BEGIN_CATCH4,
            vec![Operand::Imm(
                i32::try_from(self.catch_depth).expect("catch_depth fits in i32"),
            )],
        );
        self.catch_depth += 1;

        // Body; strip the trailing pop so its result stays on the stack (the
        // `dict update` result). Straight-line statements always end in a pop.
        let mut ugi = false;
        for stmt in &body_ir.top_level.statements {
            self.emit_stmt(stmt, &mut ugi);
        }
        if self.instructions.last().map(|i| i.op) != Some(Op::POP) {
            self.instructions.truncate(insns_mark);
            self.catch_depth = catch_mark;
            return false;
        }
        self.instructions.pop();

        // Normal epilogue: swap the key list above the body result and write the
        // locals back into the dict.
        self.emit(Op::END_CATCH, vec![]);
        self.emit(Op::REVERSE, vec![Operand::Imm(2)]);
        let end_idx = self.emit_comment(
            Op::DICT_UPDATE_END,
            vec![Operand::Imm(dict_slot), Operand::Imm(0)],
            &format!("var \"{dict_var}\""),
        );
        self.instructions[end_idx].dict_vars = Some(vars.clone());
        self.emit_comment(Op::JUMP4, vec![Operand::Label(end_lbl.clone())], "");

        // Error epilogue (dead in our VM; present for C-Tcl byte fidelity).
        self.emit(Op::PUSH_RESULT, vec![]);
        self.emit(Op::PUSH_RETURN_OPTS, vec![]);
        self.emit(Op::END_CATCH, vec![]);
        self.emit(Op::REVERSE, vec![Operand::Imm(3)]);
        let err_end_idx = self.emit_comment(
            Op::DICT_UPDATE_END,
            vec![Operand::Imm(dict_slot), Operand::Imm(0)],
            &format!("var \"{dict_var}\""),
        );
        self.instructions[err_end_idx].dict_vars = Some(vars);
        self.emit(Op::RETURN_STK, vec![]);
        self.catch_depth -= 1;

        self.place_label(&end_lbl);
        self.seen_generic_invoke = true;
        true
    }

    /// Emit a compiled `dict with DICTVAR { body }` inline, matching C Tcl:
    /// expand every key of the dict into a same-named local (`dictExpand`), run
    /// the straight-line body, then fold the locals back into the dict
    /// (`dictRecombineImm`). Returns `false` (runtime-invoke fallback) unless
    /// proc context, a plain-local dict var, and a straight-line body whose
    /// final statement yields a value. The path form (`dict with d k … {body}`)
    /// is left to the runtime invoke.
    pub fn emit_dict_with(&mut self, dict_var: &str, body_text: &str) -> bool {
        if !self.compiles_locals() || is_qualified(dict_var) || dict_var.contains('(') {
            return false;
        }
        let body_ir =
            crate::lowering::lower_to_ir_with_config(body_text, self.registry, self.lexer_config());
        if !body_ir.procedures.is_empty()
            || !body_ir.methods.is_empty()
            || body_ir.top_level.statements.is_empty()
            || !is_straight_line_body(&body_ir.top_level, self.registry)
        {
            return false;
        }

        let dict_slot = bytecode_imm(self.lvt.intern(dict_var));
        let state_name = format!("#dictwith_state{}", self.catch_depth);
        let state_slot = bytecode_imm(self.lvt.intern(&state_name));
        let end_lbl = self.fresh_label("dict_with_end");

        // Snapshot the emit state so a mid-emission bail-out (a body that does
        // not leave a trailing `pop`, e.g. one ending in `return`) rolls back to
        // a pristine context for the caller's runtime-invoke fallback.
        let insns_mark = self.instructions.len();
        let catch_mark = self.catch_depth;

        // Prologue: expand the dict into per-key locals; stash the recombine
        // state in a temp.
        self.emit_comment(
            Op::LOAD_SCALAR1,
            vec![Operand::Imm(dict_slot)],
            &format!("var \"{dict_var}\""),
        );
        self.push_lit("");
        self.emit(Op::DICT_EXPAND, vec![]);
        self.emit_comment(
            Op::STORE_SCALAR1,
            vec![Operand::Imm(state_slot)],
            &state_name,
        );
        self.emit(Op::POP, vec![]);

        self.emit(
            Op::BEGIN_CATCH4,
            vec![Operand::Imm(
                i32::try_from(self.catch_depth).expect("catch_depth fits in i32"),
            )],
        );
        self.catch_depth += 1;

        let mut ugi = false;
        for stmt in &body_ir.top_level.statements {
            self.emit_stmt(stmt, &mut ugi);
        }
        if self.instructions.last().map(|i| i.op) != Some(Op::POP) {
            self.instructions.truncate(insns_mark);
            self.catch_depth = catch_mark;
            return false;
        }
        self.instructions.pop();

        // Normal epilogue: recombine the per-key locals into the dict.
        self.emit(Op::END_CATCH, vec![]);
        self.push_lit("");
        self.emit_comment(
            Op::LOAD_SCALAR1,
            vec![Operand::Imm(state_slot)],
            &state_name,
        );
        self.emit_comment(
            Op::DICT_RECOMBINE_IMM,
            vec![Operand::Imm(dict_slot)],
            &format!("var \"{dict_var}\""),
        );
        self.emit_comment(Op::JUMP4, vec![Operand::Label(end_lbl.clone())], "");

        // Error epilogue (dead in our VM; present for C-Tcl byte fidelity).
        self.emit(Op::PUSH_RETURN_OPTS, vec![]);
        self.emit(Op::PUSH_RESULT, vec![]);
        self.emit(Op::END_CATCH, vec![]);
        self.push_lit("");
        self.emit_comment(
            Op::LOAD_SCALAR1,
            vec![Operand::Imm(state_slot)],
            &state_name,
        );
        self.emit_comment(
            Op::DICT_RECOMBINE_IMM,
            vec![Operand::Imm(dict_slot)],
            &format!("var \"{dict_var}\""),
        );
        self.emit(Op::RETURN_STK, vec![]);
        self.catch_depth -= 1;

        self.place_label(&end_lbl);
        self.seen_generic_invoke = true;
        true
    }

    /// Emit inline `beginCatch4`/`endCatch` bytecodes for `catch`.
    ///
    /// Compiles each complete body command inline, then emits the normal/handler
    /// paths and stores result/options variables.
    /// The catch return code is left on the stack.
    pub fn emit_catch_inline(
        &mut self,
        body_text: &str,
        result_var: Option<&str>,
        options_var: Option<&str>,
    ) {
        // `parse_cmd_parts` has already resolved the outer argument word. The
        // value may legitimately begin and end with braced command words
        // (`catch {{set} x {1}}`), so it must not be stripped a second time.
        let body = body_text;

        // Pre-intern result_var so it gets a lower LVT slot
        if let Some(rv) = result_var
            && self.is_proc
            && !is_qualified(rv)
        {
            self.lvt.intern(rv);
        }

        // beginCatch4 with current nesting depth. The handler label rides
        // out-of-band on the instruction (`Instruction::catch_target`, the
        // analogue of C's `ExceptionRange.catchOffset`) so the VM opens a
        // *live* range; the operand keeps C's range-index meaning and the
        // disassembly its shape. Back-patched once the handler is placed.
        let begin_idx = self.emit(
            Op::BEGIN_CATCH4,
            vec![Operand::Imm(
                i32::try_from(self.catch_depth).expect("catch_depth fits in i32"),
            )],
        );
        self.catch_depth += 1;

        // Keep every complete command in this activation so a yield freezes the
        // live catch range itself. The shared segmenter owns command boundaries;
        // `emit_catch_body` remains the one-command emitter. Discard each
        // non-final result exactly as ordinary script execution does. For a
        // fatal or incomplete tail the compiler-owned command plan keeps the
        // complete prefix executable, then raises the catchable parse error if
        // that prefix finishes normally; substitutions in the bad command do
        // not run.
        self.emit_segmented_inline_body(body, InlineBodyEmitter::Catch);

        // Normal completion: push code "0".
        self.push_lit("0");

        let handler_label = self.fresh_label("catch_handler");
        if options_var.is_some() {
            // 3-arg catch: normal path jumps to shared pushReturnOpts.
            let opts_label = self.fresh_label("catch_opts");
            self.emit(Op::JUMP1, vec![Operand::Label(opts_label.clone())]);

            // Handler entry: push caught result and return code.
            self.place_label(&handler_label);
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
            self.place_label(&handler_label);
            self.emit(Op::PUSH_RESULT, vec![]);
            self.emit(Op::PUSH_RETURN_CODE, vec![]);

            self.place_label(&end_label);
        }
        self.instructions[begin_idx].catch_target = Some(handler_label);

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
    ///
    /// Both classifications here are registry data: the
    /// `startCommand`-wrapping set is [`Traits::NEEDS_START_CMD`] and
    /// the per-command inline emitters dispatch on the spec's
    /// [`InlineCodegenHookId`]. The spec-name equality check keeps
    /// `::`-qualified spellings (`catch {::error x}`) on the generic
    /// path, exactly as the retired raw-word `match` did.
    pub fn emit_catch_body(&mut self, body: &str) {
        // The catch word has already resolved to a script value, but that
        // script still has its own Tcl parse/evaluation phase. Preserve TIP
        // 157 argument expansion in that phase through the same parser and
        // emitter used by ordinary command substitutions. Re-reading it with
        // `parse_cmd_parts` would split adjacent `{*}$args` into the literal
        // `*` and `$args`, changing the callee's argv (notably Tcltest's
        // `catch {Configure {*}$args}`).
        let expand_syntax = self
            .dialect
            .is_none_or(|profile| profile.grammar.expand_syntax);
        if expand_syntax && body.contains("{*}") {
            let expanded = parse_cmd_parts_expand(body);
            if expanded.iter().any(|(_, _, expand)| *expand) {
                self.emit_expanded_cmd_subst(&expanded);
                self.seen_generic_invoke = true;
                return;
            }
        }
        let body_parts = parse_cmd_parts(body);
        if body_parts.is_empty() {
            self.push_lit("");
            return;
        }

        let body_cmd = &body_parts[0].0;
        let body_args = &body_parts[1..];

        let spec = self
            .registry
            .get(body_cmd)
            .filter(|s| s.name == body_cmd.as_str());

        // Commands that need startCommand wrapping.
        let needs_start_cmd = spec.is_some_and(|s| s.traits.contains(Traits::NEEDS_START_CMD));

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

        // Hooks this catch-body dispatcher does not specialise
        // (`Incr`, `String`, …, which only the value-position
        // dispatcher in `cmd_subst` emits inline) fall to the generic
        // invoke arm, as do guard failures.
        match self.inline_cmd_subst_hook(body_cmd, body_args) {
            Some(InlineCodegenHookId::Return) => self.emit_catch_return(body_args),
            Some(InlineCodegenHookId::Error) => self.emit_catch_error(body_args),
            Some(InlineCodegenHookId::Break) => {
                self.emit(Op::BREAK, vec![]);
            }
            Some(InlineCodegenHookId::Continue) => {
                self.emit(Op::CONTINUE, vec![]);
            }
            Some(InlineCodegenHookId::Expr) if body_args.len() == 1 => {
                let expr_text = &body_args[0].0;
                // Parsed under the compile's dialect, as lowering parses a
                // statement-position `expr` (issue #1435).
                let node = self.parse_compile_expr(expr_text);
                if let Some((msg, opts)) = detect_const_expr_error(&node) {
                    self.push_lit(&msg);
                    self.push_lit(&opts);
                    self.emit(Op::SYNTAX, vec![Operand::Imm(1), Operand::Imm(0)]);
                } else {
                    self.emit_expr(&node);
                }
            }
            Some(InlineCodegenHookId::Try) if self.is_proc => {
                if self
                    .try_on_error_inline_bindings(body_cmd, body_args)
                    .is_some()
                {
                    let try_sc = self.fresh_label("catch_body_end");
                    self.emit_comment(
                        Op::START_CMD,
                        vec![Operand::Label(try_sc.clone()), Operand::Imm(1)],
                        "",
                    );
                    self.cmd_index += 1;
                    self.emit_try_on_error_inline(body_cmd, body_args, &try_sc);
                    self.place_label(&try_sc);
                    self.seen_generic_invoke = true;
                } else {
                    self.emit_generic_cmd_subst(body_cmd, body_args);
                    self.seen_generic_invoke = true;
                }
            }
            _ => {
                // One shared fixed-arity generic emitter owns word
                // substitution and specialised entered-command metadata.
                self.emit_generic_cmd_subst(body_cmd, body_args);
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
                    .or_else(|| {
                        self.parse_int_operand(val)
                            .and_then(|v| i32::try_from(v).ok())
                    });
                if code.is_none() {
                    break;
                }
                i += 2;
            } else if flag == "-level" && i + 1 < args.len() {
                level = self
                    .parse_int_operand(&args[i + 1].0)
                    .and_then(|v| i32::try_from(v).ok());
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

        let (ret_code, ret_level) = merge_return_operands(code, level);

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

    /// Emit the `-during` merge sequence: load saved opts, prepend
    /// the new error opts, dict-set `-during` key, store back to
    /// temps slot.  Extracted from
    /// [`Self::emit_try_on_error_inline`].
    fn emit_try_during_merge(&mut self, temp_opts_slot: i32) {
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
    }

    /// Emit the no-match (code != 1) re-raise path: pop the
    /// dispatch flag, reload saved opts + result, and `RETURN_STK`.
    fn emit_try_no_match_reraise(&mut self, temp_opts_slot: i32, temp_result_slot: i32) {
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
    }

    /// Begin one inline exception range and advance the shared range index.
    fn begin_inline_catch_range(&mut self) -> usize {
        let begin = self.emit(
            Op::BEGIN_CATCH4,
            vec![Operand::Imm(
                i32::try_from(self.catch_depth).expect("catch_depth fits in i32"),
            )],
        );
        self.catch_depth += 1;
        begin
    }

    /// Bind the caught error result/options to the handler's declared local
    /// variables. The typed try-clause parser owns which names are present;
    /// this helper owns their bytecode transfer from the shared temp slots.
    fn emit_try_error_bindings(
        &mut self,
        bindings: &TryHandlerBindings,
        result_slot: Option<i32>,
        options_slot: Option<i32>,
        temp_result_slot: i32,
        temp_opts_slot: i32,
    ) {
        if let (Some(name), Some(slot)) = (&bindings.result_var, result_slot) {
            self.emit_comment(
                Op::LOAD_SCALAR1,
                vec![Operand::Imm(temp_result_slot)],
                &format!("temp var {temp_result_slot}"),
            );
            self.emit_comment(
                Op::STORE_SCALAR1,
                vec![Operand::Imm(slot)],
                &format!("var \"{name}\""),
            );
            self.emit(Op::POP, vec![]);
        }
        if let (Some(name), Some(slot)) = (&bindings.options_var, options_slot) {
            self.emit_comment(
                Op::LOAD_SCALAR1,
                vec![Operand::Imm(temp_opts_slot)],
                &format!("temp var {temp_opts_slot}"),
            );
            self.emit_comment(
                Op::STORE_SCALAR1,
                vec![Operand::Imm(slot)],
                &format!("var \"{name}\""),
            );
            self.emit(Op::POP, vec![]);
        }
    }

    /// Emit inline `try { body } on error {var} { handler }` bytecodes.
    pub fn emit_try_on_error_inline(
        &mut self,
        command: &str,
        args: &[(String, bool)],
        normal_exit: &str,
    ) {
        let bindings = self
            .try_on_error_inline_bindings(command, args)
            .expect("caller proved the typed literal try/on-error shape");
        let try_body_text = &args[bindings.body_index].0;
        let handler_body_text = &args[bindings.handler_body_index].0;

        // Allocate LVT slots in tclsh order
        let msg_slot = bindings
            .result_var
            .as_ref()
            .map(|name| bytecode_imm(self.lvt.intern(name)));
        let handler_opts_slot = bindings
            .options_var
            .as_ref()
            .map(|name| bytecode_imm(self.lvt.intern(name)));
        let temp_result_name = format!("#temp{}", self.catch_depth);
        let temp_opts_name = format!("#temp{}", self.catch_depth + 1);
        let temp_result_slot = bytecode_imm(self.lvt.intern(&temp_result_name));
        let temp_opts_slot = bytecode_imm(self.lvt.intern(&temp_opts_name));

        let initial_depth = self.catch_depth;

        // Try body exception range
        let try_begin_idx = self.begin_inline_catch_range();

        // Compile the complete try body in this activation so a yield freezes
        // this phase's live exception range.
        self.emit_segmented_inline_body(try_body_text, InlineBodyEmitter::Catch);

        // Normal exit from try body
        self.emit(Op::END_CATCH, vec![]);
        self.emit_comment(
            Op::JUMP4,
            vec![Operand::Label(normal_exit.to_owned())],
            "try_on",
        );

        // Exception handler for try body. The out-of-band target is the VM's
        // analogue of C Tcl's exception-range catch offset.
        let try_exception = self.fresh_label("try_on_body_exception");
        self.place_label(&try_exception);
        self.instructions[try_begin_idx].catch_target = Some(try_exception);
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
        self.emit_try_error_bindings(
            &bindings,
            msg_slot,
            handler_opts_slot,
            temp_result_slot,
            temp_opts_slot,
        );

        // Handler body exception range
        let handler_begin_idx = self.begin_inline_catch_range();

        // Compile handler body
        self.emit_try_handler_body(handler_body_text);

        // Normal exit from handler body
        self.emit(Op::END_CATCH, vec![]);
        self.emit_comment(
            Op::JUMP4,
            vec![Operand::Label(normal_exit.to_owned())],
            "try_on",
        );

        // Exception handler for handler body.
        let handler_exception = self.fresh_label("try_on_handler_exception");
        self.place_label(&handler_exception);
        self.instructions[handler_begin_idx].catch_target = Some(handler_exception);
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
        self.emit_try_during_merge(temp_opts_slot);

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
        self.emit_try_no_match_reraise(temp_opts_slot, temp_result_slot);

        // Restore catch depth
        self.catch_depth = initial_depth;
    }

    /// Compile a complete try/on handler body inline with `startCommand` at
    /// each command boundary.
    pub fn emit_try_handler_body(&mut self, body_text: &str) {
        self.emit_segmented_inline_body(body_text, InlineBodyEmitter::TryHandler);
    }

    /// Compile one command of a try/on handler body.
    fn emit_try_handler_command(&mut self, body_text: &str) {
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

        let arg_refs: Vec<&str> = cmd_args.iter().map(|(arg, _)| arg.as_str()).collect();
        let inline_lowering = self.inline_lowering_hook(cmd, &arg_refs);
        match inline_lowering {
            // The registry's typed lowering hook proves that this is the
            // two-argument `set` shape, but its first word is still a Tcl
            // variable-name word. The specialised STORE_SCALAR1 path cannot
            // perform Tcl's dynamic name substitution (e.g. `set $v value`),
            // so decline it unless the shared scalar-name shape proves the
            // target is a procedure-local name.
            Some((LoweringHookId::Set, binding))
                if cmd_args.len() == 2
                    && !tcl_syntax::naming::word_is_dynamic(&cmd_args[0].0, cmd_args[0].1)
                    && is_inline_local_scalar_name(&cmd_args[0].0) =>
            {
                self.require_command_binding(&binding);
                self.emit_cmd_subst_arg(&cmd_args[1].0, cmd_args[1].1);
                self.store_var(&cmd_args[0].0);
            }
            _ => {
                self.emit_generic_cmd_subst(cmd, cmd_args);
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
        let body_blk = cfg
            .block_by_name(try_body_name)
            .expect("try body block present");
        let finally_blk = cfg
            .block_by_name(try_finally_name)
            .expect("try finally block present");

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
    ///
    /// A call whose spec carries the [`InlineCodegenHookId::Error`]
    /// inline hook (i.e. `error`, whose first argument is the message)
    /// emits the `returnImm 1 0` throw sequence directly; the raw-name
    /// equality on the spec keeps qualified spellings on the generic
    /// statement path, as the retired `command == "error"` check did.
    pub fn emit_try_body_stmt(&mut self, stmt: &Statement) {
        if let Statement::Call { command, args, .. } = stmt
            && self.inline_codegen_hook(
                command,
                &args.iter().map(String::as_str).collect::<Vec<_>>(),
            ) == Some(InlineCodegenHookId::Error)
        {
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

// Tests

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_registry::CommandRegistry;

    /// The catch-body `expr` re-parse follows the compile's dialect, and its
    /// operator set follows the target release (issue #1435): `catch {expr {2
    /// ** 3}}` compiled for 8.4 used to fold to a push of `8` and report
    /// success, while the same source evaluated through `exprStk` is rejected
    /// as C Tcl 8.4 rejects it.
    #[test]
    fn catch_body_expr_follows_the_compile_target_release() {
        let registry = CommandRegistry::build_default();

        let mut old = CodegenCtx::new(true, &[], &registry);
        old.dialect =
            Some(tcl_registry::model::ingress::resolve_environment("tcl8.4").analyser_profile());
        old.emit_catch_body("expr {2 ** 3}");
        let ops: Vec<Op> = old.instructions.iter().map(|i| i.op).collect();
        assert!(ops.contains(&Op::EXPR_STK), "{ops:?}");
        assert!(old.literals.entries().iter().any(|l| l == "2 ** 3"));

        let mut modern = CodegenCtx::new(true, &[], &registry);
        modern.dialect =
            Some(tcl_registry::model::ingress::resolve_environment("tcl8.5").analyser_profile());
        modern.emit_catch_body("expr {2 ** 3}");
        let ops: Vec<Op> = modern.instructions.iter().map(|i| i.op).collect();
        assert!(!ops.contains(&Op::EXPR_STK), "{ops:?}");
        assert!(modern.literals.entries().iter().any(|l| l == "8"));
    }

    /// A catch body is a fresh script parse. Its `{*}` marker must reach the
    /// expansion-aware command emitter instead of becoming a literal `*`
    /// argument beside the value to expand.
    #[test]
    fn catch_body_preserves_argument_expansion() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(true, &["args"], &registry);
        ctx.dialect =
            Some(tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile());

        ctx.emit_catch_body("Configure {*}$args");

        let ops: Vec<Op> = ctx.instructions.iter().map(|i| i.op).collect();
        assert!(ops.contains(&Op::EXPAND_START), "{ops:?}");
        assert!(ops.contains(&Op::EXPAND_STKTOP), "{ops:?}");
        assert!(ops.contains(&Op::INVOKE_EXPANDED), "{ops:?}");
        assert!(!ops.contains(&Op::INVOKE_STK1), "{ops:?}");
        assert!(
            ctx.literals.entries().iter().all(|literal| literal != "*"),
            "the expansion marker must not enter argv: {:?}",
            ctx.literals.entries()
        );
    }

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
    fn catch_inline_multicommand_body_preserves_each_command_boundary() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(true, &[], &registry);
        ctx.dialect =
            Some(tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile());
        let body = "set x [yield a]; error \"boom-$x\"";

        ctx.emit_catch_inline(body, Some("result"), None);

        let ops: Vec<Op> = ctx
            .instructions
            .iter()
            .map(|instruction| instruction.op)
            .collect();
        assert!(ops.contains(&Op::BEGIN_CATCH4), "{ops:?}");
        assert!(!ops.contains(&Op::EVAL_STK), "{ops:?}");
        assert!(ops.contains(&Op::END_CATCH), "{ops:?}");
        assert!(
            ops.iter().filter(|&&op| op == Op::INVOKE_STK1).count() >= 2,
            "the nested yield and enclosing set must retain distinct invokes: {ops:?}"
        );
        assert!(
            ctx.literals.entries().iter().all(|literal| literal != ";"),
            "the command separator must not become an argument: {:?}",
            ctx.literals.entries()
        );
        assert!(
            ctx.instructions.iter().any(|instruction| {
                instruction.op == Op::START_CMD
                    && instruction.source_cmd_text == "error \"boom-$x\""
            }),
            "nested replay text must retain the quoted last word: {:?}",
            ctx.instructions
                .iter()
                .filter(|instruction| instruction.op == Op::START_CMD)
                .map(|instruction| instruction.source_cmd_text.as_str())
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn catch_inline_incomplete_body_uses_catchable_script_path() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(true, &[], &registry);
        ctx.emit_catch_inline("set x {", Some("result"), None);

        let ops: Vec<Op> = ctx
            .instructions
            .iter()
            .map(|instruction| instruction.op)
            .collect();
        assert!(ops.contains(&Op::EVAL_STK), "{ops:?}");
    }

    #[test]
    fn catch_inline_runs_complete_prefix_before_fatal_command_tail() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(true, &[], &registry);
        ctx.emit_catch_inline("incr side; set x \"", Some("result"), None);

        let ops: Vec<Op> = ctx
            .instructions
            .iter()
            .map(|instruction| instruction.op)
            .collect();
        let prefix_invoke = ops
            .iter()
            .position(|op| matches!(op, Op::INVOKE_STK1 | Op::INVOKE_STK4))
            .expect("complete prefix command is emitted");
        let tail_eval = ops
            .iter()
            .position(|op| *op == Op::EVAL_STK)
            .expect("malformed tail reaches runtime eval");
        assert!(prefix_invoke < tail_eval, "{ops:?}");
        assert!(
            ctx.literals
                .entries()
                .iter()
                .any(|literal| literal == "set x \""),
            "only the malformed command suffix reaches runtime eval: {:?}",
            ctx.literals.entries()
        );
        assert!(
            ctx.literals
                .entries()
                .iter()
                .all(|literal| literal != "incr side; set x \""),
            "the complete prefix must not be replayed by runtime eval: {:?}",
            ctx.literals.entries()
        );
    }

    #[test]
    fn nested_try_multicommand_phases_keep_live_ranges_and_boundaries() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(true, &[], &registry);
        ctx.dialect =
            Some(tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile());
        let normal = ctx.fresh_label("normal");

        ctx.emit_try_on_error_inline(
            "try",
            &[
                ("set x ok; error boom-$x".into(), true),
                ("on".into(), false),
                ("error".into(), false),
                ("message".into(), false),
                (
                    "set suffix handled; set message $message/$suffix".into(),
                    true,
                ),
            ],
            &normal,
        );
        ctx.place_label(&normal);

        let begin_ranges: Vec<_> = ctx
            .instructions
            .iter()
            .filter(|instruction| instruction.op == Op::BEGIN_CATCH4)
            .collect();
        assert_eq!(begin_ranges.len(), 2);
        assert!(
            begin_ranges
                .iter()
                .all(|instruction| instruction.catch_target.is_some()),
            "both try phases need live VM exception targets: {begin_ranges:?}"
        );
        assert!(
            ctx.literals.entries().iter().all(|literal| literal != ";"),
            "phase separators must not become arguments: {:?}",
            ctx.literals.entries()
        );
        assert!(
            ctx.instructions
                .iter()
                .filter(|instruction| instruction.op == Op::POP)
                .count()
                >= 2,
            "each phase must discard its non-final command result"
        );
    }

    #[test]
    fn nested_try_binds_both_typed_handler_variables() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(true, &[], &registry);
        let normal = ctx.fresh_label("normal");

        ctx.emit_try_on_error_inline(
            "try",
            &[
                ("error boom".into(), true),
                ("on".into(), false),
                ("error".into(), false),
                ("message options".into(), true),
                ("list $message [dict get $options -code]".into(), true),
            ],
            &normal,
        );

        let lvt = ctx.lvt.entries();
        assert!(lvt.iter().any(|name| name == "message"), "{lvt:?}");
        assert!(lvt.iter().any(|name| name == "options"), "{lvt:?}");
        assert!(
            ctx.instructions
                .iter()
                .filter(|instruction| instruction.op == Op::STORE_SCALAR1)
                .count()
                >= 4,
            "result/options temps and both handler bindings must be stored"
        );
    }

    #[test]
    fn nested_try_declines_unbraced_dynamic_phase_words() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(true, &[], &registry);

        ctx.emit_catch_inline(
            "try $body on error {message options} $handler",
            Some("result"),
            None,
        );

        let ops: Vec<Op> = ctx
            .instructions
            .iter()
            .map(|instruction| instruction.op)
            .collect();
        assert_eq!(
            ops.iter().filter(|&&op| op == Op::BEGIN_CATCH4).count(),
            1,
            "only the outer catch may specialise a dynamic try: {ops:?}"
        );
        assert!(
            ops.contains(&Op::INVOKE_STK1),
            "generic try invoke: {ops:?}"
        );
        assert!(
            !ctx.lvt.entries().iter().any(|name| name == "message"),
            "a declined handler must not allocate specialised bindings"
        );
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

    // -- registry drift: catch-body classifications --

    /// The registry's `NEEDS_START_CMD` set must equal the hardcoded
    /// list `emit_catch_body` used to match — a future stamping change
    /// is then a conscious decision, not a silent bytecode change.
    #[test]
    fn needs_start_cmd_trait_matches_previous_hardcoded_set() {
        let registry = CommandRegistry::build_default();
        let mut got = registry.commands_with_trait(Traits::NEEDS_START_CMD);
        got.sort_unstable();
        assert_eq!(got, ["break", "continue", "error", "expr", "return"]);
    }

    /// `break` in a catch body keeps its inline `break` opcode under a
    /// `startCommand` wrap (hook + trait both registry-resolved).
    #[test]
    fn catch_body_break_emits_break_op() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(true, &[], &registry);
        ctx.emit_catch_body("break");
        let ops: Vec<Op> = ctx.instructions.iter().map(|i| i.op).collect();
        assert!(ops.contains(&Op::BREAK), "expected BREAK, got {ops:?}");
        assert!(
            ops.contains(&Op::START_CMD),
            "break needs a startCommand wrap, got {ops:?}"
        );
    }

    /// `::error` resolves in the registry (leading `::` falls back to
    /// the bare spec), but the retired dispatch keyed on the raw word —
    /// the qualified spelling must keep the generic invoke and no
    /// startCommand wrap.
    #[test]
    fn catch_body_qualified_error_stays_generic() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(true, &[], &registry);
        ctx.emit_catch_body("::error boom");
        let ops: Vec<Op> = ctx.instructions.iter().map(|i| i.op).collect();
        assert!(!ops.contains(&Op::RETURN_IMM), "no inline throw: {ops:?}");
        assert!(!ops.contains(&Op::START_CMD), "no startCommand: {ops:?}");
        assert!(ops.contains(&Op::INVOKE_STK1), "generic invoke: {ops:?}");
    }

    /// `throw` carries `CATCHABLE_THROW` like `error`, but its first
    /// argument is the error-code *type*, not the message — it has no
    /// `Error` inline hook, so a catch body must keep the generic
    /// invoke for it (guarding the throw ≠ error distinction).
    #[test]
    fn catch_body_throw_stays_generic() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(true, &[], &registry);
        ctx.emit_catch_body("throw {A B} boom");
        let ops: Vec<Op> = ctx.instructions.iter().map(|i| i.op).collect();
        assert!(!ops.contains(&Op::RETURN_IMM), "no inline throw: {ops:?}");
        assert!(ops.contains(&Op::INVOKE_STK1), "generic invoke: {ops:?}");
    }
}
