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

//! Which stock commands register a compiler hook, and what a command without one keeps.

use super::*;

#[test]
fn selected_no_hook_registration_keeps_generic_argv_under_unknown_mode() {
    // Implementation contract: naming.compiler.no-hook-unknown-mode (docs/design/analysis/name-resolution-proofs/no-hook-unknown-mode.md).
    // Source proof: naming.compiler.stock-procedure-null-compile-hook (docs/design/analysis/name-resolution-proofs/stock-procedure-null-compile-hook.md).
    let registry = crate::CommandRegistry::build_default();
    for version in TclVersion::ALL {
        let profile = tcl_dialect::DialectProfile::find(version.dialect_profile_name()).unwrap();
        let dialect = InvocationDialect::of_profile(profile);
        let spec = registry
            .native_compilation_for_registration("::proc", dialect)
            .unwrap();
        assert_eq!(spec.compiler_hook_presence(dialect), Some(false));
        assert_eq!(spec.grammar, NativeCompilationGrammar::NoHook);
        assert!(
            registry
                .native_compilation_for_registration("::renamedProc", dialect)
                .is_none()
        );
        let source = tcl_lexer::SourceImage::native(b"::proc p\xff {} {}".as_slice());
        let parsed = tcl_lexer::native_script_words_in(
            source.clone(),
            tcl_lexer::Span::new(0, u32::try_from(source.len()).unwrap()),
            tcl_lexer::LexerConfig::from_grammar(profile.grammar),
        )
        .unwrap();
        assert!(parsed.fatal_tail.is_none());
        assert_eq!(parsed.commands.len(), 1);
        let original = &parsed.commands[0].words;
        let captured = crate::native_compiler_words::NativeCompilerWords::capture(
            original,
            tcl_syntax::native_string::NativeStringProtocol::C(version),
        )
        .unwrap();
        assert_eq!(captured.literal(1), Some(b"p\xff".as_slice()));
        let arguments = [
            crate::InvocationWord::Literal("p"),
            crate::InvocationWord::Literal(""),
            crate::InvocationWord::Literal(""),
        ];
        let invocation =
            InvocationWords::structured(crate::InvocationWord::Literal("::proc"), &arguments)
                .with_dialect(dialect);
        for mode in [
            NativeCompilationMode::Direct,
            NativeCompilationMode::BytecodeObject,
            NativeCompilationMode::Unknown,
        ] {
            let context = NativeCompilationContext {
                mode,
                ..Default::default()
            };
            assert_eq!(
                spec.select(invocation, &captured.shapes()[1..], Some(dialect), context),
                NativeCompilationSelection::Generic
            );
            assert_eq!(
                spec.select_native_words(&captured, 1, Some(dialect), context),
                NativeCompilationSelection::Generic
            );
        }
        assert_eq!(
            spec.select_native_words(
                &captured,
                0,
                Some(dialect),
                NativeCompilationContext::default()
            ),
            NativeCompilationSelection::Unknown
        );
        assert_eq!(
            spec.select_native_words(&captured, 1, None, NativeCompilationContext::default()),
            NativeCompilationSelection::Unknown
        );
        let hooked = registry
            .native_compilation_for_registration("::return", dialect)
            .unwrap();
        assert_eq!(hooked.compiler_hook_presence(dialect), Some(true));
        assert_eq!(
            hooked.select_native_words(
                &captured,
                1,
                Some(dialect),
                NativeCompilationContext::default()
            ),
            NativeCompilationSelection::Unknown
        );
    }
}

#[test]
fn namespace_no_hook_registration_keeps_public_and_terminal_body_policy() {
    for version in [TclVersion::V8_6, TclVersion::V9_0, TclVersion::V9_1] {
        let dialect = InvocationDialect::for_version(version);
        let profile =
            tcl_dialect::DialectProfile::find(&format!("tcl{}", version.version_string())).unwrap();
        let registry = crate::CommandRegistry::build_default().project_for_profile(profile);
        let namespace = registry.get("namespace").unwrap();
        for (member, body) in [
            ("eval", NativeBodyCompilation::ScriptObject),
            ("inscope", NativeBodyCompilation::ScriptObject),
            ("ensemble", NativeBodyCompilation::Direct),
        ] {
            let public = namespace
                .resolve_subcommand_for_dialect(member, registry.own_surface_query())
                .unwrap()
                .native_compilation
                .unwrap();
            assert_eq!(public.body, body);
            assert_eq!(public.compiler_hook_presence(dialect), Some(false));
            let lookups = public.implementation_prerequisites(dialect).unwrap();
            assert_eq!(lookups.len(), 1);
            assert_eq!(lookups[0].member, member);
            let terminal = registry
                .native_compilation_for_registration(lookups[0].slot, dialect)
                .unwrap();
            assert_eq!(terminal.body, body);
            assert_eq!(terminal.compiler_hook_presence(dialect), Some(false));
            assert_eq!(
                terminal.implementation_prerequisites(dialect),
                Some(lookups)
            );
        }
    }
}

