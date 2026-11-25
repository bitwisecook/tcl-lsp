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

use tcl_dialect::model::{Family, SurfaceQuery};

use super::*;
use tcl_registry::CommandRegistry;

#[test]
fn concat_preserves_non_ascii_boundary_space() {
    let registry = CommandRegistry::build_default();
    let mut ctx = CodegenCtx::new(true, &[], &registry);
    assert!(concat_cmd(&mut ctx, &["  a  ".into(), "\u{a0}b  ".into()]));
    assert!(
        ctx.literals
            .entries()
            .iter()
            .any(|entry| entry.bytes() == "a \u{a0}b".as_bytes())
    );
}

#[test]
fn variable_declines_a_compiled_local_for_a_tail_ending_in_a_parenthesis() {
    // tclsh8.6, 9.0 and 9.1, `tcl::unsupported::disassemble proc v`:
    //   proc v {} {variable a(1)}     -> invokeStk1 (no local)
    //   proc v {} {variable b) 2}     -> invokeStk1 (no local)
    //   proc v {} {variable a(1)x 5}  -> variable %v0  # var "a(1)x"
    //   proc v {} {variable ::n::c}   -> variable %v0  # var "c"
    let registry = CommandRegistry::build_default();
    for (name, local) in [
        ("a(1)", None),
        ("b)", None),
        ("a(1)x", Some("a(1)x")),
        ("::n::c", Some("c")),
    ] {
        let mut ctx = CodegenCtx::new(true, &[], &registry);
        assert!(!variable_locals(&mut ctx, &[name.into()]));
        match local {
            Some(local) => assert!(ctx.lvt.find(local).is_some(), "{name}"),
            None => assert!(ctx.lvt.is_empty(), "{name}"),
        }
    }
}

#[test]
fn standalone_local_name_models_preserve_literal_sigils_and_empty_data() {
    let registry = CommandRegistry::build_default();
    for name in ["$literal", "${literal}", "[literal]", ""] {
        let mut global = CodegenCtx::new(true, &[], &registry);
        global.cmd_arg_braced = vec![true];
        assert!(global_cmd(&mut global, &[name.into()]));
        assert!(global.lvt.find(name).is_some(), "{name}");
        assert!(
            global
                .instructions
                .iter()
                .any(|instruction| instruction.op == Op::NSUPVAR)
        );

        let mut unset = CodegenCtx::new(true, &[], &registry);
        unset.cmd_arg_braced = vec![true];
        assert!(unset_cmd(&mut unset, &[name.into()]));
        assert!(unset.lvt.find(name).is_some(), "{name}");
        assert!(
            unset
                .instructions
                .iter()
                .any(|instruction| instruction.op == Op::UNSET_SCALAR)
        );
    }
    let mut append = CodegenCtx::new(true, &[], &registry);
    append.cmd_arg_braced = vec![true, false];
    assert!(append_cmd(
        &mut append,
        &["array($index)".into(), "value".into()]
    ));
    assert!(append.lvt.find("array").is_some());
    assert!(append.lvt.find("index").is_none());
    assert!(
        append
            .instructions
            .iter()
            .any(|instruction| instruction.op == Op::APPEND_ARRAY1)
    );
    assert!(
        append
            .literals
            .entries()
            .iter()
            .any(|entry| entry.bytes() == b"$index")
    );

    let mut upvar = CodegenCtx::new(true, &[], &registry);
    upvar.cmd_arg_braced = vec![false, true, true];
    assert!(upvar_cmd(
        &mut upvar,
        &["0".into(), "$remote".into(), "$local".into()]
    ));
    assert!(upvar.lvt.find("$local").is_some());
    assert!(upvar.lvt.find("remote").is_none());
    assert!(
        upvar
            .literals
            .entries()
            .iter()
            .any(|entry| entry.bytes() == b"$remote")
    );
}

#[test]
fn standalone_computed_name_models_do_not_claim_literal_local_slots() {
    let registry = CommandRegistry::build_default();
    for name in ["prefix$computed", "array($index)"] {
        let mut append = CodegenCtx::new(true, &[], &registry);
        assert!(append_cmd(&mut append, &[name.into(), "value".into()]));
        assert!(append.lvt.find(name).is_none());
        assert!(append.lvt.find("array").is_none());
        assert!(
            append
                .instructions
                .iter()
                .any(|instruction| instruction.op == Op::APPEND_STK)
        );

        let mut unset = CodegenCtx::new(true, &[], &registry);
        assert!(unset_cmd(&mut unset, &[name.into()]));
        assert!(unset.lvt.find(name).is_none());
        assert!(unset.lvt.find("array").is_none());
        assert!(
            unset
                .instructions
                .iter()
                .any(|instruction| instruction.op == Op::UNSET_STK)
        );
    }
    let mut global = CodegenCtx::new(true, &[], &registry);
    assert!(!global_cmd(
        &mut global,
        &["first".into(), "prefix$computed".into()]
    ));
    assert_eq!(global.instructions, [] as [tcl_bytecode::Instruction; 0]);
    assert!(global.lvt.is_empty());
    let mut upvar = CodegenCtx::new(true, &[], &registry);
    assert!(!upvar_cmd(
        &mut upvar,
        &["0".into(), "remote".into(), "prefix$computed".into()]
    ));
    assert_eq!(upvar.instructions, [] as [tcl_bytecode::Instruction; 0]);
    assert!(upvar.lvt.is_empty());
}

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
        let profile = tcl_registry::model::ingress::resolve_environment(environment).unit_profile();
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

