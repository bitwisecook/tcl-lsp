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

//! A named profile or command-surface override against the compile service: fixed, mislabelled and narrower services are refused.

use super::*;

struct FixedFallbackCompilerSvc;

impl CompileService for FixedFallbackCompilerSvc {
    type Module = tcl_bytecode::ModuleAsm;
    fn compile_script_bytes_for_profile(
        &self,
        target: tcl_runtime_api::ScriptCompileTargetBytes<'_>,
        profile: &'static tcl_dialect::DialectProfile,
    ) -> Result<Self::Module, tcl_runtime_api::CompileError> {
        if !profile.is_fallback() {
            return Err(tcl_runtime_api::CompileError::Unsupported(format!(
                "CompileService does not support dialect profile {}",
                profile.name
            )));
        }
        tcl_codegen::compile_service::BytecodeCompileService::default()
            .compile_script_bytes_for_profile(target, profile)
    }
    fn compile_substitution_with_entry(
        &self,
        target: tcl_runtime_api::native_substitution::NativeSubstitutionTarget<'_>,
        profile: &'static tcl_dialect::DialectProfile,
        entry: tcl_runtime_api::native_substitution::NativeSubstitutionCompilationEntry<'_>,
    ) -> Result<Self::Module, tcl_runtime_api::CompileError> {
        if !profile.is_fallback() {
            return Err(tcl_runtime_api::CompileError::Unsupported(format!(
                "CompileService does not support dialect profile {}",
                profile.name
            )));
        }
        tcl_codegen::compile_service::BytecodeCompileService::default()
            .compile_substitution_with_entry(target, profile, entry)
    }

    fn compile_script_bytes_with_entry(
        &self,
        target: tcl_runtime_api::ScriptCompileTargetBytes<'_>,
        profile: &'static tcl_dialect::DialectProfile,
        entry: &tcl_runtime_api::NativeCompilationEntry,
    ) -> Result<Self::Module, tcl_runtime_api::CompileError> {
        if !profile.is_fallback() {
            return Err(tcl_runtime_api::CompileError::Unsupported(format!(
                "CompileService does not support dialect profile {}",
                profile.name
            )));
        }
        tcl_codegen::compile_service::BytecodeCompileService::default()
            .compile_script_bytes_with_entry(target, profile, entry)
    }
    fn compile_plain_script_bytes_for_profile(
        &self,
        target: tcl_runtime_api::ScriptCompileTargetBytes<'_>,
        profile: &'static tcl_dialect::DialectProfile,
    ) -> Result<Self::Module, tcl_runtime_api::CompileError> {
        if !profile.is_fallback() {
            return Err(tcl_runtime_api::CompileError::Unsupported(format!(
                "CompileService does not support dialect profile {}",
                profile.name
            )));
        }
        tcl_codegen::compile_service::BytecodeCompileService::default()
            .compile_plain_script_bytes_for_profile(target, profile)
    }
    fn compile_plain_script_bytes_with_entry(
        &self,
        target: tcl_runtime_api::ScriptCompileTargetBytes<'_>,
        profile: &'static tcl_dialect::DialectProfile,
        entry: &tcl_runtime_api::NativeCompilationEntry,
    ) -> Result<Self::Module, tcl_runtime_api::CompileError> {
        if !profile.is_fallback() {
            return Err(tcl_runtime_api::CompileError::Unsupported(format!(
                "CompileService does not support dialect profile {}",
                profile.name
            )));
        }
        tcl_codegen::compile_service::BytecodeCompileService::default()
            .compile_plain_script_bytes_with_entry(target, profile, entry)
    }
    fn compile_procedure_bytes_for_profile(
        &self,
        target: tcl_runtime_api::ProcedureCompileTargetBytes<'_>,
        profile: &'static tcl_dialect::DialectProfile,
        dispatch: tcl_runtime_api::ProcedureDispatch,
    ) -> Result<Self::Module, tcl_runtime_api::CompileError> {
        if !profile.is_fallback() {
            return Err(tcl_runtime_api::CompileError::Unsupported(format!(
                "CompileService does not support dialect profile {}",
                profile.name
            )));
        }
        tcl_codegen::compile_service::BytecodeCompileService::default()
            .compile_procedure_bytes_for_profile(target, profile, dispatch)
    }
    fn compile_procedure_bytes_with_entry(
        &self,
        target: tcl_runtime_api::ProcedureCompileTargetBytes<'_>,
        profile: &'static tcl_dialect::DialectProfile,
        entry: &tcl_runtime_api::NativeCompilationEntry,
        dispatch: tcl_runtime_api::ProcedureDispatch,
    ) -> Result<Self::Module, tcl_runtime_api::CompileError> {
        if !profile.is_fallback() {
            return Err(tcl_runtime_api::CompileError::Unsupported(format!(
                "CompileService does not support dialect profile {}",
                profile.name
            )));
        }
        tcl_codegen::compile_service::BytecodeCompileService::default()
            .compile_procedure_bytes_with_entry(target, profile, entry, dispatch)
    }
    fn script_command_plan_bytes_for_profile(
        &self,
        source: &tcl_runtime_api::SourceImage,
        profile: &'static tcl_dialect::DialectProfile,
    ) -> Result<tcl_runtime_api::ScriptCommandPlan, tcl_runtime_api::CompileError> {
        if !profile.is_fallback() {
            return Err(tcl_runtime_api::CompileError::Unsupported(format!(
                "CompileService does not support dialect profile {}",
                profile.name
            )));
        }
        tcl_codegen::compile_service::BytecodeCompileService::default()
            .script_command_plan_bytes_for_profile(source, profile)
    }
    fn script_command_plan_bytes_with_entry(
        &self,
        source: &tcl_runtime_api::SourceImage,
        profile: &'static tcl_dialect::DialectProfile,
        entry: &tcl_runtime_api::NativeCompilationEntry,
    ) -> Result<tcl_runtime_api::ScriptCommandPlan, tcl_runtime_api::CompileError> {
        if !profile.is_fallback() {
            return Err(tcl_runtime_api::CompileError::Unsupported(format!(
                "CompileService does not support dialect profile {}",
                profile.name
            )));
        }
        tcl_codegen::compile_service::BytecodeCompileService::default()
            .script_command_plan_bytes_with_entry(source, profile, entry)
    }

