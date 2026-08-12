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

use tcl_registry::hooks::InlineCodegenHookId;
use tcl_registry::{CommandRegistry, Traits};

use crate::cfg::Function as CfgFunction;
use crate::expr_ast::{BinOp, ExprNode};
use crate::ir::Statement;

use super::cmd_subst::parse_cmd_parts;
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
        let body_ir = crate::lowering::lower_to_ir(body_text, self.registry);
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
        let body_ir = crate::lowering::lower_to_ir(body_text, self.registry);
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
        if !self.is_proc || rest.len() < 4 || !rest.len().is_multiple_of(2) {
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
        let body_ir = crate::lowering::lower_to_ir(body_text, self.registry);
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
        if !self.is_proc || is_qualified(dict_var) || dict_var.contains('(') {
            return false;
        }
        let body_ir = crate::lowering::lower_to_ir(body_text, self.registry);
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

        // Compile body command with startCommand wrapping.
        self.emit_catch_body(body);

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
        match spec.and_then(|s| s.inline_codegen_hook) {
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
                let node = crate::expr_parser::parse_expr(expr_text, None);
                if let Some((msg, opts)) = detect_const_expr_error(&node) {
                    self.push_lit(&msg);
                    self.push_lit(&opts);
                    self.emit(Op::SYNTAX, vec![Operand::Imm(1), Operand::Imm(0)]);
                } else {
                    self.emit_expr(&node);
                }
            }
            Some(InlineCodegenHookId::Try)
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
            && self.registry.get(command).is_some_and(|s| {
                s.name == command.as_str()
                    && s.inline_codegen_hook == Some(InlineCodegenHookId::Error)
            })
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
