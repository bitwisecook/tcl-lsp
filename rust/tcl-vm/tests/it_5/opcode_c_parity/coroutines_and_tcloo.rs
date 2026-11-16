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

//! `yield`, `yieldToInvoke` and `coroName`, and the `TclOO` opcodes (`tclooIsObject`, `tclooClass`, `tclooSelf`, `tclooNext*`).

use super::*;

// `yield` / `yieldToInvoke` / `coroName`.

/// Run `body` as the compiled body of a global proc `p`.
///
/// The seam is the compiler's pre-compiled-body cache: `cmd_proc` takes the
/// module-supplied assembly for the first definition of a name. The test
/// compiler replaces that one body while preserving the ordinary compiler's
/// profile, registry, lexer, and plain-dispatch selection. That is the only way
/// to reach an opcode from *inside* a coroutine or a method call, which is where
/// the coroutine and `TclOO` opcodes live.
///
/// Do not seed this through [`Vm::run_module`]. A public module is deliberately
/// foreign provenance; once a compiler is installed, source-bearing foreign
/// procedures are recompiled through that service before reuse. These streams
/// instead need to be artifacts of the VM's current compiler generation.
pub(super) fn vm_with_seeded_proc(body: Asm) -> Vm {
    let mut vm = tcl_vm::test_support::native_vm();
    vm.set_compiler(Box::new(compiler::seeded_proc(body.build())));
    vm.eval_source(compiler::SEEDED_PROC_DEF)
        .expect("proc defines");
    vm
}

/// `coroName` pushes the empty string outside a coroutine (C
/// `INST_COROUTINE_NAME` pushes a fresh empty object when there is no
/// `corPtr`) and the running coroutine's fully-qualified command name inside
/// one.
#[test]
fn coro_name_outside_and_inside_a_coroutine() {
    let mut a = Asm::new();
    a.op(Op::CORO_NAME, &[]);
    let (_vm, c) = run_fresh(a);
    assert_eq!(ok_str(&c), "");

    // Inside: the seeded body pushes `coroName` and yields it, so the
    // `coroutine` command's result *is* what the opcode pushed.
    let mut body = Asm::new();
    body.op(Op::CORO_NAME, &[]).op(Op::YIELD, &[]);
    let mut vm = vm_with_seeded_proc(body);
    let c = eval(&mut vm, "coroutine c p");
    assert_eq!(ok_str(&c), "::c");
}

/// Outside a coroutine both suspend opcodes report C's `INST_YIELD` /
/// `INST_YIELD_TO_INVOKE` message (C also sets errorcode
/// `TCL COROUTINE ILLEGAL_YIELD`), and nothing is left pending afterwards.
#[test]
fn yield_opcodes_outside_a_coroutine_match_c() {
    let mut a = Asm::new();
    a.push("v").op(Op::YIELD, &[]);
    let (mut vm, c) = run_fresh(a);
    assert_eq!(err_str(&c), "yield can only be called in a coroutine");

    let mut a = Asm::new();
    a.push(":: list a").op(Op::YIELD_TO_INVOKE, &[]);
    let c = run(&mut vm, a);
    assert_eq!(err_str(&c), "yieldto can only be called in a coroutine");

    // The rejected requests left no suspend armed: a following push still runs
    // to completion in this same VM.
    let mut a = Asm::new();
    a.push("after");
    let c = run(&mut vm, a);
    assert_eq!(ok_str(&c), "after");
}

/// In a coroutine the `yield` opcode suspends: the yielded value comes out of
/// the resume command, and the value the next resume delivers replaces it on the
/// operand stack (C's `pc++; cleanup = 1; TEBC_YIELD()`), so it becomes the
/// body's result.
#[test]
fn yield_opcode_suspends_and_takes_the_resume_value() {
    let mut body = Asm::new();
    body.push("yielded").op(Op::YIELD, &[]);
    let mut vm = vm_with_seeded_proc(body);
    assert_eq!(ok_str(&eval(&mut vm, "coroutine c p")), "yielded");
    assert_eq!(ok_str(&eval(&mut vm, "c resumed")), "resumed");
    // The body ran off its end, so the coroutine command is gone.
    assert_eq!(ok_str(&eval(&mut vm, "info commands c")), "");
}