    fn compile(&self, src: &str) -> Result<Self::Module, CompileError> {
        BytecodeCompileService::default().compile(src)
    }

    fn compile_procedure_for_profile(
        &self,
        target: ProcedureCompileTarget<'_>,
        profile: &'static DialectProfile,
        dispatch: ProcedureDispatch,
    ) -> Result<Self::Module, CompileError> {
        if profile.is_fallback() {
            BytecodeCompileService::default()
                .compile_procedure_for_profile(target, profile, dispatch)
        } else {
            Err(CompileError::Unsupported(format!(
                "CompileService does not support dialect profile {}",
                profile.name
            )))
        }
    }
}

struct WrongProfileCompilerSvc;

impl CompileService for WrongProfileCompilerSvc {
    type Module = tcl_bytecode::ModuleAsm;
    fn compile_script_bytes_for_profile(
        &self,
        target: tcl_runtime_api::ScriptCompileTargetBytes<'_>,
        _profile: &'static tcl_dialect::DialectProfile,
    ) -> Result<Self::Module, tcl_runtime_api::CompileError> {
        let wrong = tcl_registry::model::ingress::resolve_environment("tcl8.5").analyser_profile();
        tcl_codegen::compile_service::BytecodeCompileService::for_profile(wrong)
            .compile_script_bytes_for_profile(target, wrong)
    }
    fn compile_substitution_with_entry(
        &self,
        target: tcl_runtime_api::native_substitution::NativeSubstitutionTarget<'_>,
        _profile: &'static tcl_dialect::DialectProfile,
        entry: tcl_runtime_api::native_substitution::NativeSubstitutionCompilationEntry<'_>,
    ) -> Result<Self::Module, tcl_runtime_api::CompileError> {
        let wrong = tcl_registry::model::ingress::resolve_environment("tcl8.5").analyser_profile();
        tcl_codegen::compile_service::BytecodeCompileService::for_profile(wrong)
            .compile_substitution_with_entry(target, wrong, entry)
    }

