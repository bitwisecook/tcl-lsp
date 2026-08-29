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

//! Per-command codegen hook dispatch.
//!
//! Commands with compiled bytecode forms (beyond what
//! `cmd_subst::emit_inline_cmd_subst` already handles) are routed
//! through this dispatcher. Hook selection is registry-driven: it
//! reads the active [`tcl_registry::CommandRegistry`] from
//! `ctx.registry` so dialect-loaded specs (iRules, Tk, EDA) drive
//! codegen-hook resolution. The compiler still owns the per-variant
//! emitter; the registry decides which variant applies.

use tcl_registry::hooks::CodegenHookId;

use super::super::CodegenCtx;
use super::super::Op;
use super::super::Operand;
use super::super::values::{is_qualified, split_array_ref};
use super::super::{INDEX_END, bytecode_imm, parse_tcl_index};

/// Try to emit specialised bytecode for `cmd args...` via a per-
/// command hook. Returns `true` if the hook handled the command;
/// `false` if the caller should fall back to the generic invoke.
///
/// Hooks are responsible for leaving a single result value on top of
/// the stack followed by a `POP` (mirroring the generic invoke
/// emission), and for marking `*used_generic_invoke` when appropriate
/// so that downstream startCommand peephole passes behave.
pub fn try_bytecoded(
    ctx: &mut CodegenCtx,
    cmd: &str,
    args: &[String],
    used_generic_invoke: &mut bool,
) -> bool {
    // A name this unit renames, aliases, or shadows with a proc no longer
    // denotes the builtin the hook's emitter open-codes, so decline to
    // specialise and let the generic invoke dispatch on whatever the name
    // actually holds — the static stand-in for C Tcl's `INST_START_CMD`
    // compile-epoch re-dispatch (issue #1585).
    if !ctx.trusts_builtin(cmd) {
        return false;
    }
    let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
    // The registry's own availability mask (issues #1462/#1463): a
    // profile-built registry suppresses the specialised emission of a
    // command its release does not have, keeping it on the generic invoke
    // where the runtime's availability gate can reject it.
    let Some(resolved) =
        ctx.registry
            .resolve_call(cmd, &arg_refs, ctx.registry.own_surface_query())
    else {
        return false;
    };
    let Some(hook) = resolved.codegen_hook else {
        return false;
    };
    dispatch_codegen_hook(hook, ctx, args, used_generic_invoke)
}

/// Dispatch a typed [`CodegenHookId`] to its emitter.
///
/// Public so external pipelines (tests, future LSP feature
/// providers) can reuse the per-hook implementations without
/// duplicating the table.
pub fn dispatch_codegen_hook(
    hook: CodegenHookId,
    ctx: &mut CodegenCtx,
    args: &[String],
    used_generic_invoke: &mut bool,
) -> bool {
    match hook {
        CodegenHookId::Lassign => lassign(ctx, args),
        CodegenHookId::Llength => llength(ctx, args),
        CodegenHookId::Lrange => lrange(ctx, args),
        CodegenHookId::Linsert => linsert(ctx, args),
        CodegenHookId::Lset => lset(ctx, args),
        CodegenHookId::Dict => dict(ctx, args),
        CodegenHookId::Array => array(ctx, args, used_generic_invoke),
        CodegenHookId::Namespace => namespace_cmd(ctx, args, used_generic_invoke),
        CodegenHookId::Append => append_cmd(ctx, args),
        CodegenHookId::Lappend => lappend_cmd(ctx, args),
        CodegenHookId::Unset => unset_cmd(ctx, args),
        CodegenHookId::Tailcall => tailcall_cmd(ctx, args),
        CodegenHookId::Concat => concat_cmd(ctx, args),
        CodegenHookId::Global => global_cmd(ctx, args),
        CodegenHookId::Upvar => upvar_cmd(ctx, args),
    }
}

// list

/// `llength $list` → `emit_word list; LIST_LENGTH; POP`.
fn llength(ctx: &mut CodegenCtx, args: &[String]) -> bool {
    if args.len() != 1 {
        return false;
    }
    ctx.emit_word_arg(0, &args[0]);
    ctx.emit(Op::LIST_LENGTH, vec![]);
    ctx.emit(Op::POP, vec![]);
    true
}

/// `lassign list var1 var2 ...` → load the list, then for each var:
/// push varname; `OVER 1`; `LIST_INDEX_IMM i`; `STORE_STK`; `POP`.
/// Final: `LIST_RANGE_IMM <num_vars> end`; `POP`.
fn lassign(ctx: &mut CodegenCtx, args: &[String]) -> bool {
    if args.len() < 2 {
        return false;
    }
    ctx.emit_word_arg(0, &args[0]);
    let var_names = &args[1..];
    for (i, var) in var_names.iter().enumerate() {
        ctx.push_lit(var);
        ctx.emit(Op::OVER, vec![Operand::Imm(1)]);
        ctx.emit(Op::LIST_INDEX_IMM, vec![Operand::Imm(bytecode_imm(i))]);
        ctx.emit(Op::STORE_STK, vec![]);
        ctx.emit(Op::POP, vec![]);
    }
    ctx.emit(
        Op::LIST_RANGE_IMM,
        vec![
            Operand::Imm(bytecode_imm(var_names.len())),
            Operand::Imm(INDEX_END),
        ],
    );
    ctx.emit(Op::POP, vec![]);
    true
}

/// `lrange list first last` — emits `LIST_RANGE_IMM` when both
/// indices are compile-time constants (integers or `end[-N]`).
/// Mixed or non-constant indices fall back to the generic invoke.
fn lrange(ctx: &mut CodegenCtx, args: &[String]) -> bool {
    if args.len() != 3 {
        return false;
    }
    let Some(start_idx) = parse_tcl_index(&args[1]) else {
        return false;
    };
    let Some(end_idx) = parse_tcl_index(&args[2]) else {
        return false;
    };
    ctx.emit_word_arg(0, &args[0]);
    ctx.emit(
        Op::LIST_RANGE_IMM,
        vec![Operand::Imm(start_idx), Operand::Imm(end_idx)],
    );
    ctx.emit(Op::POP, vec![]);
    true
}

/// `linsert list index element ...` — emits `LREPLACE4 N 2` where
/// the final `2` operand distinguishes insert from replace in the
/// shared lreplace opcode family.
fn linsert(ctx: &mut CodegenCtx, args: &[String]) -> bool {
    if args.len() < 2 {
        return false;
    }
    for (i, a) in args.iter().enumerate() {
        ctx.emit_word_arg(i, a);
    }
    ctx.emit(
        Op::LREPLACE4,
        vec![Operand::Imm(bytecode_imm(args.len())), Operand::Imm(2)],
    );
    ctx.emit(Op::POP, vec![]);
    true
}

