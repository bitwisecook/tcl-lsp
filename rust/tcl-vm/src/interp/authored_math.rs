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

//! Explicit authored fixed functions and their independent interpreter state.

use super::Vm;
use crate::{TclError, Value};
use std::{cell::Cell, rc::Rc};
use tcl_runtime_api::expression_policy::AuthoredMathFunctionProvider;

pub(super) struct AuthoredMathState {
    pub(super) seed: i64,
    precision: Rc<Cell<u8>>,
}

impl Default for AuthoredMathState {
    fn default() -> Self {
        Self {
            seed: tcl_syntax::expr::rand::seed_from_wide(1),
            precision: Rc::new(Cell::new(12)),
        }
    }
}

impl Vm {
    /// Install or remove an explicit authored Tcl84 fixed-function model.
    /// The parser and numeric providers must already be installed. Each actual
    /// interpreter owns its independent deterministic seed-one stream and
    /// precision-twelve formatter. No native table or compiler receipt is issued.
    #[must_use]
    pub fn set_logical_math_function_provider(
        &mut self,
        provider: Option<AuthoredMathFunctionProvider>,
    ) -> bool {
        if let Some(selected) = provider {
            let Some(mut policy) = self.expression_evaluation_policy() else {
                return false;
            };
            policy.authored_functions = Some(selected);
            if self.actual_engine_profile.is_none()
                || tcl_registry::authored_math_functions::provider(&policy).is_none()
            {
                return false;
            }
        }
        if self.logical_providers.math_functions == provider {
            return true;
        }
        self.logical_providers.math_functions = provider;
        self.authored_math = provider.map(|_| AuthoredMathState::default());
        self.bump_cmd_epoch();
        self.profile_generation = self.profile_generation.wrapping_add(1);
        self.eval_cache.clear();
        self.eval_cache_plain.clear();
        self.module_procs.clear();
        self.install_native_precision_trace();
        true
    }

    pub(crate) fn authored_math_provider(&self) -> Option<AuthoredMathFunctionProvider> {
        tcl_registry::authored_math_functions::provider(&self.expression_evaluation_policy()?)
    }

    pub(crate) fn authored_math_format(&self) -> Option<crate::value::DoubleFormatContext> {
        self.authored_math_provider()?;
        Some(crate::value::DoubleFormatContext::authored_tcl84(
            Rc::clone(&self.authored_math.as_ref()?.precision),
        ))
    }

    pub(crate) fn format_authored_math_result(&self, value: Value) -> Value {
        match self.authored_math_format() {
            Some(context) => value.with_authored_double_format(context),
            None => value,
        }
    }

    pub(crate) fn authored_math_random(
        &mut self,
        seed: Option<&Value>,
    ) -> tcl_runtime_api::Completion<Value> {
        use tcl_syntax::logical_numeric_simulation::LogicalNumericInputStage;
        if self.authored_math_provider().is_none() {
            return self.refuse_host_command("authored random provider is unavailable".into());
        }
        let next_seed = if let Some(value) = seed {
            let Some(simulation) = self.numeric_context().simulation else {
                return self
                    .refuse_host_command("authored random numeric policy is unavailable".into());
            };
            let bytes = value.string_bytes();
            match simulation
                .parse_number(&bytes, LogicalNumericInputStage::Integer)
                .ok()
                .and_then(|number| {
                    tcl_syntax::expr::wide::parsed_literal(
                        tcl_dialect::NativeArithmetic::Tcl84Wide,
                        &number,
                    )
                    .ok()
                }) {
                Some(seed) => Some(seed),
                None => {
                    return super::err(format!("expected integer but got \"{}\"", value.to_str()));
                }
            }
        } else {
            None
        };
        let state = self
            .authored_math
            .as_mut()
            .expect("installed authored state");
        if let Some(seed) = next_seed {
            state.seed = tcl_syntax::expr::rand::seed_from_wide(seed);
        }
        let value = Value::double(tcl_syntax::expr::rand::next_draw(&mut state.seed));
        super::ok(self.format_authored_math_result(value))
    }

