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

use super::*;
use crate::SemanticOperationId;
use crate::hooks::LoweringHookId;
use crate::native_compilation::{
    NativeBodyCompilation, NativeCompilationFrame, NativeCompilationMode,
};
use tcl_dialect::TclVersion;
use tcl_lexer::{LexerConfig, SourceImage, Span, native_script_words_in};
use tcl_syntax::native_string::NativeStringProtocol;

fn concat_result_hex(bytes: &[u8]) -> String {
    use std::fmt::Write;
    bytes.iter().fold(String::new(), |mut text, byte| {
        write!(text, "{byte:02x}").unwrap();
        text
    })
}

fn project(
    source: &[u8],
    grammar: NativeCompilationGrammar,
    version: TclVersion,
) -> Result<NativeInstructionPlan, NativeInstructionPlanUnavailable> {
    project_with_selection(source, grammar, version).1
}

fn project_with_selection(
    source: &[u8],
    grammar: NativeCompilationGrammar,
    version: TclVersion,
) -> (
    NativeCompilationSelection,
    Result<NativeInstructionPlan, NativeInstructionPlanUnavailable>,
) {
    let profile =
        tcl_dialect::DialectProfile::find(&format!("tcl{}", version.version_string())).unwrap();
    let image = SourceImage::native(source);
    let end = u32::try_from(image.len()).unwrap();
    let script = native_script_words_in(
        image,
        Span::new(0, end),
        LexerConfig::from_grammar(profile.grammar),
    )
    .unwrap();
    let words =
        NativeCompilerWords::capture(&script.commands[0].words, NativeStringProtocol::C(version))
            .unwrap();
    let context = NativeCompilationContext {
        mode: NativeCompilationMode::BytecodeObject,
        frame: NativeCompilationFrame::ProcedureCode,
        loop_depth: 0,
        catch_depth: Some(0),
    };
    let spec = NativeCompilationSpec {
        grammar,
        operation: SemanticOperationId::StructuredLowering(match grammar {
            NativeCompilationGrammar::Increment => LoweringHookId::Incr,
            NativeCompilationGrammar::Return => LoweringHookId::Return,
            NativeCompilationGrammar::Uplevel => LoweringHookId::Uplevel,
            NativeCompilationGrammar::Catch => LoweringHookId::Catch,
            NativeCompilationGrammar::Foreach => LoweringHookId::Foreach,
            NativeCompilationGrammar::Try => LoweringHookId::Try,
            NativeCompilationGrammar::VariableAppend(_) => LoweringHookId::AppendOrLappend,
            _ => LoweringHookId::Set,
        }),
        body: NativeBodyCompilation::Inherit,
    };
    let dialect = InvocationDialect::for_version(version);
    let selection = spec.select_native_words(&words, 1, Some(dialect), context);
    (
        selection,
        native_instruction_plan(spec, selection, &words, 1, dialect, context),
    )
}

#[test]
fn concat_retains_native_constant_allocations_and_original_dynamic_operands() {
    use crate::native_compiler_word_projection::NativeCompilerWordOperand as Operand;
    use NativeConcatLiteralAllocation as Allocation;
    let grammar = NativeCompilationGrammar::ArgumentConcatFrom(TclVersion::V8_6);
    for version in TclVersion::ALL {
        for (source, bytes, private) in [
            (b"concat".as_slice(), b"".as_slice(), Allocation::Registered),
            (b"concat {}", b"", Allocation::PrivateEmpty),
            (b"concat {} {}", b"", Allocation::PrivateEmpty),
            (b"concat { } {\t}", b"", Allocation::PrivateString),
            (b"concat { A B } { C }", b"A B C", Allocation::PrivateString),
        ] {
            let selected = project(source, grammar, version);
            if version < TclVersion::V8_6 {
                assert!(
                    matches!(selected, Err(NativeInstructionPlanUnavailable::Selection)),
                    "{version:?}: {source:?}: {selected:?}"
                );
            } else {
                assert_eq!(
                    selected,
                    Ok(NativeInstructionPlan::Concat(
                        NativeConcatInstruction::Literal {
                            bytes: bytes.to_vec(),
                            allocation: if version == TclVersion::V9_1 {
                                private
                            } else {
                                Allocation::Registered
                            },
                        }
                    ))
                );
            }
        }
        if version >= TclVersion::V8_6 {
            assert_eq!(
                project(b"concat $a $b", grammar, version),
                Ok(NativeInstructionPlan::Concat(
                    NativeConcatInstruction::Operands(vec![
                        Operand::Original(1),
                        Operand::Original(2)
                    ])
                ))
            );
            assert!(matches!(
                project(b"concat {*}$a", grammar, version),
                Err(NativeInstructionPlanUnavailable::Selection)
            ));
            assert!(
                matches!(project(b"concat {*}{A B} $c", grammar, version), Ok(NativeInstructionPlan::Concat(NativeConcatInstruction::Operands(ref operands))) if operands.len() == 3)
            );
        }
    }
}

