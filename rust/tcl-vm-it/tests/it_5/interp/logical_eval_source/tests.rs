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

//! `eval` on an iRules simulation of the 8.4 core, which needs each logical provider.

use crate::prelude::*;
use std::{cell::RefCell, io::Write};
use tcl_registry::invocation_words::LogicalSourceWordProvider;
use tcl_registry::native_eval_object::LogicalEvalObjectProvider;

struct Capture(Rc<RefCell<Vec<u8>>>);
impl Write for Capture {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0.borrow_mut().extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Missing {
    Eval,
    Words,
    Names,
    Variables,
}

fn simulation(missing: Option<Missing>) -> (Vm, Rc<RefCell<Vec<u8>>>) {
    let output = Rc::new(RefCell::new(Vec::new()));
    let host = tcl_dialect::DialectProfile::find("tcl9.0").unwrap();
    let mut vm =
        crate::fixture::interpreter_with_output(host, Box::new(Capture(Rc::clone(&output))));
    let logical = tcl_dialect::DialectProfile::irules();
    vm.set_dialect_profile(logical);
    assert!(vm.set_command_surface_profile(host));
    assert!(vm.set_native_engine_profile(host));
    vm.set_compiler(Box::new(
        tcl_codegen::compile_service::BytecodeCompileService::for_profile(logical),
    ));
    if missing != Some(Missing::Eval) {
        assert!(
            vm.set_logical_eval_object_provider(LogicalEvalObjectProvider::Tcl84CoreSimulation)
        );
    }
    if missing != Some(Missing::Words) {
        assert!(
            vm.set_logical_source_word_provider(LogicalSourceWordProvider::Tcl84CoreSimulation)
        );
    }
    if missing != Some(Missing::Names) {
        assert!(vm.set_logical_name_provider(
            tcl_syntax::naming::NamePolicyProtocol::authored_tcl(tcl_dialect::TclVersion::V8_4)
        ));
    }
    if missing != Some(Missing::Variables) {
        assert!(vm.set_logical_compiled_variable_provider(tcl_registry::native_compiled_variables::LogicalCompiledVariableProvider::Tcl84CoreSimulation));
    }
    assert!(vm.set_logical_numeric_provider(
        tcl_syntax::logical_numeric_simulation::AuthoredLogicalNumericSimulation::Tcl84Core
    ));
    (vm, output)
}

fn eval(
    vm: &mut Vm,
    script: &[u8],
) -> Result<Completion<Value>, tcl_runtime_api::NativeExecutionError> {
    vm.try_invoke_command("eval", &[Value::new_native_string_bytes(script)])
}

#[test]
fn logical_eval_uses_fresh_words_without_native_pool_cache_or_layout() {
    let (mut vm, output) = simulation(None);
    let completion = eval(
        &mut vm,
        b"puts -nonewline reached; set logical_eval_result yes",
    )
    .unwrap();
    assert_eq!(completion.code, Code::Ok);
    assert_eq!(completion.result.string_bytes().as_ref(), b"yes");
    assert_eq!(output.borrow().as_slice(), b"reached");
}

#[test]
fn logical_eval_requires_each_actual_provider_and_physical_core() {
    for missing in [
        Missing::Eval,
        Missing::Words,
        Missing::Names,
        Missing::Variables,
    ] {
        let (mut vm, output) = simulation(Some(missing));
        let reached = eval(&mut vm, b"puts -nonewline forbidden");
        assert!(
            reached.is_err() || reached.unwrap().code == Code::Error,
            "a missing provider refuses eval"
        );
        assert!(output.borrow().is_empty());
    }
}

#[test]
fn logical_eval_keeps_data_inert_and_retains_partial_prefix_order() {
    // tclsh 8.4.20, 8.6.18 and 9.0.4 print `prefix` and then report
    // `missing close-brace`.
    let (mut vm, output) = simulation(None);
    let completion = eval(&mut vm, b"set logical_eval_data {[puts forbidden]}").unwrap();
    assert_eq!(completion.code, Code::Ok);
    assert_eq!(
        completion.result.string_bytes().as_ref(),
        b"[puts forbidden]"
    );
    assert!(output.borrow().is_empty());
    let completion = eval(&mut vm, b"puts -nonewline prefix; set incomplete {").unwrap();
    assert_eq!(completion.code, Code::Error);
    assert_eq!(
        completion.result.string_bytes().as_ref(),
        b"missing close-brace"
    );
    assert_eq!(output.borrow().as_slice(), b"prefix");
}
