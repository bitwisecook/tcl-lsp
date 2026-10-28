// SPDX-License-Identifier: AGPL-3.0-or-later
//! Bracket scripts under an authored naming simulation compile generically;
//! a native policy keeps native scheduling and its refusal.

use super::*;

const SOURCE: &[u8] = b"set y [llength [info commands ::nope]]";

fn entry(
    name_protocol: impl FnOnce(
        tcl_dialect::model::DialectPoint,
    ) -> tcl_syntax::naming::NamePolicyProtocol,
    string_protocol: bool,
) -> crate::environment_ingress::CapturedNativeEntry {
    let profile = tcl_dialect::DialectProfile::find("tcl9.0").unwrap();
    let mut entry = crate::environment_ingress::captured_native_entry(profile);
    let point = entry.execution_point.unwrap();
    entry.name_protocol = Some(name_protocol(point));
    if !string_protocol {
        entry.source_string_protocol = None;
    }
    entry
}

fn emit_bracket_word<'a>(
    registry: &'a tcl_registry::CommandRegistry,
    entry: &'a tcl_runtime_api::NativeCompilationEntry,
) -> CodegenCtx<'a> {
    let dialect = tcl_registry::InvocationDialect::of_point(entry.execution_point.unwrap());
    let parsed = tcl_lexer::native_script_words_in(
        tcl_lexer::SourceImage::native(SOURCE),
        tcl_lexer::Span::new(0, u32::try_from(SOURCE.len()).unwrap()),
        tcl_lexer::LexerConfig::from_grammar(dialect.lexer_grammar),
    )
    .unwrap();
    let mut ctx = CodegenCtx::new(false, &[], registry);
    ctx.native_entry = Some(entry);
    ctx.invocation_dialect = Some(dialect);
    ctx.source_string_protocol = dialect.native_source_string_protocol();
    ctx.compiled_variable_protocol = entry.compiled_variable_protocol;
    ctx.emit_original_native_word(&parsed.commands[0].words[2]);
    ctx
}

fn native(point: tcl_dialect::model::DialectPoint) -> tcl_syntax::naming::NamePolicyProtocol {
    tcl_syntax::naming::NamePolicyProtocol::for_native_point(point).unwrap()
}

fn authored(_: tcl_dialect::model::DialectPoint) -> tcl_syntax::naming::NamePolicyProtocol {
    tcl_syntax::naming::NamePolicyProtocol::authored_tcl(tcl_dialect::TclVersion::V8_4)
}

#[test]
fn an_authored_name_policy_compiles_nested_brackets_by_generic_dispatch() {
    let registry = tcl_registry::CommandRegistry::build_default();
    for string_protocol in [true, false] {
        let entry = entry(authored, string_protocol);
        let ctx = emit_bracket_word(&registry, &entry);
        assert!(!ctx.native_dependency_refusal, "{string_protocol}");
        assert!(
            ctx.literals
                .entries()
                .iter()
                .any(|literal| literal.bytes() == b"info"),
            "the nested head is dispatched by name"
        );
    }
}

#[test]
fn a_native_name_policy_keeps_native_scheduling_and_its_refusal() {
    let registry = tcl_registry::CommandRegistry::build_default();
    let current = entry(native, true);
    assert!(!emit_bracket_word(&registry, &current).native_dependency_refusal);
    // The same entry without its source string protocol cannot select a
    // native recipe; the bracket script still requires the provider.
    let unprepared = entry(native, false);
    assert!(emit_bracket_word(&registry, &unprepared).native_dependency_refusal);
    // A native policy of another point is still native authority: its recipe
    // mismatch refuses rather than taking the authored generic route.
    let foreign = entry(
        |_| {
            tcl_syntax::naming::NamePolicyProtocol::for_native_point(
                tcl_dialect::model::DialectPoint::for_tcl_version(tcl_dialect::TclVersion::V8_6),
            )
            .unwrap()
        },
        true,
    );
    assert_ne!(
        foreign.command_name_policy(),
        Some(native(foreign.execution_point.unwrap()))
    );
    assert!(emit_bracket_word(&registry, &foreign).native_dependency_refusal);
}

