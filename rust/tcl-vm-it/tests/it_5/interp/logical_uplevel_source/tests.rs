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

//! `uplevel` on an iRules simulation of the 8.4 core, which needs the eval and source-word providers.

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

fn simulation(eval: bool, words: bool) -> (Vm, Rc<RefCell<Vec<u8>>>) {
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
    if eval {
        assert!(
            vm.set_logical_eval_object_provider(LogicalEvalObjectProvider::Tcl84CoreSimulation)
        );
    }
    if words {
        assert!(
            vm.set_logical_source_word_provider(LogicalSourceWordProvider::Tcl84CoreSimulation)
        );
    }
    assert!(
        vm.set_logical_name_provider(tcl_syntax::naming::NamePolicyProtocol::authored_tcl(
            tcl_dialect::TclVersion::V8_4
        ))
    );
    assert!(vm.set_logical_compiled_variable_provider(tcl_registry::native_compiled_variables::LogicalCompiledVariableProvider::Tcl84CoreSimulation));
    assert!(vm.set_logical_numeric_provider(
        tcl_syntax::logical_numeric_simulation::AuthoredLogicalNumericSimulation::Tcl84Core
    ));
    (vm, output)
}

fn uplevel(
    vm: &mut Vm,
    script: &[u8],
) -> Result<Completion<Value>, tcl_runtime_api::NativeExecutionError> {
    vm.try_invoke_command(
        "uplevel",
        &[Value::string("#0"), Value::new_native_string_bytes(script)],
    )
}

#[test]
fn authored_uplevel_uses_fresh_words_without_a_native_literal_pool() {
    let (mut vm, output) = simulation(true, true);
    let completion = uplevel(
        &mut vm,
        b"puts -nonewline reached; set logical_word_effect yes",
    )
    .unwrap();
    assert_eq!(completion.code, Code::Ok);
    assert_eq!(completion.result.string_bytes().as_ref(), b"yes");
    assert_eq!(output.borrow().as_slice(), b"reached");
}

#[test]
fn authored_uplevel_requires_each_explicit_provider_before_execution() {
    for (eval, words) in [(false, true), (true, false)] {
        let (mut vm, output) = simulation(eval, words);
        let reached = uplevel(&mut vm, b"puts -nonewline forbidden");
        assert!(reached.is_err() || reached.unwrap().code == Code::Error);
        assert!(output.borrow().is_empty());
    }
}
