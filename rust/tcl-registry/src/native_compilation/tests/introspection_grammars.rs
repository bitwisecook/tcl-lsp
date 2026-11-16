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

//! The `namespace` and `info` introspection compilers.

use super::*;

#[test]
fn namespace_code_distinguishes_original_literal_builder_and_worker_fallback() {
    let spec = NativeCompilationSpec {
        grammar: NativeCompilationGrammar::NamespaceCode,
        operation: SemanticOperationId::Intrinsic(crate::IntrinsicId::NamespaceCode),
        body: NativeBodyCompilation::Inherit,
    };
    let context = context(NativeCompilationFrame::ProcedureCode);
    for version in TclVersion::ALL {
        let dialect = InvocationDialect::for_version(version);
        let modern = version >= TclVersion::V8_6;
        assert_eq!(spec.compiler_hook_presence(dialect), Some(modern));
        assert_eq!(spec.implementation_lookup(dialect).is_some(), modern);
        for shape in [
            NativeCompilationWordShape::Literal,
            NativeCompilationWordShape::QuotedLiteral,
            NativeCompilationWordShape::BracedLiteral,
        ] {
            for value in ["my tick", "::namespace inscope "] {
                assert_eq!(
                    spec.select(
                        InvocationWords::literals("::tcl::namespace::code", &[value]),
                        &[shape],
                        Some(dialect),
                        context,
                    ),
                    if modern {
                        NativeCompilationSelection::Inline {
                            operation: spec.operation,
                            guard: NativeCompilationGuard::BeforeArguments,
                        }
                    } else {
                        NativeCompilationSelection::Generic
                    }
                );
            }
        }
        for (value, shape) in [
            ("my tick", NativeCompilationWordShape::Substituted),
            ("my tick", NativeCompilationWordShape::BackslashLiteral),
            (
                "::namespace inscope :: my",
                NativeCompilationWordShape::BracedLiteral,
            ),
        ] {
            assert_eq!(
                spec.select(
                    InvocationWords::literals("::tcl::namespace::code", &[value]),
                    &[shape],
                    Some(dialect),
                    context,
                ),
                NativeCompilationSelection::Generic,
            );
            let selection = spec.select(
                InvocationWords::literals("namespace", &[value]),
                &[shape],
                Some(dialect),
                context,
            );
            assert_eq!(
                matches!(
                    selection,
                    NativeCompilationSelection::NamedInvocation {
                        protocol: NativeNamedInvocationProtocol::EnsembleRewrite,
                        ..
                    }
                ),
                modern,
            );
        }
    }
    let jim = InvocationDialect::of_point(tcl_dialect::model::DialectPoint::canonical(
        tcl_dialect::model::Release::JIM_0_84,
    ));
    assert_eq!(spec.compiler_hook_presence(jim), Some(false));
    assert_eq!(
        spec.select(
            InvocationWords::literals("::tcl::namespace::code", &["my tick"]),
            &[NativeCompilationWordShape::Literal],
            Some(jim),
            context,
        ),
        NativeCompilationSelection::Generic,
    );
}

