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
use crate::hooks::LoweringHookId;

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
fn original_literal_set_compiler_effects_do_not_grant_opcode_admission() {
    // Implementation contract: naming.compiler.original-name-effect-separation (docs/design/analysis/name-resolution-proofs/original-compiler-name-effect-separation.md).
    // Source proof: naming.compiler.literal-set-name-effects-source (docs/design/analysis/name-resolution-proofs/literal-set-name-effects-source.md).
    let registry = crate::CommandRegistry::build_default();
    for version in TclVersion::ALL {
        let profile = tcl_dialect::DialectProfile::find(version.dialect_profile_name()).unwrap();
        let dialect = InvocationDialect::of_profile(profile);
        let spec = registry
            .native_compilation_for_registration("::set", dialect)
            .unwrap();
        for (source, closed) in [
            (b"set checkpoint READY".as_slice(), true),
            (b"set {a([unevaluated])} {opaque\xff}".as_slice(), true),
            (b"set x".as_slice(), true),
            (b"set x [proc q {} {}]".as_slice(), false),
            (b"set $name 7".as_slice(), false),
            (b"set x $value".as_slice(), false),
            (b"set x 7 extra".as_slice(), false),
        ] {
            let image = tcl_lexer::SourceImage::native(source);
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
            assert_eq!(
                spec.preserves_names_before_original_arguments(&captured, 1, Some(dialect)),
                closed,
                "{version:?}: {source:?}"
            );
            assert_eq!(
                spec.select_native_words(
                    &captured,
                    1,
                    Some(dialect),
                    NativeCompilationContext::default()
                ),
                NativeCompilationSelection::Unknown
            );
            assert!(!spec.preserves_names_before_original_arguments(&captured, 0, Some(dialect)));
            assert!(!spec.preserves_names_before_original_arguments(&captured, 1, None));
            let unavailable = NativeCompilationSpec {
                grammar: NativeCompilationGrammar::Unresolved,
                ..spec
            };
            assert!(!unavailable.preserves_names_before_original_arguments(
                &captured,
                1,
                Some(dialect)
            ));
            let mismatched = NativeCompilationSpec {
                operation: SemanticOperationId::Invoke,
                ..spec
            };
            assert!(!mismatched.preserves_names_before_original_arguments(
                &captured,
                1,
                Some(dialect)
            ));
        }
    }
}

#[test]
fn original_literal_emitter_families_preserve_names_without_native_admission() {
    // Implementation contract: naming.compiler.literal-emitter-name-effects (docs/design/analysis/name-resolution-proofs/literal-emitter-name-effects.md).
    // Source proof: naming.compiler.literal-emitter-name-effects-source (docs/design/analysis/name-resolution-proofs/literal-emitter-name-effects-source.md).
    let registry = crate::CommandRegistry::build_default();
    for version in TclVersion::ALL {
        let profile = tcl_dialect::DialectProfile::find(version.dialect_profile_name()).unwrap();
        let dialect = InvocationDialect::of_profile(profile);
        for (registration, source) in [
            ("::return", b"return VALUE".as_slice()),
            ("::list", b"list a b".as_slice()),
            ("::incr", b"incr counter 1".as_slice()),
            ("::append", b"append text SUFFIX".as_slice()),
            ("::llength", b"llength {a b}".as_slice()),
            ("::lindex", b"lindex {a b} 1".as_slice()),
            ("::lassign", b"lassign {a b} first second".as_slice()),
            ("::global", b"global ::a ::b".as_slice()),
            ("::upvar", b"upvar 1 source local".as_slice()),
            ("::variable", b"variable v VALUE".as_slice()),
        ] {
            if registration == "::lassign" && version == TclVersion::V8_4 {
                // Tcl 8.4's stock command table has no lassign registration.
                assert!(
                    registry
                        .native_compilation_for_registration(registration, dialect)
                        .is_none()
                );
                continue;
            }
            let spec = registry
                .native_compilation_for_registration(registration, dialect)
                .unwrap_or_else(|| panic!("{version:?}: missing stock {registration}"));
            let image = tcl_lexer::SourceImage::native(source);
            let plan = tcl_lexer::native_script_words_in(
                image.clone(),
                tcl_lexer::Span::new(0, u32::try_from(image.len()).unwrap()),
                tcl_lexer::LexerConfig::from_grammar(profile.grammar),
            )
            .unwrap();
            let original = &plan.commands[0].words;
            let words = crate::native_compiler_words::NativeCompilerWords::capture(
                original,
                tcl_syntax::native_string::NativeStringProtocol::C(version),
            )
            .unwrap();
            assert!(
                spec.preserves_names_before_original_arguments(&words, 1, Some(dialect)),
                "{version:?} {registration}"
            );
            let admission = spec.select_native_words(
                &words,
                1,
                Some(dialect),
                NativeCompilationContext::default(),
            );
            assert_eq!(
                admission,
                if spec.compiler_hook_presence(dialect) == Some(false) {
                    NativeCompilationSelection::Generic
                } else {
                    NativeCompilationSelection::Unknown
                },
                "{version:?} {registration}"
            );
            assert!(!spec.preserves_names_before_original_arguments(&words, 0, Some(dialect)));
            assert!(!spec.preserves_names_before_original_arguments(&words, 1, None));
        }
        for registration in ["::expr", "::if", "::while", "::catch"] {
            let spec = registry
                .native_compilation_for_registration(registration, dialect)
                .unwrap();
            let image = tcl_lexer::SourceImage::native(b"command {[rename p q]}".as_slice());
            let plan = tcl_lexer::native_script_words_in(
                image.clone(),
                tcl_lexer::Span::new(0, u32::try_from(image.len()).unwrap()),
                tcl_lexer::LexerConfig::from_grammar(profile.grammar),
            )
            .unwrap();
            let words = crate::native_compiler_words::NativeCompilerWords::capture(
                &plan.commands[0].words,
                tcl_syntax::native_string::NativeStringProtocol::C(version),
            )
            .unwrap();
            assert!(
                !spec.preserves_names_before_original_arguments(&words, 1, Some(dialect)),
                "{version:?} {registration}"
            );
        }
    }
}

