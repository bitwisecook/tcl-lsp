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

//! The variable and list compilers: array, global, upvar, unset, assignment, insertion, index and concat.

use super::*;

#[test]
fn array_specialised_compilation_retains_workers_and_shape_protocols() {
    let registry = crate::model::ingress::static_context_for("tcl8.6").commands();
    let procedure = NativeCompilationContext {
        frame: NativeCompilationFrame::ProcedureCode,
        mode: NativeCompilationMode::BytecodeObject,
        ..NativeCompilationContext::default()
    };
    for (values, shapes, named) in [
        (
            vec!["exists", "a"],
            vec![NativeCompilationWordShape::Literal; 2],
            false,
        ),
        (
            vec!["set", "a", "x 1"],
            vec![NativeCompilationWordShape::Literal; 3],
            false,
        ),
        (
            vec!["set", "a", "x"],
            vec![NativeCompilationWordShape::Literal; 3],
            true,
        ),
        (
            vec!["unset", "a", "x*"],
            vec![NativeCompilationWordShape::Literal; 3],
            true,
        ),
    ] {
        let facts = registry
            .resolve_invocation("array", &values, registry.own_surface_query())
            .unwrap()
            .facts();
        let spec = facts.native_compilation.unwrap();
        let old = crate::InvocationDialect::for_version(TclVersion::V8_5);
        let current = crate::InvocationDialect::for_version(TclVersion::V8_6);
        assert_eq!(
            spec.select(
                InvocationWords::literals("array", &values),
                &shapes,
                Some(old),
                procedure
            ),
            NativeCompilationSelection::Generic
        );
        assert!(spec.implementation_lookup(old).is_none());
        assert!(spec.implementation_lookup(current).is_some());
        let selected = spec.select(
            InvocationWords::literals("array", &values),
            &shapes,
            Some(current),
            procedure,
        );
        assert_eq!(
            matches!(selected, NativeCompilationSelection::NamedInvocation { .. }),
            named
        );
        assert!(named || matches!(selected, NativeCompilationSelection::Inline { .. }));
    }
}

#[test]
fn global_alias_compilation_requires_a_local_table_and_known_scalar_tail() {
    let spec = NativeCompilationSpec {
        grammar: NativeCompilationGrammar::GlobalBindings,
        operation: SemanticOperationId::StructuredLowering(LoweringHookId::Global),
        body: NativeBodyCompilation::Inherit,
    };
    for version in TclVersion::ALL {
        for frame in [
            NativeCompilationFrame::ScriptCode,
            NativeCompilationFrame::ProcedureCode,
        ] {
            for name in ["g", "::g", "N::g", "", "g(k)"] {
                let selection = spec.select(
                    InvocationWords::literals("global", &[name]),
                    &[NativeCompilationWordShape::BracedLiteral],
                    Some(InvocationDialect::for_version(version)),
                    context(frame),
                );
                assert_eq!(
                    matches!(selection, NativeCompilationSelection::Inline { .. }),
                    version >= TclVersion::V8_5
                        && frame == NativeCompilationFrame::ProcedureCode
                        && !name.ends_with(')'),
                    "{version:?} {frame:?} {name:?}"
                );
            }
        }
    }
    assert_eq!(
        spec.select(
            InvocationWords::literals("global", &["N::g"]),
            &[NativeCompilationWordShape::Substituted],
            Some(InvocationDialect::for_version(TclVersion::V9_1)),
            context(NativeCompilationFrame::ProcedureCode),
        ),
        NativeCompilationSelection::Unknown
    );
}

#[test]
fn insertion_compiler_moves_dynamic_index_acceptance_at_c9() {
    let spec = NativeCompilationSpec {
        grammar: NativeCompilationGrammar::ListInsertion,
        operation: SemanticOperationId::Intrinsic(crate::IntrinsicId::ListInsert),
        body: NativeBodyCompilation::Inherit,
    };
    for version in TclVersion::ALL {
        for shape in [
            NativeCompilationWordShape::Literal,
            NativeCompilationWordShape::Substituted,
        ] {
            let selection = spec.select(
                InvocationWords::literals("linsert", &["a b", "1", "X"]),
                &[
                    NativeCompilationWordShape::BracedLiteral,
                    shape,
                    NativeCompilationWordShape::Substituted,
                ],
                Some(InvocationDialect::for_version(version)),
                context(NativeCompilationFrame::ScriptCode),
            );
            assert_eq!(
                matches!(selection, NativeCompilationSelection::Inline { .. }),
                version >= TclVersion::V9_0
                    || (version == TclVersion::V8_6
                        && shape == NativeCompilationWordShape::Literal)
            );
        }
    }
}