/// `lset varname ?index ...? newvalue` — proc-context with a
/// simple (non-qualified) variable name compiles to
/// `loadScalar1 SLOT; LSET_LIST | LSET_FLAT; storeScalar1 SLOT`;
/// everything else uses stack-based `loadStk` / `storeStk`.
fn lset(ctx: &mut CodegenCtx, args: &[String]) -> bool {
    if args.len() < 3 {
        return false;
    }
    let var_name = &args[0];
    let indices = &args[1..args.len() - 1];
    let value = args.last().expect("args.len() >= 3");

    if ctx.is_proc && !is_qualified(var_name) && !indices.is_empty() {
        let slot = ctx.lvt.intern(var_name);
        for idx in indices {
            ctx.emit_value_interpolated(idx);
        }
        ctx.emit_value_interpolated(value);
        let load_op = if slot < 256 {
            Op::LOAD_SCALAR1
        } else {
            Op::LOAD_SCALAR4
        };
        ctx.emit_comment(
            load_op,
            vec![Operand::Imm(i32::try_from(slot).unwrap_or(i32::MAX))],
            &format!("var \"{var_name}\""),
        );
        if indices.len() >= 2 {
            ctx.emit(
                Op::LSET_FLAT,
                vec![Operand::Imm(bytecode_imm(indices.len() + 2))],
            );
        } else {
            ctx.emit(Op::LSET_LIST, vec![]);
        }
        let op = if slot < 256 {
            Op::STORE_SCALAR1
        } else {
            Op::STORE_SCALAR4
        };
        ctx.emit_comment(
            op,
            vec![Operand::Imm(i32::try_from(slot).unwrap_or(i32::MAX))],
            &format!("var \"{var_name}\""),
        );
    } else {
        // Non-proc or qualified: stack-based form with OVER to duplicate the
        // variable reference onto the top of stack so loadStk finds it. A single
        // index uses `LSET_LIST` (the index arg is itself an index *path*, so
        // `lset l {1 0} v` works); two or more indices are flat, so they use
        // `LSET_FLAT N` like the proc path — otherwise only the last index would
        // reach `LSET_LIST`.
        ctx.push_lit(var_name);
        for idx in indices {
            ctx.emit_value_interpolated(idx);
        }
        ctx.emit_value_interpolated(value);
        ctx.emit(
            Op::OVER,
            vec![Operand::Imm(bytecode_imm(indices.len() + 1))],
        );
        ctx.emit(Op::LOAD_STK, vec![]);
        if indices.len() >= 2 {
            ctx.emit(
                Op::LSET_FLAT,
                vec![Operand::Imm(bytecode_imm(indices.len() + 2))],
            );
        } else {
            ctx.emit(Op::LSET_LIST, vec![]);
        }
        ctx.emit(Op::STORE_STK, vec![]);
    }
    ctx.emit(Op::POP, vec![]);
    true
}

// dict

/// `dict SUBCOMMAND …` — dispatch to a handful of specialised
/// opcodes when the target variable is a proc-local (non-qualified)
/// scalar. Falls back to the generic invoke otherwise.
///
/// Covered subcommands:
/// - `dict set var k1 ?k2 …? value` — `DICT_SET N slot`.
/// - `dict unset var k1 ?k2 …?` — `DICT_UNSET N slot`.
/// - `dict incr var key ?amount?` — `DICT_INCR_IMM amt slot`.
/// - `dict append var key value` — `DICT_APPEND slot`.
/// - `dict lappend var key value` — `DICT_LAPPEND slot`.
fn dict(ctx: &mut CodegenCtx, args: &[String]) -> bool {
    if args.len() < 2 {
        return false;
    }
    let sub = args[0].as_str();
    let rest = &args[1..];

    // Non-proc (or qualified-var) mutating `dict` subcommand → the top-level
    // ensemble-rewrite `INVOKE_REPLACE` form (see [`dict_ensemble`]); the
    // proc-local scalar path below keeps its specialised `DICT_*` opcodes.
    let proc_local = ctx.is_proc && !is_qualified(&rest[0]);
    if !proc_local && matches!(sub, "set" | "unset" | "incr" | "append" | "lappend") {
        dict_ensemble(ctx, sub, rest);
        return true;
    }

    if !ctx.is_proc || args.len() < 3 {
        return false;
    }
    let var_name = &rest[0];
    if is_qualified(var_name) {
        return false;
    }

    match sub {
        "set" if rest.len() >= 3 => dict_set(ctx, var_name, rest),
        "unset" if rest.len() >= 2 => dict_unset(ctx, var_name, rest),
        "incr" if matches!(rest.len(), 2 | 3) => dict_incr(ctx, var_name, rest),
        "append" if rest.len() == 3 => dict_append(ctx, var_name, rest),
        "lappend" if rest.len() == 3 => dict_lappend(ctx, var_name, rest),
        // `dict update var k1 v1 ?k2 v2 …? body` and `dict with var body`
        // compile inline (C Tcl's `dictUpdateStart`/`dictExpand` machinery) when
        // the body is straight-line; otherwise fall through to the runtime
        // invoke. The path form of `dict with` (`rest.len() > 2`) is not inlined.
        "update" if rest.len() >= 4 && rest.len().is_multiple_of(2) => ctx.emit_dict_update(rest),
        "with" if rest.len() == 2 => ctx.emit_dict_with(&rest[0], &rest[1]),
        _ => false,
    }
}

/// `dict set var k1 ?k2 …? value` → `DICT_SET N slot`.
fn dict_set(ctx: &mut CodegenCtx, var_name: &str, rest: &[String]) -> bool {
    let keys = &rest[1..rest.len() - 1];
    let value = rest.last().unwrap();
    let slot = ctx.lvt.intern(var_name);
    for k in keys {
        ctx.emit_value_interpolated(k);
    }
    ctx.emit_value_interpolated(value);
    ctx.emit_comment(
        Op::DICT_SET,
        vec![
            Operand::Imm(bytecode_imm(keys.len())),
            Operand::Imm(bytecode_imm(slot)),
        ],
        &format!("var \"{var_name}\""),
    );
    ctx.emit(Op::POP, vec![]);
    true
}

/// `dict unset var k1 ?k2 …?` → `DICT_UNSET N slot`.
fn dict_unset(ctx: &mut CodegenCtx, var_name: &str, rest: &[String]) -> bool {
    let keys = &rest[1..];
    let slot = ctx.lvt.intern(var_name);
    for k in keys {
        ctx.emit_value_interpolated(k);
    }
    ctx.emit_comment(
        Op::DICT_UNSET,
        vec![
            Operand::Imm(bytecode_imm(keys.len())),
            Operand::Imm(bytecode_imm(slot)),
        ],
        &format!("var \"{var_name}\""),
    );
    ctx.emit(Op::POP, vec![]);
    true
}

/// `dict incr var key ?amount?` → `DICT_INCR_IMM amt slot`.
fn dict_incr(ctx: &mut CodegenCtx, var_name: &str, rest: &[String]) -> bool {
    let key = &rest[1];
    let amount: i32 = if rest.len() == 3 {
        match rest[2].parse::<i32>() {
            Ok(v) => v,
            Err(_) => return false,
        }
    } else {
        1
    };
    let slot = ctx.lvt.intern(var_name);
    ctx.emit_value_interpolated(key);
    ctx.emit_comment(
        Op::DICT_INCR_IMM,
        vec![Operand::Imm(amount), Operand::Imm(bytecode_imm(slot))],
        &format!("var \"{var_name}\""),
    );
    ctx.emit(Op::POP, vec![]);
    true
}

/// `dict append var key value` → `DICT_APPEND slot`.
fn dict_append(ctx: &mut CodegenCtx, var_name: &str, rest: &[String]) -> bool {
    let key = &rest[1];
    let value = &rest[2];
    let slot = ctx.lvt.intern(var_name);
    ctx.emit_value_interpolated(key);
    ctx.emit_value_interpolated(value);
    ctx.emit_comment(
        Op::DICT_APPEND,
        vec![Operand::Imm(bytecode_imm(slot))],
        &format!("var \"{var_name}\""),
    );
    ctx.emit(Op::POP, vec![]);
    true
}

/// `dict lappend var key value` → `DICT_LAPPEND slot`.
fn dict_lappend(ctx: &mut CodegenCtx, var_name: &str, rest: &[String]) -> bool {
    let key = &rest[1];
    let value = &rest[2];
    let slot = ctx.lvt.intern(var_name);
    ctx.emit_value_interpolated(key);
    ctx.emit_value_interpolated(value);
    ctx.emit_comment(
        Op::DICT_LAPPEND,
        vec![Operand::Imm(bytecode_imm(slot))],
        &format!("var \"{var_name}\""),
    );
    ctx.emit(Op::POP, vec![]);
    true
}

/// Emit a top-level mutating `dict <sub> …` as the ensemble-rewrite
/// `INVOKE_REPLACE` form tclsh uses (`push dict <sub> <args…> ::tcl::dict::<sub>;
/// invokeReplace objc 2`). `sub_args` is the words after the subcommand (e.g.
/// the var name, keys, value); `sub` must have a registered
/// `::tcl::dict::<sub>` implementation.
fn dict_ensemble(ctx: &mut CodegenCtx, sub: &str, sub_args: &[String]) {
    ctx.push_lit("dict");
    ctx.push_lit(sub);
    for a in sub_args {
        ctx.emit_value_interpolated(a);
    }
    ctx.push_lit(&format!("::tcl::dict::{sub}"));
    let objc = bytecode_imm(2 + sub_args.len());
    ctx.emit(
        Op::INVOKE_REPLACE,
        vec![Operand::Imm(objc), Operand::Imm(2)],
    );
    ctx.emit(Op::POP, vec![]);
    ctx.seen_generic_invoke = true;
}

