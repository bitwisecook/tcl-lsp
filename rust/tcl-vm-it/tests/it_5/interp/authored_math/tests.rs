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

//! The authored 8.4 math functions on an iRules simulation, against the C
//! 8.4 captures.

use crate::prelude::*;
use tcl_native_model::{
    LogicalExpressionParseProvider, LogicalExpressionQuoteProvider, LogicalSourceWordProvider,
};
use tcl_runtime_api::expression_policy::AuthoredMathFunctionProvider;
use tcl_syntax::logical_numeric_simulation::AuthoredLogicalNumericSimulation;

fn expression(vm: &mut Vm, source: &str) -> String {
    let completion = vm.try_eval_expr(source).unwrap();
    assert_eq!(completion.code, Code::Ok, "{source}");
    completion.result.try_to_str().unwrap().to_string()
}

struct Capture(Rc<std::cell::RefCell<Vec<u8>>>);

impl std::io::Write for Capture {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0.borrow_mut().extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn simulation(output: Rc<std::cell::RefCell<Vec<u8>>>) -> Vm {
    let host = tcl_vm::test_support::profile("tcl9.0");
    let mut vm = Vm::with_native_core(
        Box::new(Capture(output)),
        Rc::new(tcl_vm::host_native::NativeHost::new()),
        host,
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .unwrap();
    let profile = tcl_dialect::DialectProfile::irules();
    vm.set_dialect_profile(profile);
    assert!(vm.set_command_surface_profile(host));
    assert!(vm.set_native_engine_profile(host));
    vm.set_compiler(Box::new(
        tcl_codegen::compile_service::BytecodeCompileService::for_profile(profile),
    ));
    assert!(vm.set_logical_eval_object_provider(
        tcl_native_model::native_eval_object::LogicalEvalObjectProvider::Tcl84CoreSimulation
    ));
    assert!(vm.set_logical_source_word_provider(LogicalSourceWordProvider::Tcl84CoreSimulation));
    assert!(vm.set_logical_expression_parse_provider(
        LogicalExpressionParseProvider::Tcl84CoreSimulation
    ));
    assert!(vm.set_logical_quote_provider(LogicalExpressionQuoteProvider::Tcl84CoreSimulation));
    assert!(vm.set_logical_numeric_provider(AuthoredLogicalNumericSimulation::Tcl84Core));
    assert!(
        vm.set_logical_name_provider(tcl_syntax::naming::NamePolicyProtocol::authored_tcl(
            tcl_dialect::TclVersion::V8_4
        ))
    );
    assert!(vm.set_logical_compiled_variable_provider(
        tcl_native_model::native_compiled_variables::LogicalCompiledVariableProvider::Tcl84CoreSimulation));
    assert!(vm.set_logical_math_function_provider(Some(AuthoredMathFunctionProvider::Tcl84Core)));
    vm
}

#[test]
fn confined_authored_random_preserves_its_seed_and_unconfined_stream() {
    let mut vm = simulation(Rc::new(std::cell::RefCell::new(Vec::new())));
    let mut control = simulation(Rc::new(std::cell::RefCell::new(Vec::new())));
    assert_eq!(
        expression(&mut vm, "srand(7)"),
        expression(&mut control, "srand(7)")
    );
    vm.set_stores_confined(true);
    for source in ["srand(9)", r#"srand("wrong")"#, "rand()"] {
        let completion = vm.try_eval_expr(source).unwrap();
        assert_eq!(completion.code, Code::Error, "{source}");
        assert!(
            completion
                .result
                .to_str()
                .contains("stores are confined to the activation")
        );
    }
    vm.set_stores_confined(false);
    assert_eq!(
        expression(&mut vm, "rand()"),
        expression(&mut control, "rand()")
    );
}

#[test]
fn resumable_authored_functions_require_their_own_provider_and_restore_host_dispatch() {
    let mut vm = simulation(Rc::new(std::cell::RefCell::new(Vec::new())));
    let completion = vm
        .try_eval_source(
            "set operand 077; set actual [expr {abs([set operand])}]; list $operand $actual",
        )
        .unwrap();
    assert_eq!(completion.code, Code::Ok);
    assert_eq!(completion.result.try_to_str().unwrap().as_ref(), "077 63");
    assert!(
        crate::fixture::root_compilation_entry(&mut vm)
            .math_functions
            .is_none()
    );
    let host = vm.try_eval_native_host_source("expr {abs(077)}").unwrap();
    assert_eq!(host.code, Code::Ok);
    assert_eq!(host.result.try_to_str().unwrap().as_ref(), "77");
    assert_eq!(expression(&mut vm, "abs(077)"), "63");
    assert!(vm.set_logical_math_function_provider(None));
    assert!(vm.try_eval_source("expr {abs([set operand])}").is_err());
    assert_eq!(
        vm.try_eval_native_host_source("expr {abs(077)}")
            .unwrap()
            .code,
        Code::Ok
    );
}

#[test]
fn authored_fixed_functions_match_original_c84_results_and_operand_order() {
    let output = Rc::new(std::cell::RefCell::new(Vec::new()));
    let mut vm = simulation(Rc::clone(&output));
    let completion = vm
        .try_eval_source(include_str!(
            "../../../../../tcl-registry/tests/data/authored_tcl84_math_functions/source.tcl"
        ))
        .unwrap();
    assert_eq!(completion.code, Code::Ok);
    assert_eq!(
        String::from_utf8(output.borrow().clone()).unwrap(),
        include_str!(
            "../../../../../tcl-registry/tests/data/authored_tcl84_math_functions/native84.txt"
        )
    );
    assert!(
        crate::fixture::root_compilation_entry(&mut vm)
            .math_functions
            .is_none()
    );
}

#[test]
fn authored_math_precision_and_random_stream_match_original_c84_controls() {
    let output = Rc::new(std::cell::RefCell::new(Vec::new()));
    let mut vm = simulation(Rc::clone(&output));
    let completion = vm
        .try_eval_source(include_str!(
            "../../../../../tcl-registry/tests/data/authored_tcl84_math_functions/format-random.tcl"
        ))
        .unwrap();
    assert_eq!(completion.code, Code::Ok);
    assert_eq!(
        String::from_utf8(output.borrow().clone()).unwrap(),
        include_str!(
            "../../../../../tcl-registry/tests/data/authored_tcl84_math_functions/format-random84.txt"
        )
    );
}

#[test]
fn authored_function_syntax_rejects_malformed_original_children_before_lookup() {
    let output = Rc::new(std::cell::RefCell::new(Vec::new()));
    let mut vm = simulation(Rc::clone(&output));
    let completion = vm
        .try_eval_source(include_str!(
            "../../../../../tcl-registry/tests/data/authored_tcl84_math_functions/child-syntax.tcl"
        ))
        .unwrap();
    assert_eq!(completion.code, Code::Ok);
    assert_eq!(
        String::from_utf8(output.borrow().clone()).unwrap(),
        include_str!(
            "../../../../../tcl-registry/tests/data/authored_tcl84_math_functions/child-syntax84.txt"
        )
    );
}

#[test]
fn authored_math_capability_cache_children_and_host_state_stay_separate() {
    let mut vm = simulation(Rc::new(std::cell::RefCell::new(Vec::new())));
    assert_eq!(expression(&mut vm, "rand()"), "7.82636925943e-06");
    assert_eq!(
        vm.try_eval_native_host_source("expr {srand(123)};set tcl_precision 5")
            .unwrap()
            .code,
        Code::Ok,
    );
    assert_eq!(expression(&mut vm, "rand()"), "0.131537788143");
    assert_eq!(expression(&mut vm, "sqrt(2)"), "1.41421356237");
    assert_eq!(expression(&mut vm, "sqrt(2)+0.0"), "1.41421356237");
    assert_eq!(expression(&mut vm, "-sqrt(2)"), "-1.41421356237");
    let child = crate::fixture::eval(
        &mut vm,
        "interp create c; list [c eval {expr {rand()}}] [c eval {expr {sqrt(2)}}]",
    );
    assert_eq!(child.code, Code::Ok);
    assert_eq!(
        child.result.to_str().as_ref(),
        "7.82636925943e-06 1.41421356237"
    );
    assert_eq!(expression(&mut vm, "rand()"), "0.755605322195");
    assert!(vm.set_logical_math_function_provider(None));
    assert!(vm.try_eval_expr("abs([set forbidden 1])").is_err());
    assert!(vm.get_var("forbidden").is_none());
    assert!(vm.set_logical_math_function_provider(Some(AuthoredMathFunctionProvider::Tcl84Core)));
    assert_eq!(expression(&mut vm, "rand()"), "7.82636925943e-06");
    assert_eq!(expression(&mut vm, "abs(077)"), "63");
}
