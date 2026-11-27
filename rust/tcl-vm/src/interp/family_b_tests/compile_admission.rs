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

//! What the compile service is asked and admits: typed refusal, Jim math and expression preparation, and procedure headers.

use super::*;

struct RefusingCompiler(tcl_runtime_api::CompileError);

impl tcl_runtime_api::CompileService for RefusingCompiler {
    type Module = tcl_bytecode::ModuleAsm;
    fn compile_script_bytes_for_profile(
        &self,
        _target: tcl_runtime_api::ScriptCompileTargetBytes<'_>,
        _profile: &'static tcl_dialect::DialectProfile,
    ) -> Result<Self::Module, tcl_runtime_api::CompileError> {
        Err(self.0.clone())
    }
    fn compile_script_bytes_with_entry(
        &self,
        _target: tcl_runtime_api::ScriptCompileTargetBytes<'_>,
        _profile: &'static tcl_dialect::DialectProfile,
        _entry: &tcl_runtime_api::NativeCompilationEntry,
    ) -> Result<Self::Module, tcl_runtime_api::CompileError> {
        Err(self.0.clone())
    }
    fn compile_plain_script_bytes_for_profile(
        &self,
        _target: tcl_runtime_api::ScriptCompileTargetBytes<'_>,
        _profile: &'static tcl_dialect::DialectProfile,
    ) -> Result<Self::Module, tcl_runtime_api::CompileError> {
        Err(self.0.clone())
    }
    fn compile_plain_script_bytes_with_entry(
        &self,
        _target: tcl_runtime_api::ScriptCompileTargetBytes<'_>,
        _profile: &'static tcl_dialect::DialectProfile,
        _entry: &tcl_runtime_api::NativeCompilationEntry,
    ) -> Result<Self::Module, tcl_runtime_api::CompileError> {
        Err(self.0.clone())
    }
    fn compile_procedure_bytes_for_profile(
        &self,
        _target: tcl_runtime_api::ProcedureCompileTargetBytes<'_>,
        _profile: &'static tcl_dialect::DialectProfile,
        _dispatch: tcl_runtime_api::ProcedureDispatch,
    ) -> Result<Self::Module, tcl_runtime_api::CompileError> {
        Err(tcl_runtime_api::CompileError::Unsupported(
            "procedure compilation unavailable".into(),
        ))
    }
    fn compile_procedure_bytes_with_entry(
        &self,
        _target: tcl_runtime_api::ProcedureCompileTargetBytes<'_>,
        _profile: &'static tcl_dialect::DialectProfile,
        _entry: &tcl_runtime_api::NativeCompilationEntry,
        _dispatch: tcl_runtime_api::ProcedureDispatch,
    ) -> Result<Self::Module, tcl_runtime_api::CompileError> {
        Err(tcl_runtime_api::CompileError::Unsupported(
            "procedure compilation unavailable".into(),
        ))
    }
    fn script_command_plan_bytes_for_profile(
        &self,
        source: &tcl_runtime_api::SourceImage,
        _profile: &'static tcl_dialect::DialectProfile,
    ) -> Result<tcl_runtime_api::ScriptCommandPlan, tcl_runtime_api::CompileError> {
        Ok(tcl_runtime_api::ScriptCommandPlan::complete(source.len()))
    }
    fn script_command_plan_bytes_with_entry(
        &self,
        source: &tcl_runtime_api::SourceImage,
        _profile: &'static tcl_dialect::DialectProfile,
        _entry: &tcl_runtime_api::NativeCompilationEntry,
    ) -> Result<tcl_runtime_api::ScriptCommandPlan, tcl_runtime_api::CompileError> {
        Ok(tcl_runtime_api::ScriptCommandPlan::complete(source.len()))
    }

    fn compile(&self, _: &str) -> Result<Self::Module, tcl_runtime_api::CompileError> {
        Err(self.0.clone())
    }

    fn compile_plain_script_for_profile(
        &self,
        _: tcl_runtime_api::ScriptCompileTarget<'_>,
        _: &'static tcl_dialect::DialectProfile,
    ) -> Result<Self::Module, tcl_runtime_api::CompileError> {
        Err(self.0.clone())
    }
}