    fn compile_script_bytes_with_entry(
        &self,
        target: tcl_runtime_api::ScriptCompileTargetBytes<'_>,
        _profile: &'static tcl_dialect::DialectProfile,
        _entry: &tcl_runtime_api::NativeCompilationEntry,
    ) -> Result<Self::Module, tcl_runtime_api::CompileError> {
        let wrong = tcl_registry::model::ingress::resolve_environment("tcl8.5").analyser_profile();
        tcl_codegen::compile_service::BytecodeCompileService::for_profile(wrong)
            .compile_script_bytes_for_profile(target, wrong)
    }
    fn compile_plain_script_bytes_for_profile(
        &self,
        target: tcl_runtime_api::ScriptCompileTargetBytes<'_>,
        _profile: &'static tcl_dialect::DialectProfile,
    ) -> Result<Self::Module, tcl_runtime_api::CompileError> {
        let wrong = tcl_registry::model::ingress::resolve_environment("tcl8.5").analyser_profile();
        tcl_codegen::compile_service::BytecodeCompileService::for_profile(wrong)
            .compile_plain_script_bytes_for_profile(target, wrong)
    }
    fn compile_plain_script_bytes_with_entry(
        &self,
        target: tcl_runtime_api::ScriptCompileTargetBytes<'_>,
        _profile: &'static tcl_dialect::DialectProfile,
        _entry: &tcl_runtime_api::NativeCompilationEntry,
    ) -> Result<Self::Module, tcl_runtime_api::CompileError> {
        let wrong = tcl_registry::model::ingress::resolve_environment("tcl8.5").analyser_profile();
        tcl_codegen::compile_service::BytecodeCompileService::for_profile(wrong)
            .compile_plain_script_bytes_for_profile(target, wrong)
    }
    fn compile_procedure_bytes_for_profile(
        &self,
        target: tcl_runtime_api::ProcedureCompileTargetBytes<'_>,
        _profile: &'static tcl_dialect::DialectProfile,
        dispatch: tcl_runtime_api::ProcedureDispatch,
    ) -> Result<Self::Module, tcl_runtime_api::CompileError> {
        let wrong = tcl_registry::model::ingress::resolve_environment("tcl8.5").analyser_profile();
        tcl_codegen::compile_service::BytecodeCompileService::for_profile(wrong)
            .compile_procedure_bytes_for_profile(target, wrong, dispatch)
    }
    fn compile_procedure_bytes_with_entry(
        &self,
        target: tcl_runtime_api::ProcedureCompileTargetBytes<'_>,
        _profile: &'static tcl_dialect::DialectProfile,
        _entry: &tcl_runtime_api::NativeCompilationEntry,
        dispatch: tcl_runtime_api::ProcedureDispatch,
    ) -> Result<Self::Module, tcl_runtime_api::CompileError> {
        let wrong = tcl_registry::model::ingress::resolve_environment("tcl8.5").analyser_profile();
        tcl_codegen::compile_service::BytecodeCompileService::for_profile(wrong)
            .compile_procedure_bytes_for_profile(target, wrong, dispatch)
    }
    fn script_command_plan_bytes_for_profile(
        &self,
        source: &tcl_runtime_api::SourceImage,
        profile: &'static tcl_dialect::DialectProfile,
    ) -> Result<tcl_runtime_api::ScriptCommandPlan, tcl_runtime_api::CompileError> {
        tcl_codegen::compile_service::BytecodeCompileService::for_profile(profile)
            .script_command_plan_bytes_for_profile(source, profile)
    }
    fn script_command_plan_bytes_with_entry(
        &self,
        source: &tcl_runtime_api::SourceImage,
        profile: &'static tcl_dialect::DialectProfile,
        entry: &tcl_runtime_api::NativeCompilationEntry,
    ) -> Result<tcl_runtime_api::ScriptCommandPlan, tcl_runtime_api::CompileError> {
        tcl_codegen::compile_service::BytecodeCompileService::for_profile(profile)
            .script_command_plan_bytes_with_entry(source, profile, entry)
    }