#[test]
fn original_compiler_effect_visits_preserve_native_control_pruning() {
    // Implementation contract: naming.compiler.original-preparation-name-effects (docs/design/analysis/name-resolution-proofs/original-preparation-name-effects.md).
    let registry = crate::CommandRegistry::build_default();
    for version in TclVersion::ALL {
        let profile = tcl_dialect::DialectProfile::find(version.dialect_profile_name()).unwrap();
        let dialect = InvocationDialect::of_profile(profile);
        let spec = registry
            .native_compilation_for_registration("::if", dialect)
            .unwrap();
        for (source, bodies) in [
            ("if {1} {package ifneeded p 1.0 {source p.tcl}}", 1),
            ("if {0} {custom_compiler}", 0),
            ("if {$condition} {set x 1} else {set x 2}", 2),
        ] {
            let image = tcl_lexer::SourceImage::document(source);
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
            let context = NativeCompilationContext::default();
            let NativeOriginalCompilerNameEffects::Visits(visits) =
                spec.original_argument_name_effects_in_context(&words, 1, Some(dialect), context)
            else {
                panic!("{version:?}: original preparation visits");
            };
            assert_eq!(
                    visits
                        .iter()
                        .filter(|visit| matches!(visit,
                    crate::native_control_compilation::NativeControlPreparationStep::Script { .. }))
                        .count(),
                    bodies
                );
            assert_eq!(
                spec.select_native_words(&words, 1, Some(dialect), context),
                NativeCompilationSelection::Unknown
            );
            assert_eq!(
                spec.original_argument_name_effects_in_context(&words, 0, Some(dialect), context),
                NativeOriginalCompilerNameEffects::Unknown
            );
        }
    }
}