#[test]
fn original_namespace_no_hook_members_select_monolithic_and_private_registration_epochs() {
    // naming.namespace.original-counted-namespace-allocation
    // docs/design/analysis/name-resolution-proofs/namespace-original-counted-allocation.md
    let context = NativeCompilationContext {
        mode: NativeCompilationMode::BytecodeObject,
        frame: NativeCompilationFrame::ScriptCode,
        ..NativeCompilationContext::default()
    };
    for version in TclVersion::ALL {
        let owner = crate::model::ingress::static_context_for(version.dialect_name());
        let dialect = InvocationDialect::for_version(version);
        let registry = owner.commands();
        let namespace = registry.get("namespace").unwrap();
        for member in ["eval", "inscope", "ensemble"] {
            let Some(member) =
                namespace.resolve_subcommand_for_dialect(member, registry.own_surface_query())
            else {
                continue;
            };
            let spec = member.native_compilation.unwrap();
            let source =
                tcl_lexer::SourceImage::document(&format!("namespace {} child {{}}", member.name));
            let plan = tcl_lexer::native_script_words_in(
                source.clone(),
                tcl_lexer::Span::new(0, u32::try_from(source.len()).unwrap()),
                tcl_lexer::LexerConfig::from_grammar(dialect.lexer_grammar),
            )
            .unwrap();
            let words = crate::native_compiler_words::NativeCompilerWords::capture(
                &plan.commands[0].words,
                tcl_syntax::native_string::NativeStringProtocol::C(version),
            )
            .unwrap();
            let path = spec.implementation_prerequisites(dialect).unwrap();
            assert_eq!(path.len(), usize::from(version >= TclVersion::V8_6));
            assert_eq!(spec.compiler_hook_presence(dialect), Some(false));
            assert_eq!(
                spec.select_native_words(&words, 2, Some(dialect), context),
                NativeCompilationSelection::Generic
            );
            assert_eq!(
                spec.original_argument_name_effects(&words, 2, Some(dialect)),
                NativeOriginalCompilerNameEffects::Preserved
            );
            if let Some(private) = path.first() {
                assert_eq!(private.member, member.name);
                assert!(
                    registry
                        .native_compilation_for_registration(private.slot, dialect)
                        .is_some()
                );
            } else {
                assert!(
                    registry
                        .native_compilation_for_registration(
                            &format!("::tcl::namespace::{}", member.name),
                            dialect,
                        )
                        .is_none()
                );
            }
            assert!(
                spec.implementation_prerequisites(InvocationDialect::of_point(
                    tcl_dialect::model::DialectPoint::canonical(
                        tcl_dialect::model::Release::JIM_0_84
                    ),
                ))
                .is_none()
            );
            assert_eq!(
                spec.select_native_words(&words, 2, None, context),
                NativeCompilationSelection::Unknown
            );
        }
    }
}

#[test]
fn no_hook_implementation_paths_keep_direct_body_and_reject_inconsistent_descriptors() {
    // naming.namespace.original-counted-namespace-allocation
    // docs/design/analysis/name-resolution-proofs/namespace-original-counted-allocation.md
    const LOOKUPS: &[NativeCompilerImplementationLookup] = &[NativeCompilerImplementationLookup {
        ensemble: "::namespace",
        member: "eval",
        slot: "::tcl::namespace::eval",
        command: "namespace",
        prepended: &["eval"],
    }];
    const DIRECT: NativeCompilationSpec = NativeCompilationSpec {
        grammar: NativeCompilationGrammar::NoHook,
        operation: SemanticOperationId::Invoke,
        body: NativeBodyCompilation::Direct,
    };
    const HOOK: NativeCompilationSpec = NativeCompilationSpec {
        grammar: NativeCompilationGrammar::HookFrom(TclVersion::V8_6),
        ..DIRECT
    };
    let direct = NativeCompilationSpec {
        grammar: NativeCompilationGrammar::WithImplementationPath {
            compiler: &DIRECT,
            lookups: LOOKUPS,
            implementation_from: TclVersion::V8_6,
            monolithic_no_hook_before: false,
        },
        operation: DIRECT.operation,
        body: DIRECT.body,
    };
    let modern = InvocationDialect::for_version(TclVersion::V9_0);
    assert_eq!(
        direct.implementation_prerequisites(modern),
        Some(LOOKUPS.to_vec())
    );
    assert_eq!(direct.compiler_hook_presence(modern), Some(false));
    assert_eq!(direct.body, NativeBodyCompilation::Direct);
    assert!(
        NativeCompilationSpec {
            body: NativeBodyCompilation::ScriptObject,
            ..direct
        }
        .implementation_prerequisites(modern)
        .is_none()
    );
    assert!(
        NativeCompilationSpec {
            grammar: NativeCompilationGrammar::WithImplementationPath {
                compiler: &HOOK,
                lookups: LOOKUPS,
                implementation_from: TclVersion::V8_6,
                monolithic_no_hook_before: false,
            },
            ..direct
        }
        .implementation_prerequisites(modern)
        .is_none()
    );
    assert!(
        direct
            .implementation_prerequisites(InvocationDialect::for_version(TclVersion::V8_5))
            .is_none()
    );
    assert!(
        NativeCompilationSpec {
            grammar: NativeCompilationGrammar::WithImplementationPath {
                compiler: &HOOK,
                lookups: LOOKUPS,
                implementation_from: TclVersion::V8_6,
                monolithic_no_hook_before: true,
            },
            ..direct
        }
        .implementation_prerequisites(InvocationDialect::for_version(TclVersion::V8_5))
        .is_none()
    );
    let jim = InvocationDialect::of_point(tcl_dialect::model::DialectPoint::canonical(
        tcl_dialect::model::Release::JIM_0_84,
    ));
    assert!(direct.implementation_prerequisites(jim).is_none());
}