    fn compile(&self, src: &str) -> Result<Self::Module, CompileError> {
        BytecodeCompileService::for_profile(
            tcl_registry::model::ingress::resolve_environment("tcl8.5").analyser_profile(),
        )
        .compile(src)
    }

    fn compile_for_profile(
        &self,
        src: &str,
        _profile: &'static DialectProfile,
    ) -> Result<Self::Module, CompileError> {
        self.compile(src)
    }

    fn compile_procedure_for_profile(
        &self,
        target: ProcedureCompileTarget<'_>,
        _profile: &'static DialectProfile,
        dispatch: ProcedureDispatch,
    ) -> Result<Self::Module, CompileError> {
        let wrong = tcl_registry::model::ingress::resolve_environment("tcl8.5").analyser_profile();
        BytecodeCompileService::for_profile(wrong)
            .compile_procedure_for_profile(target, wrong, dispatch)
    }
}

#[test]
fn named_profiles_reject_fixed_or_mislabelled_compile_services() {
    let v84 = tcl_registry::model::ingress::resolve_environment("tcl8.4").analyser_profile();
    let mut fixed = tcl_vm::test_support::native_vm_for_profile(v84, Box::new(std::io::stdout()));
    fixed.set_compiler(Box::new(FixedFallbackCompilerSvc));
    // The refusal is the host's, outside every guest catch.
    assert_eq!(
        fixed
            .eval_source("set x 1")
            .unwrap_err()
            .message_unicode()
            .expect_err("a host refusal is no guest error")
            .to_string(),
        "CompileService does not support dialect profile tcl8.4"
    );

    let mut mislabelled =
        tcl_vm::test_support::native_vm_for_profile(v84, Box::new(std::io::stdout()));
    mislabelled.set_compiler(Box::new(WrongProfileCompilerSvc));
    // A service that compiles for another profile is refused before it can
    // label a module, as the fixed one is.
    let Err(tcl_vm::VmCompilationError::Host(refusal)) =
        mislabelled.try_compile_function("lassign {a b} x")
    else {
        panic!("a mislabelled compile service is a host refusal");
    };
    assert_eq!(
        refusal.to_string(),
        "CompileService does not support dialect profile tcl8.4"
    );
}

#[test]
fn command_surface_override_accepts_the_universal_fallback_from_tcl91() {
    let v91 = tcl_registry::model::ingress::resolve_environment("tcl9.1").analyser_profile();
    let fallback = DialectProfile::plain_tcl();
    let mut vm = tcl_vm::test_support::native_vm_for_profile(v91, Box::new(std::io::stdout()));

    assert_eq!(fallback.vm_runtime_version, TclVersion::V9_0);
    assert!(
        vm.set_command_surface_profile(fallback),
        "the permissive fallback must cover every Tcl 9.1 builtin despite its inert 9.0 base"
    );
    assert!(std::ptr::eq(vm.command_surface_profile(), fallback));
}

#[test]
fn named_command_surface_override_cannot_hide_compiled_release_commands() {
    let v91 = tcl_registry::model::ingress::resolve_environment("tcl9.1").analyser_profile();
    let v90 = tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile();
    let registry = tcl_registry::model::ingress::static_context_for_profile(v91).commands();
    let config = tcl_lexer::LexerConfig::from_grammar(v91.grammar);
    let ir = lower_to_ir("lassign {a b} x; set x", registry, config, Some(v91));
    let cfg = build_cfg_codegen(&ir, false);
    let module = codegen_module(&cfg, &ir, registry).expect("original source namespace");

    let mut vm = tcl_vm::test_support::native_vm_for_profile(v91, Box::new(std::io::stdout()));
    assert!(
        !vm.set_command_surface_profile(v90),
        "a named Tcl 9.0 surface cannot host Tcl 9.1 bytecode"
    );
    assert!(std::ptr::eq(vm.command_surface_profile(), v91));
    let completion = vm.run_module(&module);
    assert!(completion.code.is_ok(), "{}", completion.result.to_str());
    assert_eq!(&*completion.result.to_str(), "a");
}

