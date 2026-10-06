// SPDX-License-Identifier: AGPL-3.0-or-later
//! Native iterator auxiliary and local-allocation compiler comparisons.

use super::*;
use tcl_lexer::{LexerConfig, SourceImage, Span};
use tcl_registry::CommandRegistry;

mod inputs {
    include!("../../../tcl-registry/tests/data/native_each_try_compilation/cases.rs");
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[test]
fn original_each_emission_matches_85_native_auxiliary_and_local_windows() {
    let engines = [
        (
            "tcl8.4",
            include_str!("../../../tcl-registry/tests/data/native_each_try_compilation/8.4.20.tsv"),
        ),
        (
            "tcl8.5",
            include_str!("../../../tcl-registry/tests/data/native_each_try_compilation/8.5.19.tsv"),
        ),
        (
            "tcl8.6",
            include_str!("../../../tcl-registry/tests/data/native_each_try_compilation/8.6.18.tsv"),
        ),
        (
            "tcl9.0",
            include_str!("../../../tcl-registry/tests/data/native_each_try_compilation/9.0.4.tsv"),
        ),
        (
            "tcl9.1",
            include_str!("../../../tcl-registry/tests/data/native_each_try_compilation/9.1.0.tsv"),
        ),
    ];
    let mut compared = 0;
    for (profile, fixture) in engines {
        let profile = tcl_dialect::DialectProfile::find(profile).unwrap();
        let registry = CommandRegistry::build_default().project_for_profile(profile);
        let entry = crate::environment_ingress::captured_native_entry(profile);
        for row in fixture.lines().skip(1).take(19) {
            let columns = row.split('\t').collect::<Vec<_>>();
            let index = columns[0].parse::<usize>().unwrap();
            if matches!(index, 13 | 14) {
                continue;
            }
            let (label, source) = inputs::CASES[index];
            let image = SourceImage::native(source);
            let parsed = tcl_lexer::native_script_words_in(
                image.clone(),
                Span::new(0, u32::try_from(image.len()).unwrap()),
                LexerConfig::from_grammar(profile.grammar),
            )
            .unwrap();
            let mut context = CodegenCtx::new(true, &[], &registry);
            context.native_entry = Some(&entry);
            context.invocation_dialect = Some(tcl_registry::InvocationDialect::of_profile(profile));
            context.source_string_protocol = entry.source_string_protocol;
            context.native_compilation.frame =
                tcl_registry::native_compilation::NativeCompilationFrame::ProcedureCode;
            for command in parsed.commands {
                context.emit_native_words(&command.words);
                assert!(
                    !context.native_dependency_refusal,
                    "{}/{label}: original command head {}",
                    profile.name,
                    command
                        .words
                        .first()
                        .map_or_else(String::new, |word| hex(word.bytes())),
                );
            }
            assert!(
                !context.native_dependency_refusal,
                "{}/{label}",
                profile.name
            );
            let native_inline = columns[4].split(',').any(|instruction| {
                instruction.ends_with(":foreach_start") || instruction.ends_with(":foreach_start4")
            });
            let actual_inline = context
                .instructions
                .iter()
                .any(|instruction| instruction.native_each.is_some());
            assert_eq!(
                actual_inline, native_inline,
                "{}/{label}: native auxiliary selection",
                profile.name
            );
            let actual_names = context
                .lvt
                .native_slot_names()
                .iter()
                .map(|name| {
                    name.as_ref()
                        .map_or_else(String::new, |name| hex(name.as_bytes()))
                })
                .collect::<Vec<_>>();
            let native_names = if columns[3].is_empty() {
                Vec::new()
            } else {
                columns[3].split(',').map(str::to_owned).collect()
            };
            assert_eq!(
                actual_names, native_names,
                "{}/{label}: complete native local preparation",
                profile.name
            );
            compared += 1;
        }
    }
    assert_eq!(compared, 85);
}

#[test]
fn compile_service_visits_original_each_values_before_body_locals() {
    use tcl_runtime_api::{CompileService, ProcedureCompileTargetBytes, ProcedureDispatch};

    let engines = [
        (
            "tcl8.4",
            include_str!("../../../tcl-registry/tests/data/native_each_try_compilation/8.4.20.tsv"),
        ),
        (
            "tcl8.5",
            include_str!("../../../tcl-registry/tests/data/native_each_try_compilation/8.5.19.tsv"),
        ),
        (
            "tcl8.6",
            include_str!("../../../tcl-registry/tests/data/native_each_try_compilation/8.6.18.tsv"),
        ),
        (
            "tcl9.0",
            include_str!("../../../tcl-registry/tests/data/native_each_try_compilation/9.0.4.tsv"),
        ),
        (
            "tcl9.1",
            include_str!("../../../tcl-registry/tests/data/native_each_try_compilation/9.1.0.tsv"),
        ),
    ];
    for (environment, fixture) in engines {
        let profile = tcl_dialect::DialectProfile::find(environment).unwrap();
        let entry = crate::environment_ingress::captured_native_entry(profile);
        let (label, source) = inputs::CASES[10];
        assert_eq!(label, "each-values-before-body");
        let image = SourceImage::native(source);
        let namespace = tcl_runtime_api::ByteNamespacePath::root();
        let module = crate::compile_service::BytecodeCompileService::for_profile(profile)
            .compile_procedure_bytes_with_entry(
                ProcedureCompileTargetBytes {
                    source: &image,
                    parameters: &[],
                    namespace: &namespace,
                },
                profile,
                &entry,
                ProcedureDispatch::Optimised,
            )
            .unwrap_or_else(|error| panic!("{environment}/{label}: {error}"));
        let row = fixture
            .lines()
            .skip(1)
            .find(|row| row.starts_with("10\t"))
            .unwrap();
        let native_names = row
            .split('\t')
            .nth(3)
            .unwrap()
            .split(',')
            .map(str::to_owned)
            .collect::<Vec<_>>();
        let actual_names = module
            .top_level_body
            .lvt
            .native_slot_names()
            .iter()
            .map(|name| {
                name.as_ref()
                    .map_or_else(String::new, |name| hex(name.as_bytes()))
            })
            .collect::<Vec<_>>();
        assert_eq!(
            actual_names, native_names,
            "{environment}/{label}: full compilation visits"
        );
        assert_eq!(
            module
                .top_level_body
                .instructions
                .iter()
                .filter(|instruction| instruction.native_each.is_some())
                .count(),
            1,
            "{environment}/{label}: one original auxiliary compiler visit"
        );
    }
}
