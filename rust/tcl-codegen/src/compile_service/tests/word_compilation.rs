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

//! How words compile: live variables in native operands, `string cat` literal folding and where a variable reference ends.

use super::*;

/// A quoted word inside a command substitution reaches a native operand
/// hook as its *written* text (`{}$z`), while the original word spells its
/// variable `${z}`. The written text once reached the compiled-word path,
/// which reads a bare `$` as data, so `string length "{}$z"` pushed the
/// four bytes `{}$z` unsubstituted and answered 4. tclsh 8.4.20, 8.5.19,
/// 8.6.18, 9.0.4 and 9.1.0 all print `3 3 2 2 3 4 3 3` for
/// `"{}$z" "$z{}" "a$z" "{}" "{$z}" "a{}$z" "\{}$z" "\{\}$z"`.
#[test]
fn native_operand_words_in_a_substitution_keep_their_variables_live() {
    let shapes = [
        (r#"string length "{}$z""#, "{}$z"),
        (r#"string length "$z{}""#, "$z{}"),
        (r#"string length "a$z""#, "a$z"),
        (r#"string length "a{}$z""#, "a{}$z"),
        (r#"string length "\{}$z""#, "{}$z"),
        (r#"string length "\{\}$z""#, "{}$z"),
        (r#"string index "{}$z" 2"#, "{}$z"),
        (r#"list "{}$z""#, "{}$z"),
    ];
    for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let Some(profile) = tcl_dialect::DialectProfile::find(dialect) else {
            continue;
        };
        let service = BytecodeCompileService::for_profile(profile);
        for (command, written) in shapes {
            let source = format!("set z x\nputs [{command}]\n");
            let module = service
                .compile_for_profile(&source, profile)
                .unwrap_or_else(|error| panic!("[{dialect}] {command}: {error:?}"));
            let literals = module.top_level.literals.entries();
            assert!(
                !literals.iter().any(|literal| literal == written),
                "[{dialect}] {command}: the written word was pushed as a value: {:?}",
                module.top_level.instructions
            );
            assert!(
                module
                    .top_level
                    .instructions
                    .iter()
                    .any(|instruction| instruction.op == tcl_bytecode::Op::STR_CONCAT1),
                "[{dialect}] {command}: the word was not built from its parts: {:?}",
                module.top_level.instructions
            );
        }
    }
}

/// `string cat` compiles as tclsh 8.6.18 and 9.0.4 disassemble it: each
/// run of compile-known words folded into one literal, `strcat` over the
/// operands, and `""` for no words — not an invocation of the command.
#[test]
fn string_cat_compiles_with_its_literal_runs_folded() {
    let source = "set b x\nset r [string cat a - $b {c} d]\nset e [string cat]\n";
    for dialect in ["tcl8.6", "tcl9.0"] {
        let profile = tcl_dialect::DialectProfile::find(dialect).expect("catalogue profile");
        let module = BytecodeCompileService::for_profile(profile)
            .compile_for_profile(source, profile)
            .unwrap_or_else(|error| panic!("{dialect}: {error:?}"));
        let instructions = &module.top_level.instructions;
        assert!(
            instructions.iter().any(
                |instruction| instruction.op == tcl_bytecode::Op::STR_CONCAT1
                    && instruction.operands == [tcl_bytecode::Operand::Imm(3)]
            ),
            "{dialect}: {instructions:?}"
        );
        assert!(
            !instructions.iter().any(|instruction| matches!(
                instruction.op,
                tcl_bytecode::Op::INVOKE_STK1 | tcl_bytecode::Op::INVOKE_STK4
            )),
            "{dialect}: {instructions:?}"
        );
        let literals = module.top_level.literals.entries();
        for folded in ["a-", "cd", ""] {
            assert!(
                literals.iter().any(|literal| literal == folded),
                "{dialect}: {folded:?} in {literals:?}"
            );
        }
    }
}

/// A variable reference ends where the release's scanner ends it
/// (review N17): `$a:b` substitutes `a` and appends `:b`, a lone colon is
/// literal, a run of two or more colons is a namespace separator, and a
/// braced `${a}` is the control. Every release agrees (tclsh 8.4.20,
/// 8.5.19, 8.6.18, 9.0.4 and 9.1.0 print the expected line for this
/// script); the result is the same compiled at top level and in a proc.
#[test]
fn a_variable_reference_ends_where_the_release_scanner_ends_it() {
    const SETUP: &str = "namespace eval a {variable b NSB}\n\
            set a x\nset a:b BAD\nset {a:} LONE\n\
            set arr(x:b) good\nset arr(BAD) bad\nset arr(x:) lone-good\n\
            set arr(LONE) lone-bad\nset arr(NSB) ns-good\n";
    const RESULT: &str = "join [list $arr($a:b) $arr(${a}:b) $arr($a:) $arr($a::b) \
            $arr($a:::b) \"$a:b\" \"$a:\" \"$a.b\" [set a]:b] |";
    const EXPECTED: &str = "good|good|lone-good|ns-good|ns-good|x:b|x:|x.b|x:b";
    let scripts = [
        format!("{SETUP}{RESULT}\n"),
        format!("proc p {{}} {{\n    global a a:b a: arr\n    {RESULT}\n}}\n{SETUP}p\n"),
    ];
    for release in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let profile = tcl_registry::model::ingress::resolve_environment(release).unit_profile();
        for script in &scripts {
            let service = BytecodeCompileService::for_profile(profile);
            let module = service
                .compile_for_profile(script, profile)
                .unwrap_or_else(|error| panic!("{release}: {error:?}\n{script}"));
            let mut vm = tcl_vm::Vm::with_native_core(
                Box::new(std::io::sink()),
                std::rc::Rc::new(tcl_vm::host_native::NativeHost::new()),
                profile,
                tcl_registry::special_vars::NativeBootstrapInputs {
                    package_path: Vec::new(),
                    default_library: None,
                },
            )
            .expect("native interpreter");
            vm.set_compiler(Box::new(BytecodeCompileService::for_profile(profile)));
            let completion = vm.run_module(&module);
            assert!(
                completion.code.is_ok(),
                "{release}: {}\n{script}",
                completion.result.to_str()
            );
            assert_eq!(
                completion.result.to_str().as_ref(),
                EXPECTED,
                "{release}\n{script}"
            );
        }
    }
}