/// The `yield` opcode keeps the yield-boundary check: reached across a host
/// re-entry (here an `TclOO` method call) it is C Tcl's
/// `cannot yield: C stack busy`, exactly as the builtin is.
#[test]
fn yield_opcode_still_rejects_a_yield_across_a_host_re_entry() {
    let mut body = Asm::new();
    body.push("v").op(Op::YIELD, &[]);
    let mut vm = vm_with_seeded_proc(body);
    eval(&mut vm, "oo::class create C {method m {} {p}}\nC create o");
    let c = eval(&mut vm, "coroutine c o m");
    assert_eq!(err_str(&c), "cannot yield: C stack busy");
}

/// `yieldToInvoke` parks the coroutine and runs the popped command list in the
/// *resuming* context, whose result the resumer receives; the next resume's
/// arguments come back as a list, replacing the list on the stack.
#[test]
fn yield_to_invoke_runs_the_command_list_in_the_resumer() {
    let mut body = Asm::new();
    body.push(":: list a b").op(Op::YIELD_TO_INVOKE, &[]);
    let mut vm = vm_with_seeded_proc(body);
    assert_eq!(ok_str(&eval(&mut vm, "coroutine c p")), "a b");
    assert_eq!(ok_str(&eval(&mut vm, "c x y")), "x y");
}

// TclOO.

/// `tclooIsObject` never errors: `0` for a name that is not an object, `1` for a
/// real one — the test `info object isa object` (the form C compiles to this
/// opcode) performs.
#[test]
fn tcloo_is_object_reports_without_erroring() {
    let mut vm = tcl_vm::test_support::native_vm();
    vm.set_compiler(Box::new(compiler::svc()));
    eval(&mut vm, "oo::class create C {}\nC create o");
    for (name, want) in [("o", "1"), ("C", "1"), ("junk", "0"), ("", "0")] {
        let mut a = Asm::new();
        a.push(name).op(Op::TCLOO_IS_OBJECT, &[]);
        let c = run(&mut vm, a);
        assert_eq!(ok_str(&c), want, "tclooIsObject {name}");
        let cmd = eval(&mut vm, &format!("info object isa object {{{name}}}"));
        assert_eq!(ok_str(&cmd), want, "info object isa object {name}");
    }
}

/// `tclooClass` / `tclooNamespace` report exactly what `info object class` /
/// `info object namespace` do (the command forms C compiles to them).
#[test]
fn tcloo_class_and_namespace_match_info_object() {
    let mut vm = tcl_vm::test_support::native_vm();
    vm.set_compiler(Box::new(compiler::svc()));
    eval(&mut vm, "oo::class create C {}\nC create o");
    for (op, cmd) in [
        (Op::TCLOO_CLASS, "info object class o"),
        (Op::TCLOO_NS, "info object namespace o"),
    ] {
        let want = ok_str(&eval(&mut vm, cmd));
        let mut a = Asm::new();
        a.push("o").op(op, &[]);
        let c = run(&mut vm, a);
        assert_eq!(ok_str(&c), want, "{} vs [{cmd}]", op.mnemonic());
    }
    // The class of a class is its metaclass.
    let mut a = Asm::new();
    a.push("C").op(Op::TCLOO_CLASS, &[]);
    let c = run(&mut vm, a);
    assert_eq!(ok_str(&c), "::oo::class");
}

/// A name that is not an object is the shared lookup error (C's
/// `%s does not refer to an object`), identical to the command form's.
#[test]
fn tcloo_class_and_namespace_reject_a_non_object() {
    let mut vm = tcl_vm::test_support::native_vm();
    vm.set_compiler(Box::new(compiler::svc()));
    for (op, cmd) in [
        (Op::TCLOO_CLASS, "info object class junk"),
        (Op::TCLOO_NS, "info object namespace junk"),
    ] {
        let want = err_str(&eval(&mut vm, cmd));
        let mut a = Asm::new();
        a.push("junk").op(op, &[]);
        let c = run(&mut vm, a);
        assert_eq!(err_str(&c), want, "{} vs [{cmd}]", op.mnemonic());
        assert!(
            want.ends_with("does not refer to an object"),
            "C's lookup message: {want}"
        );
    }
}