mod concat_expansion_controls {
    use super::*;
    include!("../../tests/data/native_concat_expansion/cases.rs");

    #[test]
    fn concat_expansion_matches_original_native_parser_and_instruction_windows() {
        use crate::native_compilation::NativeCompilationWordShape as Shape;
        use crate::native_compiler_word_projection::project_native_compiler_words;
        let grammar = NativeCompilationGrammar::ArgumentConcatFrom(TclVersion::V8_6);
        for version in TclVersion::ALL
            .into_iter()
            .filter(|v| *v >= TclVersion::V8_5)
        {
            let engine = format!("tcl{}", version.version_string());
            let profile = tcl_dialect::DialectProfile::find(&engine).unwrap();
            for (case, source) in EXPANSION_BODIES.iter().enumerate() {
                let prefix = format!("OP\t{case}\t");
                let rows = expansion_rows(&engine);
                let opcodes: Vec<_> = rows
                    .lines()
                    .filter(|line| line.starts_with(&prefix))
                    .map(|line| line.split('\t').nth(3).unwrap())
                    .collect();
                let image = SourceImage::native(*source);
                let script = native_script_words_in(
                    image,
                    Span::new(0, u32::try_from(source.len()).unwrap()),
                    LexerConfig::from_grammar(profile.grammar),
                )
                .unwrap();
                let words = NativeCompilerWords::capture(
                    &script.commands[0].words,
                    NativeStringProtocol::C(version),
                )
                .unwrap();
                let projected = project_native_compiler_words(&words, version).unwrap();
                let prefix = format!("TOKEN\t{case}\t");
                let native_shapes: Vec<_> = rows
                    .lines()
                    .filter(|line| line.starts_with(&prefix))
                    .filter_map(|line| match line.split('\t').nth(3).unwrap() {
                        "1" => Some(Shape::Substituted),
                        "2" => Some(Shape::Literal),
                        "256" => Some(Shape::Expanded),
                        _ => None,
                    })
                    .collect();
                assert_eq!(
                    projected.iter().map(|word| word.shape).collect::<Vec<_>>(),
                    native_shapes,
                    "{engine}/{case}"
                );
                let selected = project(source, grammar, version);
                if opcodes.contains(&"concatStk") {
                    let Ok(NativeInstructionPlan::Concat(NativeConcatInstruction::Operands(
                        operands,
                    ))) = selected
                    else {
                        panic!("{engine}/{case}: {selected:?}");
                    };
                    assert_eq!(
                        operands,
                        projected[1..]
                            .iter()
                            .map(|word| word.operand.clone())
                            .collect::<Vec<_>>()
                    );
                } else if version >= TclVersion::V8_6 && !opcodes.contains(&"invokeExpanded") {
                    let Ok(NativeInstructionPlan::Concat(NativeConcatInstruction::Literal {
                        bytes,
                        ..
                    })) = selected
                    else {
                        panic!("{engine}/{case}: {selected:?}");
                    };
                    assert_eq!(
                        concat_result_hex(&bytes),
                        expansion_result(&engine, case)[6]
                    );
                } else {
                    assert!(
                        matches!(selected, Err(NativeInstructionPlanUnavailable::Selection)),
                        "{engine}/{case}: {selected:?}"
                    );
                }
            }
        }
    }
}

