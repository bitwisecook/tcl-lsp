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

//! The iRules dialect operators under an authored F5 policy: string tests, glob and regex matching and the word logic operators.

use super::*;

// iRules dialect operators.

fn authored_f5_vm() -> Vm {
    let mut vm = tcl_vm::test_support::native_vm();
    vm.set_dialect_profile(tcl_dialect::DialectProfile::irules());
    assert!(vm.set_native_engine_profile(
        tcl_registry::model::resolve_environment("tcl9.0").unit_profile()
    ));
    assert!(vm.set_logical_expression_parse_provider(
        tcl_registry::invocation_words::LogicalExpressionParseProvider::Tcl84CoreSimulation,
    ));
    vm
}

fn run_f5_module(vm: &mut Vm, asm: Asm) -> Completion<Value> {
    let module = tcl_bytecode::ModuleAsm {
        profile: vm.dialect_profile(),
        source: tcl_lexer::SourceImage::default(),
        source_namespace: tcl_runtime_api::ByteNamespacePath::root(),
        plain_command_dispatch: false,
        top_level: asm.build(),
        top_level_body: FunctionAsm::default(),
        procedures: HashMap::new(),
        procedure_provenance: HashMap::new(),
        manifest: None,
    };
    vm.run_module(&module)
}

#[test]
fn bare_matches_opcode_withdraws_without_authored_policy() {
    for name in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        let mut vm = tcl_vm::test_support::native_vm_for_profile(
            tcl_registry::model::resolve_environment(name).unit_profile(),
            Box::new(std::io::stdout()),
        );
        let mut asm = Asm::new();
        asm.push("abcd").push("a*").op(Op::IRULE_MATCHES, &[]);
        let module = tcl_bytecode::ModuleAsm {
            profile: vm.dialect_profile(),
            source: tcl_lexer::SourceImage::default(),
            source_namespace: tcl_runtime_api::ByteNamespacePath::root(),
            plain_command_dispatch: false,
            top_level: asm.build(),
            top_level_body: FunctionAsm::default(),
            procedures: HashMap::new(),
            procedure_provenance: HashMap::new(),
            manifest: None,
        };
        assert!(vm.try_run_module(&module).is_err(), "{name}");
    }
}

fn run_authored_f5(asm: Asm) -> (Vm, Completion<Value>) {
    let mut vm = authored_f5_vm();
    let completion = run_f5_module(&mut vm, asm);
    (vm, completion)
}

/// The dialect string tests (`contains` / `starts_with` / `ends_with` /
/// `equals` / `matches`), both outcomes each. The operands sit on the stack subject-first
/// with the needle on top — the order `Op::from_binop` codegen pushes a binary
/// expression's operands in (left, then right), the same as `Op::ADD`'s.
#[test]
fn irule_string_tests_both_outcomes() {
    for (op, subject, operand, want) in [
        (Op::IRULE_CONTAINS, "foobar", "oob", "1"),
        (Op::IRULE_CONTAINS, "foobar", "bop", "0"),
        // Every string contains the empty one.
        (Op::IRULE_CONTAINS, "foobar", "", "1"),
        (Op::IRULE_CONTAINS, "", "x", "0"),
        (Op::IRULE_STARTS_WITH, "foobar", "foo", "1"),
        (Op::IRULE_STARTS_WITH, "foobar", "bar", "0"),
        (Op::IRULE_ENDS_WITH, "foobar", "bar", "1"),
        (Op::IRULE_ENDS_WITH, "foobar", "foo", "0"),
        (Op::IRULE_EQUALS, "foobar", "foobar", "1"),
        // `equals` is the word spelling of `eq`: case-sensitive, and a *string*
        // comparison even when both operands look numeric.
        (Op::IRULE_EQUALS, "foobar", "FOOBAR", "0"),
        (Op::IRULE_EQUALS, "1", "1.0", "0"),
        // Exact BIG-IP 21.1.0.1 build 0.0.26 wildcard discriminators.
        (Op::IRULE_MATCHES, "abcd", "a*", "1"),
        (Op::IRULE_MATCHES, "a*", "a*", "1"),
        (Op::IRULE_MATCHES, "a*", "abcd", "0"),
        (Op::IRULE_MATCHES, "abcd", "a.*", "0"),
        (Op::IRULE_MATCHES, "abcd", "bc", "0"),
        (Op::IRULE_MATCHES, "abcd", "abcd", "1"),
        (Op::IRULE_MATCHES, "abc", "abc", "1"),
        (Op::IRULE_MATCHES, "foobar", "oob", "0"),
        (Op::IRULE_MATCHES, "foobar", "FOOBAR", "0"),
    ] {
        let mut a = Asm::new();
        a.push(subject).push(operand).op(op, &[]);
        let (_vm, c) = run_authored_f5(a);
        assert_eq!(ok_str(&c), want, "{} {subject} {operand}", op.mnemonic());
    }
}