/// Outside a method the compiled forms report C's `INST_TCLOO_*` messages — not
/// the `invalid command name` the *command* forms give, since in C `self` /
/// `next` / `nextto` are only resolvable inside an object's namespace.
#[test]
fn tcloo_self_next_and_nextto_outside_a_method() {
    let mut a = Asm::new();
    a.op(Op::TCLOO_SELF, &[]);
    let (_vm, c) = run_fresh(a);
    assert_eq!(err_str(&c), "self may only be called from inside a method");

    let mut a = Asm::new();
    a.push("next").op(Op::TCLOO_NEXT, &[1]);
    let (_vm, c) = run_fresh(a);
    assert_eq!(err_str(&c), "next may only be called from inside a method");

    let mut a = Asm::new();
    a.push("nextto").push("C").op(Op::TCLOO_NEXT_CLASS, &[2]);
    let (_vm, c) = run_fresh(a);
    assert_eq!(
        err_str(&c),
        "nextto may only be called from inside a method"
    );
}

/// `tclooSelf` pushes the current object's command name (C's
/// `TclOOObjectName`), the value `self` / `self object` reports.
#[test]
fn tcloo_self_pushes_the_current_object() {
    let mut body = Asm::new();
    body.op(Op::TCLOO_SELF, &[]);
    let mut vm = vm_with_seeded_proc(body);
    // `m` compiles the stream in its own method frame; `n` asks the command.
    eval(
        &mut vm,
        "oo::class create C {\nmethod m {} {string length native_opcode_body}\nmethod n {} {self object}\n}\nC create o",
    );
    let via_opcode = ok_str(&eval(&mut vm, "o m"));
    assert_eq!(via_opcode, "::o");
    assert_eq!(via_opcode, ok_str(&eval(&mut vm, "o n")));
}

/// `tclooNext` / `tclooNextClass` invoke the next implementation on the method
/// chain through the `next` / `nextto` cores. The operand counts *all* the words
/// C pushes — the command word itself first (C's `skip`), then the arguments,
/// which for `nextto` start with the class.
#[test]
fn tcloo_next_and_next_class_invoke_the_chain() {
    let classes = "oo::class create B {method m {} {return base}}\n\
                   oo::class create D {superclass B\nmethod m {} {string length native_opcode_body}}\n\
                   D create o";
    let mut body = Asm::new();
    body.push("next").op(Op::TCLOO_NEXT, &[1]);
    let mut vm = vm_with_seeded_proc(body);
    eval(&mut vm, classes);
    assert_eq!(ok_str(&eval(&mut vm, "o m")), "base");

    let mut body = Asm::new();
    body.push("nextto").push("B").op(Op::TCLOO_NEXT_CLASS, &[2]);
    let mut vm = vm_with_seeded_proc(body);
    eval(&mut vm, classes);
    assert_eq!(ok_str(&eval(&mut vm, "o m")), "base");
}

/// At the end of the chain `tclooNext` reports the core's
/// `no next <kind> implementation` (C's `INST_TCLOO_NEXT` message for the same
/// state), and a zero word count is rejected rather than read off an empty
/// stack.
#[test]
fn tcloo_next_at_the_end_of_the_chain_and_underflow() {
    let mut body = Asm::new();
    body.push("next").op(Op::TCLOO_NEXT, &[1]);
    let mut vm = vm_with_seeded_proc(body);
    eval(
        &mut vm,
        "oo::class create B {method m {} {string length native_opcode_body}}\nB create o",
    );
    assert_eq!(
        err_str(&eval(&mut vm, "o m")),
        "no next method implementation"
    );

    let mut a = Asm::new();
    a.op(Op::TCLOO_NEXT, &[0]);
    let (_vm, c) = run_fresh(a);
    assert_eq!(err_str(&c), "tclooNext: stack underflow");
}

#[test]
fn tcloo_opcodes_reject_a_nested_ordinary_procedure() {
    for (opcode, words, message) in [
        (
            Op::TCLOO_SELF,
            vec![],
            "self may only be called from inside a method",
        ),
        (
            Op::TCLOO_NEXT,
            vec!["next"],
            "next may only be called from inside a method",
        ),
        (
            Op::TCLOO_NEXT_CLASS,
            vec!["nextto", "B"],
            "nextto may only be called from inside a method",
        ),
    ] {
        let mut body = Asm::new();
        for word in &words {
            body.push(word);
        }
        if opcode == Op::TCLOO_SELF {
            body.op(opcode, &[]);
        } else {
            body.op(opcode, &[i32::try_from(words.len()).unwrap()]);
        }
        let mut vm = vm_with_seeded_proc(body);
        eval(
            &mut vm,
            "oo::class create B {method m {} {return BASE}}; oo::class create D {superclass B; method m {} {p}}; D create o",
        );
        assert_eq!(err_str(&eval(&mut vm, "o m")), message);
    }
}