// namespace

/// `namespace eval ns body` — C Tcl compiles the ensemble to the
/// `invokeReplace` rewrite of `::tcl::namespace::eval`: it pushes the original
/// words (`namespace eval ns body`) followed by the resolved implementation
/// name, then `invokeReplace objc 2` replaces the two-word `namespace eval`
/// prefix with `::tcl::namespace::eval` at runtime. The body is pushed as an
/// unparsed literal (it is not compiled). Mirrors [`dict_ensemble`]. Other
/// `namespace` subcommands fall through to the generic invoke.
fn namespace_cmd(ctx: &mut CodegenCtx, args: &[String], used_generic_invoke: &mut bool) -> bool {
    // `namespace eval ns body ...` → args = [eval, ns, body, ...]. C Tcl uses
    // the ensemble-rewrite form for the `eval` subcommand with a namespace and
    // at least one body word, whether the body is a braced literal, a dynamic
    // `$body`, or several words concatenated into the script.
    if args.first().map(String::as_str) != Some("eval") || args.len() < 3 {
        return false;
    }
    ctx.push_lit("namespace");
    ctx.push_lit("eval");
    // Push the namespace and every body word exactly as the generic path would:
    // `emit_word_arg` reads `cmd_arg_braced[idx]` (arg 0 is `eval`), so args[i]
    // is original index `i` — a braced body word is pushed verbatim, a dynamic
    // `$body` is interpolated.
    for (i, a) in args.iter().enumerate().skip(1) {
        ctx.emit_word_arg(i, a);
    }
    ctx.push_lit("::tcl::namespace::eval");
    // objc = all original words (`namespace eval ns body ...`); replace the
    // two-word `namespace eval` prefix with the resolved implementation.
    let objc = bytecode_imm(1 + args.len());
    ctx.emit(
        Op::INVOKE_REPLACE,
        vec![Operand::Imm(objc), Operand::Imm(2)],
    );
    ctx.emit(Op::POP, vec![]);
    ctx.seen_generic_invoke = true;
    *used_generic_invoke = true;
    true
}

// array

/// `array names $arr ...` / `array size $arr` in non-proc context:
/// invoke the fully-qualified `::tcl::array::<sub>` form rather than
/// the generic `array` dispatcher.
fn array(ctx: &mut CodegenCtx, args: &[String], used_generic_invoke: &mut bool) -> bool {
    if args.len() < 2 {
        return false;
    }
    let sub = args[0].as_str();
    let rest = &args[1..];

    // `array for {k v} arr body` (Tcl 9.0): C Tcl compiles the ensemble to a
    // direct `invokeStk` of the resolved `::tcl::array::for` with the body
    // pushed as an unparsed literal — in *any* context (proc or top-level).
    // Match it: push the qualified command, then the three args exactly as the
    // generic per-word path would (so the braced `{k v}` var-list and the
    // braced body are pushed verbatim), then `invokeStk1 4`. Analysis still
    // sees the `array` barrier (registry-aware), so this is codegen-only.
    if sub == "for" && rest.len() == 3 {
        ctx.push_lit("::tcl::array::for");
        for (i, a) in rest.iter().enumerate() {
            // `emit_word_arg` reads `cmd_arg_braced[idx]`, indexed by the
            // original arg list (arg 0 is the `for` subcommand word), so
            // `rest[i]` is original index `i + 1` — pushing the braced `{k v}`
            // and braced body verbatim, exactly as the generic path would.
            ctx.emit_word_arg(i + 1, a);
        }
        let n_args = bytecode_imm(1 + rest.len());
        let op = if n_args < 256 {
            Op::INVOKE_STK1
        } else {
            Op::INVOKE_STK4
        };
        ctx.emit(op, vec![Operand::Imm(n_args)]);
        ctx.emit(Op::POP, vec![]);
        ctx.seen_generic_invoke = true;
        *used_generic_invoke = true;
        return true;
    }

    if ctx.is_proc {
        return false;
    }
    match sub {
        "names" | "size" if !rest.is_empty() => {
            ctx.push_lit(&format!("::tcl::array::{sub}"));
            for a in rest {
                ctx.emit_value_interpolated(a);
            }
            let n_args = bytecode_imm(1 + rest.len());
            let op = if n_args < 256 {
                Op::INVOKE_STK1
            } else {
                Op::INVOKE_STK4
            };
            ctx.emit(op, vec![Operand::Imm(n_args)]);
            ctx.emit(Op::POP, vec![]);
            ctx.seen_generic_invoke = true;
            *used_generic_invoke = true;
            true
        }
        _ => false,
    }
}

// append / lappend

/// True when `var` names a variable codegen can resolve to a compiled
/// local — a proc context, a plain (non-`::`-qualified) name, and not a
/// dynamic `$…` / `[…]` reference. The shared gate for the statement-
/// position `append`/`lappend` specialisations.
fn is_compilable_local(ctx: &CodegenCtx, var: &str) -> bool {
    ctx.is_proc && !is_qualified(var) && !var.starts_with('$') && !var.starts_with('[')
}

/// `append varName value ...` — statement-position specialisation for a
/// proc-local variable. A single value emits `appendScalar`/`appendArray`;
/// multiple scalar values push all, `reverse N`, then `appendScalar; pop`
/// per value (mirroring C Tcl's `TclCompileAppendCmd`). Toplevel, qualified,
/// dynamic, and multi-value array forms fall back to the generic invoke.
fn append_cmd(ctx: &mut CodegenCtx, args: &[String]) -> bool {
    if args.len() < 2 || !is_compilable_local(ctx, &args[0]) {
        return false;
    }
    let var = &args[0];
    let values = &args[1..];

    if let Some((base, key)) = split_array_ref(var) {
        if values.len() != 1 {
            return false;
        }
        let slot = ctx.lvt.intern(base);
        let op = if slot < 256 {
            Op::APPEND_ARRAY1
        } else {
            Op::APPEND_ARRAY4
        };
        ctx.push_array_key(key);
        ctx.emit_value_interpolated(&values[0]);
        ctx.emit_comment(
            op,
            vec![Operand::Imm(bytecode_imm(slot))],
            &format!("var \"{base}\""),
        );
        ctx.emit(Op::POP, vec![]);
        return true;
    }

    let slot = ctx.lvt.intern(var);
    let op = if slot < 256 {
        Op::APPEND_SCALAR1
    } else {
        Op::APPEND_SCALAR4
    };
    if values.len() == 1 {
        ctx.emit_value_interpolated(&values[0]);
        ctx.emit_comment(
            op,
            vec![Operand::Imm(bytecode_imm(slot))],
            &format!("var \"{var}\""),
        );
        ctx.emit(Op::POP, vec![]);
    } else {
        for v in values {
            ctx.emit_value_interpolated(v);
        }
        ctx.emit(Op::REVERSE, vec![Operand::Imm(bytecode_imm(values.len()))]);
        for _ in values {
            ctx.emit_comment(
                op,
                vec![Operand::Imm(bytecode_imm(slot))],
                &format!("var \"{var}\""),
            );
            ctx.emit(Op::POP, vec![]);
        }
    }
    true
}

