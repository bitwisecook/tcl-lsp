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

//! Name effects of original-argument preparation.

use super::*;

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