#[test]
fn namespace_origin_compilation_keeps_original_opcode_and_operand_layout() {
    let spec = NativeCompilationSpec {
        grammar: NativeCompilationGrammar::NamespaceOrigin,
        operation: SemanticOperationId::Intrinsic(crate::IntrinsicId::NamespaceOrigin),
        body: NativeBodyCompilation::Inherit,
    };
    let registry = crate::CommandRegistry::build_default();
    let context = context(NativeCompilationFrame::ProcedureCode);
    for version in TclVersion::ALL {
        let dialect = InvocationDialect::for_version(version);
        let modern = version >= TclVersion::V8_6;
        assert_eq!(spec.compiler_hook_presence(dialect), Some(modern));
        assert_eq!(spec.implementation_lookup(dialect).is_some(), modern);
        let arguments = ["origin", "target"];
        let facts = registry
            .resolve_invocation("namespace", &arguments, registry.own_surface_query())
            .unwrap()
            .facts();
        assert_eq!(
            spec.select_for_facts(
                InvocationWords::literals("namespace", &arguments),
                &[
                    NativeCompilationWordShape::Literal,
                    NativeCompilationWordShape::Substituted
                ],
                &facts,
                Some(dialect),
                context
            ),
            if modern {
                NativeCompilationSelection::Inline {
                    operation: spec.operation,
                    guard: NativeCompilationGuard::BeforeArguments,
                }
            } else {
                NativeCompilationSelection::Generic
            }
        );
        for count in [0, 2] {
            let arguments = vec!["target"; count];
            assert_eq!(
                spec.select(
                    InvocationWords::literals("::tcl::namespace::origin", &arguments),
                    &vec![NativeCompilationWordShape::Literal; count],
                    Some(dialect),
                    context
                ),
                NativeCompilationSelection::Generic
            );
        }
        assert_eq!(
            spec.select(
                InvocationWords::literals("::tcl::namespace::origin", &["target"]),
                &[NativeCompilationWordShape::Opaque],
                Some(dialect),
                context
            ),
            if modern {
                NativeCompilationSelection::Unknown
            } else {
                NativeCompilationSelection::Generic
            }
        );
    }
    let jim = InvocationDialect::of_point(tcl_dialect::model::DialectPoint::canonical(
        tcl_dialect::model::Release::JIM_0_84,
    ));
    assert_eq!(spec.compiler_hook_presence(jim), Some(false));
    assert_eq!(
        spec.select(
            InvocationWords::literals("namespace", &["origin", "target"]),
            &[NativeCompilationWordShape::Literal; 2],
            Some(jim),
            context
        ),
        NativeCompilationSelection::Generic
    );
}

#[test]
fn current_namespace_compilation_preserves_operand_offsets_and_release_floor() {
    let spec = NativeCompilationSpec {
        grammar: NativeCompilationGrammar::NamespaceCurrent,
        operation: SemanticOperationId::Intrinsic(crate::IntrinsicId::NamespaceCurrent),
        body: NativeBodyCompilation::Inherit,
    };
    let registry = crate::CommandRegistry::build_default();
    let context = NativeCompilationContext {
        mode: NativeCompilationMode::BytecodeObject,
        frame: NativeCompilationFrame::ProcedureCode,
        ..Default::default()
    };
    for version in TclVersion::ALL {
        let dialect = InvocationDialect::for_version(version);
        let modern = version >= TclVersion::V8_6;
        assert_eq!(spec.compiler_hook_presence(dialect), Some(modern));
        assert_eq!(spec.implementation_lookup(dialect).is_some(), modern);
        for arguments in [vec!["current"], vec!["current", "extra"]] {
            let facts = registry
                .resolve_invocation("namespace", &arguments, registry.own_surface_query())
                .unwrap()
                .facts();
            let selection = spec.select_for_facts(
                InvocationWords::literals("namespace", &arguments),
                &vec![NativeCompilationWordShape::Literal; arguments.len()],
                &facts,
                Some(dialect),
                context,
            );
            if !modern {
                assert_eq!(selection, NativeCompilationSelection::Generic);
            } else if arguments.len() == 1 {
                assert_eq!(
                    selection,
                    NativeCompilationSelection::Inline {
                        operation: spec.operation,
                        guard: NativeCompilationGuard::BeforeArguments,
                    }
                );
            } else {
                assert!(matches!(selection,
                        NativeCompilationSelection::NamedInvocation { lookup, arguments_from: 1, protocol: NativeNamedInvocationProtocol::EnsembleRewrite }
                        if lookup.slot == "::tcl::namespace::current"));
            }
        }
        assert_eq!(
            spec.select(
                InvocationWords::literals("::tcl::namespace::current", &["extra"]),
                &[NativeCompilationWordShape::Literal],
                Some(dialect),
                context,
            ),
            NativeCompilationSelection::Generic
        );
    }
}