/// `lappend varName value ...` — statement-position specialisation for a
/// proc-local variable. A single value emits `lappendScalar`/`lappendArray`;
/// multiple scalar values build a list (`list N`) and emit `lappendList`
/// (mirroring C Tcl's `TclCompileLappendCmd`). Toplevel, qualified, dynamic,
/// and multi-value array forms fall back to the generic invoke.
fn lappend_cmd(ctx: &mut CodegenCtx, args: &[String]) -> bool {
    if args.len() < 2 || !is_compilable_local(ctx, &args[0]) {
        return false;
    }
    let var = &args[0];
    let values = &args[1..];

    if let Some((base, key)) = split_array_ref(var) {
        if values.len() != 1 {
            return false;
        }
        let slot = ctx.lvt.intern(base);
        let op = if slot < 256 {
            Op::LAPPEND_ARRAY1
        } else {
            Op::LAPPEND_ARRAY4
        };
        ctx.push_array_key(key);
        ctx.emit_value_interpolated(&values[0]);
        ctx.emit_comment(
            op,
            vec![Operand::Imm(bytecode_imm(slot))],
            &format!("var \"{base}\""),
        );
        ctx.emit(Op::POP, vec![]);
        return true;
    }

    let slot = ctx.lvt.intern(var);
    if values.len() == 1 {
        ctx.emit_value_interpolated(&values[0]);
        let op = if slot < 256 {
            Op::LAPPEND_SCALAR1
        } else {
            Op::LAPPEND_SCALAR4
        };
        ctx.emit_comment(
            op,
            vec![Operand::Imm(bytecode_imm(slot))],
            &format!("var \"{var}\""),
        );
    } else {
        for v in values {
            ctx.emit_value_interpolated(v);
        }
        ctx.emit(Op::LIST, vec![Operand::Imm(bytecode_imm(values.len()))]);
        ctx.emit_comment(
            Op::LAPPEND_LIST,
            vec![Operand::Imm(bytecode_imm(slot))],
            &format!("var \"{var}\""),
        );
    }
    ctx.emit(Op::POP, vec![]);
    true
}

// unset

/// `unset ?-nocomplain? ?--? name ...` — statement-position specialisation
/// in a proc body. Each variable unsets via `unsetScalar`/`unsetArray`
/// (compiled locals) or `unsetStk` (qualified / dynamic names); the command's
/// empty-string result is then `push ""; pop` (folded to `nop`s mid-body, or
/// the bare `push ""` return in tail position). Mirrors C Tcl's
/// `TclCompileUnsetCmd`. Toplevel falls back to the generic invoke.
fn unset_cmd(ctx: &mut CodegenCtx, args: &[String]) -> bool {
    if !ctx.is_proc {
        return false;
    }
    // Leading options: `-nocomplain` clears the complain flag; `--` ends
    // option parsing. The flag operand is 1 (complain) unless suppressed.
    let mut flags: i32 = 1;
    let mut i = 0;
    while i < args.len() && args[i].starts_with('-') {
        if args[i] == "--" {
            i += 1;
            break;
        }
        if args[i] == "-nocomplain" {
            flags = 0;
        }
        i += 1;
    }
    let names = &args[i..];
    if names.is_empty() {
        return false;
    }
    for name in names {
        if !is_qualified(name) && !name.starts_with('$') && !name.starts_with('[') {
            if let Some((base, key)) = split_array_ref(name) {
                let slot = ctx.lvt.intern(base);
                ctx.push_array_key(key);
                ctx.emit_comment(
                    Op::UNSET_ARRAY,
                    vec![Operand::Imm(flags), Operand::Imm(bytecode_imm(slot))],
                    &format!("var \"{base}\""),
                );
            } else {
                let slot = ctx.lvt.intern(name);
                ctx.emit_comment(
                    Op::UNSET_SCALAR,
                    vec![Operand::Imm(flags), Operand::Imm(bytecode_imm(slot))],
                    &format!("var \"{name}\""),
                );
            }
        } else {
            // Qualified or dynamic name: push it and use the stack form.
            ctx.emit_value_interpolated(name);
            ctx.emit(Op::UNSET_STK, vec![Operand::Imm(flags)]);
        }
    }
    ctx.push_lit("");
    ctx.emit(Op::POP, vec![]);
    true
}

// tailcall

/// `tailcall command ?arg ...?` — push the literal `"tailcall"` word, then
/// each argument, and emit `tailcall N` where N counts the `"tailcall"`
/// prefix plus the arguments (mirroring C Tcl's `TclCompileTailcallCmd`).
/// The result is popped at statement level (dropped into `done` in tail
/// position). `tailcall` with no arguments falls back to the generic invoke.
fn tailcall_cmd(ctx: &mut CodegenCtx, args: &[String]) -> bool {
    if args.is_empty() {
        return false;
    }
    ctx.push_lit("tailcall");
    for a in args {
        ctx.emit_value_interpolated(a);
    }
    ctx.emit(
        Op::TAILCALL,
        vec![Operand::Imm(bytecode_imm(1 + args.len()))],
    );
    ctx.emit(Op::POP, vec![]);
    true
}

// concat

/// `concat ?arg ...?` — const-fold when every argument is a plain literal
/// (trim each, drop the empties, join with a single space — mirroring
/// `tcl_registry::const_fold::fold_concat` and C Tcl's literal `concat`
/// folding); otherwise push each word and emit `concatStk N`. No arguments
/// yields the empty result. Arguments carrying a substitution (`$`/`[`) or a
/// backslash escape are not folded — they take the `concatStk` path, which
/// substitutes correctly.
fn concat_cmd(ctx: &mut CodegenCtx, args: &[String]) -> bool {
    if args.is_empty() {
        ctx.push_lit("");
        ctx.emit(Op::POP, vec![]);
        return true;
    }
    let foldable = args.iter().all(|a| !a.contains(['$', '[', '\\']));
    if foldable {
        let folded = args
            .iter()
            .map(|a| a.trim())
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join(" ");
        ctx.push_lit(&folded);
        ctx.emit(Op::POP, vec![]);
        return true;
    }
    for (i, a) in args.iter().enumerate() {
        ctx.emit_word_arg(i, a);
    }
    ctx.emit(Op::CONCAT_STK, vec![Operand::Imm(bytecode_imm(args.len()))]);
    ctx.emit(Op::POP, vec![]);
    true
}

// global / upvar

/// `global varName ...` — link each (simple, proc-local) variable to the
/// same-named global, byte-true with C Tcl's `TclCompileGlobalCmd`:
///
///   push "::"; (push name; nsupvar %slot)…; pop; push ""; pop
///
/// The `"::"` namespace reference is pushed once and reused by each
/// `nsupvar` (which pops the variable name and leaves the namespace on the
/// stack); the trailing `pop` discards it, and the empty result folds to
/// nops (or the bare `push ""` return in tail position). Qualified/dynamic
/// names, or top-level context, fall back to the generic invoke.
fn global_cmd(ctx: &mut CodegenCtx, args: &[String]) -> bool {
    if !ctx.is_proc || args.is_empty() {
        return false;
    }
    if args
        .iter()
        .any(|n| is_qualified(n) || n.starts_with('$') || n.starts_with('['))
    {
        return false;
    }
    ctx.push_lit("::");
    for name in args {
        let slot = ctx.lvt.intern(name);
        ctx.push_lit(name);
        ctx.emit_comment(
            Op::NSUPVAR,
            vec![Operand::Imm(bytecode_imm(slot))],
            &format!("var \"{name}\""),
        );
    }
    ctx.emit(Op::POP, vec![]);
    ctx.push_lit("");
    ctx.emit(Op::POP, vec![]);
    true
}

/// Whether `arg` is a literal `upvar` level: an optional `#` followed by
/// digits (`1`, `0`, `#0`) — the form that lets `upvar` push it as a literal
/// rather than evaluating it.
fn is_upvar_level(arg: &str) -> bool {
    let digits = arg.strip_prefix('#').unwrap_or(arg);
    !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit())
}

