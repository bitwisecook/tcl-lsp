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

//! Inline `dict for`, `dict map`, `dict update` and `dict with`.

use super::super::values::is_qualified;
use super::super::{CodegenCtx, Op, Operand, bytecode_imm};
use super::{InlineBodyEmitter, is_two_plain_names};
use tcl_runtime_api::completion_options::ControlOptionPolicy;

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
    /// Emit `dict map`'s catch cleanup and rethrow. Returns the placeholder
    /// `unsetScalar` indices for
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

    /// Close `dict map` after the key/value stack entries have been consumed,
    /// unset the iterator, load the accumulated dict as the result,
    /// and unset the result temp. Returns the placeholder indices (iterator
    /// unset, result load, result unset) for the caller to back-patch.
    fn emit_dict_map_normal_exit_after_values(
        &mut self,
        iter_name: &str,
        result_name: &str,
    ) -> (usize, usize, usize) {
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
    /// → `jump` (normal exit) → catch epilogue → `pop pop` and cleanup. Returns `false`
    /// unless this is a procedure context with exactly two local targets and
    /// the original literal body.
    ///
    /// `vars_text` is the `{k v}` word, `dict_text` the dictionary operand, and
    /// `body_text` the braced body. Catch, try, and dictionary commands share the
    /// original command-at-time body compiler. Both exits release the iterator;
    /// break and continue target the same protected loop activation.
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
        if !self.compiles_locals() {
            return false;
        }
        // The original braced operand supplies body coordinates and child
        // compiler receipts. A separately lowered string supplies neither.
        if self.invocation_tokens.as_deref().is_some_and(|tokens| {
            tokens.source_binding.is_some()
                && self.authored_inline_body_base(body_text, 3).is_none()
        }) {
            // `TclCompileDictForCmd` compiles any body, its loop variables
            // locals of the procedure all the same.
            self.intern_dict_iteration_locals(&vnames, body_text);
            return false;
        }

        // Slot allocation mirrors C Tcl's `TclCompileDictForCmd`: the two loop
        // variables, then a spare temp, then the body's locals, and finally the
        // iterator temp — so the iterator gets the *highest* slot. Because the
        // body (which interns its own locals) is emitted between `dictFirst` and
        // the iterator's allocation, the `dictFirst`/`dictNext`/`unsetScalar`
        // operands are emitted with a placeholder and back-patched once the
        // iterator slot is known.
        let (k_slot, v_slot) = self.dict_iteration_variable_slots(&vnames);
        // C Tcl allocates an unused companion temp right after the loop vars.
        let _spare = self
            .lvt
            .intern_synthetic(&format!("#dictfor_spare{}", self.catch_depth));
        let iter_name = format!("#dictfor{}", self.catch_depth);
        let loop_lbl = self.fresh_label("dict_for_loop");
        let end_lbl = self.fresh_label("dict_for_end");
        let body_start = self.fresh_label("dict_for_body");
        let body_end = self.fresh_label("dict_for_body_end");
        let step_label = self.fresh_label("dict_for_step");
        let break_label = self.fresh_label("dict_for_break");

        // Load the dict value, then begin the iterator under a catch range.
        self.emit_value(dict_text, true);
        let begin_idx = self.emit(
            Op::BEGIN_CATCH4,
            vec![Operand::Imm(
                i32::try_from(self.catch_depth).expect("catch_depth fits in i32"),
            )],
        );
        self.mark_completion_option_scope(begin_idx, ControlOptionPolicy::FRESH_SETTLED);
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
        let iteration_idx =
            self.emit_comment(Op::STORE_SCALAR1, vec![Operand::Imm(k_slot)], &vnames[0]);
        self.mark_completion_option_scope(iteration_idx, ControlOptionPolicy::FRESH_SETTLED);
        self.emit(Op::POP, vec![]);
        self.emit_comment(Op::STORE_SCALAR1, vec![Operand::Imm(v_slot)], &vnames[1]);
        self.emit(Op::POP, vec![]);
        self.place_label(&body_start);
        self.with_inline_body_operand(body_text, 3, |ctx| {
            ctx.emit_segmented_inline_body(body_text, InlineBodyEmitter::Catch);
        });
        self.emit(Op::POP, vec![]);
        self.place_label(&body_end);
        self.place_label(&step_label);
        let dict_next_idx = self.emit(Op::DICT_NEXT, vec![Operand::Imm(0)]);
        self.emit_comment(Op::JUMP_FALSE4, vec![Operand::Label(loop_lbl)], "dict_for");
        self.emit_comment(Op::JUMP4, vec![Operand::Label(end_lbl.clone())], "");

        let handler = self.fresh_label("dict_for_cleanup");
        self.instructions[begin_idx].catch_target = Some(handler.clone());
        self.place_label(&handler);
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
        let iter_slot = bytecode_imm(self.lvt.intern_synthetic(&iter_name));
        self.instructions[dict_first_idx].operands = vec![Operand::Imm(iter_slot)];
        self.instructions[dict_next_idx].operands = vec![Operand::Imm(iter_slot)];
        self.instructions[unset_idx].operands = vec![Operand::Imm(0), Operand::Imm(iter_slot)];

        // Normal exit: drop the leftover key/value from the final dictNext.
        self.place_label(&end_lbl);
        let settle_idx = self.emit(Op::POP, vec![]);
        self.mark_completion_option_scope(settle_idx, ControlOptionPolicy::FRESH_SETTLED);
        self.emit(Op::POP, vec![]);
        self.place_label(&break_label);
        self.emit(Op::END_CATCH, vec![]);
        self.emit_comment(
            Op::UNSET_SCALAR,
            vec![Operand::Imm(0), Operand::Imm(iter_slot)],
            &iter_name,
        );
        // `dict for` yields the empty string.
        self.push_lit("");
        self.inline_loop_regions
            .push(super::super::InlineLoopRegion {
                start: body_start,
                end: body_end,
                continue_target: Some(step_label),
                break_target: break_label,
            });
        self.seen_generic_invoke = true;
        true
    }

    fn intern_dict_iteration_locals(&mut self, names: &[String; 2], body: &str) {
        for name in names {
            self.intern_compiled_local(name);
        }
        self.intern_script_locals(body);
    }

    fn dict_iteration_variable_slots(&mut self, names: &[String; 2]) -> (i32, i32) {
        let key = bytecode_imm(
            self.command_variable_slot(names[0].as_bytes())
                .expect("selected native local target"),
        );
        let value = bytecode_imm(
            self.command_variable_slot(names[1].as_bytes())
                .expect("selected native local target"),
        );
        (key, value)
    }

    fn dict_map_variable_names(&self, vars_text: &str, body_text: &str) -> Option<[String; 2]> {
        let names = is_two_plain_names(vars_text)?;
        if !self.compiles_locals()
            || self.invocation_tokens.as_deref().is_some_and(|tokens| {
                tokens.source_binding.is_some()
                    && self.authored_inline_body_base(body_text, 3).is_none()
            })
        {
            return None;
        }
        Some(names)
    }

    /// Emit a compiled `dict map {k v} DICT { body }` inline, matching C Tcl:
    /// like `dict for` but accumulating each iteration's body result into a new
    /// dictionary (`dictSet result[k] = body-result`) which becomes the value.
    /// Requires a procedure-local target pair and the original literal body;
    /// body commands share the catch/try command-at-time compiler.
    pub fn emit_dict_map(&mut self, vars_text: &str, dict_text: &str, body_text: &str) -> bool {
        let Some(vnames) = self.dict_map_variable_names(vars_text, body_text) else {
            // `TclCompileDictMapCmd` compiles any body, its loop variables
            // locals of the procedure all the same.
            if self.compiles_locals()
                && let Some(names) = is_two_plain_names(vars_text)
            {
                self.intern_dict_iteration_locals(&names, body_text);
            }
            return false;
        };

        // Slots: the result accumulator and iterator temps are allocated after
        // the loop vars and the body's locals (interned during body emission),
        // so both are back-patched once known.
        let (k_slot, v_slot) = self.dict_iteration_variable_slots(&vnames);
        let result_name = format!("#dictmap_res{}", self.catch_depth);
        let iter_name = format!("#dictmap_it{}", self.catch_depth);
        let loop_lbl = self.fresh_label("dict_map_loop");
        let end_lbl = self.fresh_label("dict_map_end");
        let body_start = self.fresh_label("dict_map_body");
        let body_end = self.fresh_label("dict_map_body_end");
        let step_label = self.fresh_label("dict_map_step");
        let break_label = self.fresh_label("dict_map_break");

        // Initialise the accumulator to the empty dict.
        self.push_lit("");
        let res_store_idx =
            self.emit_comment(Op::STORE_SCALAR1, vec![Operand::Imm(0)], &result_name);
        self.emit(Op::POP, vec![]);

        self.emit_value(dict_text, true);
        let begin_idx = self.emit(
            Op::BEGIN_CATCH4,
            vec![Operand::Imm(
                i32::try_from(self.catch_depth).expect("catch_depth fits in i32"),
            )],
        );
        self.mark_completion_option_scope(begin_idx, ControlOptionPolicy::FRESH_FORWARDED);
        self.catch_depth += 1;
        let dict_first_idx = self.emit(Op::DICT_FIRST, vec![Operand::Imm(0)]);
        self.emit_comment(
            Op::JUMP_TRUE4,
            vec![Operand::Label(end_lbl.clone())],
            "dict_for",
        );

        self.place_label(&loop_lbl);
        let iteration_idx =
            self.emit_comment(Op::STORE_SCALAR1, vec![Operand::Imm(k_slot)], &vnames[0]);
        self.mark_completion_option_scope(iteration_idx, ControlOptionPolicy::FRESH_FORWARDED);
        self.emit(Op::POP, vec![]);
        self.emit_comment(Op::STORE_SCALAR1, vec![Operand::Imm(v_slot)], &vnames[1]);
        self.emit(Op::POP, vec![]);
        self.place_label(&body_start);
        self.with_inline_body_operand(body_text, 3, |ctx| {
            ctx.emit_segmented_inline_body(body_text, InlineBodyEmitter::Catch);
        });
        self.place_label(&body_end);
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

        self.place_label(&step_label);
        let dict_next_idx = self.emit(Op::DICT_NEXT, vec![Operand::Imm(0)]);
        self.emit_comment(Op::JUMP_FALSE4, vec![Operand::Label(loop_lbl)], "dict_for");
        self.emit_comment(Op::JUMP4, vec![Operand::Label(end_lbl.clone())], "");

        let handler = self.fresh_label("dict_map_cleanup");
        self.instructions[begin_idx].catch_target = Some(handler.clone());
        self.place_label(&handler);
        let (unset_it_err_idx, unset_res_err_idx) =
            self.emit_dict_map_err_epilogue(&iter_name, &result_name);
        self.catch_depth -= 1;

        // Normal exit: drop the leftover key/value, close the catch, unset the
        // iterator, load the accumulated dict as the result, unset the temp.
        self.place_label(&end_lbl);
        self.emit(Op::POP, vec![]);
        self.emit(Op::POP, vec![]);
        self.place_label(&break_label);
        let (unset_it_exit_idx, res_load_idx, unset_res_exit_idx) =
            self.emit_dict_map_normal_exit_after_values(&iter_name, &result_name);

        // Now intern the result + iterator temps (highest slots) and back-patch.
        let res_slot = bytecode_imm(self.lvt.intern_synthetic(&result_name));
        let iter_slot = bytecode_imm(self.lvt.intern_synthetic(&iter_name));
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
        self.inline_loop_regions
            .push(super::super::InlineLoopRegion {
                start: body_start,
                end: body_end,
                continue_target: Some(step_label),
                break_target: break_label,
            });

        self.seen_generic_invoke = true;
        true
    }

    /// Emit a compiled `dict update DICTVAR key1 var1 ?key2 var2 …? { body }`
    /// Bind keyed values to actual local slots, compile the original body, and
    /// write the locals back on normal and exceptional exits. `rest` is
    /// `[dictvar, k1, v1, …, body]`; every target must be a procedure-local scalar.
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
        let body_operand = rest.len();
        if self.invocation_tokens.as_deref().is_some_and(|tokens| {
            tokens.source_binding.is_some()
                && self
                    .authored_inline_body_base(body_text, body_operand)
                    .is_none()
        }) {
            // `TclCompileDictUpdateCmd` compiles any body with the command,
            // and its variables are locals of the procedure all the same.
            self.intern_compiled_local(dict_var);
            for v in &vars {
                self.intern_compiled_local(v);
            }
            self.intern_script_locals(body_text);
            return false;
        }

        let dict_slot = bytecode_imm(
            self.command_variable_slot(dict_var.as_bytes())
                .expect("selected native dictionary local"),
        );
        let variable_slots = vars
            .iter()
            .map(|name| {
                self.command_variable_slot(name.as_bytes())
                    .expect("selected native dictionary target")
            })
            .collect::<Vec<_>>();
        let end_lbl = self.fresh_label("dict_update_end");

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
        self.instructions[start_idx].dict_vars = Some(variable_slots.clone());

        let begin_idx = self.emit(
            Op::BEGIN_CATCH4,
            vec![Operand::Imm(
                i32::try_from(self.catch_depth).expect("catch_depth fits in i32"),
            )],
        );
        self.catch_depth += 1;

        self.with_inline_body_operand(body_text, body_operand, |ctx| {
            ctx.emit_segmented_inline_body(body_text, InlineBodyEmitter::Catch);
        });

        // Normal epilogue: swap the key list above the body result and write the
        // locals back into the dict.
        self.emit(Op::END_CATCH, vec![]);
        self.emit(Op::REVERSE, vec![Operand::Imm(2)]);
        let end_idx = self.emit_comment(
            Op::DICT_UPDATE_END,
            vec![Operand::Imm(dict_slot), Operand::Imm(0)],
            &format!("var \"{dict_var}\""),
        );
        self.instructions[end_idx].dict_vars = Some(variable_slots.clone());
        self.emit_comment(Op::JUMP4, vec![Operand::Label(end_lbl.clone())], "");

        // Every non-OK body completion still publishes the target locals before
        // propagating its original result and return options.
        let handler = self.fresh_label("dict_body_cleanup");
        self.instructions[begin_idx].catch_target = Some(handler.clone());
        self.place_label(&handler);
        self.emit(Op::PUSH_RESULT, vec![]);
        self.emit(Op::PUSH_RETURN_OPTS, vec![]);
        self.emit(Op::END_CATCH, vec![]);
        self.emit(Op::REVERSE, vec![Operand::Imm(3)]);
        let err_end_idx = self.emit_comment(
            Op::DICT_UPDATE_END,
            vec![Operand::Imm(dict_slot), Operand::Imm(0)],
            &format!("var \"{dict_var}\""),
        );
        self.instructions[err_end_idx].dict_vars = Some(variable_slots);
        self.emit(Op::RETURN_STK, vec![]);
        self.catch_depth -= 1;

        self.place_label(&end_lbl);
        self.seen_generic_invoke = true;
        true
    }

    /// Emit a compiled `dict with DICTVAR { body }` inline, matching C Tcl:
    /// Expand keys into same-named locals, compile the original body, and fold
    /// the locals back on normal and exceptional exits. The path form and
    /// nonlocal dictionary targets use the runtime command.
    pub fn emit_dict_with(&mut self, dict_var: &str, body_text: &str) -> bool {
        if !self.compiles_locals() || is_qualified(dict_var) || dict_var.contains('(') {
            return false;
        }
        if self.invocation_tokens.as_deref().is_some_and(|tokens| {
            tokens.source_binding.is_some()
                && self.authored_inline_body_base(body_text, 2).is_none()
        }) {
            // As `emit_dict_update`: `TclCompileDictWithCmd` compiles any body.
            self.intern_compiled_local(dict_var);
            self.intern_script_locals(body_text);
            return false;
        }

        let dict_slot = bytecode_imm(
            self.command_variable_slot(dict_var.as_bytes())
                .expect("selected native dictionary local"),
        );
        let state_name = format!("#dictwith_state{}", self.catch_depth);
        let state_slot = bytecode_imm(self.lvt.intern_synthetic(&state_name));
        let end_lbl = self.fresh_label("dict_with_end");

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

        let begin_idx = self.emit(
            Op::BEGIN_CATCH4,
            vec![Operand::Imm(
                i32::try_from(self.catch_depth).expect("catch_depth fits in i32"),
            )],
        );
        self.catch_depth += 1;

        self.with_inline_body_operand(body_text, 2, |ctx| {
            ctx.emit_segmented_inline_body(body_text, InlineBodyEmitter::Catch);
        });

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
        self.emit_comment(
            Op::UNSET_SCALAR,
            vec![Operand::Imm(0), Operand::Imm(state_slot)],
            &state_name,
        );
        self.emit_comment(Op::JUMP4, vec![Operand::Label(end_lbl.clone())], "");

        // Every non-OK body completion still publishes the target locals before
        // propagating its original result and return options.
        let handler = self.fresh_label("dict_body_cleanup");
        self.instructions[begin_idx].catch_target = Some(handler.clone());
        self.place_label(&handler);
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
        self.emit_comment(
            Op::UNSET_SCALAR,
            vec![Operand::Imm(0), Operand::Imm(state_slot)],
            &state_name,
        );
        self.emit(Op::RETURN_STK, vec![]);
        self.catch_depth -= 1;

        self.place_label(&end_lbl);
        self.seen_generic_invoke = true;
        true
    }
}