#[test]
fn stack_level_compilation_retains_original_operands_and_release_floor() {
    let spec = NativeCompilationSpec {
        grammar: NativeCompilationGrammar::InfoLevel,
        operation: SemanticOperationId::Intrinsic(crate::IntrinsicId::InfoLevel),
        body: NativeBodyCompilation::Inherit,
    };
    let context = NativeCompilationContext {
        mode: NativeCompilationMode::BytecodeObject,
        frame: NativeCompilationFrame::ProcedureCode,
        catch_depth: Some(0),
        ..Default::default()
    };
    for version in TclVersion::ALL {
        let dialect = InvocationDialect::for_version(version);
        for shapes in [vec![], vec![NativeCompilationWordShape::Substituted]] {
            let arguments = vec!["0"; shapes.len()];
            assert_eq!(
                spec.select(
                    InvocationWords::literals("info", &arguments),
                    &shapes,
                    Some(dialect),
                    context
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
        let shapes = [NativeCompilationWordShape::Literal; 2];
        assert_eq!(
            matches!(
                spec.select(
                    InvocationWords::literals("info", &["extra", "args"]),
                    &shapes,
                    Some(dialect),
                    context
                ),
                NativeCompilationSelection::Generic
            ),
            (version < TclVersion::V8_6)
        );
        assert_eq!(
            spec.select(
                InvocationWords::literals("::tcl::info::level", &["extra", "args"]),
                &shapes,
                Some(dialect),
                context
            ),
            NativeCompilationSelection::Generic
        );
        assert_eq!(
            spec.compiler_hook_presence(dialect),
            Some(version >= TclVersion::V8_6)
        );
        assert_eq!(
            spec.implementation_lookup(dialect).is_some(),
            version >= TclVersion::V8_5
        );
    }
}

#[test]
fn installed_info_commands_worker_keeps_original_selector_arity() {
    // naming.info.original-command-inventory-option-and-scope
    // docs/design/analysis/name-resolution-proofs/info-original-command-inventory-option-and-scope.md
    // Original public rows retain error usage; these independently authored
    // source-policy controls assert no observed opcode or private binding.
    let registry = crate::CommandRegistry::build_default();
    for version in [TclVersion::V8_6, TclVersion::V9_0, TclVersion::V9_1] {
        let dialect = InvocationDialect::for_version(version);
        let profile =
            tcl_dialect::DialectProfile::find(&format!("tcl{}", version.version_string())).unwrap();
        let spec = registry
            .native_compilation_for_registration("::tcl::info::commands", dialect)
            .unwrap();
        for (source, expected) in [
            (b"info commands".as_slice(), 0),
            (b"info com $pattern".as_slice(), 0),
            (b"info commands ::literal".as_slice(), 1),
            (b"info commands -all r2286*".as_slice(), 2),
            (b"info commands a b".as_slice(), 2),
        ] {
            let image = tcl_lexer::SourceImage::native(source);
            let parsed = tcl_lexer::native_script_words_in(
                image.clone(),
                tcl_lexer::Span::new(0, u32::try_from(image.len()).unwrap()),
                tcl_lexer::LexerConfig::from_grammar(profile.grammar),
            )
            .unwrap();
            let words = crate::native_compiler_words::NativeCompilerWords::capture(
                &parsed.commands[0].words,
                tcl_syntax::native_string::NativeStringProtocol::C(version),
            )
            .unwrap();
            let selected = spec.select_registered_worker_native_words(
                &words,
                2,
                Some(dialect),
                NativeCompilationContext {
                    mode: NativeCompilationMode::BytecodeObject,
                    frame: NativeCompilationFrame::ScriptCode,
                    catch_depth: Some(0),
                    loop_depth: 0,
                },
            );
            assert!(
                match expected {
                    0 => matches!(selected, NativeCompilationSelection::NamedInvocation { .. }),
                    1 => matches!(selected, NativeCompilationSelection::Inline { .. }),
                    _ => selected == NativeCompilationSelection::Generic,
                },
                "{version:?}/{source:?}: {selected:?}"
            );
        }
    }
}

#[test]
fn command_enumeration_distinguishes_named_calls_from_absolute_resolution() {
    let spec = NativeCompilationSpec {
        grammar: NativeCompilationGrammar::InfoCommands,
        operation: SemanticOperationId::Intrinsic(crate::IntrinsicId::InfoCommandsResolve),
        body: NativeBodyCompilation::Inherit,
    };
    let context = NativeCompilationContext {
        mode: NativeCompilationMode::BytecodeObject,
        frame: NativeCompilationFrame::ScriptCode,
        catch_depth: Some(0),
        loop_depth: 0,
    };
    for version in TclVersion::ALL {
        for (args, shapes, resolves) in [
            (
                vec!["commands"],
                vec![NativeCompilationWordShape::Literal],
                false,
            ),
            (
                vec!["commands", "::x"],
                vec![NativeCompilationWordShape::Literal; 2],
                true,
            ),
            (
                vec!["commands", "::x*"],
                vec![NativeCompilationWordShape::Literal; 2],
                false,
            ),
            (
                vec!["commands", "::x\\y"],
                vec![NativeCompilationWordShape::Literal; 2],
                false,
            ),
            (
                vec!["commands", "$pattern"],
                vec![
                    NativeCompilationWordShape::Literal,
                    NativeCompilationWordShape::Substituted,
                ],
                false,
            ),
        ] {
            let selected = spec.select(
                InvocationWords::literals("info", &args),
                &shapes,
                Some(InvocationDialect::for_version(version)),
                context,
            );
            if version < TclVersion::V8_6 {
                assert_eq!(selected, NativeCompilationSelection::Generic);
            } else if resolves {
                assert!(
                    matches!(selected, NativeCompilationSelection::Inline { operation, .. }
                        if operation == spec.operation)
                );
            } else {
                assert!(
                    matches!(selected, NativeCompilationSelection::NamedInvocation { arguments_from: 1, lookup, protocol: NativeNamedInvocationProtocol::Direct }
                        if lookup.slot == "::tcl::info::commands")
                );
            }
        }
    }
}

#[test]
fn existence_compiler_requires_original_private_implementation_on_modern_c() {
    let spec = NativeCompilationSpec {
        grammar: NativeCompilationGrammar::InfoExists,
        operation: SemanticOperationId::Intrinsic(crate::IntrinsicId::InfoExists),
        body: NativeBodyCompilation::Inherit,
    };
    for version in TclVersion::ALL {
        let dialect = InvocationDialect::for_version(version);
        let lookup = spec.implementation_lookup(dialect);
        if version == TclVersion::V8_4 {
            assert!(lookup.is_none());
        } else {
            let lookup = lookup.unwrap();
            assert_eq!(lookup.ensemble, "::info");
            assert_eq!(lookup.member, "exists");
            assert_eq!(lookup.slot, "::tcl::info::exists");
            assert_eq!(lookup.command, "info");
            assert_eq!(lookup.prepended, &["exists"]);
            let registry = crate::CommandRegistry::build_default();
            assert!(
                registry
                    .native_compiler_implementation_slots(dialect)
                    .contains(&lookup)
            );
            assert!(
                registry
                    .effective_semantics_for_dialect(dialect)
                    .binding_names()
                    .contains(lookup.slot)
            );
        }
    }
    let profile = tcl_dialect::DialectProfile::projected_from_point(
        "jim",
        &[],
        "Jim",
        tcl_dialect::model::DialectPoint::canonical(tcl_dialect::model::Release::JIM_0_84),
    );
    let jim = InvocationDialect::of_profile(&profile);
    assert!(spec.implementation_lookup(jim).is_none());
    assert_eq!(
        crate::CommandRegistry::build_default()
            .native_compiler_implementation_slots(jim)
            .len(),
        0
    );
}

#[test]
fn existence_hook_accepts_substituted_names_without_a_local_frame() {
    let spec = NativeCompilationSpec {
        grammar: NativeCompilationGrammar::InfoExists,
        operation: SemanticOperationId::Intrinsic(crate::IntrinsicId::InfoExists),
        body: NativeBodyCompilation::Inherit,
    };
    for version in TclVersion::ALL {
        for frame in [
            NativeCompilationFrame::ScriptCode,
            NativeCompilationFrame::ProcedureCode,
        ] {
            for shape in [
                NativeCompilationWordShape::Literal,
                NativeCompilationWordShape::Substituted,
            ] {
                assert_eq!(
                    spec.select(
                        InvocationWords::literals("info", &["a(k)"]),
                        &[shape],
                        Some(InvocationDialect::for_version(version)),
                        context(frame)
                    ),
                    if version == TclVersion::V8_4 {
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
                    InvocationWords::literals("info", &[]),
                    &[],
                    Some(InvocationDialect::for_version(version)),
                    context(frame)
                ),
                NativeCompilationSelection::Generic
            );
        }
    }
}