// Native proof: naming.variable.uplevel-original-compilation-opcode-and-concat-frontier
// docs/design/analysis/name-resolution-proofs/variable.uplevel-original-compilation-opcode-and-concat-frontier.md
#[test]
fn original_uplevel_instruction_keeps_level_and_script_stack_geometry() {
    for row in include_str!("../../tests/data/native_uplevel_compilation/instructions.tsv")
        .lines()
        .skip(1)
    {
        let fields = row.split('\t').collect::<Vec<_>>();
        let version = match fields[0] {
            "9.0" => TclVersion::V9_0,
            "9.1" => TclVersion::V9_1,
            _ => unreachable!("measured compiler fixture"),
        };
        let selected = project(
            fields[2].as_bytes(),
            NativeCompilationGrammar::Uplevel,
            version,
        );
        assert_eq!(
            selected.is_ok(),
            fields[3] == "1",
            "{}/{}",
            fields[0],
            fields[1]
        );
        if let Ok(NativeInstructionPlan::Uplevel(recipe)) = selected {
            assert_eq!(recipe.script_words.len() > 1, fields[4] == "1");
        }
    }
    for (source, level, scripts) in [
        (b"uplevel 1 {set x OUTER}".as_slice(), Some(1), 2..3),
        (b"uplevel {set x OUTER}".as_slice(), None, 1..2),
        (b"uplevel 1 set x OUTER".as_slice(), Some(1), 2..5),
    ] {
        assert_eq!(
            project(source, NativeCompilationGrammar::Uplevel, TclVersion::V9_1),
            Ok(NativeInstructionPlan::Uplevel(NativeUplevelInstruction {
                level_word: level,
                script_words: scripts,
            })),
        );
        assert!(project(source, NativeCompilationGrammar::Uplevel, TclVersion::V9_0).is_err());
    }
    for source in [
        b"uplevel $level {set x OUTER}".as_slice(),
        b"uplevel -1 {set x OUTER}",
        b"uplevel 1",
    ] {
        assert!(project(source, NativeCompilationGrammar::Uplevel, TclVersion::V9_1).is_err());
    }
}

#[test]
fn installed_named_worker_uses_original_post_selector_operands() {
    use crate::native_compiler_word_projection::NativeCompilerWordOperand as Operand;
    let registry = crate::CommandRegistry::build_default();
    for version in [
        TclVersion::V8_5,
        TclVersion::V8_6,
        TclVersion::V9_0,
        TclVersion::V9_1,
    ] {
        let dialect = InvocationDialect::for_version(version);
        let spec = registry
            .native_compilation_for_registration("::tcl::info::locals", dialect)
            .unwrap();
        let profile =
            tcl_dialect::DialectProfile::find(&format!("tcl{}", version.version_string())).unwrap();
        for (source, operand_count, accepted) in [
            (b"info locals".as_slice(), 0, true),
            (b"info loc $pattern".as_slice(), 1, true),
            (b"info locals {*}{a*}".as_slice(), 1, true),
            (b"info locals a b".as_slice(), 2, false),
            (b"info locals {*}$patterns".as_slice(), 1, false),
        ] {
            let image = SourceImage::native(source);
            let script = native_script_words_in(
                image.clone(),
                Span::new(0, u32::try_from(image.len()).unwrap()),
                LexerConfig::from_grammar(profile.grammar),
            )
            .unwrap();
            let words = NativeCompilerWords::capture(
                &script.commands[0].words,
                NativeStringProtocol::C(version),
            )
            .unwrap();
            let result =
                native_named_worker_instruction(spec, &words, 2, dialect, b"::actual:::locals");
            if !accepted || version == TclVersion::V8_5 {
                assert_eq!(result, NativeNamedWorkerCompilation::Generic);
                continue;
            }
            let NativeNamedWorkerCompilation::Named(recipe) = result else {
                panic!("selected installed worker recipe: {source:?}")
            };
            assert_eq!(recipe.name, b"::actual:::locals");
            assert_eq!(recipe.arguments_from, 1);
            assert_eq!(recipe.words.len(), operand_count);
            assert!(recipe.expanded.iter().all(|expanded| !expanded));
            if source == b"info loc $pattern" {
                assert_eq!(
                    recipe.words,
                    [NativeNamedInvocationWord::Original(Operand::Original(2))]
                );
            }
            assert_eq!(
                native_named_worker_instruction(
                    spec,
                    &words,
                    words.shapes().len() + 1,
                    dialect,
                    b"ignored"
                ),
                NativeNamedWorkerCompilation::Unavailable
            );
        }
    }
}

