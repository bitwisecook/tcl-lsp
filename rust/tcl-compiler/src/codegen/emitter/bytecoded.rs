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
//! Commands with compiled bytecode forms are routed through this dispatcher.
//! Statement-only [`CodegenHookId`] emitters live here; statement-safe forms
//! of a value-position [`tcl_registry::hooks::InlineCodegenHookId`] delegate
//! to their existing emitter. Hook selection is registry-driven: it
//! reads the active [`tcl_registry::CommandRegistry`] from
//! `ctx.registry` so dialect-loaded specs (iRules, Tk, EDA) drive
//! codegen-hook resolution. The compiler still owns the per-variant
//! emitter; the registry decides which variant applies.

use tcl_registry::hooks::CodegenHookId;

use crate::ir::CommandTokens;

use super::super::Op;
use super::super::Operand;
use super::super::values::{is_qualified, split_array_ref};
use super::super::{CodegenCtx, SiteBinding};
use super::super::{INDEX_END, bytecode_imm, parse_tcl_index};

/// Try to emit specialised bytecode for `cmd args...` via a typed registry
/// hook. Returns `true` if the hook handled the command;
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
    try_bytecoded_with_tokens(ctx, cmd, args, None, used_generic_invoke)
}

/// Source-aware variant used by executable IR emission. Hand-built callers
/// have no lexical word facts and therefore take the conservative wrapper.
pub fn try_bytecoded_with_tokens(
    ctx: &mut CodegenCtx,
    cmd: &str,
    args: &[String],
    tokens: Option<&CommandTokens>,
    used_generic_invoke: &mut bool,
) -> bool {
    let operands = args
        .iter()
        .enumerate()
        .map(|(index, word)| {
            (
                word.clone(),
                ctx.cmd_arg_braced.get(index).copied().unwrap_or(false),
            )
        })
        .collect::<Vec<_>>();
    ctx.with_native_hook_operands(cmd, &operands, |ctx, logical| {
        let arguments = logical
            .iter()
            .map(|(word, _)| word.clone())
            .collect::<Vec<_>>();
        try_bytecoded_in_layout(ctx, cmd, &arguments, tokens, used_generic_invoke)
    })
}

fn try_bytecoded_in_layout(
    ctx: &mut CodegenCtx,
    cmd: &str,
    args: &[String],
    tokens: Option<&CommandTokens>,
    used_generic_invoke: &mut bool,
) -> bool {
    if let Some((hook, site)) = resolved_codegen_hook(ctx, cmd, args) {
        let emitted = dispatch_codegen_hook(hook, ctx, args, used_generic_invoke);
        if emitted {
            ctx.require_site_binding(&site);
            return true;
        }
    }
    ctx.try_inline_statement_codegen(cmd, args, tokens, used_generic_invoke)
}

/// Return the registry hook and binding identity available to the bytecode
/// emitter before its command-specific shape guard runs.
fn resolved_codegen_hook(
    ctx: &CodegenCtx,
    cmd: &str,
    args: &[String],
) -> Option<(CodegenHookId, SiteBinding)> {
    if ctx.plain_command_dispatch || !ctx.invocation_specialisation_proved() {
        return None;
    }
    registry_codegen_hook(ctx, cmd, args)
}

/// Resolve the registry-owned typed hook for this exact command shape without
/// applying a compilation unit's conservative source-level trust summary.
///
/// The registry/profile answer is the semantic fact. Both specialised
/// statements and generic value-position surrogates retain the returned
/// binding, so the VM can admit or deoptimise the artifact against the live
/// command table. That runtime proof is more precise than rejecting a hook
/// merely because another procedure in the same source can mutate its name.
fn registry_codegen_hook(
    ctx: &CodegenCtx,
    cmd: &str,
    args: &[String],
) -> Option<(CodegenHookId, SiteBinding)> {
    if let Some(tokens) = ctx.invocation_tokens.as_deref()
        && tokens.source_binding.is_some()
    {
        let admitted = crate::registry_invocation::admitted_native_compiler_invocation(
            ctx.registry,
            None,
            tokens,
        )?;
        return Some((
            admitted.codegen_hook()?,
            SiteBinding {
                binding: ctx.command_binding_identity(cmd, admitted.canonical_registration_name()),
                claim: None,
            },
        ));
    }
    let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
    // The retained native point selects backend metadata independently of
    // the assisting catalogue's command surface.
    let resolved = ctx
        .registry
        .resolve_call(cmd, &arg_refs, ctx.invocation_surface_query())?;
    let hook = resolved.codegen_hook?;
    // The builtin a pack command is, when the hook is that builtin's own
    // (`alias_of`, the one admissible source): the VM's alias hop resolves
    // the pack name to the target, so recording the target is what lets the
    // specialised site be admitted at all, and the pack facts behind the
    // claim travel with it. Everything else records its own name.
    let site = ctx.stamped_binding(
        cmd,
        &resolved,
        tcl_registry::codegen_stamp::CodegenStamp::Codegen(hook),
    );
    Some((hook, site))
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
        CodegenHookId::Uplevel => uplevel_cmd(ctx, args),
    }
}

// list

