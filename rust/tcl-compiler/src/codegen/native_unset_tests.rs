// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original unset opcode and compiler-prerequisite consumers.

use super::*;

fn emit<'a>(
    registry: &'a tcl_registry::CommandRegistry,
    entry: &'a tcl_runtime_api::NativeCompilationEntry,
    source: &[u8],
) -> CodegenCtx<'a> {
    let dialect = tcl_registry::InvocationDialect::of_point(entry.execution_point.unwrap());
    let parsed = tcl_lexer::native_script_words_in(
        tcl_lexer::SourceImage::native(source),
        tcl_lexer::Span::new(0, u32::try_from(source.len()).unwrap()),
        tcl_lexer::LexerConfig::from_grammar(dialect.lexer_grammar),
    )
    .unwrap();
    let mut ctx = CodegenCtx::new(true, &["name", "other", "names"], registry);
    ctx.native_entry = Some(entry);
    ctx.invocation_dialect = Some(dialect);
    ctx.source_string_protocol = entry.source_string_protocol;
    ctx.compiled_variable_protocol = entry.compiled_variable_protocol;
    ctx.native_compilation.frame =
        tcl_registry::native_compilation::NativeCompilationFrame::ProcedureCode;
    ctx.emit_native_words(&parsed.commands[0].words);
    ctx
}

#[test]
fn original_unset_emits_native_receivers_and_sequential_callback_operands() {
    for name in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let profile = tcl_dialect::DialectProfile::find(name).unwrap();
        let registry = tcl_registry::CommandRegistry::build_default().project_for_profile(profile);
        let entry = crate::environment_ingress::captured_native_entry(profile);
        let modern =
            entry.execution_point.unwrap().tcl_version().unwrap() >= tcl_dialect::TclVersion::V8_6;
        for (source, opcodes) in [
            (
                b"unset x a(k) ::g".as_slice(),
                vec![Op::UNSET_SCALAR, Op::UNSET_ARRAY, Op::UNSET_STK],
            ),
            (b"unset -- $name $other", vec![Op::UNSET_STK, Op::UNSET_STK]),
            (
                b"unset a([set ::seen FIRST]) b([set ::seen SECOND])",
                vec![Op::UNSET_ARRAY, Op::UNSET_ARRAY],
            ),
        ] {
            let ctx = emit(&registry, &entry, source);
            assert!(!ctx.native_dependency_refusal, "{name}: {source:?}");
            let actual: Vec<_> = ctx
                .instructions
                .iter()
                .filter_map(|instruction| {
                    matches!(
                        instruction.op,
                        Op::UNSET_SCALAR | Op::UNSET_ARRAY | Op::UNSET_STK | Op::UNSET_ARRAY_STK
                    )
                    .then_some(instruction.op)
                })
                .collect();
            assert_eq!(
                actual,
                if modern { opcodes } else { Vec::new() },
                "{name}: {source:?}"
            );
            if modern {
                assert!(
                    ctx.instructions
                        .iter()
                        .any(|instruction| instruction.native_compiler_selection.is_some())
                );
            }
        }
        let declined = emit(&registry, &entry, b"unset x $name");
        assert!(!declined.native_dependency_refusal);
        assert!(
            !declined
                .instructions
                .iter()
                .any(|instruction| matches!(instruction.op, Op::UNSET_STK | Op::UNSET_SCALAR))
        );
    }
}

#[test]
fn original_unset_changed_worker_keeps_generic_and_unknown_hook_withdrawal() {
    use tcl_runtime_api::native_compilation::{
        NativeCommandImplementation, NativeCompilerHookPresence,
    };
    let profile = tcl_dialect::DialectProfile::find("tcl9.0").unwrap();
    let registry = tcl_registry::CommandRegistry::build_default().project_for_profile(profile);
    let mut entry = crate::environment_ingress::captured_native_entry(profile);
    let token = entry
        .lookup_command_bytes(entry.current_namespace, b"unset")
        .unwrap()
        .unwrap()
        .token;
    let target = entry
        .commands
        .iter_mut()
        .find(|binding| binding.token == token)
        .unwrap();
    target.token += 10_000;
    target.implementation_generation += 1;
    target.implementation = NativeCommandImplementation::Opaque;
    target.compiler_hook = NativeCompilerHookPresence::Absent;
    target.compiler = None;
    let ctx = emit(&registry, &entry, b"unset x");
    assert!(!ctx.native_dependency_refusal);
    assert!(
        !ctx.instructions
            .iter()
            .any(|instruction| instruction.op == Op::UNSET_SCALAR)
    );
    entry
        .commands
        .iter_mut()
        .find(|binding| binding.slot.simple.as_bytes() == b"unset")
        .unwrap()
        .compiler_hook = NativeCompilerHookPresence::Unknown;
    assert!(emit(&registry, &entry, b"unset x").native_dependency_refusal);
}
