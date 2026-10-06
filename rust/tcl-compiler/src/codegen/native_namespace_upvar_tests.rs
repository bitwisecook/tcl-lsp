// SPDX-License-Identifier: AGPL-3.0-or-later
//! C8.5 original namespace-upvar compiler pools, locals and rollback.

use super::*;
use tcl_lexer::{LexerConfig, SourceImage, Span};
use tcl_registry::CommandRegistry;

mod inputs {
    include!("../../../tcl-registry/tests/data/native_namespace_upvar_compilation/cases.rs");
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[test]
fn original_c85_namespace_upvar_preserves_ordered_preparation_and_rollback() {
    let profile = tcl_dialect::DialectProfile::find("tcl8.5").unwrap();
    let registry = CommandRegistry::build_default().project_for_profile(profile);
    let entry = crate::environment_ingress::captured_native_entry(profile);
    let mut count = 0;
    for row in include_str!(
        "../../../tcl-registry/tests/data/native_namespace_upvar_compilation/8.5.19.tsv"
    )
    .lines()
    .skip(1)
    {
        let fields: Vec<_> = row.split('\t').collect();
        let index = fields[0].parse::<usize>().unwrap();
        let (label, source) = inputs::CASES[index];
        let parsed = tcl_lexer::native_script_words_in(
            SourceImage::native(source),
            Span::new(0, u32::try_from(source.len()).unwrap()),
            LexerConfig::from_grammar(profile.grammar),
        )
        .unwrap();
        let mut context = CodegenCtx::new(true, &["pairs", "local"], &registry);
        context.native_entry = Some(&entry);
        context.invocation_dialect = Some(tcl_registry::InvocationDialect::of_profile(profile));
        context.source_string_protocol = entry.source_string_protocol;
        context.compiled_variable_protocol = entry.compiled_variable_protocol;
        context.emit_native_words(&parsed.commands[0].words);
        assert!(!context.native_dependency_refusal, "{label}");
        assert_eq!(
            context
                .instructions
                .iter()
                .filter(|instruction| instruction.op == Op::NSUPVAR)
                .count(),
            fields[1].parse::<usize>().unwrap(),
            "{label}"
        );
        let locals = context
            .lvt
            .native_slot_names()
            .iter()
            .map(|name| hex(name.as_ref().unwrap().as_bytes()))
            .collect::<Vec<_>>();
        assert_eq!(
            locals,
            fields[2].split(',').map(str::to_owned).collect::<Vec<_>>(),
            "{label}: complete native local order"
        );
        let literals = context
            .literals
            .entries()
            .iter()
            .map(|value| hex(value.bytes()))
            .collect::<Vec<_>>();
        assert_eq!(
            literals,
            fields[3].split(',').map(str::to_owned).collect::<Vec<_>>(),
            "{label}: prefix pool survives instruction rollback"
        );
        if fields[1] != "0" || [6, 8, 9, 10, 11].contains(&index) {
            assert!(
                context
                    .instructions
                    .iter()
                    .any(|instruction| instruction.native_compiler_selection.is_some()),
                "{label}: accepted or declined-prefix original compiler guard"
            );
        }
        count += 1;
    }
    assert_eq!(count, 15);
}

#[test]
fn compile_service_preserves_original_namespace_upvar_preparation() {
    use tcl_runtime_api::{CompileService, ProcedureCompileTargetBytes, ProcedureDispatch};
    let profile = tcl_dialect::DialectProfile::find("tcl8.5").unwrap();
    let entry = crate::environment_ingress::captured_native_entry(profile);
    let parameters = [
        tcl_runtime_api::NameBytes::from(b"pairs".as_slice()),
        tcl_runtime_api::NameBytes::from(b"local".as_slice()),
    ];
    let namespace = tcl_runtime_api::ByteNamespacePath::root();
    for index in [3, 10, 11, 13] {
        let (label, source) = inputs::CASES[index];
        let image = SourceImage::native(source);
        let module = crate::compile_service::BytecodeCompileService::for_profile(profile)
            .compile_procedure_bytes_with_entry(
                ProcedureCompileTargetBytes {
                    source: &image,
                    parameters: &parameters,
                    namespace: &namespace,
                },
                profile,
                &entry,
                ProcedureDispatch::Optimised,
            )
            .unwrap_or_else(|error| panic!("{label}: {error}"));
        let row = include_str!(
            "../../../tcl-registry/tests/data/native_namespace_upvar_compilation/8.5.19.tsv"
        )
        .lines()
        .skip(1)
        .nth(index)
        .unwrap()
        .split('\t')
        .collect::<Vec<_>>();
        let locals = module
            .top_level_body
            .lvt
            .native_slot_names()
            .iter()
            .map(|name| hex(name.as_ref().unwrap().as_bytes()))
            .collect::<Vec<_>>();
        assert_eq!(
            locals,
            row[2].split(',').map(str::to_owned).collect::<Vec<_>>(),
            "{label}: actual compile service original visits"
        );
        assert_eq!(
            module
                .top_level_body
                .instructions
                .iter()
                .filter(|instruction| instruction.op == Op::NSUPVAR)
                .count(),
            row[1].parse::<usize>().unwrap(),
            "{label}"
        );
    }
}