/// `upvar ?level? otherVar localVar ...` — link each caller variable to a
/// (simple, proc-local) local, byte-true with C Tcl's `TclCompileUpvarCmd`:
///
///   push <level>; (push other; upvar %slot)…; pop; push ""; pop
///
/// The level (an explicit literal `#?N`, else the default `"1"`) is pushed
/// once and reused by each `upvar`; the trailing `pop` discards it. `other`
/// may be any word (pushed through the interpolating path); each `local`
/// must be a simple proc-local name. A dynamic level, a malformed pair
/// count, a qualified/dynamic local, or top-level context fall back.
fn upvar_cmd(ctx: &mut CodegenCtx, args: &[String]) -> bool {
    if !ctx.is_proc || args.len() < 2 {
        return false;
    }
    let (level, pairs): (&str, &[String]) = if is_upvar_level(&args[0]) {
        (&args[0], &args[1..])
    } else {
        ("1", args)
    };
    if pairs.is_empty() || pairs.len() % 2 != 0 {
        return false;
    }
    // Every `local` (the odd-indexed words) must be a simple compiled local.
    for pair in pairs.as_chunks::<2>().0 {
        let local = &pair[1];
        if is_qualified(local) || local.starts_with('$') || local.starts_with('[') {
            return false;
        }
    }
    ctx.push_lit(level);
    for pair in pairs.as_chunks::<2>().0 {
        let (other, local) = (&pair[0], &pair[1]);
        let slot = ctx.lvt.intern(local);
        ctx.emit_value_interpolated(other);
        ctx.emit_comment(
            Op::UPVAR,
            vec![Operand::Imm(bytecode_imm(slot))],
            &format!("var \"{local}\""),
        );
    }
    ctx.emit(Op::POP, vec![]);
    ctx.push_lit("");
    ctx.emit(Op::POP, vec![]);
    true
}

#[cfg(test)]
mod tests {
    use tcl_dialect::model::{SurfaceQuery, Family};
    
    use super::*;
    use tcl_registry::CommandRegistry;