/// `matches_glob` is a case-sensitive `string match`; `matches_regex` runs the
/// Tcl 9 ARE engine (the `[[:<:]]` word edge proves it is the real engine, not a
/// look-alike), also case-sensitive — neither dialect operator has a `-nocase`
/// form.
#[test]
fn irule_glob_and_regex_matching() {
    for (op, subject, pattern, want) in [
        (Op::IRULE_MATCHES_GLOB, "foobar", "foo*", "1"),
        (Op::IRULE_MATCHES_GLOB, "foobar", "*bar", "1"),
        (Op::IRULE_MATCHES_GLOB, "foobar", "f?obar", "1"),
        (Op::IRULE_MATCHES_GLOB, "foobar", "foo", "0"),
        (Op::IRULE_MATCHES_GLOB, "foobar", "FOO*", "0"),
        (Op::IRULE_MATCHES_REGEX, "foobar", "o+b", "1"),
        (Op::IRULE_MATCHES_REGEX, "foobar", "^foo", "1"),
        (Op::IRULE_MATCHES_REGEX, "foobar", "^bar", "0"),
        (Op::IRULE_MATCHES_REGEX, "foobar", "FOO", "0"),
        (Op::IRULE_MATCHES_REGEX, "foobar", "[[:<:]]bar", "0"),
        (Op::IRULE_MATCHES_REGEX, "foo bar", "[[:<:]]bar", "1"),
    ] {
        let mut a = Asm::new();
        a.push(subject).push(pattern).op(op, &[]);
        let (_vm, c) = run_authored_f5(a);
        assert_eq!(ok_str(&c), want, "{} {subject} {pattern}", op.mnemonic());
    }
}

/// An uncompilable regex is an error, not a silent `0` — the same engine
/// detail the compiled `regexp` opcode reports for the same pattern (both drive
/// the shared ARE core). `Op::REGEXP` adds C's `CompileRegexp` prefix for the
/// pinned release, as `INST_REGEXP` does.
#[test]
fn irule_matches_regex_rejects_a_bad_pattern() {
    let mut a = Asm::new();
    a.push("foobar").push("[").op(Op::IRULE_MATCHES_REGEX, &[]);
    let (_vm, c) = run_fresh(a);
    assert_eq!(err_str(&c), "brackets [] not balanced");

    // `Op::REGEXP` takes its operands the other way round (pattern under the
    // string) and `TCL_REG_ADVANCED` as its flags operand.
    let mut a = Asm::new();
    a.push("[").push("foobar").op(Op::REGEXP, &[3]);
    let (_vm, regexp_op) = run_fresh(a);
    assert_eq!(
        err_str(&regexp_op),
        format!("cannot compile regular expression pattern: {}", err_str(&c))
    );
}

/// `and` / `or` reduce Tcl truthiness (the boolean words included) to `1`/`0`.
#[test]
fn irule_word_and_or_truth_table() {
    for (op, left, right, want) in [
        (Op::IRULE_WORD_AND, "1", "1", "1"),
        (Op::IRULE_WORD_AND, "1", "0", "0"),
        (Op::IRULE_WORD_AND, "0", "1", "0"),
        (Op::IRULE_WORD_AND, "0", "0", "0"),
        (Op::IRULE_WORD_AND, "true", "yes", "1"),
        (Op::IRULE_WORD_AND, "2", "-1", "1"),
        (Op::IRULE_WORD_OR, "1", "1", "1"),
        (Op::IRULE_WORD_OR, "1", "0", "1"),
        (Op::IRULE_WORD_OR, "0", "1", "1"),
        (Op::IRULE_WORD_OR, "0", "0", "0"),
        (Op::IRULE_WORD_OR, "no", "off", "0"),
    ] {
        let mut a = Asm::new();
        a.push(left).push(right).op(op, &[]);
        let (_vm, c) = run_fresh(a);
        assert_eq!(ok_str(&c), want, "{} {left} {right}", op.mnemonic());
    }
}

