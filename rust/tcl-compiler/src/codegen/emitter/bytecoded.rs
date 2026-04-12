//! Per-command codegen hook dispatch.
//!
//! Commands with compiled bytecode forms (beyond what
//! `cmd_subst::emit_inline_cmd_subst` already handles) are routed
//! through this dispatcher. For the initial C21 landing we match on
//! command name directly — mirroring the Rust lowering-hook
//! dispatch pattern in `lowering_hooks::try_lower_hook`. A future
//! chunk may migrate this to an ID-based table that reads
//! `CommandSpec::codegen_hook` from the registry.
//!
//! Ported from `core/compiler/codegen/_bytecoded.py` (C21).

use super::super::CodegenCtx;
use super::super::Op;
use super::super::Operand;
use super::super::INDEX_END;

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
    match cmd {
        "lassign" => lassign(ctx, args),
        "llength" => llength(ctx, args),
        "array" => array(ctx, args, used_generic_invoke),
        _ => false,
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
    #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
    for (i, var) in var_names.iter().enumerate() {
        ctx.push_lit(var);
        ctx.emit(Op::OVER, vec![Operand::Imm(1)]);
        ctx.emit(Op::LIST_INDEX_IMM, vec![Operand::Imm(i as i32)]);
        ctx.emit(Op::STORE_STK, vec![]);
        ctx.emit(Op::POP, vec![]);
    }
    #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
    let n = var_names.len() as i32;
    ctx.emit(
        Op::LIST_RANGE_IMM,
        vec![Operand::Imm(n), Operand::Imm(INDEX_END)],
    );
    ctx.emit(Op::POP, vec![]);
    true
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
            #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
            let n_args = (1 + rest.len()) as i32;
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

    #[test]
    fn lassign_rejects_wrong_arity() {
        let mut ctx = CodegenCtx::new(false, &[]);
        let mut used = false;
        // 0 args
        assert!(!try_bytecoded(&mut ctx, "lassign", &[], &mut used));
        // only list, no vars
        let args = vec!["${lst}".to_string()];
        assert!(!try_bytecoded(&mut ctx, "lassign", &args, &mut used));
    }

    #[test]
    fn lassign_emits_expected_sequence() {
        let mut ctx = CodegenCtx::new(false, &[]);
        let args = vec![
            "${lst}".to_string(),
            "a".to_string(),
            "b".to_string(),
        ];
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
        let mut ctx = CodegenCtx::new(false, &[]);
        let args = vec!["${lst}".to_string()];
        let mut used = false;
        assert!(try_bytecoded(&mut ctx, "llength", &args, &mut used));
        let ops: Vec<Op> = ctx.instructions.iter().map(|i| i.op).collect();
        assert!(ops.contains(&Op::LIST_LENGTH));
    }

    #[test]
    fn llength_wrong_arity_rejects() {
        let mut ctx = CodegenCtx::new(false, &[]);
        let mut used = false;
        assert!(!try_bytecoded(&mut ctx, "llength", &[], &mut used));
        let args = vec!["a".into(), "b".into()];
        assert!(!try_bytecoded(&mut ctx, "llength", &args, &mut used));
    }

    #[test]
    fn array_names_emits_fq_invoke() {
        let mut ctx = CodegenCtx::new(false, &[]);
        let args = vec!["names".into(), "${arr}".into()];
        let mut used = false;
        assert!(try_bytecoded(&mut ctx, "array", &args, &mut used));
        assert!(used);
        // Should push the FQ name as a literal.
        assert!(ctx.literals.entries().iter().any(|e| e == "::tcl::array::names"));
    }

    #[test]
    fn array_in_proc_context_rejects() {
        let mut ctx = CodegenCtx::new(true, &[]);
        let args = vec!["names".into(), "${arr}".into()];
        let mut used = false;
        assert!(!try_bytecoded(&mut ctx, "array", &args, &mut used));
    }

    #[test]
    fn unknown_command_returns_false() {
        let mut ctx = CodegenCtx::new(false, &[]);
        let mut used = false;
        assert!(!try_bytecoded(&mut ctx, "foobar", &[], &mut used));
    }
}