#[test]
fn original_assignment_selection_keeps_opaque_target_bytes_and_modes_separate() {
    // Native proof: naming.list.assignment-literal-versus-alias-target-evaluation
    // docs/design/analysis/name-resolution-proofs/assignment-literal-versus-alias-target-evaluation.md
    let spec = NativeCompilationSpec {
        grammar: NativeCompilationGrammar::ListAssignment,
        operation: SemanticOperationId::Intrinsic(crate::IntrinsicId::ListAssign),
        body: NativeBodyCompilation::Inherit,
    };
    for version in TclVersion::ALL {
        let profile = tcl_dialect::DialectProfile::find(version.dialect_profile_name()).unwrap();
        let dialect = InvocationDialect::of_profile(profile);
        for image in [
            tcl_lexer::SourceImage::document(r"lassign {one two} n\uD800 a(k)"),
            tcl_lexer::SourceImage::native(b"lassign {one two} n\xff a(k)".as_slice()),
        ] {
            let parsed = tcl_lexer::native_script_words_in(
                image.clone(),
                tcl_lexer::Span::new(0, u32::try_from(image.len()).unwrap()),
                tcl_lexer::LexerConfig::from_grammar(profile.grammar),
            )
            .unwrap();
            assert!(parsed.fatal_tail.is_none());
            let captured = crate::native_compiler_words::NativeCompilerWords::capture(
                &parsed.commands[0].words,
                tcl_syntax::native_string::NativeStringProtocol::C(version),
            )
            .unwrap();
            assert!(captured.literal(2).is_some());
            let entered = context(NativeCompilationFrame::ScriptCode);
            let selected = spec.select_native_words(&captured, 1, Some(dialect), entered);
            assert_eq!(
                matches!(selected, NativeCompilationSelection::Inline { .. }),
                version >= TclVersion::V8_5,
                "{version:?}/{image:?}",
            );
            assert_eq!(
                spec.select_native_words(
                    &captured,
                    1,
                    Some(dialect),
                    NativeCompilationContext {
                        mode: NativeCompilationMode::Direct,
                        ..entered
                    },
                ),
                NativeCompilationSelection::Generic
            );
            assert_eq!(
                spec.select_native_words(
                    &captured,
                    1,
                    Some(dialect),
                    NativeCompilationContext {
                        mode: NativeCompilationMode::Unknown,
                        ..entered
                    },
                ),
                if version >= TclVersion::V8_5 {
                    NativeCompilationSelection::Unknown
                } else {
                    NativeCompilationSelection::Generic
                }
            );
            assert_eq!(
                spec.select_native_words(&captured, 1, None, entered),
                NativeCompilationSelection::Unknown
            );
        }
    }
}

#[test]
fn native_list_operands_keep_compilation_and_store_timing_separate() {
    let range = NativeCompilationSpec {
        grammar: NativeCompilationGrammar::ListRange,
        operation: SemanticOperationId::Intrinsic(crate::IntrinsicId::ListRange),
        body: NativeBodyCompilation::Inherit,
    };
    let assign = NativeCompilationSpec {
        grammar: NativeCompilationGrammar::ListAssignment,
        operation: SemanticOperationId::Intrinsic(crate::IntrinsicId::ListAssign),
        body: NativeBodyCompilation::Inherit,
    };
    for version in TclVersion::ALL {
        let dialect = Some(InvocationDialect::for_version(version));
        let entered = context(NativeCompilationFrame::ProcedureCode);
        let selected = range.select(
            InvocationWords::literals("lrange", &["a b", "0", "end"]),
            &[NativeCompilationWordShape::Literal; 3],
            dialect,
            entered,
        );
        assert_eq!(
            matches!(selected, NativeCompilationSelection::Inline { .. }),
            version >= TclVersion::V8_6
        );
        let dynamic_index = range.select(
            InvocationWords::literals("lrange", &["a b", "0", "end"]),
            &[
                NativeCompilationWordShape::Literal,
                NativeCompilationWordShape::Substituted,
                NativeCompilationWordShape::Literal,
            ],
            dialect,
            entered,
        );
        assert_eq!(dynamic_index, NativeCompilationSelection::Generic);
        let selected = assign.select(
            InvocationWords::literals("lassign", &["a b", "x"]),
            &[
                NativeCompilationWordShape::Substituted,
                NativeCompilationWordShape::QuotedLiteral,
            ],
            dialect,
            entered,
        );
        assert_eq!(
            matches!(selected, NativeCompilationSelection::Inline { .. }),
            version >= TclVersion::V8_5
        );
        let dynamic_name = assign.select(
            InvocationWords::literals("lassign", &["a b", "x"]),
            &[
                NativeCompilationWordShape::Literal,
                NativeCompilationWordShape::Substituted,
            ],
            dialect,
            entered,
        );
        assert_eq!(
            dynamic_name,
            if version >= TclVersion::V8_5 {
                // TclCompileLassignCmd uses PushVarNameWord for each
                // dynamic target (native_list_operations case 13).
                NativeCompilationSelection::Inline {
                    operation: assign.operation,
                    guard: NativeCompilationGuard::BeforeArguments,
                }
            } else {
                NativeCompilationSelection::Generic
            }
        );
    }
}