/// The iRules shape: a C9 core running an authored C8.4 naming simulation.
fn authored_vm() -> tcl_vm::Vm {
    let host = tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile();
    let mut vm = tcl_vm::Vm::with_native_core(
        Box::new(std::io::sink()),
        std::rc::Rc::new(tcl_vm::host_native::NativeHost::new()),
        host,
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .expect("C9 core");
    let profile = tcl_dialect::DialectProfile::irules();
    vm.set_dialect_profile(profile);
    assert!(vm.set_command_surface_profile(host));
    vm.set_compiler(Box::new(
        crate::compile_service::BytecodeCompileService::for_profile(profile),
    ));
    assert!(vm.set_native_engine_profile(host));
    assert!(vm.set_logical_eval_object_provider(
        tcl_registry::native_eval_object::LogicalEvalObjectProvider::Tcl84CoreSimulation
    ));
    assert!(vm.set_logical_source_word_provider(
        tcl_registry::invocation_words::LogicalSourceWordProvider::Tcl84CoreSimulation
    ));
    assert!(vm.set_logical_expression_parse_provider(
        tcl_registry::invocation_words::LogicalExpressionParseProvider::Tcl84CoreSimulation,
    ));
    assert!(vm.set_logical_quote_provider(
        tcl_registry::invocation_words::LogicalExpressionQuoteProvider::Tcl84CoreSimulation,
    ));
    assert!(vm.set_logical_numeric_provider(
        tcl_syntax::logical_numeric_simulation::AuthoredLogicalNumericSimulation::Tcl84Core
    ));
    assert!(
        vm.set_logical_name_provider(tcl_syntax::naming::NamePolicyProtocol::authored_tcl(
            tcl_dialect::TclVersion::V8_4
        ))
    );
    assert!(vm.set_logical_compiled_variable_provider(
        tcl_registry::native_compiled_variables::LogicalCompiledVariableProvider::Tcl84CoreSimulation
    ));
    assert!(vm.set_logical_source_file_provider(Some(
        tcl_registry::source_file::LogicalSourceFileProvider::Tcl84CoreSimulation,
    )));
    assert!(vm.set_logical_math_function_provider(Some(
        tcl_runtime_api::expression_policy::AuthoredMathFunctionProvider::Tcl84Core,
    )));
    vm
}

/// What `script` answers, or `None` when the VM refuses it.
fn answered(vm: &mut tcl_vm::Vm, script: &str) -> Option<String> {
    let completion = vm.eval_source(script).ok()?;
    assert_eq!(completion.code, tcl_vm::Code::Ok, "{script}");
    Some(completion.result.to_str().to_string())
}

/// C Tcl dispatches a bracket child through the command's current binding: a
/// procedure compiled before `llength` is replaced answers the replacement.
/// Bytecode compiled for the builtin must never answer `0` afterwards.
#[test]
fn an_authored_bracket_child_never_runs_a_superseded_builtin() {
    let mut vm = authored_vm();
    let child = "set y [llength [info commands ::nope]]";
    assert_eq!(answered(&mut vm, child).as_deref(), Some("0"));
    assert!(answered(&mut vm, &format!("proc p {{}} {{{child}}}")).is_some());
    assert_eq!(answered(&mut vm, "p").as_deref(), Some("0"));
    assert!(
        answered(
            &mut vm,
            "rename llength ::saved_llength; proc llength args {return rebound}"
        )
        .is_some()
    );
    for script in [child, "p"] {
        let after = answered(&mut vm, script);
        assert!(
            after.is_none() || after.as_deref() == Some("rebound"),
            "{script}: {after:?}"
        );
    }
}
