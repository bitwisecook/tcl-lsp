//! Per-command codegen hook dispatch.
//!
//! Commands with compiled bytecode forms (beyond what
//! `cmd_subst::emit_inline_cmd_subst` already handles) are routed
//! through this dispatcher. ARCH2 made hook selection
//! registry-driven; ARCH5 finished the threading by reading the
//! active [`CommandRegistry`] from `ctx.registry` so dialect-loaded
//! specs (iRules, Tk, EDA) drive codegen-hook resolution. The
//! compiler still owns the per-variant emitter; the registry
//! decides which variant applies.
//!
//! Ported from `core/compiler/codegen/_bytecoded.py` (C21).

use tcl_registry::dialects::DialectSet;
use tcl_registry::hooks::CodegenHookId;

use super::super::values::is_qualified;
use super::super::CodegenCtx;
use super::super::Op;
use super::super::Operand;
use super::super::{bytecode_imm, parse_tcl_index, INDEX_END};

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
    let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
    let Some(resolved) = ctx
        .registry
        .resolve_call(cmd, &arg_refs, DialectSet::empty())
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
    }
}

// ── list ──────────────────────────────────────────────────────────

/// `llength $list` → `emit_value list; LIST_LENGTH; POP`.
fn llength(ctx: &mut CodegenCtx, args: &[String]) -> bool {
    if args.len() != 1 {
        return false;
    }
    ctx.emit_value_interpolated(&args[0]);
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
    ctx.emit_value_interpolated(&args[0]);
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
    ctx.emit_value_interpolated(&args[0]);
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
    ctx.emit_value_interpolated(&args[0]);
    for a in &args[1..] {
        ctx.emit_value_interpolated(a);
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
        // Non-proc or qualified: stack-based lsetList with OVER
        // to duplicate the variable reference onto the top of
        // stack so loadStk finds it.
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
        ctx.emit(Op::LSET_LIST, vec![]);
        ctx.emit(Op::STORE_STK, vec![]);
    }
    ctx.emit(Op::POP, vec![]);
    true
}

// ── dict ──────────────────────────────────────────────────────────

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
    if !ctx.is_proc || args.len() < 3 {
        return false;
    }
    let sub = args[0].as_str();
    let rest = &args[1..];
    let var_name = &rest[0];
    if is_qualified(var_name) {
        return false;
    }

    match sub {
        "set" if rest.len() >= 3 => {
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
        "unset" if rest.len() >= 2 => {
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
        "incr" if matches!(rest.len(), 2 | 3) => {
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
        "append" if rest.len() == 3 => {
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
        "lappend" if rest.len() == 3 => {
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
        _ => false,
    }
}

// ── array ─────────────────────────────────────────────────────────

/// `array names $arr ...` / `array size $arr` in non-proc context:
/// invoke the fully-qualified `::tcl::array::<sub>` form rather than
/// the generic `array` dispatcher.
fn array(ctx: &mut CodegenCtx, args: &[String], used_generic_invoke: &mut bool) -> bool {
    if ctx.is_proc || args.len() < 2 {
        return false;
    }
    let sub = args[0].as_str();
    let rest = &args[1..];
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

#[cfg(test)]
mod tests {
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
        assert!(ctx
            .literals
            .entries()
            .iter()
            .any(|e| e == "::tcl::array::names"));
    }

    #[test]
    fn array_in_proc_context_rejects() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(true, &[], &registry);
        let args = vec!["names".into(), "${arr}".into()];
        let mut used = false;
        assert!(!try_bytecoded(&mut ctx, "array", &args, &mut used));
    }

    // -- C21 follow-ups: lrange / linsert / lset --

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

    // -- C21e4: dict subcommands --

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
    fn dict_in_non_proc_context_rejects() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(false, &[], &registry);
        let args = vec!["set".into(), "d".into(), "k".into(), "v".into()];
        let mut used = false;
        assert!(!try_bytecoded(&mut ctx, "dict", &args, &mut used));
    }

    #[test]
    fn dict_with_qualified_name_rejects() {
        let registry = CommandRegistry::build_default();
        let mut ctx = CodegenCtx::new(true, &[], &registry);
        let args = vec!["set".into(), "::global::d".into(), "k".into(), "v".into()];
        let mut used = false;
        assert!(!try_bytecoded(&mut ctx, "dict", &args, &mut used));
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
                tcl_registry::dialects::DialectSet::TCL86,
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
                tcl_registry::dialects::DialectSet::IRULES,
            )
            .expect("HTTP::header resolves under iRules");
        assert_eq!(resolved.spec.name, "HTTP::header");
    }

    /// ARCH5 — `try_bytecoded` reads its registry from `ctx.registry`,
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
        assert!(default
            .resolve_call(
                "HTTP::header",
                &["names"],
                tcl_registry::dialects::DialectSet::IRULES,
            )
            .is_none());

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
                tcl_registry::dialects::DialectSet::IRULES,
            )
            .expect("HTTP::header resolves once iRules is loaded");

        // Wire it through CodegenCtx: the ctx's registry field is
        // what try_bytecoded reads.
        let ctx = CodegenCtx::new(true, &[], &irules);
        assert!(std::ptr::eq(ctx.registry, &irules));
        assert_eq!(resolved.spec.name, "HTTP::header");
    }
}