#[test]
fn list_index_compilation_requires_the_native_hook_and_original_word_count() {
    let spec = NativeCompilationSpec {
        grammar: NativeCompilationGrammar::ListIndex,
        operation: SemanticOperationId::Intrinsic(crate::IntrinsicId::ListIndex),
        body: NativeBodyCompilation::Inherit,
    };
    for version in TclVersion::ALL {
        let dialect = Some(InvocationDialect::for_version(version));
        for frame in [
            NativeCompilationFrame::ScriptCode,
            NativeCompilationFrame::ProcedureCode,
        ] {
            for arguments in [vec!["a b"], vec!["a b", "1"], vec!["a b", "0", "end"]] {
                let shapes = vec![NativeCompilationWordShape::Substituted; arguments.len()];
                let selection = spec.select(
                    InvocationWords::literals("lindex", &arguments),
                    &shapes,
                    dialect,
                    context(frame),
                );
                assert_eq!(
                    selection,
                    NativeCompilationSelection::Inline {
                        operation: spec.operation,
                        guard: NativeCompilationGuard::for_c_version(version),
                    },
                );
            }
            assert_eq!(
                spec.select(
                    InvocationWords::literals("lindex", &[]),
                    &[],
                    dialect,
                    context(frame)
                ),
                NativeCompilationSelection::Generic,
            );
            assert_eq!(
                spec.select(
                    InvocationWords::literals("lindex", &["a b", "0"]),
                    &[
                        NativeCompilationWordShape::Expanded,
                        NativeCompilationWordShape::Literal
                    ],
                    dialect,
                    context(frame),
                ),
                NativeCompilationSelection::Generic,
            );
        }
    }
    let jim = crate::model::ingress::resolve_environment("jim").analyser_profile();
    assert_eq!(
        spec.select(
            InvocationWords::literals("lindex", &["a b", "0"]),
            &[NativeCompilationWordShape::Literal; 2],
            Some(InvocationDialect::of_profile(jim)),
            context(NativeCompilationFrame::ProcedureCode),
        ),
        NativeCompilationSelection::Generic,
    );
}

#[test]
fn concat_compiler_accepts_retained_variadic_operands_from_c86() {
    let registry = crate::CommandRegistry::build_default();
    let spec = registry.get("concat").unwrap().native_compilation.unwrap();
    for version in [
        TclVersion::V8_4,
        TclVersion::V8_5,
        TclVersion::V8_6,
        TclVersion::V9_0,
        TclVersion::V9_1,
    ] {
        for args in [vec![], vec!["a"], vec!["a", "b"]] {
            let words = InvocationWords::literals("concat", &args);
            let shapes = vec![NativeCompilationWordShape::Substituted; args.len()];
            let dialect = InvocationDialect::for_version(version);
            let selected = spec.select(
                words,
                &shapes,
                Some(dialect),
                context(NativeCompilationFrame::ScriptCode),
            );
            assert_eq!(
                spec.compiler_hook_presence(dialect),
                Some(version >= TclVersion::V8_6)
            );
            assert_eq!(
                matches!(selected, NativeCompilationSelection::Inline { .. }),
                version >= TclVersion::V8_6
            );
            assert_eq!(
                matches!(selected, NativeCompilationSelection::Generic),
                version < TclVersion::V8_6
            );
        }
    }
}

