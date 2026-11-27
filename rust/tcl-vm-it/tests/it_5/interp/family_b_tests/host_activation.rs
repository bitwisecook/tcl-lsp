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

//! Host source under an explicit host activation, beside the user's own
//! logical profile.

use super::*;

/// An iRules user over a Tcl 9.0 host and engine, with the user's compiler.
fn irules_over_tcl90() -> Vm {
    let user = tcl_dialect::DialectProfile::irules();
    let host = tcl_vm::test_support::profile("tcl9.0");
    let mut vm = tcl_vm::test_support::native_vm();
    vm.set_dialect_profile(user);
    assert!(vm.set_command_surface_profile(host));
    assert!(vm.set_native_engine_profile(host));
    assert!(vm.set_logical_quote_provider(
        tcl_native_model::LogicalExpressionQuoteProvider::Tcl84CoreSimulation,
    ));
    assert!(
        vm.set_logical_name_provider(tcl_syntax::naming::NamePolicyProtocol::authored_tcl(
            tcl_dialect::TclVersion::V8_4
        ))
    );
    assert!(vm.set_logical_numeric_provider(
        tcl_syntax::logical_numeric_simulation::AuthoredLogicalNumericSimulation::Tcl84Core
    ));
    vm.set_compiler(Box::new(
        tcl_codegen::compile_service::BytecodeCompileService::for_profile(user),
    ));
    vm
}

/// Its words, joined by `|`.
struct Words;

impl tcl_vm::NativeCommand for Words {
    fn invoke(&self, _: &mut Vm, words: &[Value]) -> Completion<Value> {
        Completion::new(
            Code::Ok,
            Value::string(
                words
                    .iter()
                    .map(|word| word.to_str().to_string())
                    .collect::<Vec<_>>()
                    .join("|"),
            ),
            Value::empty(),
        )
    }
}

/// The same text is host source with Tcl 9.0's `{*}` and iRules user
/// source, where a brace closes a word, in either order: the user's policy
/// comes back after host source and neither reuses the other's compiled body.
#[test]
fn explicit_host_activation_restores_user_policy_and_separates_cached_bodies() {
    let mut vm = irules_over_tcl90();
    vm.register_native_command("words", Rc::new(Words));
    let source = "words {*}{A B}";
    for _ in 0..2 {
        let host = vm.try_eval_native_host_source(source).expect("host source");
        assert_eq!(host.code, Code::Ok);
        assert_eq!(host.result.to_str().as_ref(), "A|B");
        let user = vm.try_eval_source(source).expect("user source");
        assert_eq!(user.code, Code::Ok);
        assert_eq!(user.result.to_str().as_ref(), "*|A B");
    }
    assert_eq!(
        vm.dialect_profile().cache_key(),
        tcl_dialect::DialectProfile::irules().cache_key()
    );
}

#[test]
fn host_script_and_its_nested_substitution_use_the_explicit_source_activation() {
    let mut vm = irules_over_tcl90();
    let completion = vm
        .try_eval_native_host_source("set original [list {*}{A B}]; set original")
        .expect("explicit host source admission");
    assert_eq!(completion.code, Code::Ok);
    assert_eq!(completion.result.string_bytes().as_ref(), b"A B");
    assert_eq!(
        vm.dialect_profile().cache_key(),
        tcl_dialect::DialectProfile::irules().cache_key()
    );
}