/// Emit the already selected native frame operation from its frozen argv layout.
fn uplevel_cmd(ctx: &mut CodegenCtx, args: &[String]) -> bool {
    let Some(tokens) = ctx.invocation_tokens.as_deref() else {
        return false;
    };
    let Some(invocation) =
        crate::registry_invocation::admitted_native_compiler_invocation(ctx.registry, None, tokens)
    else {
        return false;
    };
    let Some(spec) = invocation.native_compilation() else {
        return false;
    };
    let Some(layout) =
        invocation.with_argument_words(|words| spec.uplevel_operands(words.arguments()))
    else {
        return false;
    };
    if let Some(level) = layout.level {
        ctx.emit_word_arg(level, &args[level]);
    } else {
        ctx.push_lit("1");
    }
    for (index, argument) in args.iter().enumerate().skip(layout.script_from) {
        ctx.emit_word_arg(index, argument);
    }
    let scripts = args.len() - layout.script_from;
    if scripts > 1 {
        ctx.emit(Op::CONCAT_STK, vec![Operand::Imm(bytecode_imm(scripts))]);
    }
    ctx.emit(Op::UPLEVEL, vec![]);
    ctx.emit(Op::POP, vec![]);
    true
}

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

/// Emit the admitted lset compiler using original words and its actual
/// scalar, array or stack address. The load occurs after index/value argv.
fn lset(ctx: &mut CodegenCtx, args: &[String]) -> bool {
    if args.len() < 2 {
        return false;
    }
    let address = emit_append_address(ctx, &args[0]);
    for (index, argument) in args.iter().enumerate().skip(1) {
        ctx.emit_word_arg(index, argument);
    }
    let (load, store, operands) = match address {
        AppendAddress::Scalar(slot) => (
            if slot < 256 {
                Op::LOAD_SCALAR1
            } else {
                Op::LOAD_SCALAR4
            },
            if slot < 256 {
                Op::STORE_SCALAR1
            } else {
                Op::STORE_SCALAR4
            },
            vec![Operand::Imm(bytecode_imm(slot))],
        ),
        AppendAddress::Array(slot) => {
            ctx.emit(Op::OVER, vec![Operand::Imm(bytecode_imm(args.len() - 1))]);
            (
                if slot < 256 {
                    Op::LOAD_ARRAY1
                } else {
                    Op::LOAD_ARRAY4
                },
                if slot < 256 {
                    Op::STORE_ARRAY1
                } else {
                    Op::STORE_ARRAY4
                },
                vec![Operand::Imm(bytecode_imm(slot))],
            )
        }
        AppendAddress::ArrayStack => {
            // Preserve the original root/key below the index and value words.
            let depth = bytecode_imm(args.len());
            ctx.emit(Op::OVER, vec![Operand::Imm(depth)]);
            ctx.emit(Op::OVER, vec![Operand::Imm(depth)]);
            (Op::LOAD_ARRAY_STK, Op::STORE_ARRAY_STK, vec![])
        }
        AppendAddress::Stack => {
            ctx.emit(Op::OVER, vec![Operand::Imm(bytecode_imm(args.len() - 1))]);
            (Op::LOAD_STK, Op::STORE_STK, vec![])
        }
    };
    ctx.emit(load, operands.clone());
    if args.len() == 3 {
        ctx.emit(Op::LSET_LIST, vec![]);
    } else {
        ctx.emit(Op::LSET_FLAT, vec![Operand::Imm(bytecode_imm(args.len()))]);
    }
    ctx.emit(store, operands);
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

    if rest.len() == 3 && ctx.cmd_arg_braced.get(3).copied() == Some(true) {
        let emitted = match sub {
            "for" => ctx.emit_dict_for(&rest[0], &rest[1], &rest[2]),
            "map" => ctx.emit_dict_map(&rest[0], &rest[1], &rest[2]),
            _ => false,
        };
        if emitted {
            ctx.emit(Op::POP, vec![]);
            return true;
        }
    }

    // Non-proc (or qualified-var) mutating `dict` subcommand → the top-level
    // ensemble-rewrite `INVOKE_REPLACE` form (see [`dict_ensemble`]); the
    // proc-local scalar path below keeps its specialised `DICT_*` opcodes.
    let proc_local = ctx.compiles_locals() && !is_qualified(&rest[0]);
    if !proc_local && matches!(sub, "set" | "unset" | "incr" | "append" | "lappend") {
        if let Some((registration, prefix)) = ctx.native_hook_layout.clone() {
            ctx.push_lit(&registration);
            for (index, argument) in args.iter().enumerate().skip(prefix) {
                ctx.emit_word_arg(index, argument);
            }
            let count = bytecode_imm(1 + args.len() - prefix);
            ctx.emit(
                if count < 256 {
                    Op::INVOKE_STK1
                } else {
                    Op::INVOKE_STK4
                },
                vec![Operand::Imm(count)],
            );
            ctx.emit(Op::POP, vec![]);
            return true;
        }
        let Some(namespace) =
            native_ensemble_namespace(ctx, tcl_registry::EnsembleImplementationFamily::Dict)
        else {
            return false;
        };
        dict_ensemble(ctx, namespace, sub, rest);
        return true;
    }

    if !ctx.compiles_locals() || args.len() < 3 {
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
        // Compile literal bodies with the shared command-at-time owner.
        // The path form of `dict with` uses the runtime command.
        "update"
            if rest.len() >= 4
                && rest.len().is_multiple_of(2)
                && ctx.cmd_arg_braced.get(args.len() - 1).copied() == Some(true) =>
        {
            let emitted = ctx.emit_dict_update(rest);
            if emitted {
                ctx.emit(Op::POP, vec![]);
            }
            emitted
        }
        "with" if rest.len() == 2 && ctx.cmd_arg_braced.get(2).copied() == Some(true) => {
            let emitted = ctx.emit_dict_with(&rest[0], &rest[1]);
            if emitted {
                ctx.emit(Op::POP, vec![]);
            }
            emitted
        }
        _ => false,
    }
}