#[test]
fn original_named_compiler_effects_require_separate_fixed_lookup() {
    // Implementation contract: naming.compiler.fixed-helper-name-effects (docs/design/analysis/name-resolution-proofs/fixed-helper-name-effects.md).
    let registry = crate::CommandRegistry::build_default();
    for version in [TclVersion::V8_6, TclVersion::V9_0, TclVersion::V9_1] {
        let profile = tcl_dialect::DialectProfile::find(version.dialect_profile_name()).unwrap();
        let dialect = InvocationDialect::of_profile(profile);
        let spec = registry
            .native_compilation_for_registration("::tcl::namespace::exists", dialect)
            .unwrap();
        let source = tcl_lexer::SourceImage::native(b"namespace exists ::A".as_slice());
        let plan = tcl_lexer::native_script_words_in(
            source.clone(),
            tcl_lexer::Span::new(0, u32::try_from(source.len()).unwrap()),
            tcl_lexer::LexerConfig::from_grammar(profile.grammar),
        )
        .unwrap();
        let words = crate::native_compiler_words::NativeCompilerWords::capture(
            &plan.commands[0].words,
            tcl_syntax::native_string::NativeStringProtocol::C(version),
        )
        .unwrap();
        let NativeOriginalCompilerNameEffects::FixedLookup(lookup) =
            spec.original_argument_name_effects(&words, 2, Some(dialect))
        else {
            panic!("{version:?}: unresolved original helper lookup");
        };
        assert_eq!(lookup.slot, "::tcl::namespace::exists");
        assert!(!spec.preserves_names_before_original_arguments(&words, 2, Some(dialect)));
        assert_eq!(
            spec.select_native_words(
                &words,
                2,
                Some(dialect),
                NativeCompilationContext::default()
            ),
            NativeCompilationSelection::Unknown
        );
        assert_eq!(
            spec.original_argument_name_effects(&words, 0, Some(dialect)),
            NativeOriginalCompilerNameEffects::Unknown
        );
        assert_eq!(
            spec.original_argument_name_effects(&words, 2, None),
            NativeOriginalCompilerNameEffects::Unknown
        );
        let spec = registry
            .native_compilation_for_registration("::tcl::namespace::eval", dialect)
            .unwrap();
        assert_eq!(spec.compiler_hook_presence(dialect), Some(false));
        assert_eq!(
            spec.original_argument_name_effects(&words, 2, Some(dialect)),
            NativeOriginalCompilerNameEffects::Preserved
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
fn coroutine_compilation_preserves_native_arity_and_expansion_gates() {
    let context = NativeCompilationContext {
        mode: NativeCompilationMode::BytecodeObject,
        frame: NativeCompilationFrame::ScriptCode,
        ..NativeCompilationContext::default()
    };
    let registry = crate::model::ingress::static_context_for("tcl9.1").commands();
    let yield_spec = registry.get("yield").unwrap().native_compilation.unwrap();
    let relay = registry.get("yieldto").unwrap().native_compilation.unwrap();
    for version in [TclVersion::V8_6, TclVersion::V9_0, TclVersion::V9_1] {
        let dialect = Some(InvocationDialect::for_version(version));
        for count in 0..=2 {
            let values = vec!["value"; count];
            let shapes = vec![NativeCompilationWordShape::Substituted; count];
            let selected = yield_spec.select(
                InvocationWords::literals("yield", &values),
                &shapes,
                dialect,
                context,
            );
            assert_eq!(
                matches!(selected, NativeCompilationSelection::Inline { .. }),
                count <= 1
            );
            let selected = relay.select(
                InvocationWords::literals("yieldto", &values),
                &shapes,
                dialect,
                context,
            );
            assert_eq!(
                matches!(selected, NativeCompilationSelection::Inline { .. }),
                count > 0 || version == TclVersion::V9_1
            );
        }
        let words = [crate::InvocationWord::Expanded];
        let selected = relay.select(
            InvocationWords::structured(crate::InvocationWord::Literal("yieldto"), &words),
            &[NativeCompilationWordShape::Expanded],
            dialect,
            context,
        );
        assert_eq!(
            matches!(selected, NativeCompilationSelection::Inline { .. }),
            version == TclVersion::V9_1
        );
    }
    assert_eq!(
        relay.select(
            InvocationWords::literals("yieldto", &["command"]),
            &[NativeCompilationWordShape::Opaque],
            Some(InvocationDialect::for_version(TclVersion::V9_1)),
            context,
        ),
        NativeCompilationSelection::Unknown
    );
}

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
fn possible_handler_bodies_share_grammar_without_native_compiler_licence() {
    let registry = crate::model::ingress::static_context_for("f5-irules").commands();
    let dialect = crate::InvocationDialect::of_profile(tcl_dialect::DialectProfile::irules());
    let words = ["$ready", "set flag 1", "else", "set flag 2"];
    let facts = registry
        .resolve_invocation("if", &words, registry.own_surface_query())
        .unwrap()
        .facts();
    let arguments = crate::InvocationArguments::literals(&words).with_dialect(dialect);
    let entry = facts
        .possible_handler_body_flow(registry, arguments, crate::VariableAliasFrame::Procedure)
        .expect("audited native handler candidate bodies");
    assert_eq!(
        entry.flow,
        crate::script_body_flow::ScriptBodyFlow::Conditional(vec![(Some(0), 1), (None, 3)])
    );
    assert!(
        facts
            .successful_handler_effects(arguments, crate::VariableAliasFrame::Procedure)
            .is_none()
    );
    assert!(
        facts
            .possible_handler_body_flow(registry, arguments, crate::VariableAliasFrame::Unknown)
            .is_none()
    );
}

#[test]
fn successful_handler_metadata_does_not_depend_on_vendor_compiler_selection() {
    let registry = crate::model::ingress::static_context_for("f5-irules").commands();
    let dialect = crate::InvocationDialect::of_profile(tcl_dialect::DialectProfile::irules());
    for (head, words) in [
        ("IP::client_addr", vec![]),
        ("append", vec!["::counter", "x"]),
        ("unset", vec!["::counter"]),
        ("array", vec!["set", "::items", "a 1"]),
        ("scan", vec!["value", "%s", "::parsed"]),
        ("catch", vec!["set x 1", "::err"]),
    ] {
        let facts = registry
            .resolve_invocation(head, &words, registry.own_surface_query())
            .unwrap()
            .facts();
        let arguments = crate::InvocationArguments::literals(&words).with_dialect(dialect);
        assert!(
            facts
                .successful_handler_effects(arguments, crate::VariableAliasFrame::Procedure)
                .is_some(),
            "{head}"
        );
        assert!(facts.successful_handler.is_some(), "{head}");
        if head != "IP::client_addr" {
            assert!(
                facts
                    .successful_handler_effects(arguments, crate::VariableAliasFrame::Unknown)
                    .is_none(),
                "{head}: unknown frame cannot license address transfer"
            );
        }
    }
}

#[test]
fn catch_output_binding_phase_and_legacy_arity_stay_explicit() {
    assert_eq!(
        SuccessfulHandlerSpec::CatchOutputs.variable_binding_phase(),
        VariableOperandBindingPhase::NormalContinuation
    );
    let registry = crate::model::ingress::static_context_for("f5-irules").commands();
    let words = ["set x 1", "::result", "::options"];
    let facts = registry
        .resolve_invocation("catch", &words, registry.own_surface_query())
        .unwrap()
        .facts();
    let args = crate::InvocationArguments::literals(&words).with_dialect(
        crate::InvocationDialect::of_profile(tcl_dialect::DialectProfile::irules()),
    );
    assert!(
        facts
            .successful_handler_effects(args, crate::VariableAliasFrame::Procedure)
            .is_none()
    );
}

fn context(frame: NativeCompilationFrame) -> NativeCompilationContext {
    NativeCompilationContext {
        mode: NativeCompilationMode::BytecodeObject,
        frame,
        loop_depth: 0,
        catch_depth: Some(0),
    }
}

#[test]
fn uplevel_compiler_preserves_original_frame_probe_and_release_floor() {
    let spec = NativeCompilationSpec {
        grammar: NativeCompilationGrammar::Uplevel,
        operation: SemanticOperationId::StructuredLowering(LoweringHookId::Uplevel),
        body: NativeBodyCompilation::Uplevel,
    };
    for version in TclVersion::ALL {
        let dialect = InvocationDialect::for_version(version);
        for (first, valid) in [
            ("#0", true),
            ("set", true),
            ("", true),
            ("-0", true),
            ("0d0", true),
            ("1.0", true),
            ("-1", false),
            ("2147483648", false),
            ("#2147483648", false),
        ] {
            let selection = spec.select(
                InvocationWords::literals("uplevel", &[first, "set ::x RIGHT"]),
                &[NativeCompilationWordShape::BracedLiteral; 2],
                Some(dialect),
                context(NativeCompilationFrame::ProcedureCode),
            );
            assert_eq!(
                matches!(selection, NativeCompilationSelection::Inline { .. }),
                version == TclVersion::V9_1 && valid,
                "{version:?} first={first:?}",
            );
        }
        for (frame, first_shape) in [
            (
                NativeCompilationFrame::ScriptCode,
                NativeCompilationWordShape::Literal,
            ),
            (
                NativeCompilationFrame::ProcedureCode,
                NativeCompilationWordShape::Substituted,
            ),
        ] {
            assert_eq!(
                spec.select(
                    InvocationWords::literals("uplevel", &["#0", "set ::x RIGHT"]),
                    &[first_shape, NativeCompilationWordShape::Substituted],
                    Some(dialect),
                    context(frame),
                ),
                NativeCompilationSelection::Generic
            );
        }
    }
}

#[test]
fn checked_arity_retains_native_entry_error_and_argument_timing() {
    let registry = crate::model::ingress::static_context_for("tcl8.6").commands();
    let facts = registry
        .resolve_invocation("llength", &["items"], registry.own_surface_query())
        .unwrap()
        .facts();
    let spec = facts.native_compilation.unwrap();
    for version in TclVersion::ALL {
        let dialect = InvocationDialect::for_version(version);
        let entry = context(NativeCompilationFrame::ScriptCode);
        assert!(matches!(
            spec.select(
                InvocationWords::literals("renamed", &["items"]),
                &[NativeCompilationWordShape::Substituted],
                Some(dialect),
                entry,
            ),
            NativeCompilationSelection::Inline { .. }
        ));
        let selection = spec.select(
            InvocationWords::literals("renamed", &["items", "extra"]),
            &[NativeCompilationWordShape::Substituted; 2],
            Some(dialect),
            entry,
        );
        assert_eq!(
            selection,
            if version == TclVersion::V8_4 {
                NativeCompilationSelection::CompileError
            } else {
                NativeCompilationSelection::Generic
            }
        );
        assert_eq!(
            spec.failure_for_selection(
                selection,
                InvocationWords::literals("renamed", &["items", "extra"]),
                &facts,
                Some(dialect),
            )
            .is_some(),
            version == TclVersion::V8_4,
        );
    }
}

#[test]
fn string_equal_retains_raw_member_and_versioned_private_fallback() {
    let spec = NativeCompilationSpec {
        grammar: NativeCompilationGrammar::StringEqual(
            crate::native_scalar_compilation::NativeScalarScope::PublicMember,
        ),
        operation: SemanticOperationId::Intrinsic(crate::IntrinsicId::StringEqual),
        body: NativeBodyCompilation::Inherit,
    };
    for version in TclVersion::ALL {
        let dialect = InvocationDialect::for_version(version);
        for shape in [
            NativeCompilationWordShape::Literal,
            NativeCompilationWordShape::QuotedLiteral,
            NativeCompilationWordShape::BracedLiteral,
        ] {
            let selection = spec.select(
                InvocationWords::literals("string", &["equal", "a", "a"]),
                &[
                    shape,
                    NativeCompilationWordShape::Substituted,
                    NativeCompilationWordShape::Substituted,
                ],
                Some(dialect),
                context(NativeCompilationFrame::ScriptCode),
            );
            assert_eq!(
                matches!(selection, NativeCompilationSelection::Inline { .. }),
                version > TclVersion::V8_4 || shape == NativeCompilationWordShape::Literal,
            );
        }
        let options = spec.select(
            InvocationWords::literals("string", &["equal", "-nocase", "a", "A"]),
            &[NativeCompilationWordShape::Literal; 4],
            Some(dialect),
            context(NativeCompilationFrame::ScriptCode),
        );
        assert_eq!(
            matches!(options, NativeCompilationSelection::NamedInvocation { .. }),
            version >= TclVersion::V8_6,
        );
        assert_eq!(
            spec.implementation_lookup(dialect).is_some(),
            version >= TclVersion::V8_5
        );
    }
}

#[test]
fn string_length_requires_exact_member_shape_and_retains_native_arity_fallback() {
    let spec = NativeCompilationSpec {
        grammar: NativeCompilationGrammar::StringLength(
            crate::native_scalar_compilation::NativeScalarScope::PublicMember,
        ),
        operation: SemanticOperationId::Intrinsic(crate::IntrinsicId::StringLength),
        body: NativeBodyCompilation::Inherit,
    };
    for version in TclVersion::ALL {
        let dialect = InvocationDialect::for_version(version);
        for shape in [
            NativeCompilationWordShape::Literal,
            NativeCompilationWordShape::QuotedLiteral,
            NativeCompilationWordShape::BracedLiteral,
        ] {
            let selected = spec.select(
                InvocationWords::literals("string", &["length", "value"]),
                &[shape, NativeCompilationWordShape::Substituted],
                Some(dialect),
                context(NativeCompilationFrame::ScriptCode),
            );
            assert_eq!(
                matches!(selected, NativeCompilationSelection::Inline { .. }),
                version > TclVersion::V8_4 || shape == NativeCompilationWordShape::Literal
            );
        }
        let selected = spec.select(
            InvocationWords::literals("string", &["length", "a", "b"]),
            &[NativeCompilationWordShape::Literal; 3],
            Some(dialect),
            context(NativeCompilationFrame::ProcedureCode),
        );
        assert_eq!(
            matches!(
                selected,
                NativeCompilationSelection::NamedInvocation {
                    protocol: NativeNamedInvocationProtocol::EnsembleRewrite,
                    ..
                }
            ),
            version >= TclVersion::V8_6
        );
        assert_eq!(
            spec.implementation_lookup(dialect).is_some(),
            version >= TclVersion::V8_5
        );
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
fn known_return_options_have_their_actual_versioned_compiler_protocol() {
    let registry = crate::CommandRegistry::build_default();
    let spec = registry.get("return").unwrap().native_compilation.unwrap();
    for version in [
        TclVersion::V8_4,
        TclVersion::V8_5,
        TclVersion::V8_6,
        TclVersion::V9_0,
        TclVersion::V9_1,
    ] {
        let dialect = InvocationDialect::for_version(version);
        let selected = spec.select(
            InvocationWords::literals("return", &["-code", "error", "absent"]),
            &[NativeCompilationWordShape::Literal; 3],
            Some(dialect),
            context(NativeCompilationFrame::ProcedureCode),
        );
        if version == TclVersion::V8_4 {
            assert_eq!(selected, NativeCompilationSelection::Generic);
        } else {
            assert!(matches!(
                selected,
                NativeCompilationSelection::Inline { .. }
            ));
        }
        assert_eq!(
            spec.select(
                InvocationWords::literals("return", &["-level", "-1", "absent"]),
                &[NativeCompilationWordShape::Literal; 3],
                Some(dialect),
                context(NativeCompilationFrame::ProcedureCode),
            ),
            NativeCompilationSelection::Generic
        );
    }
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
fn sequence_normal_effects_require_no_native_expression_callback() {
    let registry = crate::CommandRegistry::build_default();
    for version in [TclVersion::V9_0, TclVersion::V9_1] {
        let dialect = InvocationDialect::for_version(version);
        for (operands, closed) in [
            (vec!["3"], true),
            (vec!["1", "to", "5", "by", "2"], true),
            (vec!["1+2"], false),
            (vec!["[set ::changed yes]"], false),
            (vec!["1", "count"], false),
        ] {
            let words = InvocationWords::literals("lseq", &operands).with_dialect(dialect);
            let resolution =
                registry.resolve_structured_invocation(words, dialect.authoring_query());
            let facts = resolution.resolved().unwrap().facts();
            assert_eq!(
                facts
                    .successful_value_leaf_world(words.arguments())
                    .is_some(),
                closed,
                "{version:?}: {operands:?}"
            );
            if facts.arity_accepts_frozen_arguments() == Some(true) {
                assert!(
                    facts
                        .normal_list_method_provider(words.arguments())
                        .is_some()
                );
            }
            let spec = facts.native_compilation.unwrap();
            assert_eq!(
                spec.select(
                    words,
                    &vec![NativeCompilationWordShape::Literal; operands.len()],
                    Some(dialect),
                    context(NativeCompilationFrame::ProcedureCode)
                ),
                if version == TclVersion::V9_0 {
                    NativeCompilationSelection::Generic
                } else {
                    NativeCompilationSelection::Unknown
                }
            );
        }
    }
}

#[test]
fn successful_handler_effects_do_not_select_compiler_or_body_protocol() {
    let registry = crate::CommandRegistry::build_default();
    let arguments = crate::InvocationArguments::literals(&["x", "value"]);
    for name in ["set", "incr", "list"] {
        let spec = registry.get(name).unwrap().native_compilation.unwrap();
        let effects = spec
            .successful_handler_effects(arguments, crate::VariableAliasFrame::Procedure)
            .expect("normative normal handler effects");
        assert_eq!(effects.operation, spec.operation);
        if name != "list" {
            assert!(
                spec.successful_handler_effects(arguments, crate::VariableAliasFrame::Unknown)
                    .is_none()
            );
        }
    }
    for name in ["expr", "if", "while", "return", "proc"] {
        let spec = registry.get(name).unwrap().native_compilation.unwrap();
        assert!(
            spec.successful_handler_effects(arguments, crate::VariableAliasFrame::Procedure)
                .is_none(),
            "{name}: body/control protocol must remain unproved"
        );
    }
    let spec = registry.get("set").unwrap().native_compilation.unwrap();
    assert!(
        spec.successful_handler_effects(
            crate::InvocationArguments::structured(&[
                crate::InvocationWord::Dynamic,
                crate::InvocationWord::Literal("value"),
            ]),
            crate::VariableAliasFrame::Procedure,
        )
        .is_none()
    );
    assert!(
        spec.successful_handler_effects(
            crate::InvocationArguments::literals(&["x", "value", "extra"]),
            crate::VariableAliasFrame::Procedure,
        )
        .is_none()
    );
}

#[test]
fn expanded_no_body_compilation_is_independent_of_runtime_argv_length() {
    use crate::InvocationWord::{Expanded, Literal};
    let registry = crate::model::ingress::static_context_for("tcl9.1").commands();
    let dialect = InvocationDialect::for_version(TclVersion::V9_1);
    let arguments = [Literal("x"), Expanded];
    let shapes = [
        NativeCompilationWordShape::Literal,
        NativeCompilationWordShape::Expanded,
    ];
    for head in ["lappend", "proc", "foreach", "eval", "try"] {
        let words = InvocationWords::structured(Literal(head), &arguments).with_dialect(dialect);
        let resolution = registry.resolve_structured_invocation(words, dialect.authoring_query());
        let facts = resolution
            .resolved()
            .expect("actual stock invocation")
            .facts();
        let spec = facts.native_compilation.expect("authored native compiler");
        let selected = spec.select(
            words,
            &shapes,
            Some(dialect),
            context(NativeCompilationFrame::ProcedureCode),
        );
        assert_eq!(words.arguments().exact_argv_len(), None);
        if head == "lappend" {
            assert!(matches!(
                selected,
                NativeCompilationSelection::Inline { .. }
            ));
            assert!(facts.arg_roles_complete);
            assert_eq!(
                spec.compilation_steps(selected, words, &shapes, &facts, Some(dialect)),
                NativeCompilationSteps::Known(Vec::new())
            );
            let mut incomplete = facts.clone();
            incomplete.arg_roles_complete = false;
            assert_eq!(
                spec.compiled_bodies(selected, words, &shapes, &incomplete, Some(dialect)),
                NativeCompiledBodies::Unknown
            );
        } else {
            assert!(
                !matches!(selected, NativeCompilationSelection::Inline { .. }),
                "{head}: expansion cannot select an original inline body"
            );
        }
        assert_eq!(
            spec.compiled_bodies(
                NativeCompilationSelection::Unknown,
                words,
                &shapes,
                &facts,
                Some(dialect)
            ),
            NativeCompiledBodies::Unknown,
            "{head}: no-body metadata cannot close unknown compiler selection"
        );
    }
}

fn compiled_arguments(head: &str, arguments: &[&str]) -> NativeCompiledBodies {
    let registry =
        crate::cache::registry_for_profile(tcl_dialect::DialectProfile::find("tcl8.4").unwrap());
    let facts = registry
        .resolve_invocation(head, arguments, registry.own_surface_query())
        .expect("stock native invocation")
        .facts();
    let spec = registry.get(head).unwrap().native_compilation.unwrap();
    let words = InvocationWords::literals(head, arguments);
    let shapes = vec![NativeCompilationWordShape::BracedLiteral; arguments.len()];
    let dialect = Some(InvocationDialect::for_version(TclVersion::V8_4));
    let selected = spec.select(
        words,
        &shapes,
        dialect,
        context(NativeCompilationFrame::ProcedureCode),
    );
    assert!(matches!(
        selected,
        NativeCompilationSelection::Inline { .. }
    ));
    spec.compiled_bodies(selected, words, &shapes, &facts, dialect)
}

#[test]
fn compiled_bodies_use_literal_boolean_pruning_instead_of_runtime_folding() {
    use NativeCompilationFailureScope::EnclosingChunk;
    let known =
        |indices: &[usize], loops: &[usize], annotations: &[NativeCompiledBodyErrorContext]| {
            NativeCompiledBodies::Known(
                indices
                    .iter()
                    .zip(annotations)
                    .map(|(argument, annotation)| NativeCompiledBodyOperand {
                        error_context: *annotation,
                        argument: *argument,
                        context: if loops.contains(argument) {
                            NativeCompiledBodyContext::Loop
                        } else {
                            NativeCompiledBodyContext::Inherit
                        },
                        failure_scope: EnclosingChunk,
                    })
                    .collect(),
            )
        };
    for condition in ["0", "00", "0.0", "off", "false", "0x0"] {
        assert_eq!(
            compiled_arguments("if", &[condition, "bad", "else", "good"]),
            known(&[3], &[], &[NativeCompiledBodyErrorContext::IfElse])
        );
        assert_eq!(
            compiled_arguments("while", &[condition, "bad"]),
            known(&[], &[], &[])
        );
    }
    for condition in ["0+0", "2-2", "$flag"] {
        assert_eq!(
            compiled_arguments("if", &[condition, "bad", "else", "good"]),
            known(
                &[1, 3],
                &[],
                &[
                    NativeCompiledBodyErrorContext::IfThen,
                    NativeCompiledBodyErrorContext::IfElse
                ]
            )
        );
        assert_eq!(
            compiled_arguments("while", &[condition, "bad"]),
            known(&[1], &[1], &[NativeCompiledBodyErrorContext::WhileBody])
        );
    }
    assert_eq!(
        compiled_arguments("if", &["1", "good", "else", "bad"]),
        known(&[1], &[], &[NativeCompiledBodyErrorContext::IfThen])
    );
    assert_eq!(
        compiled_arguments("for", &["init", "0", "next", "body"]),
        known(
            &[0, 3, 2],
            &[3, 2],
            &[
                NativeCompiledBodyErrorContext::ForInitial,
                NativeCompiledBodyErrorContext::ForBody,
                NativeCompiledBodyErrorContext::ForNext
            ]
        )
    );
    assert_eq!(
        compiled_arguments("foreach", &["x", "", "body"]),
        known(&[2], &[2], &[NativeCompiledBodyErrorContext::ForeachBody])
    );
    assert_eq!(
        compiled_arguments("catch", &["body"]),
        NativeCompiledBodies::Known(vec![NativeCompiledBodyOperand {
            argument: 0,
            context: NativeCompiledBodyContext::Inherit,
            error_context: NativeCompiledBodyErrorContext::None,
            failure_scope: NativeCompilationFailureScope::FallbackToGeneric
        }])
    );
}

#[test]
fn compiler_rejection_keeps_native_usage_after_command_renaming() {
    let registry =
        crate::cache::registry_for_profile(tcl_dialect::DialectProfile::find("tcl8.4").unwrap());
    let arguments = ["x", "extra", "bad"];
    let facts = registry
        .resolve_invocation("set", &arguments, registry.own_surface_query())
        .unwrap()
        .facts();
    let spec = registry.get("set").unwrap().native_compilation.unwrap();
    let words = InvocationWords::literals("renamed", &arguments);
    let dialect = Some(InvocationDialect::for_version(TclVersion::V8_4));
    let selected = spec.select(
        words,
        &[NativeCompilationWordShape::Literal; 3],
        dialect,
        context(NativeCompilationFrame::ProcedureCode),
    );
    assert_eq!(selected, NativeCompilationSelection::CompileError);
    assert_eq!(
        spec.failure_for_selection(selected, words, &facts, dialect),
        Some(NativeCompilationFailure {
            message: Some("wrong # args: should be \"set varName ?newValue?\"".into()),
            error_code: Some("NONE".into()),
            error_info: None,
        })
    );
    assert_eq!(
        spec.compiled_bodies(
            NativeCompilationSelection::Unknown,
            words,
            &[],
            &facts,
            dialect
        ),
        NativeCompiledBodies::Unknown
    );
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
fn inline_exception_ranges_preserve_unknown_depth_and_checked_overflow() {
    let entry = context(NativeCompilationFrame::ProcedureCode);
    assert_eq!(entry.with_inline_exception_range().catch_depth, Some(1));
    let child = NativeCompiledBodyOperand {
        error_context: NativeCompiledBodyErrorContext::None,
        argument: 0,
        context: NativeCompiledBodyContext::Loop,
        failure_scope: NativeCompilationFailureScope::EnclosingChunk,
    };
    assert_eq!(child.entered_context(entry).unwrap().loop_depth, 1);
    assert_eq!(
        child.entered_context(NativeCompilationContext {
            loop_depth: u32::MAX,
            ..entry
        }),
        None
    );
    for depth in [None, Some(u32::MAX)] {
        let context = NativeCompilationContext {
            catch_depth: depth,
            ..entry
        };
        assert_eq!(context.with_inline_exception_range().catch_depth, None);
    }
}

#[test]
fn every_sampled_stock_registration_has_a_matching_availability_descriptor() {
    let registry = crate::CommandRegistry::build_default();
    let mut observed = 0;
    for row in include_str!("../native_compilation_hooks.tsv")
        .lines()
        .filter(|row| !row.starts_with('#'))
    {
        let cells: Vec<_> = row.split('\t').collect();
        assert_eq!(cells.len(), 6);
        for (version, registration) in TclVersion::ALL.into_iter().zip(&cells[1..]) {
            if *registration == "ABSENT" {
                continue;
            }
            let dialect = InvocationDialect::for_version(version);
            let spec = registry
                .get_for_surface(cells[0], dialect.authoring_query())
                .unwrap_or_else(|| panic!("missing stock command {}", cells[0]));
            let native = spec
                .native_compilation
                .unwrap_or_else(|| panic!("missing native contract {}", cells[0]));
            let hook = native
                .compiler_hook_presence(InvocationDialect::of_profile(
                    tcl_dialect::DialectProfile::find(version.dialect_profile_name()).unwrap(),
                ))
                .expect("pinned native C registration");
            assert_eq!(
                hook,
                *registration != "NULL",
                "{} {version:?}: {registration}",
                cells[0]
            );
            observed += 1;
        }
    }
    assert_eq!(observed, 421);
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
fn regexp_capture_and_option_forms_decline_native_compilation() {
    let registry = crate::CommandRegistry::build_default();
    let native = registry.get("regexp").unwrap().native_compilation.unwrap();
    for version in TclVersion::ALL {
        for arguments in [
            &[][..],
            &["pattern"][..],
            &["pattern", "text", "capture"][..],
            &["-nocase", "pattern", "text", "capture"][..],
            &["--", "pattern", "text", "capture"][..],
            &["-expanded", "pattern", "text"][..],
            &["-start", "0", "pattern", "text"][..],
        ] {
            assert_eq!(
                native.select(
                    InvocationWords::literals("regexp", arguments),
                    &vec![NativeCompilationWordShape::Literal; arguments.len()],
                    Some(InvocationDialect::for_version(version)),
                    context(NativeCompilationFrame::ProcedureCode),
                ),
                NativeCompilationSelection::Generic,
                "{version:?} {arguments:?}",
            );
        }
        for arguments in [
            &["pattern", "text"][..],
            &["-nocase", "pattern", "text"][..],
            &["-n", "--", "pattern", "text"][..],
        ] {
            assert_eq!(
                native.select(
                    InvocationWords::literals("regexp", arguments),
                    &vec![NativeCompilationWordShape::Literal; arguments.len()],
                    Some(InvocationDialect::for_version(version)),
                    context(NativeCompilationFrame::ProcedureCode),
                ),
                regexp_selection(native, version),
                "{version:?} {arguments:?}",
            );
        }
    }
}

// tclsh 8.6.18/9.0.4 disassembly of `proc p {s} {...}`: every form below
// except the last pushes "regsub" and invokes it; `regsub -all {x} $s y`
// compiles to push "x"; push "y"; loadScalar; strmap, which stays unmodelled.
#[test]
fn regsub_invokes_every_form_its_compiler_declines() {
    use NativeCompilationWordShape::{BracedLiteral, Literal, Substituted};
    let registry = crate::CommandRegistry::build_default();
    let native = registry.get("regsub").unwrap().native_compilation.unwrap();
    let select = |version, arguments: &[&str], shapes: &[NativeCompilationWordShape]| {
        native.select(
            InvocationWords::literals("regsub", arguments),
            shapes,
            Some(InvocationDialect::for_version(version)),
            context(NativeCompilationFrame::ProcedureCode),
        )
    };
    for version in TclVersion::ALL {
        for (arguments, shapes) in [
            (
                &["x", "$s", "y", "a"][..],
                &[BracedLiteral, Substituted, Literal, Literal][..],
            ),
            (
                &["-all", "x", "$s", "y", "a"][..],
                &[Literal, BracedLiteral, Substituted, Literal, Literal][..],
            ),
            (
                &["-all", "--", "x", "$s", "y", "a"][..],
                &[
                    Literal,
                    Literal,
                    BracedLiteral,
                    Substituted,
                    Literal,
                    Literal,
                ][..],
            ),
            (
                &["x", "$s", "y"][..],
                &[BracedLiteral, Substituted, Literal][..],
            ),
            (
                &["-nocase", "-all", "x", "$s", "y"][..],
                &[Literal, Literal, BracedLiteral, Substituted, Literal][..],
            ),
            (
                &["-all", "a.c", "$s", "y"][..],
                &[Literal, BracedLiteral, Substituted, Literal][..],
            ),
            (
                &["-all", "^x$", "$s", "y"][..],
                &[Literal, BracedLiteral, Substituted, Literal][..],
            ),
            (
                &["-all", ".*x", "$s", "y"][..],
                &[Literal, BracedLiteral, Substituted, Literal][..],
            ),
            (
                &["-all", "x", "$s", "&"][..],
                &[Literal, BracedLiteral, Substituted, Literal][..],
            ),
            (
                &["-all", "$p", "$s", "y"][..],
                &[Literal, Substituted, Substituted, Literal][..],
            ),
        ] {
            assert_eq!(
                select(version, arguments, shapes),
                NativeCompilationSelection::Generic,
                "{version:?} {arguments:?}"
            );
        }
        let accepted = select(
            version,
            &["-all", "x", "$s", "y"],
            &[Literal, BracedLiteral, Substituted, Literal],
        );
        let expected = if version < TclVersion::V8_6 {
            NativeCompilationSelection::Generic
        } else {
            NativeCompilationSelection::Unknown
        };
        assert_eq!(accepted, expected, "{version:?}");
    }
}

// tclsh 9.0.4/9.1.0 `regexp $re $s`: loadScalar1; loadScalar1; regexp +3.
// The pre-9 regexp compilers are unmodelled.
/// A plain literal pattern: C8.4 compiles it under its chunk-entry guard,
/// C8.5 is unmodelled, and C8.6+ compiles it before its arguments.
fn regexp_selection(
    native: NativeCompilationSpec,
    version: TclVersion,
) -> NativeCompilationSelection {
    NativeCompilationSelection::Inline {
        operation: native.operation,
        guard: NativeCompilationGuard::for_c_version(version),
    }
}

#[test]
fn regexp_operand_layout_is_independent_of_unresolved_operation_selection() {
    let registry = crate::CommandRegistry::build_default();
    let native = registry.get("regexp").unwrap().native_compilation.unwrap();
    let arguments = [
        crate::InvocationWord::Dynamic,
        crate::InvocationWord::Dynamic,
    ];
    let words = InvocationWords::structured(crate::InvocationWord::Literal("regexp"), &arguments);
    let shapes = [NativeCompilationWordShape::Substituted; 2];
    let compilation = context(NativeCompilationFrame::ProcedureCode);
    for version in TclVersion::ALL {
        let dialect = Some(InvocationDialect::for_version(version));
        // C8.4 compiles only a literal pattern; a substituted one declines.
        let expected = if version == TclVersion::V8_4 {
            NativeCompilationSelection::Generic
        } else {
            regexp_selection(native, version)
        };
        assert_eq!(
            native.select(words, &shapes, dialect, compilation),
            expected,
            "{version:?}"
        );
        assert_eq!(
            native.operand_layout(words, &shapes, dialect, compilation),
            (version >= TclVersion::V8_5).then_some(NativeCompilationOperandLayout::Pattern {
                argument_index: 0,
                guard: NativeCompilationGuard::BeforeArguments,
            })
        );
    }
    let dialect = Some(InvocationDialect::for_version(TclVersion::V9_1));
    for compilation in [
        NativeCompilationContext {
            mode: NativeCompilationMode::Direct,
            ..compilation
        },
        NativeCompilationContext {
            frame: NativeCompilationFrame::Unknown,
            ..compilation
        },
    ] {
        assert_eq!(
            native.operand_layout(words, &shapes, dialect, compilation),
            None
        );
    }
    assert_eq!(
        native.operand_layout(words, &shapes, None, compilation),
        None
    );
    assert_eq!(
        native.operand_layout(
            words,
            &[
                NativeCompilationWordShape::Expanded,
                NativeCompilationWordShape::Substituted
            ],
            dialect,
            compilation
        ),
        None
    );
}

#[test]
fn regexp_options_preserve_original_operand_layout() {
    let registry = crate::CommandRegistry::build_default();
    let native = registry.get("regexp").unwrap().native_compilation.unwrap();
    let dialect = Some(InvocationDialect::for_version(TclVersion::V9_1));
    let compilation = context(NativeCompilationFrame::ProcedureCode);
    let options = [
        crate::InvocationWord::Literal("-nocase"),
        crate::InvocationWord::Dynamic,
        crate::InvocationWord::Dynamic,
    ];
    assert_eq!(
        native.operand_layout(
            InvocationWords::structured(crate::InvocationWord::Literal("regexp"), &options),
            &[
                NativeCompilationWordShape::Literal,
                NativeCompilationWordShape::Substituted,
                NativeCompilationWordShape::Substituted
            ],
            dialect,
            compilation
        ),
        Some(NativeCompilationOperandLayout::Pattern {
            argument_index: 1,
            guard: NativeCompilationGuard::BeforeArguments
        })
    );
    let unknown_option = [crate::InvocationWord::Dynamic; 3];
    assert_eq!(
        native.operand_layout(
            InvocationWords::structured(crate::InvocationWord::Literal("regexp"), &unknown_option),
            &[
                NativeCompilationWordShape::Literal,
                NativeCompilationWordShape::Substituted,
                NativeCompilationWordShape::Substituted
            ],
            dialect,
            compilation,
        ),
        None,
    );
    let generic = [
        crate::InvocationWord::Literal("-about"),
        crate::InvocationWord::Dynamic,
    ];
    assert_eq!(
        native.operand_layout(
            InvocationWords::structured(crate::InvocationWord::Literal("regexp"), &generic),
            &[
                NativeCompilationWordShape::Literal,
                NativeCompilationWordShape::Substituted
            ],
            dialect,
            compilation
        ),
        Some(NativeCompilationOperandLayout::Pattern {
            argument_index: 0,
            guard: NativeCompilationGuard::BeforeArguments
        })
    );
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
fn expression_compilation_does_not_invent_a_legacy_math_function_table() {
    let operand = NativeCompiledExpressionOperand {
        argument: 0,
        error_context: NativeCompiledExpressionErrorContext::None,
    };
    for version in TclVersion::ALL {
        let profile = tcl_dialect::DialectProfile::find(version.dialect_profile_name()).unwrap();
        let context = tcl_syntax::expr::parser::ExprParseContext::for_profile(profile);
        assert_eq!(
            operand.validate("1 + 2", &context),
            NativeExpressionCompilation::Accepted
        );
        for expression in ["future_function(1)", "abs()", "int(1)"] {
            assert_eq!(
                operand.validate(expression, &context),
                if version == TclVersion::V8_4 {
                    NativeExpressionCompilation::Unknown
                } else {
                    NativeExpressionCompilation::Accepted
                }
            );
        }
        assert!(
            matches!(operand.validate("+", &context),
                NativeExpressionCompilation::Rejected(_) if version == TclVersion::V8_4)
                || operand.validate("+", &context) == NativeExpressionCompilation::Accepted
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

#[test]
fn selected_opcode_guard_precedes_substituted_arguments() {
    let spec = NativeCompilationSpec {
        grammar: NativeCompilationGrammar::VariableLoadStore,
        operation: SemanticOperationId::StructuredLowering(LoweringHookId::Set),
        body: NativeBodyCompilation::Inherit,
    };
    assert!(spec.requires_original_word_preparation());
    let arguments = [
        crate::InvocationWord::Literal("x"),
        crate::InvocationWord::Dynamic,
    ];
    let words = InvocationWords::structured(crate::InvocationWord::Literal("set"), &arguments);
    let shapes = [
        NativeCompilationWordShape::Literal,
        NativeCompilationWordShape::Substituted,
    ];
    for version in TclVersion::ALL {
        assert_eq!(
            spec.select(
                words,
                &shapes,
                Some(InvocationDialect::for_version(version)),
                context(NativeCompilationFrame::ScriptCode)
            ),
            NativeCompilationSelection::Inline {
                operation: spec.operation,
                guard: NativeCompilationGuard::for_c_version(version),
            }
        );
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

#[test]
fn selected_interpreter_controls_independent_body_boundaries() {
    let spec = NativeCompilationSpec {
        grammar: NativeCompilationGrammar::NoHook,
        operation: SemanticOperationId::Invoke,
        body: NativeBodyCompilation::Uplevel,
    };
    let enclosing = context(NativeCompilationFrame::ProcedureCode);
    assert_eq!(
        spec.body_context(
            Some(InvocationDialect::for_version(TclVersion::V8_4)),
            enclosing
        )
        .mode,
        NativeCompilationMode::Direct
    );
    assert_eq!(
        spec.body_context(
            Some(InvocationDialect::for_version(TclVersion::V8_5)),
            enclosing
        ),
        context(NativeCompilationFrame::ScriptCode)
    );
    let profile = crate::model::ingress::resolve_environment("jim").unit_profile();
    assert_eq!(
        spec.body_context(Some(InvocationDialect::of_profile(profile)), enclosing)
            .mode,
        NativeCompilationMode::Direct
    );
    assert_eq!(
        spec.body_context(None, enclosing),
        NativeCompilationContext::default()
    );
}
#[test]
fn fixed_math_compiler_visits_required_children_before_argument_count_errors() {
    let profile = crate::model::ingress::resolve_environment("tcl8.4").unit_profile();
    let context = tcl_syntax::expr::parser::ExprParseContext::for_profile(profile);
    let operand = NativeCompiledExpressionOperand {
        argument: 0,
        error_context: NativeCompiledExpressionErrorContext::None,
    };
    for (text, script_count, message) in [
        (
            "future_function([set x extra bad])",
            0,
            "unknown math function \"future_function\"",
        ),
        (
            "abs([set x extra bad],2)",
            1,
            "too many arguments for math function",
        ),
        (
            "pow([set x extra bad])",
            1,
            "too few arguments for math function",
        ),
        (
            "abs(1,[set x extra bad])",
            0,
            "too many arguments for math function",
        ),
        ("abs(unknown(1),2)", 0, "unknown math function \"unknown\""),
    ] {
        let steps = operand.compiler_steps(text, &context, |name| match name {
            "abs" => NativeMathFunctionResolution::Known { arity: 1 },
            "pow" => NativeMathFunctionResolution::Known { arity: 2 },
            _ => NativeMathFunctionResolution::Absent,
        });
        assert_eq!(
            steps
                .iter()
                .filter(|step| matches!(step, NativeExpressionCompilerStep::Script(_)))
                .count(),
            script_count,
            "{text}"
        );
        assert!(
            matches!(steps.last(), Some(NativeExpressionCompilerStep::Failure(failure)) if failure.message.as_deref()==Some(message)),
            "{text}: {steps:?}"
        );
    }
    assert_eq!(
        operand.compiler_steps("abs(1)", &context, |_| {
            NativeMathFunctionResolution::Unknown
        }),
        vec![NativeExpressionCompilerStep::Unknown]
    );
}
