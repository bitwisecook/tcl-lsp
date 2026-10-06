// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original native switch arm emission on the same compiler work stack.
use super::{CodegenCtx, NativeEmissionTask as Task, Op, Operand};
use tcl_registry::native_switch_compilation::{
    NativeSwitchInstruction, NativeSwitchMatch, NativeSwitchMode,
};
impl CodegenCtx<'_> {
    fn native_switch_table_tasks(
        recipe: &NativeSwitchInstruction,
        image: &tcl_lexer::SourceImage,
        config: tcl_lexer::LexerConfig,
        labels: &[String],
        end: &str,
        fallback: &str,
    ) -> Vec<Task> {
        let mut tasks = Vec::new();
        if recipe.nocase && recipe.mode == NativeSwitchMode::Exact {
            tasks.push(Task::SwitchOperation(Op::STR_LOWER, vec![], recipe.version));
        }
        let mut bytes = std::collections::HashMap::new();
        let mut integers = std::collections::HashMap::new();
        for (index, arm) in recipe.arms.iter().enumerate() {
            if recipe.terminal_default && index + 1 == recipe.arms.len() {
                continue;
            }
            let label = labels[arm.target].clone();
            if let Some(integer) = arm.integer {
                integers.entry(integer).or_insert(label);
            } else {
                let key = if recipe.nocase {
                    tcl_syntax::native_glob::lower_c_string_bytes(recipe.version, &arm.pattern)
                } else {
                    tcl_core_types::c_string_extent(&arm.pattern).to_vec()
                };
                bytes.entry(key).or_insert(label);
            }
        }
        tasks.push(Task::SwitchTable(
            recipe.version,
            recipe.mode == NativeSwitchMode::Integer,
            bytes,
            integers,
        ));
        let default = if recipe.terminal_default {
            labels.last().unwrap().clone()
        } else {
            fallback.to_owned()
        };
        tasks.push(Task::Operation(Op::JUMP4, vec![Operand::Label(default)]));
        for (index, arm) in recipe.arms.iter().enumerate() {
            if arm.body.is_none() || !arm.compile_body {
                continue;
            }
            tasks.push(Task::Label(labels[index].clone()));
            tasks.push(Task::Script(image.clone(), arm.body.unwrap(), config));
            tasks.push(Task::Operation(
                Op::JUMP4,
                vec![Operand::Label(end.to_owned())],
            ));
        }
        tasks
    }
    fn native_switch_match_tasks(
        &mut self,
        recipe: &NativeSwitchInstruction,
        image: &tcl_lexer::SourceImage,
        config: tcl_lexer::LexerConfig,
        labels: &[String],
        end: &str,
    ) -> Vec<Task> {
        let mut tasks = Vec::new();
        for (index, arm) in recipe.arms.iter().enumerate() {
            let next = self.fresh_label("native_switch_next");
            match &arm.matcher {
                NativeSwitchMatch::Always => {
                    let op = if recipe.terminal_default && index + 1 == recipe.arms.len() {
                        Op::JUMP4
                    } else {
                        tasks.push(Task::Literal(b"1".to_vec()));
                        Op::JUMP_TRUE4
                    };
                    tasks.push(Task::Operation(
                        op,
                        vec![Operand::Label(labels[arm.target].clone())],
                    ));
                }
                NativeSwitchMatch::Glob(pattern)
                | NativeSwitchMatch::Equal(pattern)
                | NativeSwitchMatch::Regexp(pattern) => {
                    tasks.push(Task::Literal(pattern.clone()));
                    tasks.push(Task::Operation(Op::OVER, vec![Operand::Imm(1)]));
                    let (op, flags) = match &arm.matcher {
                        NativeSwitchMatch::Glob(_) => {
                            (Op::STR_MATCH, vec![Operand::Imm(i32::from(recipe.nocase))])
                        }
                        NativeSwitchMatch::Regexp(_) => (
                            Op::REGEXP,
                            vec![Operand::Imm(3 | if recipe.nocase { 8 } else { 0 })],
                        ),
                        _ => (Op::STR_EQ, vec![]),
                    };
                    tasks.push(Task::SwitchOperation(op, flags, recipe.version));
                    tasks.push(Task::Operation(
                        Op::JUMP_TRUE4,
                        vec![Operand::Label(labels[arm.target].clone())],
                    ));
                }
                _ => unreachable!("table matcher"),
            }
            tasks.push(Task::Label(next));
            // Native bodies are compiled in arm order, after each test;
            // a fallthrough test jumps directly to the next real body.
            if let Some(body) = arm.body {
                let skip = self.fresh_label("native_switch_skip_body");
                tasks.push(Task::Operation(
                    Op::JUMP4,
                    vec![Operand::Label(skip.clone())],
                ));
                tasks.push(Task::Label(labels[index].clone()));
                tasks.push(Task::Operation(Op::POP, vec![]));
                tasks.push(Task::Script(image.clone(), body, config));
                tasks.push(Task::Operation(
                    Op::JUMP4,
                    vec![Operand::Label(end.to_owned())],
                ));
                tasks.push(Task::Label(skip));
            }
        }
        tasks.push(Task::Operation(Op::POP, vec![]));
        tasks
    }
    pub(super) fn native_switch_tasks(
        &mut self,
        command: &tcl_lexer::NativeScriptCommandWords,
        recipe: &NativeSwitchInstruction,
    ) -> Vec<Task> {
        use tcl_registry::native_compiler_word_projection::NativeCompilerWordOperand;
        let image = command.words[0].image().clone();
        let config = command.words[0].config();
        let mut tasks = vec![match &recipe.subject {
            NativeCompilerWordOperand::Original(index) => Task::Word(command.words[*index].clone()),
            NativeCompilerWordOperand::LiteralExpansion { value, .. } => {
                Task::Literal(value.clone())
            }
        }];
        let end = self.fresh_label("native_switch_end");
        let fallback = self.fresh_label("native_switch_default");
        let labels = recipe
            .arms
            .iter()
            .map(|_| self.fresh_label("native_switch_arm"))
            .collect::<Vec<_>>();
        let table = matches!(
            recipe.mode,
            NativeSwitchMode::Exact | NativeSwitchMode::Integer
        );
        tasks.extend(if table {
            Self::native_switch_table_tasks(recipe, &image, config, &labels, &end, &fallback)
        } else {
            self.native_switch_match_tasks(recipe, &image, config, &labels, &end)
        });
        if !recipe.terminal_default {
            tasks.push(Task::Label(fallback));
            tasks.push(Task::Literal(Vec::new()));
        }
        tasks.push(Task::Label(end));
        tasks
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_lexer::Span;
    use tcl_registry::CommandRegistry;
    fn bytes(hex: &str) -> Vec<u8> {
        hex.as_bytes()
            .chunks_exact(2)
            .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
            .collect()
    }
    #[test]
    fn unentered_switch_arms_keep_original_cpp_namespace_and_return_operands() {
        use tcl_runtime_api::CompileService;
        for name in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let profile = tcl_dialect::DialectProfile::find(name).unwrap();
            let entry = crate::environment_ingress::captured_native_entry(profile);
            let service = crate::compile_service::BytecodeCompileService::for_profile(profile);
            let source = tcl_lexer::SourceImage::native(
                b"switch $x {A {return ONE} default {return OTHER}}".as_slice(),
            );
            let parameters = [tcl_core_types::NameBytes::from("x")];
            let namespace = tcl_core_types::ByteNamespacePath::root();
            let module = service
                .compile_procedure_bytes_with_entry(
                    tcl_runtime_api::ProcedureCompileTargetBytes {
                        source: &source,
                        parameters: &parameters,
                        namespace: &namespace,
                    },
                    profile,
                    &entry,
                    tcl_runtime_api::ProcedureDispatch::Optimised,
                )
                .expect("original compiler visits both arms without entering either");
            for value in [b"ONE".as_slice(), b"OTHER".as_slice()] {
                assert!(
                    module
                        .top_level
                        .literals
                        .entries()
                        .iter()
                        .any(|literal| literal.bytes() == value),
                    "{name}: {value:?}"
                );
            }
            assert!(
                !module
                    .top_level
                    .instructions
                    .iter()
                    .any(|instruction| instruction.comment.starts_with("barrier:")),
                "{name}"
            );
            assert_eq!(module.source, source);
            module
                .top_level
                .validate_native_compilation_entry()
                .unwrap();
        }
    }

    #[test]
    fn compile_service_keeps_masked_switch_arms_out_of_executable_visits() {
        use tcl_runtime_api::CompileService;
        for name in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let profile = tcl_dialect::DialectProfile::find(name).unwrap();
            let entry = crate::environment_ingress::captured_native_entry(profile);
            let service = crate::compile_service::BytecodeCompileService::for_profile(profile);
            let source = tcl_lexer::SourceImage::native(
                b"switch $x {A {return ONE} A {error MASKED} default {return OTHER}}".as_slice(),
            );
            let parameters = [tcl_core_types::NameBytes::from("x")];
            let namespace = tcl_core_types::ByteNamespacePath::root();
            let module = service
                .compile_procedure_bytes_with_entry(
                    tcl_runtime_api::ProcedureCompileTargetBytes {
                        source: &source,
                        parameters: &parameters,
                        namespace: &namespace,
                    },
                    profile,
                    &entry,
                    tcl_runtime_api::ProcedureDispatch::Optimised,
                )
                .expect("native duplicate arm masking retains original compiler visits");
            assert!(
                module
                    .top_level
                    .literals
                    .entries()
                    .iter()
                    .any(|value| value.bytes() == b"ONE")
            );
            assert!(
                module
                    .top_level
                    .literals
                    .entries()
                    .iter()
                    .any(|value| value.bytes() == b"OTHER")
            );
            assert!(
                !module
                    .top_level
                    .literals
                    .entries()
                    .iter()
                    .any(|value| value.bytes() == b"MASKED")
            );
            module
                .top_level
                .validate_native_compilation_entry()
                .unwrap();
        }
    }

    #[test]
    fn native_switch_emission_matches_56_original_compiler_frontiers() {
        let mut compared = 0;
        for row in include_str!("../../../tcl-registry/tests/data/registered-switch56.tsv").lines()
        {
            let fields = row.split('\t').collect::<Vec<_>>();
            let profile = tcl_dialect::DialectProfile::find(match fields[0] {
                "8.5.19" => "tcl8.5",
                "8.6.18" => "tcl8.6",
                "9.0.4" => "tcl9.0",
                "9.1.0" => "tcl9.1",
                _ => panic!("native version"),
            })
            .unwrap();
            let registry = CommandRegistry::build_default().project_for_profile(profile);
            let entry = crate::environment_ingress::captured_native_entry(profile);
            let image = tcl_lexer::SourceImage::native(bytes(fields[2]));
            let script = tcl_lexer::native_script_words_in(
                image.clone(),
                Span::new(0, image.len() as u32),
                tcl_lexer::LexerConfig::from_grammar(profile.grammar),
            )
            .unwrap();
            let mut context = CodegenCtx::new(true, &["x"], &registry);
            context.native_entry = Some(&entry);
            context.invocation_dialect = Some(tcl_registry::InvocationDialect::of_profile(profile));
            context.source_string_protocol = entry.source_string_protocol;
            context.native_compilation.frame =
                tcl_registry::native_compilation::NativeCompilationFrame::ProcedureCode;
            context.emit_native_words(&script.commands[0].words);
            assert!(
                !context.native_dependency_refusal,
                "{}/{}",
                fields[0], fields[1]
            );
            let selected = context
                .instructions
                .iter()
                .any(|instruction| instruction.native_switch_version.is_some());
            assert_eq!(
                selected,
                fields[5] == "inline",
                "{}/{}",
                fields[0],
                fields[1]
            );
            if selected {
                assert!(
                    !context.instructions.iter().any(|instruction| matches!(
                        instruction.op,
                        Op::INVOKE_STK1 | Op::INVOKE_STK4
                    )),
                    "inline Switch retains original body instructions: {}/{}",
                    fields[0],
                    fields[1]
                );
                assert!(
                    !context
                        .literals
                        .entries()
                        .iter()
                        .any(|literal| literal.bytes().is_empty()),
                    "terminal default emits no implicit empty literal: {}/{}",
                    fields[0],
                    fields[1]
                );
                if fields[1] == "8" {
                    assert!(
                        !context
                            .literals
                            .entries()
                            .iter()
                            .any(|literal| literal.bytes() == b"MASKED")
                    );
                }
            }
            compared += 1;
        }
        assert_eq!(compared, 56);
    }
}