/// `dict set var k1 ?k2 …? value` → `DICT_SET N slot`.
fn dict_set(ctx: &mut CodegenCtx, var_name: &str, rest: &[String]) -> bool {
    let keys = &rest[1..rest.len() - 1];
    let value = rest.last().unwrap();
    let Some(slot) = ctx.command_variable_slot(var_name.as_bytes()) else {
        return false;
    };
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
    let Some(slot) = ctx.command_variable_slot(var_name.as_bytes()) else {
        return false;
    };
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
    let Some(slot) = ctx.command_variable_slot(var_name.as_bytes()) else {
        return false;
    };
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
    let Some(slot) = ctx.command_variable_slot(var_name.as_bytes()) else {
        return false;
    };
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
    let Some(slot) = ctx.command_variable_slot(var_name.as_bytes()) else {
        return false;
    };
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
fn native_ensemble_namespace(
    ctx: &CodegenCtx,
    family: tcl_registry::EnsembleImplementationFamily,
) -> Option<&'static str> {
    ctx.registry
        .profile()
        .map(tcl_registry::InvocationDialect::of_profile)
        .and_then(|dialect| dialect.ensemble_implementation_namespace(family))
}

fn dict_ensemble(ctx: &mut CodegenCtx, namespace: &str, sub: &str, sub_args: &[String]) {
    ctx.push_lit("dict");
    ctx.push_lit(sub);
    for a in sub_args {
        ctx.emit_value_interpolated(a);
    }
    ctx.push_lit(&format!("{namespace}::{sub}"));
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
    let Some(namespace) =
        native_ensemble_namespace(ctx, tcl_registry::EnsembleImplementationFamily::Namespace)
    else {
        return false;
    };
    ctx.push_lit("namespace");
    ctx.push_lit("eval");
    // Push the namespace and every body word exactly as the generic path would:
    // `emit_word_arg` reads `cmd_arg_braced[idx]` (arg 0 is `eval`), so args[i]
    // is original index `i` — a braced body word is pushed verbatim, a dynamic
    // `$body` is interpolated.
    for (i, a) in args.iter().enumerate().skip(1) {
        ctx.emit_word_arg(i, a);
    }
    ctx.push_lit(&format!("{namespace}::eval"));
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
        let Some(namespace) =
            native_ensemble_namespace(ctx, tcl_registry::EnsembleImplementationFamily::Array)
        else {
            return false;
        };
        ctx.push_lit(&format!("{namespace}::for"));
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

    if ctx.compiles_locals() {
        return false;
    }
    match sub {
        "names" | "size" if !rest.is_empty() => {
            let Some(namespace) =
                native_ensemble_namespace(ctx, tcl_registry::EnsembleImplementationFamily::Array)
            else {
                return false;
            };
            ctx.push_lit(&format!("{namespace}::{sub}"));
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
    ctx.compiles_locals() && !is_qualified(var) && !var.starts_with('$') && !var.starts_with('[')
}

/// [`is_compilable_local`] restricted to a *scalar*: an `arr(k)`-shaped name is
/// rejected too.
///
/// The link commands (`global`, `upvar`) take this stricter gate, mirroring C's
/// `LocalScalar` (`tclCompCmds.c` — `TclPushVarName` with `TCL_NO_ELEMENT`,
/// which answers -1 and abandons the compile for an element-looking name). It
/// is not a nicety: the link is the *point* at which C reports `bad variable
/// name "…": can't create a scalar variable that looks like an array element`,
/// and that report lives in the commands. Compiling these names to
/// `nsupvar`/`upvar` would silently create the mislinked variable instead.
/// `append`/`lappend`/`unset` keep the looser gate — they have real element
/// opcodes.
fn is_compilable_scalar_local(ctx: &CodegenCtx, var: &str) -> bool {
    is_compilable_local(ctx, var) && split_array_ref(var).is_none()
}

/// Native variable-name emission keeps source eligibility separate from its
/// evaluated value. A literal array key is data, even when it contains `$`.
#[derive(Clone, Copy)]
enum AppendAddress {
    Scalar(usize),
    Array(usize),
    ArrayStack,
    Stack,
}

fn compiled_append_local_name(ctx: &CodegenCtx, variable: &str) -> Option<String> {
    if !ctx.compiles_locals() {
        return None;
    }
    let source = ctx
        .original_hook_argument(0)
        .and_then(|index| ctx.invocation_tokens.as_deref()?.words().get(index + 1));
    let direct = match source {
        Some(word) => {
            crate::registry_invocation::compiled_local_name_value(word, ctx.escapes, ctx.word_rules)
        }
        // Hand-built hooks supply literal names without a source carrier.
        None => is_compilable_local(ctx, variable).then(|| variable.to_owned()),
    }?;
    (!is_qualified(&direct)).then_some(direct)
}

fn emit_append_address(ctx: &mut CodegenCtx, variable: &str) -> AppendAddress {
    use tcl_syntax::native_variable_words::NativeVariableWordOperand;
    let source = ctx
        .original_hook_argument(0)
        .and_then(|index| ctx.invocation_tokens.as_deref()?.words().get(index + 1))
        .cloned();
    let projected = source
        .as_ref()
        .and_then(|word| ctx.original_variable_operand(word));
    match projected {
        Some(NativeVariableWordOperand::Literal { name, index, .. }) => {
            let slot = ctx.command_variable_slot(&name);
            if slot.is_none() {
                ctx.push_lit_bytes_exact(&name);
            }
            if let Some(index) = index {
                ctx.push_lit_bytes_exact(&index);
                return slot.map_or(AppendAddress::ArrayStack, AppendAddress::Array);
            }
            return slot.map_or(AppendAddress::Stack, AppendAddress::Scalar);
        }
        Some(NativeVariableWordOperand::CompoundArray { name, index, .. }) => {
            let slot = ctx.command_variable_slot(&name);
            if slot.is_none() {
                ctx.push_lit_bytes_exact(&name);
            }
            ctx.emit_executable_arena(&index);
            return slot.map_or(AppendAddress::ArrayStack, AppendAddress::Array);
        }
        Some(NativeVariableWordOperand::DynamicWord) => {
            ctx.emit_word_arg(0, variable);
            return AppendAddress::Stack;
        }
        None => {}
    }
    // Authored hooks without retained lexical geometry consume resolved data.
    if ctx.native_entry.is_some() {
        ctx.refuse_native_dependency();
    } else if let Some(name) = compiled_append_local_name(ctx, variable) {
        let (base, index) =
            split_array_ref(&name).map_or((name.as_str(), None), |(base, key)| (base, Some(key)));
        if let Some(slot) = ctx.command_variable_slot(base.as_bytes()) {
            if let Some(key) = index {
                ctx.push_lit_exact(key);
                return AppendAddress::Array(slot);
            }
            return AppendAddress::Scalar(slot);
        }
    }
    ctx.emit_word_arg(0, variable);
    AppendAddress::Stack
}

fn append_single_op(address: AppendAddress, list: bool) -> (Op, Vec<Operand>) {
    let (op, slot) = match (address, list) {
        (AppendAddress::Scalar(slot), false) => (
            if slot < 256 {
                Op::APPEND_SCALAR1
            } else {
                Op::APPEND_SCALAR4
            },
            Some(slot),
        ),
        (AppendAddress::Array(slot), false) => (
            if slot < 256 {
                Op::APPEND_ARRAY1
            } else {
                Op::APPEND_ARRAY4
            },
            Some(slot),
        ),
        (AppendAddress::Scalar(slot), true) => (
            if slot < 256 {
                Op::LAPPEND_SCALAR1
            } else {
                Op::LAPPEND_SCALAR4
            },
            Some(slot),
        ),
        (AppendAddress::Array(slot), true) => (
            if slot < 256 {
                Op::LAPPEND_ARRAY1
            } else {
                Op::LAPPEND_ARRAY4
            },
            Some(slot),
        ),
        (AppendAddress::ArrayStack, false) => (Op::APPEND_ARRAY_STK, None),
        (AppendAddress::ArrayStack, true) => (Op::LAPPEND_ARRAY_STK, None),
        (AppendAddress::Stack, false) => (Op::APPEND_STK, None),
        (AppendAddress::Stack, true) => (Op::LAPPEND_STK, None),
    };
    (
        op,
        slot.map(|slot| vec![Operand::Imm(bytecode_imm(slot))])
            .unwrap_or_default(),
    )
}

fn append_cmd(ctx: &mut CodegenCtx, args: &[String]) -> bool {
    let Some(variable) = args.first() else {
        return false;
    };
    if args.len() > 2
        && compiled_append_local_name(ctx, variable)
            .is_none_or(|name| split_array_ref(&name).is_some())
    {
        return false;
    }
    let address = emit_append_address(ctx, variable);
    let values = &args[1..];
    if values.is_empty() {
        let (op, operands) = match address {
            AppendAddress::Scalar(slot) => (
                if slot < 256 {
                    Op::LOAD_SCALAR1
                } else {
                    Op::LOAD_SCALAR4
                },
                vec![Operand::Imm(bytecode_imm(slot))],
            ),
            AppendAddress::Array(slot) => (
                if slot < 256 {
                    Op::LOAD_ARRAY1
                } else {
                    Op::LOAD_ARRAY4
                },
                vec![Operand::Imm(bytecode_imm(slot))],
            ),
            AppendAddress::ArrayStack => (Op::LOAD_ARRAY_STK, vec![]),
            AppendAddress::Stack => (Op::LOAD_STK, vec![]),
        };
        ctx.emit(op, operands);
        ctx.emit(Op::POP, vec![]);
        return true;
    }
    for (index, value) in values.iter().enumerate() {
        ctx.emit_word_arg(index + 1, value);
    }
    let (op, operands) = append_single_op(address, false);
    if values.len() > 1 {
        // The registry admits this protocol only for a direct scalar local.
        debug_assert!(matches!(address, AppendAddress::Scalar(_)));
        ctx.emit(Op::REVERSE, vec![Operand::Imm(bytecode_imm(values.len()))]);
    }
    for _ in values {
        ctx.emit(op, operands.clone());
        ctx.emit(Op::POP, vec![]);
    }
    true
}

fn append_list_op(address: AppendAddress) -> (Op, Vec<Operand>) {
    match address {
        AppendAddress::Scalar(slot) => (Op::LAPPEND_LIST, vec![Operand::Imm(bytecode_imm(slot))]),
        AppendAddress::Array(slot) => (
            Op::LAPPEND_LIST_ARRAY,
            vec![Operand::Imm(bytecode_imm(slot))],
        ),
        AppendAddress::ArrayStack => (Op::LAPPEND_LIST_ARRAY_STK, vec![]),
        AppendAddress::Stack => (Op::LAPPEND_LIST_STK, vec![]),
    }
}

pub(crate) fn emit_lappend_values(
    ctx: &mut CodegenCtx,
    variable: &str,
    values: &[(String, bool, bool)],
) {
    let address = emit_append_address(ctx, variable);
    let script_frame = !ctx.compiles_locals();
    let single = values.len() == 1 && !values[0].2 && !script_frame;
    if single {
        ctx.emit_word_arg(1, &values[0].0);
    } else {
        ctx.emit_native_argument_list(
            values
                .iter()
                .map(|(word, braced, expanded)| (word.as_str(), *braced, *expanded)),
            1,
            0,
        );
        if values.len() == 1 && values[0].2 {
            // C Tcl 9.1 drops the expanded list's string representation before
            // the selected variable operation can retain its object.
            ctx.emit(
                Op::LIST_RANGE_IMM,
                vec![Operand::Imm(0), Operand::Imm(INDEX_END)],
            );
        }
    }
    let (op, operands) = if single {
        append_single_op(address, true)
    } else {
        append_list_op(address)
    };
    ctx.emit(op, operands);
}

pub(crate) fn try_expanded_lappend(
    ctx: &mut CodegenCtx,
    command: &str,
    arguments: &[(String, bool, bool)],
) -> bool {
    let Some((variable, _, false)) = arguments.first() else {
        return false;
    };
    let words = arguments
        .iter()
        .map(|(word, _, _)| word.clone())
        .collect::<Vec<_>>();
    let Some((CodegenHookId::Lappend, binding)) = resolved_codegen_hook(ctx, command, &words)
    else {
        return false;
    };
    emit_lappend_values(ctx, variable, &arguments[1..]);
    ctx.require_site_binding(&binding);
    true
}

fn lappend_cmd(ctx: &mut CodegenCtx, args: &[String]) -> bool {
    let Some(variable) = args.first() else {
        return false;
    };
    let values = args[1..]
        .iter()
        .enumerate()
        .map(|(index, value)| {
            (
                value.clone(),
                ctx.cmd_arg_braced.get(index + 1).copied().unwrap_or(false),
                false,
            )
        })
        .collect::<Vec<_>>();
    emit_lappend_values(ctx, variable, &values);
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
    if !ctx.compiles_locals() {
        return false;
    }
    let Some(protocol) = ctx
        .native_hook_dialect()
        .and_then(tcl_registry::InvocationDialect::unset_option_protocol)
    else {
        return false;
    };
    let options = protocol
        .parse_known_source(args.len(), |index| {
            Ok::<_, std::convert::Infallible>(args[index].as_bytes())
        })
        .expect("original source arguments are available");
    let flags = i32::from(options.complain);
    let names = &args[options.names_from..];
    if names.is_empty() {
        return false;
    }
    for name in names {
        if !is_qualified(name) && !name.starts_with('$') && !name.starts_with('[') {
            if let Some((base, key)) = split_array_ref(name) {
                let Some(slot) = ctx.command_variable_slot(base.as_bytes()) else {
                    return false;
                };
                ctx.push_array_key(key);
                ctx.emit_comment(
                    Op::UNSET_ARRAY,
                    vec![Operand::Imm(flags), Operand::Imm(bytecode_imm(slot))],
                    &format!("var \"{base}\""),
                );
            } else {
                let Some(slot) = ctx.command_variable_slot(name.as_bytes()) else {
                    return false;
                };
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

/// Emit a retained tailcall namespace and original argv. The registry owns
/// release-specific namespace capture and list-versus-stack selection.
fn tailcall_cmd(ctx: &mut CodegenCtx, args: &[String]) -> bool {
    let words = args
        .iter()
        .enumerate()
        .map(|(index, argument)| {
            (
                argument.as_str(),
                ctx.cmd_arg_braced.get(index).copied().unwrap_or(false),
                false,
            )
        })
        .collect::<Vec<_>>();
    if !emit_tailcall(ctx, &words) {
        return false;
    }
    ctx.emit(Op::POP, vec![]);
    true
}

fn emit_tailcall(ctx: &mut CodegenCtx, words: &[(&str, bool, bool)]) -> bool {
    use tcl_registry::native_compilation::NativeTailcallStack;
    let Some(protocol) = ctx.native_hook_dialect().and_then(|dialect| {
        NativeTailcallStack::for_invocation(dialect, words.len(), words.iter().any(|word| word.2))
    }) else {
        return false;
    };
    match protocol {
        NativeTailcallStack::NamespacePrefixedList => {
            ctx.emit(Op::CURRENT_NAMESPACE, vec![]);
            ctx.emit_native_argument_list(words.iter().copied(), 0, 1);
            ctx.emit(Op::TAILCALL_LIST, vec![]);
        }
        NativeTailcallStack::NamespaceBeforeArguments
        | NativeTailcallStack::NamespaceAfterArguments => {
            if protocol == NativeTailcallStack::NamespaceBeforeArguments {
                ctx.emit(Op::CURRENT_NAMESPACE, vec![]);
            }
            for (index, (word, _, _)) in words.iter().enumerate() {
                ctx.emit_word_arg(index, word);
            }
            if protocol == NativeTailcallStack::NamespaceAfterArguments {
                ctx.emit(Op::CURRENT_NAMESPACE, vec![]);
                ctx.emit(
                    Op::REVERSE,
                    vec![Operand::Imm(bytecode_imm(words.len() + 1))],
                );
                if words.len() > 1 {
                    ctx.emit(Op::REVERSE, vec![Operand::Imm(bytecode_imm(words.len()))]);
                }
            }
            ctx.emit(
                if protocol == NativeTailcallStack::NamespaceBeforeArguments {
                    Op::TAILCALL4
                } else {
                    Op::TAILCALL
                },
                vec![Operand::Imm(bytecode_imm(words.len() + 1))],
            );
        }
    }
    true
}

/// Value-position bridge for admitted statement hooks. Original operand
/// quoting and private-worker layout share the ordinary statement owner.
pub(in crate::codegen) fn try_value_bytecoded(
    ctx: &mut CodegenCtx,
    command: &str,
    args: &[(String, bool)],
) -> bool {
    ctx.with_native_hook_operands(command, args, |ctx, logical| {
        try_value_bytecoded_in_layout(ctx, command, logical)
    })
}

fn try_value_bytecoded_in_layout(
    ctx: &mut CodegenCtx,
    command: &str,
    args: &[(String, bool)],
) -> bool {
    // The same authenticated primitive leaves one value in statement, value,
    // catch and try-handler contexts; each caller owns only its result use.
    if let Some(
        hook @ (tcl_registry::hooks::InlineCodegenHookId::NamespaceOrigin
        | tcl_registry::hooks::InlineCodegenHookId::NamespaceCode),
    ) = ctx.inline_cmd_subst_hook_candidate(command, args)
    {
        if ctx.inline_cmd_subst_hook(command, args) != Some(hook) {
            return false;
        }
        let tokens = ctx.invocation_tokens.clone();
        return match hook {
            tcl_registry::hooks::InlineCodegenHookId::NamespaceOrigin => {
                ctx.emit_inline_namespace_origin(tokens.as_deref())
            }
            _ => ctx.emit_inline_namespace_code(tokens.as_deref()),
        };
    }
    let values = args
        .iter()
        .map(|(argument, _)| argument.clone())
        .collect::<Vec<_>>();
    let Some((
        hook @ (CodegenHookId::Lset
        | CodegenHookId::Tailcall
        | CodegenHookId::Lappend
        | CodegenHookId::Dict),
        binding,
    )) = resolved_codegen_hook(ctx, command, &values)
    else {
        return false;
    };
    if hook == CodegenHookId::Lappend {
        // This result protocol belongs to the original compiler admission.
        // A generic no-value handler can resolve a new cell after a read trace;
        // the compiled operation must keep its selected receiver instead.
        if ctx
            .invocation_tokens
            .as_deref()
            .and_then(|tokens| tokens.source_binding.as_ref())
            .and_then(|binding| binding.admitted_inline_invocation())
            .is_none()
        {
            return false;
        }
        let Some((variable, _)) = args.first() else {
            return false;
        };
        let additions = args[1..]
            .iter()
            .map(|(word, braced)| (word.clone(), *braced, false))
            .collect::<Vec<_>>();
        emit_lappend_values(ctx, variable, &additions);
        ctx.require_site_binding(&binding);
        ctx.used_inline_cmd_subst = true;
        return true;
    }
    let mut generic = false;
    if !dispatch_codegen_hook(hook, ctx, &values, &mut generic) {
        return false;
    }
    debug_assert_eq!(
        ctx.instructions.last().map(|instruction| instruction.op),
        Some(Op::POP)
    );
    ctx.instructions.pop();
    ctx.require_site_binding(&binding);
    ctx.used_inline_cmd_subst = true;
    true
}

/// Expanded tailcall retains the actual compiler's namespace-prefixed list.
pub(in crate::codegen) fn try_expanded_tailcall(
    ctx: &mut CodegenCtx,
    command: &str,
    args: &[(String, bool, bool)],
) -> bool {
    let values = args
        .iter()
        .map(|argument| argument.0.clone())
        .collect::<Vec<_>>();
    let Some((CodegenHookId::Tailcall, binding)) = resolved_codegen_hook(ctx, command, &values)
    else {
        return false;
    };
    let words = args
        .iter()
        .map(|(word, braced, expanded)| (word.as_str(), *braced, *expanded))
        .collect::<Vec<_>>();
    if !emit_tailcall(ctx, &words) {
        return false;
    }
    ctx.require_site_binding(&binding);
    ctx.used_inline_cmd_subst = true;
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
    if !ctx.compiles_locals() || args.is_empty() {
        return false;
    }
    if !args.iter().all(|n| is_compilable_scalar_local(ctx, n)) {
        return false;
    }
    ctx.push_lit("::");
    for name in args {
        let Some(slot) = ctx.command_variable_slot(name.as_bytes()) else {
            return false;
        };
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
    if !ctx.compiles_locals() || args.len() < 2 {
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
    // Every `local` (the odd-indexed words) must be a simple compiled *scalar*
    // local. The `other` side may legally be an element (`upvar 0 arr(k) v`).
    for pair in pairs.as_chunks::<2>().0 {
        if !is_compilable_scalar_local(ctx, &pair[1]) {
            return false;
        }
    }
    ctx.push_lit(level);
    for pair in pairs.as_chunks::<2>().0 {
        let (other, local) = (&pair[0], &pair[1]);
        let Some(slot) = ctx.command_variable_slot(local.as_bytes()) else {
            return false;
        };
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
    use tcl_dialect::model::{Family, SurfaceQuery};

    use super::*;
    use tcl_registry::CommandRegistry;

    #[test]
    fn private_ensemble_rewrites_require_a_native_implementation() {
        for (environment, namespace_rewrite, dict_rewrite) in [
            ("tcl8.4", false, false),
            ("tcl8.5", false, true),
            ("tcl8.6", true, true),
            ("tcl9.0", true, true),
            ("tcl9.1", true, true),
            ("jim", false, false),
        ] {
            let profile =
                tcl_registry::model::ingress::resolve_environment(environment).unit_profile();
            let registry = CommandRegistry::build_default().project_for_profile(profile);
            let mut ctx = CodegenCtx::new(false, &[], &registry);
            assert_eq!(
                namespace_cmd(
                    &mut ctx,
                    &["eval".into(), "::N".into(), "set x 1".into()],
                    &mut false
                ),
                namespace_rewrite,
                "{environment}"
            );
            let mut ctx = CodegenCtx::new(false, &[], &registry);
            assert_eq!(
                dict(
                    &mut ctx,
                    &["set".into(), "d".into(), "k".into(), "v".into()]
                ),
                dict_rewrite,
                "{environment}"
            );
        }
    }

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
        let registry = CommandRegistry::build_default()
            .project_for_profile(tcl_dialect::DialectProfile::find("tcl8.6").unwrap());
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

    // Lrange / linsert / lset.

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
        let args = vec!["lst".to_string()];
        let mut used = false;
        assert!(!try_bytecoded(&mut ctx, "lset", &args, &mut used));
    }

    #[test]
    fn lset_without_indices_replaces_value_after_loading_the_original_cell() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(true, &[], &registry);
        let mut used = false;
        assert!(try_bytecoded(
            &mut ctx,
            "lset",
            &["lst".into(), "new".into()],
            &mut used
        ));
        assert!(ctx.instructions.iter().any(|instruction| {
            instruction.op == Op::LSET_FLAT && instruction.operands == [Operand::Imm(2)]
        }));
        assert!(
            ctx.instructions
                .iter()
                .any(|instruction| instruction.op == Op::LOAD_SCALAR1)
        );
        assert!(
            ctx.instructions
                .iter()
                .any(|instruction| instruction.op == Op::STORE_SCALAR1)
        );
    }

    #[test]
    fn lset_array_reuses_the_captured_key_for_post_argv_load_and_store() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(true, &[], &registry);
        let mut used = false;
        assert!(try_bytecoded(
            &mut ctx,
            "lset",
            &["a(k)".into(), "0".into(), "new".into()],
            &mut used
        ));
        let ops = ctx
            .instructions
            .iter()
            .map(|instruction| instruction.op)
            .collect::<Vec<_>>();
        assert_eq!(
            ops,
            [
                Op::PUSH1,
                Op::PUSH1,
                Op::PUSH1,
                Op::OVER,
                Op::LOAD_ARRAY1,
                Op::LSET_LIST,
                Op::STORE_ARRAY1,
                Op::POP
            ]
        );
        assert_eq!(ctx.literals.entries()[0], "k");
    }

    // Dict subcommands.

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
        let registry = CommandRegistry::build_default()
            .project_for_profile(tcl_dialect::DialectProfile::find("tcl8.6").unwrap());
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
        let registry = CommandRegistry::build_default()
            .project_for_profile(tcl_dialect::DialectProfile::find("tcl8.6").unwrap());
        let mut ctx = CodegenCtx::new(true, &[], &registry);
        let args = vec!["set".into(), "::global::d".into(), "k".into(), "v".into()];
        let mut used = false;
        assert!(try_bytecoded(&mut ctx, "dict", &args, &mut used));
        let ops: Vec<Op> = ctx.instructions.iter().map(|i| i.op).collect();
        assert!(ops.contains(&Op::INVOKE_REPLACE));
        assert!(!ops.contains(&Op::DICT_SET));
    }

    // Append / lappend statement-position specialisations.

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
    fn append_toplevel_uses_the_evaluated_stack_name() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(false, &[], &registry);
        let args = vec!["x".into(), "a".into()];
        let mut used = false;
        assert!(try_bytecoded(&mut ctx, "append", &args, &mut used));
        assert!(ctx.instructions.iter().any(|i| i.op == Op::APPEND_STK));
        assert!(ctx.lvt.is_empty());
    }

    #[test]
    fn append_qualified_and_dynamic_names_use_the_stack_once() {
        let registry = CommandRegistry::build_default();
        for name in ["::g::x", "$dyn"] {
            let mut ctx = CodegenCtx::new(true, &[], &registry);
            let mut used = false;
            assert!(try_bytecoded(
                &mut ctx,
                "append",
                &[name.into(), "a".into()],
                &mut used
            ));
            assert_eq!(
                ctx.instructions
                    .iter()
                    .filter(|i| i.op == Op::APPEND_STK)
                    .count(),
                1
            );
            let mut multi = CodegenCtx::new(true, &[], &registry);
            assert!(!try_bytecoded(
                &mut multi,
                "append",
                &[name.into(), "a".into(), "b".into()],
                &mut used
            ));
            assert!(multi.instructions.is_empty());
        }
        let mut ctx = CodegenCtx::new(true, &[], &registry);
        let mut used = false;
        assert!(try_bytecoded(&mut ctx, "append", &["x".into()], &mut used));
        assert!(ctx.instructions.iter().any(|i| i.op == Op::LOAD_SCALAR1));
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
    fn lappend_multi_array_captures_one_key_and_builds_one_list() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(true, &[], &registry);
        let args = vec!["arr(k)".into(), "a".into(), "b".into()];
        let mut used = false;
        assert!(try_bytecoded(&mut ctx, "lappend", &args, &mut used));
        let ops: Vec<Op> = ctx.instructions.iter().map(|i| i.op).collect();
        assert_eq!(
            ops,
            vec![
                Op::PUSH1,
                Op::PUSH1,
                Op::PUSH1,
                Op::LIST,
                Op::LAPPEND_LIST_ARRAY,
                Op::POP
            ]
        );
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

    // Unset statement-position specialisation.

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
    fn unset_unknown_and_repeated_options_remain_variable_operands() {
        let registry = CommandRegistry::build_default();
        for (args, first_name, flag) in [
            (vec!["-bad".into(), "x".into()], "-bad", 1),
            (
                vec!["-nocomplain".into(), "-nocomplain".into(), "x".into()],
                "-nocomplain",
                0,
            ),
        ] {
            let mut ctx = CodegenCtx::new(true, &[], &registry);
            let mut used = false;
            assert!(try_bytecoded(&mut ctx, "unset", &args, &mut used));
            let targets: Vec<_> = ctx
                .instructions
                .iter()
                .filter(|instruction| instruction.op == Op::UNSET_SCALAR)
                .collect();
            assert_eq!(targets.len(), 2);
            assert_eq!(targets[0].operands[0], Operand::Imm(flag));
            assert_eq!(
                targets[0].operands[1],
                Operand::Imm(bytecode_imm(ctx.lvt.find(first_name).unwrap()))
            );
        }
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

    // Tailcall statement-position specialisation.

    #[test]
    fn tailcall_captures_namespace_after_legacy_arguments() {
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
                Op::CURRENT_NAMESPACE,
                Op::REVERSE,
                Op::REVERSE,
                Op::TAILCALL,
                Op::POP
            ]
        );
        // Operand counts the retained namespace plus the three arguments.
        let tc = ctx
            .instructions
            .iter()
            .find(|i| i.op == Op::TAILCALL)
            .unwrap();
        assert_eq!(tc.operands[0], Operand::Imm(4));
        assert_eq!(ctx.literals.entries()[0], "foo");
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

    // Concat statement-position specialisation.

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

    // Global / upvar statement-position specialisation.

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
        // `global` scans `::` even inside an array-looking index. The generic
        // command owns that unusual tail rule; NSUPVAR's scalar LVT form does
        // not represent it.
        assert!(!try_bytecoded(
            &mut proc_ctx,
            "global",
            &["v(x::y)".into()],
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
            .resolve_call(
                "HTTP::header",
                &["names"],
                Some(SurfaceQuery::any_release(Family::F5Irules)),
            )
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
                .resolve_call(
                    "HTTP::header",
                    &["names"],
                    Some(SurfaceQuery::any_release(Family::F5Irules)),
                )
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
            .resolve_call(
                "HTTP::header",
                &["names"],
                Some(SurfaceQuery::any_release(Family::F5Irules)),
            )
            .expect("HTTP::header resolves once iRules is loaded");

        // Wire it through CodegenCtx: the ctx's registry field is
        // what try_bytecoded reads.
        let ctx = CodegenCtx::new(true, &[], &irules);
        assert!(std::ptr::eq(ctx.registry, &raw const irules));
        assert_eq!(resolved.spec.name, "HTTP::header");
    }
}