#[test]
fn upvar_compiler_uses_literal_probe_and_local_scalar_destinations() {
    let registry = crate::CommandRegistry::build_default();
    let spec = registry.get("upvar").unwrap().native_compilation.unwrap();
    for version in TclVersion::ALL {
        let select = |values: &[&str], shapes: &[NativeCompilationWordShape]| {
            spec.select(
                InvocationWords::literals("upvar", values),
                shapes,
                Some(InvocationDialect::for_version(version)),
                context(NativeCompilationFrame::ProcedureCode),
            )
        };
        let literals = [NativeCompilationWordShape::Literal; 3];
        for values in [["0", "x", "a"], ["#0", "x", "a"], ["1", "x", "a"]] {
            let selection = select(&values, &literals);
            if version == TclVersion::V8_4 {
                assert_eq!(selection, NativeCompilationSelection::Generic);
            } else {
                assert!(matches!(
                    selection,
                    NativeCompilationSelection::Inline { .. }
                ));
            }
        }
        assert_eq!(
            select(&["1", "a"], &literals[..2]),
            NativeCompilationSelection::Generic,
            "numeric otherVar is a runtime pair in modern Tcl but not a compiled pair"
        );
        assert_eq!(
            select(&["0", "x", "a(k)"], &literals),
            NativeCompilationSelection::Generic
        );
        assert_eq!(
            select(
                &["0", "x", "a"],
                &[
                    NativeCompilationWordShape::Substituted,
                    literals[1],
                    literals[2]
                ]
            ),
            NativeCompilationSelection::Generic
        );
    }
}

#[test]
fn unset_hook_proves_literal_operands_and_retains_dynamic_order_uncertainty() {
    let spec = NativeCompilationSpec {
        grammar: NativeCompilationGrammar::LiteralUnset,
        operation: SemanticOperationId::Invoke,
        body: NativeBodyCompilation::Inherit,
    };
    for version in TclVersion::ALL {
        for words in [
            &[][..],
            &["x"][..],
            &["-nocomplain", "--", "a(k)", "::g"][..],
        ] {
            let shapes = vec![NativeCompilationWordShape::Literal; words.len()];
            assert_eq!(
                spec.select(
                    InvocationWords::literals("unset", words),
                    &shapes,
                    Some(InvocationDialect::for_version(version)),
                    context(NativeCompilationFrame::ScriptCode)
                ),
                if version < TclVersion::V8_6 {
                    NativeCompilationSelection::Generic
                } else {
                    NativeCompilationSelection::Inline {
                        operation: spec.operation,
                        guard: NativeCompilationGuard::BeforeArguments,
                    }
                }
            );
        }
        assert_eq!(
            spec.select(
                InvocationWords::literals("unset", &["x"]),
                &[NativeCompilationWordShape::Substituted],
                Some(InvocationDialect::for_version(version)),
                context(NativeCompilationFrame::ProcedureCode)
            ),
            if version < TclVersion::V8_6 {
                NativeCompilationSelection::Generic
            } else {
                NativeCompilationSelection::Unknown
            }
        );
    }
}

#[test]
fn procedure_only_variable_hook_declines_script_frames() {
    let registry = crate::CommandRegistry::build_default();
    let spec = registry
        .get("variable")
        .unwrap()
        .native_compilation
        .unwrap();
    for version in TclVersion::ALL {
        let dialect = InvocationDialect::for_version(version);
        assert_eq!(
            spec.compiler_hook_presence(dialect),
            Some(version >= TclVersion::V8_5)
        );
        for frame in [
            NativeCompilationFrame::ScriptCode,
            NativeCompilationFrame::ProcedureCode,
            NativeCompilationFrame::Unknown,
        ] {
            let selection = spec.select(
                InvocationWords::literals("variable", &["x", "10"]),
                &[NativeCompilationWordShape::Literal; 2],
                Some(dialect),
                context(frame),
            );
            assert_eq!(
                selection,
                if version < TclVersion::V8_5 || frame == NativeCompilationFrame::ScriptCode {
                    NativeCompilationSelection::Generic
                } else if frame == NativeCompilationFrame::ProcedureCode {
                    // Original known local tail/value matches the
                    // TclCompileVariableCmd IndexTailVarIfKnown path.
                    NativeCompilationSelection::Inline {
                        operation: spec.operation,
                        guard: NativeCompilationGuard::BeforeArguments,
                    }
                } else {
                    NativeCompilationSelection::Unknown
                }
            );
        }
    }
}