#[test]
fn named_layout_preserves_original_expansions_and_canonical_rewrite_words() {
    use crate::native_compilation::NativeNamedInvocationProtocol as Protocol;
    use crate::native_compiler_word_projection::NativeCompilerWordOperand as Operand;
    let version = TclVersion::V8_6;
    let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
    let image = SourceImage::native(b"ensemble selected {*}{A B} {*}$tail".as_slice());
    let script = native_script_words_in(
        image.clone(),
        Span::new(0, u32::try_from(image.len()).unwrap()),
        LexerConfig::from_grammar(profile.grammar),
    )
    .unwrap();
    let words =
        NativeCompilerWords::capture(&script.commands[0].words, NativeStringProtocol::C(version))
            .unwrap();
    let direct = native_named_invocation_instruction(
        &words,
        InvocationDialect::for_version(version),
        b"::private",
        1,
        Protocol::Direct,
        &[],
    )
    .unwrap();
    assert_eq!(direct.words.len(), 3);
    assert_eq!(direct.expanded, [false, false, true]);
    assert!(
        matches!(&direct.words[0], NativeNamedInvocationWord::Original(Operand::LiteralExpansion { value, original_word: 2, .. }) if value == b"A")
    );
    assert_eq!(
        direct.words[2],
        NativeNamedInvocationWord::Original(Operand::Original(3))
    );
    assert_eq!(
        native_named_invocation_instruction(
            &words,
            InvocationDialect::for_version(version),
            b"::private",
            1,
            Protocol::EnsembleRewrite,
            &[b"selected".to_vec()]
        ),
        Err(NativeInstructionPlanUnavailable::OperandGeometry)
    );
    let image = SourceImage::native(b"ensemble sel value".as_slice());
    let script = native_script_words_in(
        image.clone(),
        Span::new(0, u32::try_from(image.len()).unwrap()),
        LexerConfig::from_grammar(profile.grammar),
    )
    .unwrap();
    let words =
        NativeCompilerWords::capture(&script.commands[0].words, NativeStringProtocol::C(version))
            .unwrap();
    let rewrite = native_named_invocation_instruction(
        &words,
        InvocationDialect::for_version(version),
        b"::private",
        1,
        Protocol::EnsembleRewrite,
        &[b"selected".to_vec()],
    )
    .unwrap();
    assert_eq!(
        rewrite.words,
        [
            NativeNamedInvocationWord::Original(Operand::Original(0)),
            NativeNamedInvocationWord::Replacement(b"selected".to_vec()),
            NativeNamedInvocationWord::Original(Operand::Original(2))
        ]
    );
    assert!(
        native_named_invocation_instruction(
            &words,
            InvocationDialect::for_version(version),
            b"::private",
            1,
            Protocol::EnsembleRewrite,
            &[]
        )
        .is_err()
    );
}

#[test]
fn rejected_each_retains_its_original_list_failure() {
    use crate::native_control_compilation::NativeControlOutcome;
    let source = b"foreach \"{i\" {A} {set reached 1}";
    let NativeInstructionPlan::Each(recipe) =
        project(source, NativeCompilationGrammar::Foreach, TclVersion::V8_4).unwrap()
    else {
        panic!("original iterator recipe");
    };
    let NativeControlOutcome::Rejected(failure) = recipe.outcome else {
        panic!("native compiler rejects malformed variable list");
    };
    assert_eq!(
        failure.message.as_deref(),
        Some("unmatched open brace in list")
    );
    for version in TclVersion::ALL
        .into_iter()
        .filter(|version| *version > TclVersion::V8_4)
    {
        let NativeInstructionPlan::Each(recipe) =
            project(source, NativeCompilationGrammar::Foreach, version).unwrap()
        else {
            panic!("original iterator recipe");
        };
        assert_eq!(recipe.outcome, NativeControlOutcome::Generic);
    }
}