#[test]
fn compile_service_capabilities_and_host_definitions_preserve_typed_refusal() {
    use crate::VmCompilationError;
    use tcl_runtime_api::{CompileError, NativeCompilationAdmissionScope, NativeExecutionError};
    let mut vm = Vm::new();
    vm.set_compiler(Box::new(RefusingCompiler(CompileError::Unsupported(
        "provider capability unavailable".into(),
    ))));
    let error = vm
        .try_compile_function("set untouched CHANGED")
        .unwrap_err();
    let VmCompilationError::Host(NativeExecutionError::CompileServiceRefusal(payload)) = error
    else {
        panic!("typed script capability refusal expected: {error:?}")
    };
    assert_eq!(payload.source.bytes(), b"set untouched CHANGED");
    assert_eq!(payload.scope, NativeCompilationAdmissionScope::Script);
    assert_eq!(payload.compilation_namespace, NamespacePath::root());
    assert_eq!(
        payload.native_profile,
        vm.native_execution_profile().cache_key()
    );
    assert_eq!(payload.frame, 0);
    assert!(vm.get_var("untouched").is_none());

    // The service's procedure capability is independently unsupported. As
    // in C, the definition does not compile its body: the typed refusal
    // comes from the first call, before any of the body runs.
    vm.try_define_procedure("missing", &[], "set untouched CHANGED")
        .expect("a procedure definition defers body compilation");
    let error = vm.try_invoke_command("missing", &[]).unwrap_err();
    let NativeExecutionError::CompileServiceRefusal(payload) = error else {
        panic!("typed procedure capability refusal expected: {error:?}")
    };
    assert_eq!(payload.source.bytes(), b"set untouched CHANGED");
    assert_eq!(
        payload.scope,
        NativeCompilationAdmissionScope::ProcedureBody
    );
    assert_eq!(payload.compilation_namespace, NamespacePath::root());
    assert!(vm.get_var("untouched").is_none());

    vm.set_compiler(Box::new(RefusingCompiler(CompileError::Message(
        "provider capability unavailable".into(),
    ))));
    let error = vm
        .try_compile_function("set untouched CHANGED")
        .unwrap_err();
    assert!(matches!(error, VmCompilationError::Tcl(error)
            if error.message_bytes().is_ok_and(|message| message.as_ref() == b"provider capability unavailable")));
}

#[test]
fn jim_math_numeric_results_follow_the_measured_native_protocol() {
    let mut vm = crate::native_fixture::core(release_profile("jim"));
    for (source, expected) in [
        ("pow(2,2)", "4"),
        ("pow(2.0,2)", "4.0"),
        ("pow(2,-1)", "0"),
        ("pow(-1,-3)", "-1"),
        ("pow(2,63)", "-9223372036854775808"),
        ("pow(2,64)", "0"),
        ("abs(-9223372036854775808)", "-9223372036854775808"),
        ("round(0.49999999999999994)", "1"),
        ("round(4503599627370497.0)", "4503599627370498"),
        ("int(1e20)", "-9223372036854775808"),
        ("sqrt(-1)", "NaN"),
        ("fmod(1,0)", "NaN"),
    ] {
        let completion = vm.try_eval_expr(source).unwrap();
        assert_eq!(completion.code, Code::Ok, "{source}");
        assert_eq!(completion.result.to_str().as_ref(), expected, "{source}");
    }
    let completion = vm.try_eval_expr("pow(0,-1)").unwrap();
    assert_eq!(completion.code, Code::Error);
    assert_eq!(
        completion.result.to_str().as_ref(),
        "exponentiation of zero by negative power"
    );
}

#[test]
fn moved_math_handler_keeps_its_registered_implementation_identity() {
    let mut vm = Vm::new();
    assert_eq!(
        vm.try_invoke_command(
            "rename",
            &[
                Value::string("::tcl::mathfunc::abs"),
                Value::string("::savedAbs"),
            ]
        )
        .unwrap()
        .code,
        Code::Ok
    );
    assert_eq!(
        vm.try_invoke_command(
            "interp",
            &[
                Value::string("alias"),
                Value::empty(),
                Value::string("::tcl::mathfunc::abs"),
                Value::empty(),
                Value::string("::savedAbs"),
            ]
        )
        .unwrap()
        .code,
        Code::Ok
    );
    let result = vm.try_eval_expr("abs(-3)").unwrap();
    assert_eq!(result.code, Code::Ok);
    assert_eq!(result.result.to_str().as_ref(), "3");
}