/// The shared tree-walk coerces `and`/`or`'s right operand only when the left
/// does not already decide the result; with both operands on the stack the
/// opcode reproduces that, so a junk right operand errors in exactly the cases
/// the walker treats as an error.
#[test]
fn irule_word_logic_short_circuits_its_coercion() {
    for (op, left, want) in [
        (Op::IRULE_WORD_AND, "0", "0"),
        (Op::IRULE_WORD_OR, "1", "1"),
    ] {
        let mut a = Asm::new();
        a.push(left).push("zzz").op(op, &[]);
        let (_vm, c) = run_fresh(a);
        assert_eq!(
            ok_str(&c),
            want,
            "{} decided by the left operand",
            op.mnemonic()
        );
    }
    for (op, left) in [(Op::IRULE_WORD_AND, "1"), (Op::IRULE_WORD_OR, "0")] {
        let mut a = Asm::new();
        a.push(left).push("zzz").op(op, &[]);
        let (_vm, c) = run_fresh(a);
        assert_eq!(
            err_str(&c),
            "expected boolean value but got \"zzz\"",
            "{} must coerce the right operand",
            op.mnemonic()
        );
    }
    // A junk *left* operand always errors, whatever the operator.
    let mut a = Asm::new();
    a.push("zzz").push("1").op(Op::IRULE_WORD_AND, &[]);
    let (_vm, c) = run_fresh(a);
    assert_eq!(err_str(&c), "expected boolean value but got \"zzz\"");
}

/// `not` is the word spelling of `!`: it shares the boolean coercion and, as
/// BIG-IP does (`evidence/tcl/bigip-r2286m-expressions.md`), reports `!` in the
/// operand error.
#[test]
fn irule_word_not_negates_truthiness() {
    for (v, want) in [
        ("1", "0"),
        ("0", "1"),
        ("yes", "0"),
        ("off", "1"),
        ("7", "0"),
    ] {
        let mut a = Asm::new();
        a.push(v).op(Op::IRULE_WORD_NOT, &[]);
        let (_vm, c) = run_fresh(a);
        assert_eq!(ok_str(&c), want, "not {v}");
    }
    let mut a = Asm::new();
    a.push("zzz").op(Op::IRULE_WORD_NOT, &[]);
    let (_vm, c) = run_fresh(a);
    assert_eq!(
        err_str(&c),
        "cannot use non-numeric string \"zzz\" as operand of \"!\""
    );
}

/// Each dialect operator agrees with the core command it is defined in terms of,
/// so the opcodes cannot drift from `[string first]` / `[string equal]` /
/// `[string match]` / `[regexp]`.
#[test]
fn irule_operators_agree_with_the_core_commands() {
    let mut vm = tcl_vm::test_support::native_vm();
    vm.set_compiler(Box::new(compiler::svc()));
    let mut predicate_vm = authored_f5_vm();
    for (op, subject, operand, cmd) in [
        (
            Op::IRULE_CONTAINS,
            "foobar",
            "oob",
            "expr {[string first oob foobar] >= 0}",
        ),
        (
            Op::IRULE_CONTAINS,
            "foobar",
            "bop",
            "expr {[string first bop foobar] >= 0}",
        ),
        (
            Op::IRULE_EQUALS,
            "foobar",
            "FOOBAR",
            "string equal foobar FOOBAR",
        ),
        (
            Op::IRULE_EQUALS,
            "foobar",
            "foobar",
            "string equal foobar foobar",
        ),
        (
            Op::IRULE_MATCHES_GLOB,
            "foobar",
            "f?o*",
            "string match f?o* foobar",
        ),
        (
            Op::IRULE_MATCHES_REGEX,
            "foobar",
            "o+b",
            "regexp {o+b} foobar",
        ),
    ] {
        let want = vm
            .eval_source(cmd)
            .expect("compiles")
            .result
            .to_str()
            .to_string();
        let mut a = Asm::new();
        a.push(subject).push(operand).op(op, &[]);
        let c = run_f5_module(&mut predicate_vm, a);
        assert_eq!(ok_str(&c), want, "{} vs [{cmd}]", op.mnemonic());
    }
}