#[test]
fn physical_caller_frame_does_not_invent_a_compiler_local_table() {
    let spec = NativeCompilationSpec {
        grammar: NativeCompilationGrammar::Catch,
        operation: SemanticOperationId::StructuredLowering(LoweringHookId::Catch),
        body: NativeBodyCompilation::Inherit,
    };
    let words = InvocationWords::literals("catch", &["set x 1", "result"]);
    let shapes = [
        NativeCompilationWordShape::BracedLiteral,
        NativeCompilationWordShape::Literal,
    ];
    let dialect = Some(InvocationDialect::for_version(TclVersion::V8_6));
    assert_eq!(
        spec.select(
            words,
            &shapes,
            dialect,
            context(NativeCompilationFrame::ScriptCode)
        ),
        NativeCompilationSelection::Generic
    );
    let selection = spec.select(
        words,
        &shapes,
        dialect,
        context(NativeCompilationFrame::ProcedureCode),
    );
    assert!(matches!(
        selection,
        NativeCompilationSelection::Inline { .. }
    ));
    assert_eq!(
        spec.body_context_for_operand(
            dialect,
            context(NativeCompilationFrame::ProcedureCode),
            selection,
            NativeCompilationWordShape::BracedLiteral
        ),
        context(NativeCompilationFrame::ProcedureCode).with_inline_exception_range()
    );
    assert_eq!(
        spec.body_context_for_operand(
            dialect,
            context(NativeCompilationFrame::ProcedureCode),
            NativeCompilationSelection::Generic,
            NativeCompilationWordShape::BracedLiteral
        ),
        context(NativeCompilationFrame::ScriptCode)
    );
}

/// `TclCompileUpvarCmd` takes a leading word as a level by its spelling under
/// the release, never by the level current at compile time. Each row was
/// measured with `tcl::unsupported::disassemble` of a fresh procedure
/// `upvar WORD x y` (and `upvar WORD x y z`) on tclsh 8.5.19, 8.6.18, 9.0.4
/// and 9.1.0: `L` compiles WORD as the level (the two-pair form only), `N`
/// pushes `1` and compiles WORD as the other variable (the odd form only).
#[test]
fn upvar_level_words_compile_as_each_release_reads_them() {
    use TclVersion::{V8_5, V8_6, V9_0, V9_1};
    let rows: &[(&str, [u8; 4])] = &[
        ("0", *b"LLLL"),
        ("2", *b"LLLL"),
        ("5", *b"LLLL"),
        ("#0", *b"LLLL"),
        ("#5", *b"LLLL"),
        ("#+1", *b"LLLL"),
        ("#0x1", *b"LLLL"),
        ("0x10", *b"LLLL"),
        ("0b1", *b"LLLL"),
        ("2 ", *b"LLLL"),
        ("4294967295", *b"LLLL"),
        ("-1", *b"NLLL"),
        ("+1", *b"NLLL"),
        (" 2", *b"NLLL"),
        ("4294967296", *b"NLLL"),
        ("99999999999999999999", *b"NLLL"),
        ("1.5", *b"NLNN"),
        ("1e0", *b"NLNN"),
        ("08", *b"NNLL"),
        ("#08", *b"NNLL"),
        ("#-1", *b"NNNL"),
        ("#abc", *b"NNNL"),
        ("#1.5", *b"NNNL"),
        ("abc", *b"NNNN"),
        ("", *b"NNNN"),
        ("x#", *b"NNNN"),
    ];
    for &(word, expected) in rows {
        for (version, kind) in [V8_5, V8_6, V9_0, V9_1].into_iter().zip(expected) {
            let (pairs, odd) = if kind == b'L' {
                (Ok(1), Err(NativeCompilationSelection::Generic))
            } else {
                (Err(NativeCompilationSelection::Generic), Ok(0))
            };
            assert_eq!(
                original_upvar_level_width(word, 3, version),
                pairs,
                "{word:?} {version:?} x y"
            );
            assert_eq!(
                original_upvar_level_width(word, 4, version),
                odd,
                "{word:?} {version:?} x y z"
            );
        }
    }
}
