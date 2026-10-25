// SPDX-License-Identifier: AGPL-3.0-or-later
//! Authored uplevel source words, independent of native literal registration.

use super::{InterpState, Value};
use crate::compiled::NativeCompilerPolicy;
use tcl_registry::native_eval_object::{EvalObjectPurpose, NativeEvalObjectProtocol};
use tcl_syntax::value::ValueError;

#[derive(Clone, Copy)]
pub(crate) struct LogicalUpLevelSourceEntry {
    policy: NativeCompilerPolicy,
}

impl LogicalUpLevelSourceEntry {
    pub(crate) fn is_current(self, policy: &NativeCompilerPolicy) -> bool {
        self.policy == *policy
    }

    pub(crate) fn value(self, literal: &tcl_bytecode::NativeStringLiteral) -> Value {
        debug_assert!(self.policy.source_word_provider.is_some());
        Value::new_native_string_bytes(literal.bytes())
    }
}

impl InterpState {
    pub(crate) fn logical_uplevel_source_entry(
        &self,
        selected: NativeEvalObjectProtocol,
        purpose: EvalObjectPurpose,
    ) -> Result<Option<LogicalUpLevelSourceEntry>, ValueError> {
        let dialect = self.native_invocation_dialect();
        if purpose != EvalObjectPurpose::UpLevel || dialect.native_eval_object_protocol().is_some()
        {
            return Ok(None);
        }
        let Some(provider) = self.logical_providers.eval_object else {
            return Ok(None);
        };
        if dialect.eval_object_protocol(Some(provider)) != Some(selected)
            || selected.compiles_source(purpose)
        {
            return Ok(None);
        }
        let unavailable =
            || ValueError::CommandProtocolUnavailable("logical uplevel source word issuer");
        let words = self
            .logical_providers
            .source_words
            .ok_or_else(unavailable)?;
        let source = dialect
            .logical_source_string_protocol(words, self.native_execution_profile())
            .ok_or_else(unavailable)?;
        if self.source_string_protocol() != Some(source) {
            return Err(unavailable());
        }
        let engine = self.actual_engine_profile.ok_or_else(unavailable)?;
        if self
            .native_core_pin
            .as_ref()
            .is_none_or(|core| !std::ptr::eq(core.profile, engine))
        {
            return Err(unavailable());
        }
        Ok(Some(LogicalUpLevelSourceEntry {
            policy: self.native_compiler_policy(),
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiled::CompilerProvenance;
    use std::{cell::RefCell, io::Write, rc::Rc};
    use tcl_core_types::Code;
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

    fn simulation(eval: bool, words: bool) -> (crate::Vm, Rc<RefCell<Vec<u8>>>) {
        let output = Rc::new(RefCell::new(Vec::new()));
        let host = tcl_dialect::DialectProfile::find("tcl9.0").unwrap();
        let mut vm = crate::native_fixture::interpreter_with_output(
            host,
            Box::new(Capture(Rc::clone(&output))),
        );
        let logical = tcl_dialect::DialectProfile::irules();
        vm.set_dialect_profile(logical);
        assert!(vm.set_command_surface_profile(host));
        assert!(vm.set_native_engine_profile(host));
        vm.set_compiler(Box::new(
            tcl_compiler::compile_service::BytecodeCompileService::for_profile(logical),
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
        assert!(vm.set_logical_name_provider(
            tcl_syntax::naming::NamePolicyProtocol::authored_tcl(tcl_dialect::TclVersion::V8_4)
        ));
        assert!(vm.set_logical_compiled_variable_provider(tcl_registry::native_compiled_variables::LogicalCompiledVariableProvider::Tcl84CoreSimulation));
        assert!(vm.set_logical_numeric_provider(
            tcl_syntax::logical_numeric_simulation::AuthoredLogicalNumericSimulation::Tcl84Core
        ));
        (vm, output)
    }

    #[test]
    fn authored_uplevel_uses_fresh_words_without_a_native_literal_pool() {
        let (mut vm, output) = simulation(true, true);
        let source = Value::new_native_string_bytes(
            b"puts -nonewline reached; set logical_word_effect yes".as_slice(),
        );
        let prepared = vm
            .prepare_script_commands_value_for(&source, EvalObjectPurpose::UpLevel)
            .unwrap();
        let unit = prepared.prefix.unwrap();
        assert!(matches!(
            unit.compiler,
            CompilerProvenance::LogicalUpLevel(_)
        ));
        assert!(unit.literal_pool.is_err());
        assert!(unit.native_cache.is_none());
        assert!(unit.compiled_local_layout.is_none());
        let completion = vm.run_compiled_unit(unit);
        assert!(vm.execution_refusal.is_none());
        assert_eq!(completion.code, Code::Ok);
        assert_eq!(completion.result.string_bytes().as_ref(), b"yes");
        assert_eq!(output.borrow().as_slice(), b"reached");
    }

    #[test]
    fn authored_uplevel_requires_each_explicit_provider_before_execution() {
        for (eval, words) in [(false, true), (true, false)] {
            let (mut vm, output) = simulation(eval, words);
            let source = Value::new_native_string_bytes(b"puts -nonewline forbidden".as_slice());
            assert!(
                vm.prepare_script_commands_value_for(&source, EvalObjectPurpose::UpLevel)
                    .is_err()
            );
            assert!(output.borrow().is_empty());
        }
    }

    #[test]
    fn authored_uplevel_receipt_cannot_enter_after_its_invocation_policy_changes() {
        let (mut vm, output) = simulation(true, true);
        let source = Value::new_native_string_bytes(b"puts -nonewline stale".as_slice());
        let unit = vm
            .prepare_script_commands_value_for(&source, EvalObjectPurpose::UpLevel)
            .unwrap()
            .prefix
            .unwrap();
        let operands = unit.direct_source_operands.unwrap();
        assert!(operands.is_current(&vm.native_compiler_policy()));
        vm.set_dialect_profile(tcl_dialect::DialectProfile::find("tcl8.6").unwrap());
        assert!(!operands.is_current(&vm.native_compiler_policy()));
        let completion = vm.run_compiled_unit(unit);
        assert_eq!(completion.code, Code::Error);
        assert!(output.borrow().is_empty());
    }
}