/// `eval_source` reports a script it could not run *at all* through `Err`,
/// the channel it has always used, and a script that ran a prefix before a
/// malformed tail through the completion (#1603).
///
/// The boundary is whether a **command** ran, not whether the prefix has
/// bytes: when the first command is the malformed one, the prefix still spans
/// any leading whitespace and comments, so a byte-length test would send
/// `" \n set x \""` down the completion channel having executed nothing.
#[test]
fn eval_source_reports_a_script_that_ran_nothing_through_the_error_channel() {
    let mut vm = tcl_vm::test_support::native_vm();
    vm.set_compiler(Box::new(
        tcl_codegen::compile_service::BytecodeCompileService::default(),
    ));

    // Nothing parses at all.
    assert!(vm.eval_source("set x \"").is_err(), "no command parsed");
    // Nothing parses, behind leading trivia: whitespace, then a comment.
    assert!(
        vm.eval_source(" \n set x \"").is_err(),
        "leading whitespace runs no command"
    );
    assert!(
        vm.eval_source("# just a comment\nset x \"").is_err(),
        "a leading comment runs no command"
    );

    // A prefix that *did* run reports through the completion, carrying the
    // side effect the prefix performed — this is the half the `Err` channel
    // must not swallow.
    let completion = vm
        .eval_source("set ::ran 1\nset x \"")
        .expect("a script that ran a command reports through its completion");
    assert_eq!(completion.code, Code::Error);
    assert_eq!(&*completion.result.to_str(), "missing \"");
    assert_eq!(
        &*vm.eval_source("set ::ran")
            .expect("prefix side effect")
            .result
            .to_str(),
        "1",
        "the clean prefix ran before the parse error"
    );

    // Positive control: a script that parses whole still reports success, so
    // none of the above can pass because `eval_source` started failing.
    let ok = vm.eval_source("set ::fine 2").expect("clean script");
    assert!(ok.code.is_ok(), "{}", ok.result.to_str());
}

#[test]
fn command_surface_override_rejects_a_same_release_narrower_profile() {
    let bpf = tcl_registry::model::ingress::resolve_environment("bpf").analyser_profile();
    let bigip = tcl_registry::model::ingress::resolve_environment("f5-bigip").analyser_profile();
    let registry = tcl_registry::model::ingress::static_context_for_profile(bpf).commands();
    let config = tcl_lexer::LexerConfig::from_grammar(bpf.grammar);
    // `lassign` becomes list/variable opcodes, so it proves that module
    // profile validation alone cannot enforce a later command-surface change.
    let ir = lower_to_ir("lassign {a b} x; set x", registry, config, Some(bpf));
    let cfg = build_cfg_codegen(&ir, false);
    let module = codegen_module(&cfg, &ir, registry).expect("original source namespace");

    let mut vm = tcl_vm::test_support::native_vm();
    vm.set_dialect_profile(bpf);
    assert!(
        !vm.set_command_surface_profile(bigip),
        "the config-only BIG-IP surface hides BPF's Tcl bytecode commands"
    );
    assert!(std::ptr::eq(vm.command_surface_profile(), bpf));
    let completion = vm.run_module(&module);
    assert!(completion.code.is_ok(), "{}", completion.result.to_str());
    assert_eq!(&*completion.result.to_str(), "a");
}

#[test]
fn compatible_irules_host_surface_override_remains_accepted() {
    let irules = DialectProfile::irules();
    let host = tcl_registry::model::ingress::resolve_environment("tcl8.4").analyser_profile();
    let mut vm = tcl_vm::test_support::native_vm();
    vm.set_dialect_profile(irules);
    assert!(vm.set_command_surface_profile(host));
    assert!(std::ptr::eq(vm.command_surface_profile(), host));
}