#[test]
fn instruction_operands_keep_original_opaque_names_and_version_geometry() {
    for version in TclVersion::ALL {
        let plan = project(
            b"set x\xff\0tail VALUE",
            NativeCompilationGrammar::VariableLoadStore,
            version,
        )
        .unwrap();
        assert!(matches!(plan, NativeInstructionPlan::Store {
                target_word: 1, target: NativeVariableWordOperand::Literal { ref name, index: None, .. }, value_word: 2
            } if name == b"x\xff\0tail"));
        let braced = project(
            b"set {a(k)} VALUE",
            NativeCompilationGrammar::VariableLoadStore,
            version,
        )
        .unwrap();
        assert_eq!(
            matches!(
                braced,
                NativeInstructionPlan::Store {
                    target: NativeVariableWordOperand::DynamicWord,
                    ..
                }
            ),
            version == TclVersion::V8_4
        );
    }
}

#[test]
fn increment_immediates_use_the_shared_original_amount_recipe() {
    for version in TclVersion::ALL {
        for (source, expected) in [
            (b"incr x -127".as_slice(), Some(-127)),
            (b"incr x -128".as_slice(), None),
        ] {
            assert!(
                matches!(project(source, NativeCompilationGrammar::Increment, version).unwrap(),
                    NativeInstructionPlan::Increment { amount_word: Some(2), immediate, .. } if immediate == expected)
            );
        }
    }
}

#[test]
fn list_append_empty_publication_retains_selected_native_instruction_branches() {
    for version in [
        TclVersion::V8_5,
        TclVersion::V8_6,
        TclVersion::V9_0,
        TclVersion::V9_1,
    ] {
        let policy = native_list_append_empty_publication(NativeStringProtocol::C(version))
            .expect("actual List-append instruction");
        assert_eq!(policy.skip_existing_store, version == TclVersion::V9_1);
        assert_eq!(policy.fresh_missing_receiver, version == TclVersion::V9_1);
    }
    assert!(
        native_list_append_empty_publication(NativeStringProtocol::C(TclVersion::V8_4)).is_none()
    );
}

#[test]
fn append_retains_original_value_order_and_native_expansion_stack_shape() {
    for version in TclVersion::ALL {
        let grammar = NativeCompilationGrammar::VariableAppend(NativeAppendKind::List);
        let one = project(b"lappend x\xff\0tail $value", grammar, version).unwrap();
        assert!(matches!(one, NativeInstructionPlan::Append {
                target_word: 1,
                target: NativeVariableWordOperand::Literal { ref name, index: None, .. },
                recipe: NativeAppendInstruction { ref values, operands: NativeAppendOperands::ListElement, .. }
            } if name == b"x\xff\0tail" && values == &(2..3)));
        let string = project(
            b"append x\xff\0tail $first",
            NativeCompilationGrammar::VariableAppend(NativeAppendKind::String),
            version,
        )
        .unwrap();
        assert!(matches!(
            string,
            NativeInstructionPlan::Append {
                recipe: NativeAppendInstruction {
                    operands: NativeAppendOperands::StringObjects,
                    ..
                },
                ..
            }
        ));
        assert!(matches!(
            project(
                b"append x",
                NativeCompilationGrammar::VariableAppend(NativeAppendKind::String),
                version
            )
            .unwrap(),
            NativeInstructionPlan::Load { .. }
        ));
    }
    for version in [TclVersion::V8_6, TclVersion::V9_0, TclVersion::V9_1] {
        let many = project(
            b"lappend x $first $second",
            NativeCompilationGrammar::VariableAppend(NativeAppendKind::List),
            version,
        )
        .unwrap();
        assert!(matches!(many, NativeInstructionPlan::Append {
                recipe: NativeAppendInstruction { operands: NativeAppendOperands::ListElements { ref steps, strip_single_expanded: false }, .. }, ..
            } if steps == &[NativeArgumentListStep::Word(2), NativeArgumentListStep::Word(3), NativeArgumentListStep::List(2)]));
    }
    let expanded = project(
        b"lappend x A {*}$middle Z",
        NativeCompilationGrammar::VariableAppend(NativeAppendKind::List),
        TclVersion::V9_1,
    )
    .unwrap();
    assert!(matches!(expanded, NativeInstructionPlan::Append {
        recipe: NativeAppendInstruction { operands: NativeAppendOperands::ListElements { ref steps, strip_single_expanded: false }, .. }, ..
    } if steps == &[
        NativeArgumentListStep::Word(2), NativeArgumentListStep::List(1),
        NativeArgumentListStep::Word(3), NativeArgumentListStep::Concat,
        NativeArgumentListStep::Word(4), NativeArgumentListStep::List(1), NativeArgumentListStep::Concat,
    ]));
    let expanded = project(
        b"lappend x {*}$values",
        NativeCompilationGrammar::VariableAppend(NativeAppendKind::List),
        TclVersion::V9_1,
    )
    .unwrap();
    assert!(matches!(expanded, NativeInstructionPlan::Append {
            recipe: NativeAppendInstruction { operands: NativeAppendOperands::ListElements { ref steps, strip_single_expanded: true }, .. }, ..
        } if steps == &[NativeArgumentListStep::Word(2)]));
    let empty = project(
        b"lappend x",
        NativeCompilationGrammar::VariableAppend(NativeAppendKind::List),
        TclVersion::V9_1,
    )
    .unwrap();
    assert!(matches!(empty, NativeInstructionPlan::Append {
            recipe: NativeAppendInstruction { operands: NativeAppendOperands::ListElements { ref steps, strip_single_expanded: false }, .. }, ..
        } if steps == &[NativeArgumentListStep::List(0)]));
}