#[test]
fn dict_ensemble_preserves_braced_keys_and_values() {
    let registry = CommandRegistry::build_default()
        .project_for_profile(tcl_dialect::DialectProfile::find("tcl8.6").unwrap());
    let mut ctx = CodegenCtx::new(false, &[], &registry);
    let args = vec![
        "set".into(),
        "d".into(),
        "$key".into(),
        "[error literal]".into(),
    ];
    ctx.cmd_arg_braced = vec![false, false, true, true];
    let mut used = false;
    assert!(try_bytecoded(&mut ctx, "dict", &args, &mut used));
    assert!(ctx.literals.entries().iter().any(|value| value == "$key"));
    assert!(
        ctx.literals
            .entries()
            .iter()
            .any(|value| value == "[error literal]")
    );
    assert!(ctx.instructions.iter().all(|instruction| {
        matches!(
            instruction.op,
            Op::PUSH1 | Op::PUSH4 | Op::INVOKE_REPLACE | Op::POP
        )
    }));
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
        assert_eq!(multi.instructions, [] as [tcl_bytecode::Instruction; 0]);
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
fn standalone_upvar_uses_the_shared_compiler_probe_before_emission() {
    let registry = CommandRegistry::build_default();
    let mut ctx = CodegenCtx::new(true, &[], &registry);
    assert!(upvar_cmd(
        &mut ctx,
        &["+0".into(), "other".into(), "local".into()]
    ));
    assert_eq!(ctx.literals.entries()[0], "+0");
    assert!(
        ctx.instructions
            .iter()
            .any(|instruction| instruction.op == Op::UPVAR)
    );
    // A first word that is no level with two words after it is left to the
    // invoked command: tclsh 8.5.19-9.1.0 disassemble `upvar abc other local`
    // as invokeStk (`upvar 12 other local` compiles with the level 12).
    let mut unavailable = CodegenCtx::new(true, &[], &registry);
    assert!(!upvar_cmd(
        &mut unavailable,
        &["abc".into(), "other".into(), "local".into()]
    ));
    assert_eq!(
        unavailable.instructions,
        [] as [tcl_bytecode::Instruction; 0]
    );
    assert!(unavailable.lvt.is_empty());
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
#[test]
fn unset_literal_operand_keeps_name_and_index_data_in_compiler_slots() {
    use tcl_syntax::native_variable_words::NativeVariableWordOperand;
    let registry = CommandRegistry::build_default();
    for original in [
        b"$literal".as_slice(),
        b"[literal]".as_slice(),
        b"".as_slice(),
    ] {
        let mut ctx = CodegenCtx::new(true, &[], &registry);
        emit_original_unset_operand(
            &mut ctx,
            0,
            "unused",
            NativeVariableWordOperand::Literal {
                name: original.to_vec(),
                name_span: tcl_lexer::Span::new(0, 0),
                index: None,
            },
            1,
        );
        let slot = ctx
            .lvt
            .find_native(ctx.compiled_variable_protocol.unwrap(), original)
            .unwrap();
        assert_eq!(ctx.instructions.last().unwrap().op, Op::UNSET_SCALAR);
        assert_eq!(
            ctx.instructions.last().unwrap().operands,
            vec![Operand::Imm(1), Operand::Imm(bytecode_imm(slot))]
        );
    }
    let mut ctx = CodegenCtx::new(true, &[], &registry);
    emit_original_unset_operand(
        &mut ctx,
        0,
        "unused",
        NativeVariableWordOperand::Literal {
            name: b"array".to_vec(),
            name_span: tcl_lexer::Span::new(0, 0),
            index: Some(b"$index".to_vec()),
        },
        0,
    );
    assert_eq!(ctx.instructions.last().unwrap().op, Op::UNSET_ARRAY);
    assert!(
        ctx.literals
            .entries()
            .iter()
            .any(|literal| literal.bytes() == b"$index")
    );
    assert!(
        ctx.instructions
            .iter()
            .all(|instruction| !matches!(instruction.op, Op::LOAD_SCALAR1 | Op::LOAD_SCALAR4))
    );
}
#[test]
fn unset_source_without_original_compiler_ownership_emits_no_partial_mutation() {
    let registry = CommandRegistry::build_default();
    let module = tcl_compiler::lowering::lower_to_ir("unset {$literal} {array($index)}", &registry);
    let tokens = module
        .top_level
        .statements
        .iter()
        .find_map(|statement| match statement {
            tcl_compiler::ir::Statement::Call { tokens, .. }
            | tcl_compiler::ir::Statement::Barrier { tokens, .. } => tokens.clone(),
            _ => None,
        })
        .expect("actual original source invocation");
    assert_eq!(tokens.words().len(), 3);
    let mut ctx = CodegenCtx::new(true, &[], &registry);
    ctx.set_source_image(module.source.clone());
    ctx.invocation_tokens = Some(Box::new(tokens));
    assert!(!unset_cmd(
        &mut ctx,
        &["$literal".to_owned(), "array($index)".to_owned()]
    ));
    assert_eq!(ctx.instructions, [] as [tcl_bytecode::Instruction; 0]);
}

#[test]
fn link_local_operand_uses_typed_literal_data_without_reference_prefixes() {
    use tcl_syntax::native_variable_words::NativeVariableWordOperand;
    for name in ["$literal", "${literal}", "[literal]", ""] {
        let operand = NativeVariableWordOperand::Literal {
            name: name.as_bytes().to_vec(),
            name_span: tcl_lexer::Span::new(0, 0),
            index: None,
        };
        assert_eq!(
            literal_link_local_name(operand).as_deref(),
            Some(name.as_bytes())
        );
    }
    assert!(
        literal_link_local_name(NativeVariableWordOperand::Literal {
            name: b"array".to_vec(),
            name_span: tcl_lexer::Span::new(0, 0),
            index: Some(b"key".to_vec()),
        })
        .is_none()
    );
    assert!(
        literal_link_local_name(NativeVariableWordOperand::Literal {
            name: b"::qualified".to_vec(),
            name_span: tcl_lexer::Span::new(0, 0),
            index: None,
        })
        .is_none()
    );
    assert!(literal_link_local_name(NativeVariableWordOperand::DynamicWord).is_none());
}

#[test]
fn source_link_operands_without_original_compiler_ownership_emit_no_partial_links() {
    let registry = CommandRegistry::build_default();
    for source in ["global {$literal}", "upvar 0 {$other} {$literal}"] {
        let module = tcl_compiler::lowering::lower_to_ir(source, &registry);
        let statement = module.top_level.statements.first().unwrap();
        let tokens = statement.tokens().unwrap();
        let args = tokens.argv_texts[1..].to_vec();
        let mut ctx = CodegenCtx::new(true, &[], &registry);
        ctx.set_source_image(module.source.clone());
        ctx.invocation_tokens = Some(Box::new(tokens.clone()));
        assert!(ctx.original_variable_operand(&tokens.words()[1]).is_none());
        assert!(!if source.starts_with("global") {
            global_cmd(&mut ctx, &args)
        } else {
            upvar_cmd(&mut ctx, &args)
        });
        assert_eq!(ctx.instructions, [] as [tcl_bytecode::Instruction; 0]);
    }
}

#[test]
fn captured_upvar_prefix_without_written_remote_words_declines_the_whole_layout() {
    let registry = CommandRegistry::build_default();
    let source = "interp alias {} bind {} upvar 0 {$remote}; bind local";
    let module = tcl_compiler::lowering::lower_to_ir(source, &registry);
    let call = module
        .top_level
        .statements
        .iter()
        .find(|statement| {
            statement
                .tokens()
                .is_some_and(|tokens| tokens.argv_texts.first().is_some_and(|head| head == "bind"))
        })
        .expect("actual original alias call");
    let tokens = call.tokens().unwrap();
    assert_eq!(tokens.words().len(), 2);
    let mut ctx = CodegenCtx::new(true, &[], &registry);
    ctx.set_source_image(module.source.clone());
    ctx.invocation_tokens = Some(Box::new(tokens.clone()));
    // Operand-layout model only: no Native alias compiler is asserted.
    ctx.native_hook_layout = Some(("upvar".to_owned(), 2));
    assert_eq!(ctx.original_hook_argument(2), Some(0));
    assert_eq!(ctx.original_hook_argument(1), None);
    assert!(ctx.original_variable_operand(&tokens.words()[1]).is_none());
    assert!(!upvar_cmd(
        &mut ctx,
        &["0".to_owned(), "$remote".to_owned(), "local".to_owned()]
    ));
    assert_eq!(ctx.instructions, [] as [tcl_bytecode::Instruction; 0]);
    assert!(ctx.lvt.is_empty());
}