    #[test]
    fn lassign_rejects_wrong_arity() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(false, &[], &registry);
        let mut used = false;
        // 0 args
        assert!(!try_bytecoded(&mut ctx, "lassign", &[], &mut used));
        // only list, no vars
        let args = vec!["${lst}".to_string()];
        assert!(!try_bytecoded(&mut ctx, "lassign", &args, &mut used));
    }

    #[test]
    fn lassign_emits_expected_sequence() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(false, &[], &registry);
        let args = vec!["${lst}".to_string(), "a".to_string(), "b".to_string()];
        let mut used = false;
        assert!(try_bytecoded(&mut ctx, "lassign", &args, &mut used));
        let ops: Vec<Op> = ctx.instructions.iter().map(|i| i.op).collect();
        // load + for each var (push, over, listIndexImm, storeStk, pop)
        // + listRangeImm + pop
        assert!(ops.contains(&Op::LIST_INDEX_IMM));
        assert!(ops.contains(&Op::LIST_RANGE_IMM));
        assert!(ops.contains(&Op::OVER));
        // Last two ops should be listRangeImm + pop.
        assert_eq!(ops[ops.len() - 2], Op::LIST_RANGE_IMM);
        assert_eq!(ops[ops.len() - 1], Op::POP);
    }

    #[test]
    fn llength_single_arg() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(false, &[], &registry);
        let args = vec!["${lst}".to_string()];
        let mut used = false;
        assert!(try_bytecoded(&mut ctx, "llength", &args, &mut used));
        let ops: Vec<Op> = ctx.instructions.iter().map(|i| i.op).collect();
        assert!(ops.contains(&Op::LIST_LENGTH));
    }

    #[test]
    fn llength_wrong_arity_rejects() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(false, &[], &registry);
        let mut used = false;
        assert!(!try_bytecoded(&mut ctx, "llength", &[], &mut used));
        let args = vec!["a".into(), "b".into()];
        assert!(!try_bytecoded(&mut ctx, "llength", &args, &mut used));
    }

    #[test]
    fn array_names_emits_fq_invoke() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(false, &[], &registry);
        let args = vec!["names".into(), "${arr}".into()];
        let mut used = false;
        assert!(try_bytecoded(&mut ctx, "array", &args, &mut used));
        assert!(used);
        // Should push the FQ name as a literal.
        assert!(
            ctx.literals
                .entries()
                .iter()
                .any(|e| e == "::tcl::array::names")
        );
    }

    #[test]
    fn array_in_proc_context_rejects() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(true, &[], &registry);
        let args = vec!["names".into(), "${arr}".into()];
        let mut used = false;
        assert!(!try_bytecoded(&mut ctx, "array", &args, &mut used));
    }

    // -- lrange / linsert / lset --

    #[test]
    fn lrange_constant_indices() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(false, &[], &registry);
        let args = vec!["${lst}".to_string(), "0".into(), "end".into()];
        let mut used = false;
        assert!(try_bytecoded(&mut ctx, "lrange", &args, &mut used));
        let ops: Vec<Op> = ctx.instructions.iter().map(|i| i.op).collect();
        assert!(ops.contains(&Op::LIST_RANGE_IMM));
    }

    #[test]
    fn lrange_non_constant_indices_rejects() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(false, &[], &registry);
        let args = vec!["${lst}".to_string(), "$i".into(), "end".into()];
        let mut used = false;
        assert!(!try_bytecoded(&mut ctx, "lrange", &args, &mut used));
    }

    #[test]
    fn linsert_emits_lreplace4_with_op2() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(false, &[], &registry);
        let args = vec!["${lst}".to_string(), "2".into(), "hello".into()];
        let mut used = false;
        assert!(try_bytecoded(&mut ctx, "linsert", &args, &mut used));
        let ops: Vec<Op> = ctx.instructions.iter().map(|i| i.op).collect();
        assert!(ops.contains(&Op::LREPLACE4));
    }

    #[test]
    fn lset_proc_single_index_uses_lset_list() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(true, &[], &registry);
        let args = vec!["lst".to_string(), "1".into(), "new".into()];
        let mut used = false;
        assert!(try_bytecoded(&mut ctx, "lset", &args, &mut used));
        let ops: Vec<Op> = ctx.instructions.iter().map(|i| i.op).collect();
        assert!(ops.contains(&Op::LSET_LIST));
        assert!(ops.contains(&Op::LOAD_SCALAR1));
        assert!(ops.contains(&Op::STORE_SCALAR1));
    }

    #[test]
    fn lset_proc_multi_index_uses_lset_flat() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(true, &[], &registry);
        let args = vec!["lst".to_string(), "0".into(), "2".into(), "new".into()];
        let mut used = false;
        assert!(try_bytecoded(&mut ctx, "lset", &args, &mut used));
        let ops: Vec<Op> = ctx.instructions.iter().map(|i| i.op).collect();
        assert!(ops.contains(&Op::LSET_FLAT));
    }

    #[test]
    fn lset_toplevel_uses_stk_form() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(false, &[], &registry);
        let args = vec!["lst".to_string(), "1".into(), "new".into()];
        let mut used = false;
        assert!(try_bytecoded(&mut ctx, "lset", &args, &mut used));
        let ops: Vec<Op> = ctx.instructions.iter().map(|i| i.op).collect();
        assert!(ops.contains(&Op::OVER));
        assert!(ops.contains(&Op::LOAD_STK));
        assert!(ops.contains(&Op::STORE_STK));
        assert!(ops.contains(&Op::LSET_LIST));
    }

    #[test]
    fn lset_rejects_too_few_args() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(false, &[], &registry);
        let args = vec!["lst".to_string(), "new".into()];
        let mut used = false;
        assert!(!try_bytecoded(&mut ctx, "lset", &args, &mut used));
    }

    // -- dict subcommands --

    #[test]
    fn dict_set_proc_uses_dict_set_opcode() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(true, &[], &registry);
        let args = vec!["set".into(), "d".into(), "k".into(), "v".into()];
        let mut used = false;
        assert!(try_bytecoded(&mut ctx, "dict", &args, &mut used));
        let ops: Vec<Op> = ctx.instructions.iter().map(|i| i.op).collect();
        assert!(ops.contains(&Op::DICT_SET));
    }

    #[test]
    fn dict_incr_with_default_amount() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(true, &[], &registry);
        let args = vec!["incr".into(), "d".into(), "k".into()];
        let mut used = false;
        assert!(try_bytecoded(&mut ctx, "dict", &args, &mut used));
        let ops: Vec<Op> = ctx.instructions.iter().map(|i| i.op).collect();
        assert!(ops.contains(&Op::DICT_INCR_IMM));
    }

    #[test]
    fn dict_incr_with_explicit_amount() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(true, &[], &registry);
        let args = vec!["incr".into(), "d".into(), "k".into(), "5".into()];
        let mut used = false;
        assert!(try_bytecoded(&mut ctx, "dict", &args, &mut used));
    }

    #[test]
    fn dict_incr_rejects_non_integer_amount() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(true, &[], &registry);
        let args = vec!["incr".into(), "d".into(), "k".into(), "$amt".into()];
        let mut used = false;
        assert!(!try_bytecoded(&mut ctx, "dict", &args, &mut used));
    }

    #[test]
    fn dict_unset_uses_dict_unset_opcode() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(true, &[], &registry);
        let args = vec!["unset".into(), "d".into(), "k".into()];
        let mut used = false;
        assert!(try_bytecoded(&mut ctx, "dict", &args, &mut used));
        let ops: Vec<Op> = ctx.instructions.iter().map(|i| i.op).collect();
        assert!(ops.contains(&Op::DICT_UNSET));
    }

    #[test]
    fn dict_append_uses_dict_append_opcode() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(true, &[], &registry);
        let args = vec!["append".into(), "d".into(), "k".into(), "v".into()];
        let mut used = false;
        assert!(try_bytecoded(&mut ctx, "dict", &args, &mut used));
        let ops: Vec<Op> = ctx.instructions.iter().map(|i| i.op).collect();
        assert!(ops.contains(&Op::DICT_APPEND));
    }

    #[test]
    fn dict_lappend_uses_dict_lappend_opcode() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(true, &[], &registry);
        let args = vec!["lappend".into(), "d".into(), "k".into(), "v".into()];
        let mut used = false;
        assert!(try_bytecoded(&mut ctx, "dict", &args, &mut used));
        let ops: Vec<Op> = ctx.instructions.iter().map(|i| i.op).collect();
        assert!(ops.contains(&Op::DICT_LAPPEND));
    }

    #[test]
    fn dict_in_non_proc_context_uses_ensemble() {
        // Top-level `dict set` compiles to the ensemble-rewrite invokeReplace
        // form (tclsh's top-level codegen), not the proc-local DICT_* opcodes.
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(false, &[], &registry);
        let args = vec!["set".into(), "d".into(), "k".into(), "v".into()];
        let mut used = false;
        assert!(try_bytecoded(&mut ctx, "dict", &args, &mut used));
        let ops: Vec<Op> = ctx.instructions.iter().map(|i| i.op).collect();
        assert!(ops.contains(&Op::INVOKE_REPLACE));
        assert!(!ops.contains(&Op::DICT_SET));
    }

    #[test]
    fn dict_with_qualified_name_uses_ensemble() {
        // A qualified target var can't use the proc-local DICT_* slot form, so it
        // takes the same ensemble-rewrite invokeReplace path.
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(true, &[], &registry);
        let args = vec!["set".into(), "::global::d".into(), "k".into(), "v".into()];
        let mut used = false;
        assert!(try_bytecoded(&mut ctx, "dict", &args, &mut used));
        let ops: Vec<Op> = ctx.instructions.iter().map(|i| i.op).collect();
        assert!(ops.contains(&Op::INVOKE_REPLACE));
        assert!(!ops.contains(&Op::DICT_SET));
    }

    // -- append / lappend statement-position specialisations --

    #[test]
    fn append_scalar_single_uses_append_scalar1() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(true, &[], &registry);
        let args = vec!["x".into(), "a".into()];
        let mut used = false;
        assert!(try_bytecoded(&mut ctx, "append", &args, &mut used));
        let ops: Vec<Op> = ctx.instructions.iter().map(|i| i.op).collect();
        assert_eq!(ops, vec![Op::PUSH1, Op::APPEND_SCALAR1, Op::POP]);
    }

    #[test]
    fn append_multi_uses_reverse_then_append_per_value() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(true, &[], &registry);
        let args = vec!["x".into(), "a".into(), "b".into(), "c".into()];
        let mut used = false;
        assert!(try_bytecoded(&mut ctx, "append", &args, &mut used));
        let ops: Vec<Op> = ctx.instructions.iter().map(|i| i.op).collect();
        // push×3, reverse 3, then (appendScalar1; pop)×3.
        assert_eq!(
            ops,
            vec![
                Op::PUSH1,
                Op::PUSH1,
                Op::PUSH1,
                Op::REVERSE,
                Op::APPEND_SCALAR1,
                Op::POP,
                Op::APPEND_SCALAR1,
                Op::POP,
                Op::APPEND_SCALAR1,
                Op::POP,
            ]
        );
    }

    #[test]
    fn append_array_element_uses_append_array1() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(true, &[], &registry);
        let args = vec!["arr(k)".into(), "v".into()];
        let mut used = false;
        assert!(try_bytecoded(&mut ctx, "append", &args, &mut used));
        let ops: Vec<Op> = ctx.instructions.iter().map(|i| i.op).collect();
        assert_eq!(ops, vec![Op::PUSH1, Op::PUSH1, Op::APPEND_ARRAY1, Op::POP]);
    }

    #[test]
    fn append_toplevel_falls_back() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(false, &[], &registry);
        let args = vec!["x".into(), "a".into()];
        let mut used = false;
        assert!(!try_bytecoded(&mut ctx, "append", &args, &mut used));
    }

    #[test]
    fn append_qualified_and_dynamic_fall_back() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(true, &[], &registry);
        let mut used = false;
        assert!(!try_bytecoded(
            &mut ctx,
            "append",
            &["::g::x".into(), "a".into()],
            &mut used
        ));
        assert!(!try_bytecoded(
            &mut ctx,
            "append",
            &["$dyn".into(), "a".into()],
            &mut used
        ));
        // No value: nothing to append → generic invoke.
        assert!(!try_bytecoded(&mut ctx, "append", &["x".into()], &mut used));
    }

    #[test]
    fn lappend_scalar_single_uses_lappend_scalar1() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(true, &[], &registry);
        let args = vec!["y".into(), "a".into()];
        let mut used = false;
        assert!(try_bytecoded(&mut ctx, "lappend", &args, &mut used));
        let ops: Vec<Op> = ctx.instructions.iter().map(|i| i.op).collect();
        assert_eq!(ops, vec![Op::PUSH1, Op::LAPPEND_SCALAR1, Op::POP]);
    }

    #[test]
    fn lappend_multi_builds_list_then_lappend_list() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(true, &[], &registry);
        let args = vec!["y".into(), "a".into(), "b".into()];
        let mut used = false;
        assert!(try_bytecoded(&mut ctx, "lappend", &args, &mut used));
        let ops: Vec<Op> = ctx.instructions.iter().map(|i| i.op).collect();
        assert_eq!(
            ops,
            vec![Op::PUSH1, Op::PUSH1, Op::LIST, Op::LAPPEND_LIST, Op::POP]
        );
    }

    #[test]
    fn lappend_array_element_uses_lappend_array1() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(true, &[], &registry);
        let args = vec!["arr(k)".into(), "v".into()];
        let mut used = false;
        assert!(try_bytecoded(&mut ctx, "lappend", &args, &mut used));
        let ops: Vec<Op> = ctx.instructions.iter().map(|i| i.op).collect();
        assert_eq!(ops, vec![Op::PUSH1, Op::PUSH1, Op::LAPPEND_ARRAY1, Op::POP]);
    }

    #[test]
    fn lappend_multi_array_falls_back() {
        // Multi-value array lappend is not specialised (would need the key
        // re-pushed per element); it falls back to the generic invoke.
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(true, &[], &registry);
        let args = vec!["arr(k)".into(), "a".into(), "b".into()];
        let mut used = false;
        assert!(!try_bytecoded(&mut ctx, "lappend", &args, &mut used));
    }

    #[test]
    fn registry_append_lappend_specs_carry_codegen_hook() {
        let registry = CommandRegistry::build_default();
        assert_eq!(
            registry
                .get("append")
                .expect("append registered")
                .codegen_hook,
            Some(CodegenHookId::Append)
        );
        assert_eq!(
            registry
                .get("lappend")
                .expect("lappend registered")
                .codegen_hook,
            Some(CodegenHookId::Lappend)
        );
    }

    // -- unset statement-position specialisation --

    #[test]
    fn unset_scalar_uses_unset_scalar() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(true, &[], &registry);
        let mut used = false;
        assert!(try_bytecoded(&mut ctx, "unset", &["x".into()], &mut used));
        let ops: Vec<Op> = ctx.instructions.iter().map(|i| i.op).collect();
        // unsetScalar, then the empty result push + pop (folded to nops later).
        assert_eq!(ops, vec![Op::UNSET_SCALAR, Op::PUSH1, Op::POP]);
        assert_eq!(ctx.instructions[0].operands[0], Operand::Imm(1)); // complain flag
    }

    #[test]
    fn unset_nocomplain_clears_flag() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(true, &[], &registry);
        let mut used = false;
        let args = vec!["-nocomplain".into(), "x".into()];
        assert!(try_bytecoded(&mut ctx, "unset", &args, &mut used));
        assert_eq!(ctx.instructions[0].op, Op::UNSET_SCALAR);
        assert_eq!(ctx.instructions[0].operands[0], Operand::Imm(0));
    }

    #[test]
    fn unset_double_dash_ends_options() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(true, &[], &registry);
        let mut used = false;
        // `--` terminates options; `-x` is then a (compiled-local) var name.
        let args = vec!["--".into(), "-x".into()];
        assert!(try_bytecoded(&mut ctx, "unset", &args, &mut used));
        assert_eq!(ctx.instructions[0].op, Op::UNSET_SCALAR);
    }

    #[test]
    fn unset_array_uses_unset_array() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(true, &[], &registry);
        let mut used = false;
        assert!(try_bytecoded(
            &mut ctx,
            "unset",
            &["arr(k)".into()],
            &mut used
        ));
        let ops: Vec<Op> = ctx.instructions.iter().map(|i| i.op).collect();
        assert_eq!(ops, vec![Op::PUSH1, Op::UNSET_ARRAY, Op::PUSH1, Op::POP]);
    }

    #[test]
    fn unset_multi_unsets_each_then_one_result() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(true, &[], &registry);
        let mut used = false;
        let args = vec!["x".into(), "y".into()];
        assert!(try_bytecoded(&mut ctx, "unset", &args, &mut used));
        let ops: Vec<Op> = ctx.instructions.iter().map(|i| i.op).collect();
        assert_eq!(
            ops,
            vec![Op::UNSET_SCALAR, Op::UNSET_SCALAR, Op::PUSH1, Op::POP]
        );
    }

    #[test]
    fn unset_qualified_uses_stk_form() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(true, &[], &registry);
        let mut used = false;
        assert!(try_bytecoded(
            &mut ctx,
            "unset",
            &["::g::v".into()],
            &mut used
        ));
        let ops: Vec<Op> = ctx.instructions.iter().map(|i| i.op).collect();
        assert_eq!(ops, vec![Op::PUSH1, Op::UNSET_STK, Op::PUSH1, Op::POP]);
    }

    #[test]
    fn unset_toplevel_falls_back() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(false, &[], &registry);
        let mut used = false;
        assert!(!try_bytecoded(&mut ctx, "unset", &["x".into()], &mut used));
    }

    #[test]
    fn unset_no_names_falls_back() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(true, &[], &registry);
        let mut used = false;
        assert!(!try_bytecoded(
            &mut ctx,
            "unset",
            &["-nocomplain".into()],
            &mut used
        ));
    }

    #[test]
    fn registry_unset_spec_carries_codegen_hook() {
        let registry = CommandRegistry::build_default();
        assert_eq!(
            registry
                .get("unset")
                .expect("unset registered")
                .codegen_hook,
            Some(CodegenHookId::Unset)
        );
    }

    // -- tailcall statement-position specialisation --

    #[test]
    fn tailcall_pushes_literal_prefix_then_args() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(true, &[], &registry);
        let mut used = false;
        let args = vec!["foo".into(), "a".into(), "b".into()];
        assert!(try_bytecoded(&mut ctx, "tailcall", &args, &mut used));
        let ops: Vec<Op> = ctx.instructions.iter().map(|i| i.op).collect();
        assert_eq!(
            ops,
            vec![
                Op::PUSH1,
                Op::PUSH1,
                Op::PUSH1,
                Op::PUSH1,
                Op::TAILCALL,
                Op::POP
            ]
        );
        // operand counts the "tailcall" prefix plus the three args.
        let tc = ctx
            .instructions
            .iter()
            .find(|i| i.op == Op::TAILCALL)
            .unwrap();
        assert_eq!(tc.operands[0], Operand::Imm(4));
        assert_eq!(ctx.literals.entries()[0], "tailcall");
    }

    #[test]
    fn tailcall_no_args_falls_back() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(true, &[], &registry);
        let mut used = false;
        assert!(!try_bytecoded(&mut ctx, "tailcall", &[], &mut used));
    }

    #[test]
    fn registry_tailcall_spec_carries_codegen_hook() {
        let registry = CommandRegistry::build_default();
        assert_eq!(
            registry
                .get("tailcall")
                .expect("tailcall registered")
                .codegen_hook,
            Some(CodegenHookId::Tailcall)
        );
    }

    // -- concat statement-position specialisation --

    #[test]
    fn concat_all_literal_folds() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(true, &[], &registry);
        let mut used = false;
        let args = vec!["a".into(), "b".into(), "c".into()];
        assert!(try_bytecoded(&mut ctx, "concat", &args, &mut used));
        let ops: Vec<Op> = ctx.instructions.iter().map(|i| i.op).collect();
        assert_eq!(ops, vec![Op::PUSH1, Op::POP]);
        assert_eq!(ctx.literals.entries()[0], "a b c");
    }

    #[test]
    fn concat_fold_trims_and_drops_empties() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(true, &[], &registry);
        let mut used = false;
        let args = vec!["  a ".into(), String::new(), " b".into()];
        assert!(try_bytecoded(&mut ctx, "concat", &args, &mut used));
        assert_eq!(ctx.literals.entries()[0], "a b");
    }

    #[test]
    fn concat_with_substitution_uses_concat_stk() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(true, &[], &registry);
        let mut used = false;
        // A `$`-bearing argument is not folded; the command takes the
        // concatStk path (the byte-true loadScalar emission for `$x` in a
        // real proc body is covered by the concat-mixed golden fixture).
        let args = vec!["a".into(), "$x".into(), "b".into()];
        assert!(try_bytecoded(&mut ctx, "concat", &args, &mut used));
        let ops: Vec<Op> = ctx.instructions.iter().map(|i| i.op).collect();
        assert_eq!(ops.last(), Some(&Op::POP));
        let cc = ctx
            .instructions
            .iter()
            .find(|i| i.op == Op::CONCAT_STK)
            .expect("concatStk emitted for a non-foldable concat");
        assert_eq!(cc.operands[0], Operand::Imm(3));
    }

    #[test]
    fn concat_backslash_arg_not_folded() {
        // A backslash escape must not be naively folded; it takes concatStk.
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(true, &[], &registry);
        let mut used = false;
        let args = vec!["a".into(), "b\\tc".into()];
        assert!(try_bytecoded(&mut ctx, "concat", &args, &mut used));
        let ops: Vec<Op> = ctx.instructions.iter().map(|i| i.op).collect();
        assert!(ops.contains(&Op::CONCAT_STK));
    }

    #[test]
    fn concat_no_args_pushes_empty() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(true, &[], &registry);
        let mut used = false;
        assert!(try_bytecoded(&mut ctx, "concat", &[], &mut used));
        let ops: Vec<Op> = ctx.instructions.iter().map(|i| i.op).collect();
        assert_eq!(ops, vec![Op::PUSH1, Op::POP]);
        assert_eq!(ctx.literals.entries()[0], "");
    }

    #[test]
    fn registry_concat_spec_carries_codegen_hook() {
        let registry = CommandRegistry::build_default();
        assert_eq!(
            registry
                .get("concat")
                .expect("concat registered")
                .codegen_hook,
            Some(CodegenHookId::Concat)
        );
    }

    // -- global / upvar statement-position specialisation --

    #[test]
    fn global_single_uses_nsupvar() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(true, &[], &registry);
        let mut used = false;
        assert!(try_bytecoded(&mut ctx, "global", &["x".into()], &mut used));
        let ops: Vec<Op> = ctx.instructions.iter().map(|i| i.op).collect();
        // push "::"; push "x"; nsupvar; pop; push ""; pop
        assert_eq!(
            ops,
            vec![
                Op::PUSH1,
                Op::PUSH1,
                Op::NSUPVAR,
                Op::POP,
                Op::PUSH1,
                Op::POP
            ]
        );
        assert_eq!(ctx.literals.entries()[0], "::");
    }

    #[test]
    fn global_multi_reuses_namespace() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(true, &[], &registry);
        let mut used = false;
        let args = vec!["x".into(), "y".into()];
        assert!(try_bytecoded(&mut ctx, "global", &args, &mut used));
        let ops: Vec<Op> = ctx.instructions.iter().map(|i| i.op).collect();
        assert_eq!(
            ops,
            vec![
                Op::PUSH1,
                Op::PUSH1,
                Op::NSUPVAR,
                Op::PUSH1,
                Op::NSUPVAR,
                Op::POP,
                Op::PUSH1,
                Op::POP
            ]
        );
    }

    #[test]
    fn global_qualified_or_toplevel_falls_back() {
        let registry = CommandRegistry::build_default();
        let mut proc_ctx = CodegenCtx::new(true, &[], &registry);
        let mut used = false;
        assert!(!try_bytecoded(
            &mut proc_ctx,
            "global",
            &["::g::x".into()],
            &mut used
        ));
        let mut top = CodegenCtx::new(false, &[], &registry);
        assert!(!try_bytecoded(&mut top, "global", &["x".into()], &mut used));
    }

    #[test]
    fn upvar_with_level_uses_upvar_op() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(true, &[], &registry);
        let mut used = false;
        let args = vec!["1".into(), "foo".into(), "bar".into()];
        assert!(try_bytecoded(&mut ctx, "upvar", &args, &mut used));
        let ops: Vec<Op> = ctx.instructions.iter().map(|i| i.op).collect();
        assert_eq!(
            ops,
            vec![Op::PUSH1, Op::PUSH1, Op::UPVAR, Op::POP, Op::PUSH1, Op::POP]
        );
        assert_eq!(ctx.literals.entries()[0], "1");
    }

    #[test]
    fn upvar_without_level_defaults_to_one() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(true, &[], &registry);
        let mut used = false;
        let args = vec!["foo".into(), "bar".into()];
        assert!(try_bytecoded(&mut ctx, "upvar", &args, &mut used));
        // The implicit level is the literal "1".
        assert_eq!(ctx.literals.entries()[0], "1");
        assert!(ctx.instructions.iter().any(|i| i.op == Op::UPVAR));
    }

    #[test]
    fn upvar_hash_level_recognised() {
        assert!(is_upvar_level("#0"));
        assert!(is_upvar_level("1"));
        assert!(is_upvar_level("12"));
        assert!(!is_upvar_level("foo"));
        assert!(!is_upvar_level("#"));
        assert!(!is_upvar_level("$n"));
    }

    #[test]
    fn upvar_multi_pair_reuses_level() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(true, &[], &registry);
        let mut used = false;
        let args = vec!["1".into(), "a".into(), "x".into(), "b".into(), "y".into()];
        assert!(try_bytecoded(&mut ctx, "upvar", &args, &mut used));
        let ops: Vec<Op> = ctx.instructions.iter().map(|i| i.op).collect();
        assert_eq!(
            ops,
            vec![
                Op::PUSH1,
                Op::PUSH1,
                Op::UPVAR,
                Op::PUSH1,
                Op::UPVAR,
                Op::POP,
                Op::PUSH1,
                Op::POP
            ]
        );
    }

    #[test]
    fn upvar_qualified_local_falls_back() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(true, &[], &registry);
        let mut used = false;
        let args = vec!["1".into(), "foo".into(), "::g::bar".into()];
        assert!(!try_bytecoded(&mut ctx, "upvar", &args, &mut used));
    }

    #[test]
    fn registry_global_upvar_specs_carry_codegen_hook() {
        let registry = CommandRegistry::build_default();
        assert_eq!(
            registry
                .get("global")
                .expect("global registered")
                .codegen_hook,
            Some(CodegenHookId::Global)
        );
        assert_eq!(
            registry
                .get("upvar")
                .expect("upvar registered")
                .codegen_hook,
            Some(CodegenHookId::Upvar)
        );
    }

    #[test]
    fn unknown_command_returns_false() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(false, &[], &registry);
        let mut used = false;
        assert!(!try_bytecoded(&mut ctx, "foobar", &[], &mut used));
    }

    /// Registry-side hook ID for `llength` is the canonical source
    /// of truth for codegen routing.
    #[test]
    fn registry_llength_spec_carries_codegen_hook() {
        let registry = CommandRegistry::build_default();
        let spec = registry.get("llength").expect("llength is registered");
        assert_eq!(spec.codegen_hook, Some(CodegenHookId::Llength));
    }

    /// Registry-side hook ID for `dict` covers all dict subcommand
    /// emissions through one hook.
    #[test]
    fn registry_dict_spec_carries_codegen_hook() {
        let registry = CommandRegistry::build_default();
        let spec = registry.get("dict").expect("dict is registered");
        assert_eq!(spec.codegen_hook, Some(CodegenHookId::Dict));
    }

    /// Resolving the `dict set` subcommand call yields the parent
    /// `dict` command's `Dict` codegen hook.
    #[test]
    fn resolve_call_routes_dict_set_subcommand_to_dict_hook() {
        let registry = CommandRegistry::build_default();
        let resolved = registry
            .resolve_call(
                "dict",
                &["set", "d", "k", "v"],
                Some(SurfaceQuery::core(Family::Tcl, "8.6")),
            )
            .expect("dict set resolves");
        assert_eq!(resolved.codegen_hook, Some(CodegenHookId::Dict));
        assert_eq!(
            resolved.sub.expect("dict set has a subcommand entry").name,
            "set",
        );
    }

    /// Resolving an iRules command form: the `HTTP::header` command
    /// must come back from the registry (after loading the iRules
    /// dialect) so the resolved-call API can drive iRules-aware
    /// pipelines without command-name dispatch tables in the
    /// compiler.
    #[test]
    fn resolve_call_irules_http_header_form() {
        let mut registry = CommandRegistry::build_default();
        registry.load_irules();
        let resolved = registry
            .resolve_call("HTTP::header", &["names"], Some(SurfaceQuery::any_release(Family::F5Irules)))
            .expect("HTTP::header resolves under iRules");
        assert_eq!(resolved.spec.name, "HTTP::header");
    }

    /// `try_bytecoded` reads its registry from `ctx.registry`,
    /// not from a private static. Loading the iRules dialect into the
    /// registry passed into the context must therefore make
    /// dialect-only specs visible to codegen-hook resolution. Today
    /// no iRules command carries a `codegen_hook`, so the visible
    /// proof is that the dialect-only spec resolves; coupled with
    /// the absence of any `OnceLock<CommandRegistry>` static, this
    /// is the registry-threading contract.
    #[test]
    fn ctx_registry_drives_codegen_hook_resolution() {
        // Plain default registry: an iRules-only command is unknown,
        // so resolve_call returns None and try_bytecoded would skip
        // the hook path even if one existed.
        let default = CommandRegistry::build_default();
        assert!(
            default
                .resolve_call("HTTP::header", &["names"], Some(SurfaceQuery::any_release(Family::F5Irules)),)
                .is_none()
        );

        // A registry with iRules loaded sees the same name. The
        // CodegenCtx built from this registry would see whatever
        // codegen_hook the iRules spec carries (currently None,
        // since no iRules command has a registry-side codegen
        // hook yet). The point is `ctx.registry` is what drives
        // resolution, not a hardcoded default.
        let mut irules = CommandRegistry::build_default();
        irules.load_irules();
        let resolved = irules
            .resolve_call("HTTP::header", &["names"], Some(SurfaceQuery::any_release(Family::F5Irules)))
            .expect("HTTP::header resolves once iRules is loaded");

        // Wire it through CodegenCtx: the ctx's registry field is
        // what try_bytecoded reads.
        let ctx = CodegenCtx::new(true, &[], &irules);
        assert!(std::ptr::eq(ctx.registry, &raw const irules));
        assert_eq!(resolved.spec.name, "HTTP::header");
    }
}