#[test]
fn jim_expression_preparation_uses_native_tree_and_eager_fixed_function_pass() {
    let mut vm = crate::native_fixture::core(release_profile("jim"));
    for (source, expected) in [("abs(pow(2),2)", "4"), ("abs(-3)", "3")] {
        let value = Value::string(source);
        // A warm conventional AST must never bypass native reassociation.
        for _ in 0..2 {
            let node = vm.prepare_expression_value(&value).unwrap();
            let mut ops = ExprEval::new(&mut vm);
            assert_eq!(eval(&node, &mut ops).unwrap().to_str().as_ref(), expected);
        }
    }
    for (source, expected) in [
        ("pow(abs(2,2))", "too many arguments for math function"),
        ("pow(1)", "too few arguments for math function"),
        (
            "0 && future_function(1)",
            "syntax error in expression: \"0 && future_function(1)\"",
        ),
        (
            "1 ? 2 : future_function(1)",
            "syntax error in expression: \"1 ? 2 : future_function(1)\"",
        ),
    ] {
        let completion = vm.try_eval_expr(source).unwrap();
        assert_eq!(completion.code, Code::Error, "{source}");
        assert_eq!(completion.result.to_str().as_ref(), expected, "{source}");
    }
}

#[test]
fn compile_service_admission_preserves_the_neutral_host_channel() {
    use tcl_runtime_api::{CompileError, NativeCompilationAdmissionError, NativeExecutionError};
    let mut vm = Vm::new();
    vm.set_var("before", Value::string("PRIOR")).unwrap();
    vm.set_var("errorInfo", Value::string("ORIGINAL")).unwrap();
    vm.set_compiler(Box::new(RefusingCompiler(
        CompileError::NativeCompilationAdmission(
            NativeCompilationAdmissionError::NativePreflightRequired,
        ),
    )));
    assert_eq!(
        vm.try_eval_source("set after CHANGED").unwrap_err(),
        NativeExecutionError::CompilationAdmission(
            NativeCompilationAdmissionError::NativePreflightRequired,
        )
    );
    assert_eq!(vm.get_var("before").unwrap().to_str().as_ref(), "PRIOR");
    assert_eq!(
        vm.get_var("errorInfo").unwrap().to_str().as_ref(),
        "ORIGINAL"
    );
    assert!(vm.get_var("after").is_none());
    // An ordinary compiler diagnostic is still a Tcl failure. Its bytes
    // may equal a host diagnostic without acquiring the host's meaning.
    vm.set_compiler(Box::new(RefusingCompiler(CompileError::Message(
        NativeCompilationAdmissionError::NativePreflightRequired.to_string(),
    ))));
    let completion = vm.try_eval_source("set after CHANGED").unwrap();
    assert_eq!(completion.code, Code::Error);
    assert_eq!(
        completion.result.to_str().as_ref(),
        "native compiler preflight provider required"
    );
    assert!(vm.get_var("after").is_none());
}

#[test]
fn procedure_header_admission_retains_lookup_and_callable_incarnations() {
    use tcl_runtime_api::native_compilation::NativeProcedureHeaderPrerequisite;
    let mut vm = Vm::new();
    vm.set_dialect_profile(tcl_dialect::DialectProfile::find("tcl8.6").unwrap());
    run(&mut vm, &["proc", "noop", "args", ""]);
    vm.activate_namespace_operand(b"::N").unwrap();
    let entry = vm
        .native_compilation_entry_for_namespace("N", false)
        .unwrap();
    let callable = entry
        .commands
        .iter()
        .find(|row| row.slot.simple == "noop")
        .unwrap();
    let required = NativeProcedureHeaderPrerequisite {
        interpreter: entry.interpreter,
        lookup_namespace_token: entry.current_namespace,
        invocation_word: "noop".into(),
        slot: callable.slot.clone(),
        namespace_token: callable.namespace_token,
        token: callable.token,
        implementation_generation: callable.implementation_generation,
        header: tcl_dialect::NativeProcedureHeaderCompilation::NoOp,
        guard: tcl_runtime_api::CommandBindingGuard::BeforeArguments,
    };
    assert!(vm.native_procedure_header_prerequisite_matches(&required));
    let mut foreign = required.clone();
    foreign.lookup_namespace_token = u64::MAX;
    assert!(!vm.native_procedure_header_prerequisite_matches(&foreign));
    foreign = required.clone();
    foreign.interpreter.owner += 1;
    assert!(!vm.native_procedure_header_prerequisite_matches(&foreign));
    run(&mut vm, &["proc", "N::noop", "args", "return LOCAL"]);
    assert!(!vm.native_procedure_header_prerequisite_matches(&required));
    let mut absolute = required.clone();
    absolute.invocation_word = "::noop".into();
    assert!(vm.native_procedure_header_prerequisite_matches(&absolute));
    run(&mut vm, &["rename", "noop", ""]);
    run(&mut vm, &["proc", "noop", "args", ""]);
    assert!(!vm.native_procedure_header_prerequisite_matches(&absolute));
}