#[test]
fn stock_tcloo_commands_have_generic_dispatch_without_native_compiler_hooks() {
    let registry = crate::CommandRegistry::build_default();
    for version in [TclVersion::V8_6, TclVersion::V9_0, TclVersion::V9_1] {
        for name in [
            "oo::class",
            "oo::object",
            "oo::define",
            "oo::objdefine",
            "oo::copy",
        ] {
            let native = registry.get(name).unwrap().native_compilation.unwrap();
            let words = InvocationWords::literals(name, &["create", "C", "{}"]);
            assert_eq!(
                native.select(
                    words,
                    &[NativeCompilationWordShape::Literal; 3],
                    Some(InvocationDialect::for_version(version)),
                    context(NativeCompilationFrame::ProcedureCode),
                ),
                NativeCompilationSelection::Generic,
                "{name} {version:?}",
            );
        }
    }
}

#[test]
fn error_hook_accepts_retained_operands_only_from_its_authored_release() {
    let registry = crate::CommandRegistry::build_default();
    for version in TclVersion::ALL {
        let native = registry
            .get_for_surface(
                "error",
                InvocationDialect::for_version(version).authoring_query(),
            )
            .unwrap()
            .native_compilation
            .unwrap();
        for frame in [
            NativeCompilationFrame::ScriptCode,
            NativeCompilationFrame::ProcedureCode,
        ] {
            for count in 0..=4 {
                let arguments = vec!["message"; count];
                let shapes = vec![NativeCompilationWordShape::Substituted; count];
                let selected = native.select(
                    InvocationWords::literals("error", &arguments),
                    &shapes,
                    Some(InvocationDialect::for_version(version)),
                    context(frame),
                );
                assert_eq!(
                    selected,
                    if version >= TclVersion::V8_6 && (1..=3).contains(&count) {
                        NativeCompilationSelection::Inline {
                            operation: native.operation,
                            guard: NativeCompilationGuard::BeforeArguments,
                        }
                    } else {
                        NativeCompilationSelection::Generic
                    }
                );
            }
        }
    }
}

#[test]
fn actual_stock_hook_availability_and_return_catch_context_are_version_aware() {
    let registry = crate::CommandRegistry::build_default();
    for version in TclVersion::ALL {
        let dialect = Some(InvocationDialect::for_version(version));
        let empty = InvocationWords::literals("puts", &[]);
        assert_eq!(
            registry
                .get("puts")
                .unwrap()
                .native_compilation
                .unwrap()
                .select(
                    empty,
                    &[],
                    dialect,
                    context(NativeCompilationFrame::ProcedureCode)
                ),
            NativeCompilationSelection::Generic
        );
        let upvar = registry.get("upvar").unwrap().native_compilation.unwrap();
        assert_eq!(
            upvar.select(
                InvocationWords::literals("upvar", &[]),
                &[],
                dialect,
                context(NativeCompilationFrame::ProcedureCode)
            ),
            NativeCompilationSelection::Generic
        );
        let spec = registry.get("return").unwrap().native_compilation.unwrap();
        let words = InvocationWords::literals("return", &[]);
        assert!(matches!(
            spec.select(
                words,
                &[],
                dialect,
                context(NativeCompilationFrame::ProcedureCode)
            ),
            NativeCompilationSelection::Inline { .. }
        ));
        let mut protected = context(NativeCompilationFrame::ProcedureCode);
        protected.catch_depth = Some(1);
        assert_eq!(
            matches!(
                spec.select(words, &[], dialect, protected),
                NativeCompilationSelection::Generic
            ),
            version == TclVersion::V8_4
        );
        let uplevel = registry.get("uplevel").unwrap().native_compilation.unwrap();
        assert_eq!(
            uplevel.select(
                InvocationWords::literals("uplevel", &[]),
                &[],
                dialect,
                context(NativeCompilationFrame::ProcedureCode)
            ),
            NativeCompilationSelection::Generic
        );
    }
}