    pub(super) fn prepare_authored_expression_bytes(
        &mut self,
        source: &[u8],
    ) -> Option<Result<tcl_syntax::expr::NativeExprNode, TclError>> {
        use tcl_registry::authored_math_functions::AuthoredFunctionPreparationError as Error;
        let policy = self.expression_evaluation_policy()?;
        tcl_registry::authored_math_functions::provider(&policy)?;
        Some(
            match tcl_registry::authored_math_functions::prepare(source, &policy) {
                Ok(tree) => Ok(tree),
                Err(Error::Rejected(diagnostic)) => Err(TclError::from_completion(
                    tcl_runtime_api::Completion::new_error_metadata(
                        tcl_runtime_api::Code::Error,
                        Value::from_string_bytes(diagnostic.message),
                        crate::command::options_dict(tcl_runtime_api::Code::Error, 0, &[]),
                    ),
                )),
                Err(Error::Unavailable) => Err(self.refuse_expression_bytes(
                    source,
                    tcl_runtime_api::NativeExpressionRefusal::FunctionDispatchPolicyUnavailable,
                )),
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_registry::invocation_words::{
        LogicalExpressionParseProvider, LogicalExpressionQuoteProvider, LogicalSourceWordProvider,
    };
    use tcl_runtime_api::Code;
    use tcl_syntax::logical_numeric_simulation::AuthoredLogicalNumericSimulation;

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
        let host = tcl_registry::model::ingress::resolve_environment("tcl9.0").unit_profile();
        let mut vm = Vm::with_native_core(
            Box::new(Capture(output)),
            Rc::new(crate::host_native::NativeHost::new()),
            host,
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .unwrap();
        let profile = tcl_dialect::DialectProfile::irules();
        vm.set_dialect_profile(profile);
        assert!(vm.set_command_surface_profile(host));
        assert!(vm.set_native_engine_profile(host));
        vm.set_compiler(Box::new(
            tcl_compiler::compile_service::BytecodeCompileService::for_profile(profile),
        ));
        assert!(vm.set_logical_eval_object_provider(
            tcl_registry::native_eval_object::LogicalEvalObjectProvider::Tcl84CoreSimulation
        ));
        assert!(
            vm.set_logical_source_word_provider(LogicalSourceWordProvider::Tcl84CoreSimulation)
        );
        assert!(vm.set_logical_expression_parse_provider(
            LogicalExpressionParseProvider::Tcl84CoreSimulation
        ));
        assert!(vm.set_logical_quote_provider(LogicalExpressionQuoteProvider::Tcl84CoreSimulation));
        assert!(vm.set_logical_numeric_provider(AuthoredLogicalNumericSimulation::Tcl84Core));
        assert!(vm.set_logical_name_provider(
            tcl_syntax::naming::NamePolicyProtocol::authored_tcl(tcl_dialect::TclVersion::V8_4)
        ));
        assert!(vm.set_logical_compiled_variable_provider(
            tcl_registry::native_compiled_variables::LogicalCompiledVariableProvider::Tcl84CoreSimulation));
        assert!(
            vm.set_logical_math_function_provider(Some(AuthoredMathFunctionProvider::Tcl84Core))
        );
        vm
    }

    fn expression(vm: &mut Vm, source: &str) -> String {
        let completion = vm.try_eval_expr(source).unwrap();
        assert_eq!(completion.code, Code::Ok, "{source}");
        completion.result.try_to_str().unwrap().to_string()
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
            vm.native_compilation_entry_for_namespace("", false)
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
                "../../../tcl-registry/tests/data/authored_tcl84_math_functions/source.tcl"
            ))
            .unwrap();
        assert_eq!(completion.code, Code::Ok);
        assert_eq!(
            String::from_utf8(output.borrow().clone()).unwrap(),
            include_str!(
                "../../../tcl-registry/tests/data/authored_tcl84_math_functions/native84.txt"
            )
        );
        assert!(
            vm.native_compilation_entry_for_namespace("", false)
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
                "../../../tcl-registry/tests/data/authored_tcl84_math_functions/format-random.tcl"
            ))
            .unwrap();
        assert_eq!(completion.code, Code::Ok);
        assert_eq!(
            String::from_utf8(output.borrow().clone()).unwrap(),
            include_str!(
                "../../../tcl-registry/tests/data/authored_tcl84_math_functions/format-random84.txt"
            )
        );
    }

    #[test]
    fn authored_function_syntax_rejects_malformed_original_children_before_lookup() {
        let output = Rc::new(std::cell::RefCell::new(Vec::new()));
        let mut vm = simulation(Rc::clone(&output));
        let completion = vm
            .try_eval_source(include_str!(
                "../../../tcl-registry/tests/data/authored_tcl84_math_functions/child-syntax.tcl"
            ))
            .unwrap();
        assert_eq!(completion.code, Code::Ok);
        assert_eq!(
            String::from_utf8(output.borrow().clone()).unwrap(),
            include_str!(
                "../../../tcl-registry/tests/data/authored_tcl84_math_functions/child-syntax84.txt"
            )
        );
    }

    #[test]
    fn authored_math_capability_cache_children_and_host_state_stay_separate() {
        let mut vm = simulation(Rc::new(std::cell::RefCell::new(Vec::new())));
        let original_policy = vm.native_compiler_policy();
        assert_eq!(expression(&mut vm, "rand()"), "7.82636925943e-06");
        assert_eq!(
            vm.try_eval_native_host_source("expr {srand(123)};set tcl_precision 5")
                .unwrap()
                .code,
            Code::Ok,
        );
        assert_eq!(vm.native_compiler_policy(), original_policy);
        assert_eq!(expression(&mut vm, "rand()"), "0.131537788143");
        assert_eq!(expression(&mut vm, "sqrt(2)"), "1.41421356237");
        assert_eq!(expression(&mut vm, "sqrt(2)+0.0"), "1.41421356237");
        assert_eq!(expression(&mut vm, "-sqrt(2)"), "-1.41421356237");
        let child = vm.fork_child_state();
        let child_id = vm.new_interp_slot(child, vm.cur);
        vm.in_interp(child_id, |vm| {
            assert_eq!(expression(vm, "rand()"), "7.82636925943e-06");
            assert_eq!(expression(vm, "sqrt(2)"), "1.41421356237");
        });
        assert_eq!(expression(&mut vm, "rand()"), "0.755605322195");
        assert!(vm.set_logical_math_function_provider(None));
        assert_ne!(vm.native_compiler_policy(), original_policy);
        assert!(vm.try_eval_expr("abs([set forbidden 1])").is_err());
        assert!(vm.get_var("forbidden").is_none());
        assert!(
            vm.set_logical_math_function_provider(Some(AuthoredMathFunctionProvider::Tcl84Core))
        );
        assert_eq!(expression(&mut vm, "rand()"), "7.82636925943e-06");
        assert_eq!(expression(&mut vm, "abs(077)"), "63");
    }
}