#[test]
fn original_c86_insertion_retains_its_immediate_list_recipe() {
    use crate::native_compiler_word_projection::NativeCompilerWordOperand as Operand;
    use crate::native_list_operations_compilation::{
        NativeListInsertionIndex, NativeListOperationInstruction,
    };
    let plan = project(
        b"linsert $args 0 interp alias {} $name {}",
        NativeCompilationGrammar::ListInsertion,
        TclVersion::V8_6,
    )
    .unwrap();
    assert!(
        matches!(plan, NativeInstructionPlan::ListOperations(NativeListOperationInstruction::Insert {
            list: Operand::Original(1),
            index: NativeListInsertionIndex::Immediate(index),
            elements,
        }) if index.encoded() == 0 && elements == (3..8).map(Operand::Original).collect::<Vec<_>>())
    );
    assert_eq!(
        project_with_selection(
            b"linsert $args $index X",
            NativeCompilationGrammar::ListInsertion,
            TclVersion::V8_6
        ),
        (
            NativeCompilationSelection::Generic,
            Err(NativeInstructionPlanUnavailable::Selection)
        )
    );
}

#[test]
fn original_c9_insertion_retains_each_release_instruction_and_original_index() {
    use crate::native_compiler_word_projection::NativeCompilerWordOperand as Operand;
    use crate::native_list_operations_compilation::{
        NativeListInsertionIndex, NativeListInsertionInstruction, NativeListOperationInstruction,
    };
    for (version, expected) in [
        (
            TclVersion::V9_0,
            NativeListInsertionInstruction::ReplaceFour,
        ),
        (TclVersion::V9_1, NativeListInsertionInstruction::Replace),
    ] {
        let plan = project(
            b"linsert $args $index X",
            NativeCompilationGrammar::ListInsertion,
            version,
        )
        .unwrap();
        assert_eq!(
            plan,
            NativeInstructionPlan::ListOperations(NativeListOperationInstruction::Insert {
                list: Operand::Original(1),
                index: NativeListInsertionIndex::Original {
                    operand: Operand::Original(2),
                    instruction: expected
                },
                elements: vec![Operand::Original(3)],
            })
        );
    }
}

#[test]
fn original_scalar_descriptor_has_a_retained_native_instruction() {
    use crate::native_scalar_compilation::{NativeScalarOperation, NativeScalarScope};
    let recipe = project(
        b"string length value",
        NativeCompilationGrammar::StringLength(NativeScalarScope::PublicMember),
        TclVersion::V8_5,
    )
    .unwrap();
    assert!(matches!(
        recipe,
        NativeInstructionPlan::Scalar(scalar)
            if scalar.operation == NativeScalarOperation::StringLength
                && scalar.operands == [crate::native_compiler_word_projection::NativeCompilerWordOperand::Original(2)]
    ));
}

#[test]
fn return_preserves_original_result_operand_and_selected_options_recipe() {
    use crate::native_return_compilation::{
        NativeReturnExit as Exit, NativeReturnOptionsOperand as Options,
    };
    for version in TclVersion::ALL {
        for (source, value_word) in [
            (b"return".as_slice(), None),
            (b"return \xff\0tail".as_slice(), Some(1)),
            (b"return $result".as_slice(), Some(1)),
        ] {
            let NativeInstructionPlan::ReturnOptions(recipe) =
                project(source, NativeCompilationGrammar::Return, version).unwrap()
            else {
                panic!("original Return plan")
            };
            assert_eq!(recipe.value_word, value_word);
            assert_eq!(
                recipe.compiler_word_visits().collect::<Vec<_>>(),
                value_word.into_iter().collect::<Vec<_>>()
            );
            assert_eq!(recipe.exit, Exit::Done);
            let Options::Static(options) = recipe.options else {
                panic!("native default controls")
            };
            assert_eq!((options.code, options.level, options.size), (0, 1, 0));
        }
    }
    let NativeInstructionPlan::ReturnOptions(recipe) = project(
        b"return -code ok VALUE",
        NativeCompilationGrammar::Return,
        TclVersion::V9_0,
    )
    .unwrap() else {
        panic!("static options")
    };
    assert_eq!(recipe.exit, Exit::Immediate);
    assert_eq!(recipe.value_word, Some(3));
    let Options::Static(options) = recipe.options else {
        panic!("merged private header")
    };
    assert_eq!((options.code, options.level, options.size), (0, 1, 0));
}
#[test]
fn native_return_compilation_retains_static_stack_and_generic_frontiers() {
    use crate::native_return_compilation::{
        NativeReturnExit as Exit, NativeReturnOptionsOperand as Options,
    };
    for version in [
        TclVersion::V8_5,
        TclVersion::V8_6,
        TclVersion::V9_0,
        TclVersion::V9_1,
    ] {
        let NativeInstructionPlan::ReturnOptions(recipe) = project(
            b"return -level 0 -code error -options {-custom kept -errorcode CUSTOM} BODY",
            NativeCompilationGrammar::Return,
            version,
        )
        .unwrap() else {
            panic!("retained Return")
        };
        assert_eq!(recipe.exit, Exit::Immediate);
        assert_eq!(recipe.value_word, Some(7));
        assert_eq!(recipe.compiler_word_visits().collect::<Vec<_>>(), vec![7]);
        let Options::Static(literal) = recipe.options else {
            panic!("private options")
        };
        assert_eq!((literal.code, literal.level, literal.size), (1, 0, 2));
        let NativeInstructionPlan::ReturnOptions(recipe) = project(
            b"return -options $options $result",
            NativeCompilationGrammar::Return,
            version,
        )
        .unwrap() else {
            panic!("stack Return")
        };
        assert_eq!(recipe.options, Options::StackWord(2));
        assert_eq!(recipe.exit, Exit::Stack);
        assert_eq!(
            recipe.compiler_word_visits().collect::<Vec<_>>(),
            vec![2, 3]
        );
        let dynamic = project(
            b"return -code $code $result",
            NativeCompilationGrammar::Return,
            version,
        );
        if version == TclVersion::V8_5 {
            assert_eq!(dynamic, Err(NativeInstructionPlanUnavailable::Selection));
        } else {
            let Ok(NativeInstructionPlan::ReturnOptions(recipe)) = &dynamic else {
                panic!("original dynamic return recipe");
            };
            assert_eq!(
                recipe.compiler_word_visits().collect::<Vec<_>>(),
                vec![1, 2, 3]
            );
            assert!(
                matches!(dynamic,Ok(NativeInstructionPlan::ReturnOptions(crate::native_return_compilation::NativeReturnInstruction{options:Options::StackPairs(range),..})) if range==(1..3))
            );
        }
        assert_eq!(
            project(
                b"return -level -1 BODY",
                NativeCompilationGrammar::Return,
                version
            ),
            Err(NativeInstructionPlanUnavailable::Selection)
        );
    }
}
