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

//! `TclCompileStringMapCmd` (C8.6, C9.0 and C9.1).
//!
//! A compile-known two-element map emits its key and replacement literals,
//! the subject word and `STR_MAP`; an empty key pushes only the subject. A
//! dynamic, malformed or other-length map selects `TclCompileBasic2ArgCmd`.
//! Any other word count fails the member compiler: the public ensemble then
//! invokes the implementation through `INVOKE_REPLACE`, while a direct call
//! of the private worker compiles as an ordinary invocation.

use crate::native_compilation::{
    NativeCompilationSelection, NativeCompilationWordShape as Shape,
    NativeCompilerImplementationLookup, NativeNamedInvocationProtocol,
};
use crate::native_compiler_word_projection::NativeCompilerWordOperand;
use crate::native_compiler_words::NativeCompilerWords;
use crate::native_scalar_compilation::NativeScalarScope;
use tcl_dialect::TclVersion;

pub(crate) const STRING_MAP_IMPLEMENTATION: NativeCompilerImplementationLookup =
    NativeCompilerImplementationLookup {
        ensemble: "::string",
        member: "map",
        slot: "::tcl::string::map",
        command: "string",
        prepended: &["map"],
    };

/// The inline outcome of a compile-known two-element map.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeStringMapInstruction {
    /// An empty key: only the original subject word is pushed.
    Identity(NativeCompilerWordOperand),
    /// Push the key and replacement literals and the subject, then `STR_MAP`.
    Pair {
        /// The map's first element.
        key: Vec<u8>,
        /// The map's second element.
        replacement: Vec<u8>,
        /// The original subject word.
        subject: NativeCompilerWordOperand,
    },
}

fn pair(map: &[u8], version: TclVersion) -> Option<[Vec<u8>; 2]> {
    let elements = tcl_syntax::list::split_native_list_bytes(
        map,
        tcl_syntax::native_string::NativeStringProtocol::C(version),
    )
    .ok()?;
    let [key, replacement] = elements.as_slice() else {
        return None;
    };
    Some([key.to_vec(), replacement.to_vec()])
}

pub(crate) fn select(
    shapes: &[Shape],
    member: Option<&[u8]>,
    map: Option<&[u8]>,
    scope: NativeScalarScope,
    version: TclVersion,
) -> NativeCompilationSelection {
    use NativeCompilationSelection as Selection;
    if !matches!(
        version,
        TclVersion::V8_6 | TclVersion::V9_0 | TclVersion::V9_1
    ) {
        return Selection::Unknown;
    }
    let public = scope == NativeScalarScope::PublicMember;
    let from = usize::from(public);
    if public
        && (!shapes.first().is_some_and(|shape| {
            matches!(
                shape,
                Shape::Literal | Shape::QuotedLiteral | Shape::BracedLiteral
            )
        }) || member != Some(b"map"))
    {
        return Selection::Unknown;
    }
    let Some(operands) = shapes.get(from..) else {
        return Selection::Unknown;
    };
    if operands
        .iter()
        .any(|shape| matches!(shape, Shape::Opaque | Shape::Expanded))
    {
        return Selection::Unknown;
    }
    if operands.len() != 2 {
        return if public {
            Selection::NamedInvocation {
                lookup: &STRING_MAP_IMPLEMENTATION,
                arguments_from: from,
                protocol: NativeNamedInvocationProtocol::EnsembleRewrite,
            }
        } else {
            Selection::Generic
        };
    }
    let basic = Selection::NamedInvocation {
        lookup: &STRING_MAP_IMPLEMENTATION,
        arguments_from: from,
        protocol: NativeNamedInvocationProtocol::Direct,
    };
    let Some(map) = map else {
        return if operands[0] == Shape::Substituted {
            basic
        } else {
            Selection::Unknown
        };
    };
    if pair(map, version).is_none() {
        return basic;
    }
    Selection::Inline {
        operation: crate::SemanticOperationId::Invoke,
        guard: crate::native_compilation::NativeCompilationGuard::BeforeArguments,
    }
}

pub(crate) fn select_original(
    words: &NativeCompilerWords<'_>,
    operand_from: usize,
    scope: NativeScalarScope,
    version: TclVersion,
) -> NativeCompilationSelection {
    if words.source_protocol() != tcl_syntax::native_string::NativeStringProtocol::C(version) {
        return NativeCompilationSelection::Unknown;
    }
    let Some(shapes) = operand_from
        .checked_sub(1)
        .and_then(|_| words.shapes().get(operand_from..))
    else {
        return NativeCompilationSelection::Unknown;
    };
    let from = usize::from(scope == NativeScalarScope::PublicMember);
    select(
        shapes,
        words.literal(operand_from),
        words.literal(operand_from + from),
        scope,
        version,
    )
}

/// The inline instruction for a selected compile-known two-element map.
pub(crate) fn instruction(
    words: &NativeCompilerWords<'_>,
    operand_from: usize,
    scope: NativeScalarScope,
    version: TclVersion,
) -> Option<NativeStringMapInstruction> {
    if !matches!(
        select_original(words, operand_from, scope, version),
        NativeCompilationSelection::Inline { .. }
    ) {
        return None;
    }
    let map_index =
        operand_from.checked_add(usize::from(scope == NativeScalarScope::PublicMember))?;
    let subject_index = map_index.checked_add(1)?;
    words.original_words().get(subject_index)?;
    let subject = NativeCompilerWordOperand::Original(subject_index);
    let [key, replacement] = pair(words.literal(map_index)?, version)?;
    Some(if key.is_empty() {
        NativeStringMapInstruction::Identity(subject)
    } else {
        NativeStringMapInstruction::Pair {
            key,
            replacement,
            subject,
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native_compilation::{
        NativeCompilationContext, NativeCompilationSpec, NativeOriginalCompilerNameEffects,
    };

    fn selected(
        source: &str,
        version: TclVersion,
        scope: NativeScalarScope,
    ) -> NativeCompilationSelection {
        let dialect = crate::InvocationDialect::for_version(version);
        let parsed = tcl_lexer::native_script_words_in(
            tcl_lexer::SourceImage::native(source.as_bytes()),
            tcl_lexer::Span::new(0, u32::try_from(source.len()).unwrap()),
            tcl_lexer::LexerConfig::from_grammar(dialect.lexer_grammar),
        )
        .unwrap();
        let words = NativeCompilerWords::capture(
            &parsed.commands[0].words,
            dialect.native_string_protocol().unwrap(),
        )
        .unwrap();
        select_original(&words, 1, scope, version)
    }

    #[test]
    fn non_pair_mapping_keeps_the_actual_private_worker_and_original_values() {
        for source in [
            "string map {a b c d} $s",
            "string map {} $s",
            "string map {a} [side_effect]",
            "string map {a b c} $s",
        ] {
            assert!(
                matches!(selected(source, TclVersion::V9_0, NativeScalarScope::PublicMember), NativeCompilationSelection::NamedInvocation { lookup, arguments_from: 1, protocol: NativeNamedInvocationProtocol::Direct } if *lookup == STRING_MAP_IMPLEMENTATION)
            );
        }
        assert!(matches!(
            selected(
                "::tcl::string::map {a b c d} $s",
                TclVersion::V9_0,
                NativeScalarScope::PrivateOperands
            ),
            NativeCompilationSelection::NamedInvocation {
                arguments_from: 0,
                protocol: NativeNamedInvocationProtocol::Direct,
                ..
            }
        ));
    }

    #[test]
    fn dynamic_map_preserves_fixed_lookup_and_ordered_original_visits() {
        use crate::native_compiler_word_projection::NativeCompilerWordOperand as Word;
        use crate::native_control_compilation::NativeControlPreparationStep as Step;
        let source = "string map [side mapping] [side string]";
        let dialect = crate::InvocationDialect::for_version(TclVersion::V9_0);
        let parsed = tcl_lexer::native_script_words_in(
            tcl_lexer::SourceImage::native(source.as_bytes()),
            tcl_lexer::Span::new(0, u32::try_from(source.len()).unwrap()),
            tcl_lexer::LexerConfig::from_grammar(dialect.lexer_grammar),
        )
        .unwrap();
        let words = NativeCompilerWords::capture(
            &parsed.commands[0].words,
            dialect.native_string_protocol().unwrap(),
        )
        .unwrap();
        let spec = NativeCompilationSpec {
            grammar: crate::native_compilation::NativeCompilationGrammar::StringMap(
                NativeScalarScope::PublicMember,
            ),
            operation: crate::SemanticOperationId::Invoke,
            body: crate::native_compilation::NativeBodyCompilation::Inherit,
        };
        let NativeOriginalCompilerNameEffects::FixedLookupThenVisits { lookup, visits } = spec
            .original_argument_name_effects_in_context(
                &words,
                1,
                Some(dialect),
                NativeCompilationContext::default(),
            )
        else {
            panic!("dynamic map must select Basic2Arg effects")
        };
        assert_eq!(*lookup, STRING_MAP_IMPLEMENTATION);
        assert_eq!(
            visits,
            vec![Step::Word(Word::Original(2)), Step::Word(Word::Original(3))]
        );
    }

    fn words(source: &str, version: TclVersion) -> Vec<tcl_lexer::NativeWord> {
        let dialect = crate::InvocationDialect::for_version(version);
        tcl_lexer::native_script_words_in(
            tcl_lexer::SourceImage::native(source.as_bytes()),
            tcl_lexer::Span::new(0, u32::try_from(source.len()).unwrap()),
            tcl_lexer::LexerConfig::from_grammar(dialect.lexer_grammar),
        )
        .unwrap()
        .commands
        .remove(0)
        .words
    }

    fn recipe(
        source: &str,
        version: TclVersion,
        scope: NativeScalarScope,
    ) -> Option<NativeStringMapInstruction> {
        let dialect = crate::InvocationDialect::for_version(version);
        let words = words(source, version);
        let words = NativeCompilerWords::capture(&words, dialect.native_string_protocol().unwrap())
            .unwrap();
        instruction(&words, 1, scope, version)
    }

    const RELEASES: [TclVersion; 3] = [TclVersion::V8_6, TclVersion::V9_0, TclVersion::V9_1];

    #[test]
    fn dynamic_map_selects_the_private_basic_two_argument_fallback() {
        for version in RELEASES {
            for source in [
                "string map $mapping $s",
                "string map [side mapping] [side string]",
                "string map \"{\" $s",
            ] {
                assert!(
                    matches!(selected(source, version, NativeScalarScope::PublicMember),
                    NativeCompilationSelection::NamedInvocation { lookup, arguments_from: 1, protocol: NativeNamedInvocationProtocol::Direct } if *lookup == STRING_MAP_IMPLEMENTATION),
                    "{source}"
                );
            }
            assert!(
                matches!(selected("::tcl::string::map $mapping $s", version, NativeScalarScope::PrivateOperands),
                NativeCompilationSelection::NamedInvocation { lookup, arguments_from: 0, protocol: NativeNamedInvocationProtocol::Direct } if *lookup == STRING_MAP_IMPLEMENTATION)
            );
        }
        for version in [TclVersion::V8_4, TclVersion::V8_5] {
            assert_eq!(
                selected(
                    "string map $mapping $s",
                    version,
                    NativeScalarScope::PublicMember
                ),
                NativeCompilationSelection::Unknown
            );
        }
    }

    #[test]
    fn two_element_maps_push_their_literals_or_only_the_subject() {
        use NativeCompilerWordOperand::Original;
        for version in RELEASES {
            assert_eq!(
                recipe(
                    "string map {a b} $s",
                    version,
                    NativeScalarScope::PublicMember
                ),
                Some(NativeStringMapInstruction::Pair {
                    key: b"a".to_vec(),
                    replacement: b"b".to_vec(),
                    subject: Original(3),
                })
            );
            assert_eq!(
                recipe(
                    "::tcl::string::map {{a b} {}} $s",
                    version,
                    NativeScalarScope::PrivateOperands
                ),
                Some(NativeStringMapInstruction::Pair {
                    key: b"a b".to_vec(),
                    replacement: Vec::new(),
                    subject: Original(2),
                })
            );
            for source in [
                "string map {{} x} $s",
                "string map {{} !} [side]",
                "string map {{} {}} $s",
            ] {
                assert_eq!(
                    recipe(source, version, NativeScalarScope::PublicMember),
                    Some(NativeStringMapInstruction::Identity(Original(3))),
                    "{source}"
                );
            }
        }
    }

    #[test]
    fn other_word_counts_fail_the_member_compiler() {
        for version in RELEASES {
            for source in [
                "string map -nocase {a b c d} $s",
                "string map {a b c d}",
                "string map {a b} $s extra",
            ] {
                assert!(
                    matches!(
                        selected(source, version, NativeScalarScope::PublicMember),
                        NativeCompilationSelection::NamedInvocation {
                            arguments_from: 1,
                            protocol: NativeNamedInvocationProtocol::EnsembleRewrite,
                            ..
                        }
                    ),
                    "{source}"
                );
            }
            assert_eq!(
                selected(
                    "::tcl::string::map {a b} $s extra",
                    version,
                    NativeScalarScope::PrivateOperands
                ),
                NativeCompilationSelection::Generic
            );
        }
    }

    #[test]
    fn expansion_and_unselected_members_stay_unknown() {
        for source in [
            "string map {*}$mapping $s",
            "string map {a b} {*}$s",
            "string m {a b c d} $s",
        ] {
            assert_eq!(
                selected(source, TclVersion::V9_0, NativeScalarScope::PublicMember),
                NativeCompilationSelection::Unknown,
                "{source}"
            );
        }
    }

    #[test]
    fn inline_map_visits_only_the_subject() {
        use crate::native_compiler_word_projection::NativeCompilerWordOperand as Word;
        use crate::native_control_compilation::NativeControlPreparationStep as Step;
        let dialect = crate::InvocationDialect::for_version(TclVersion::V9_0);
        let words = words("string map {a b} [side]", TclVersion::V9_0);
        let words = NativeCompilerWords::capture(&words, dialect.native_string_protocol().unwrap())
            .unwrap();
        let spec = NativeCompilationSpec {
            grammar: crate::native_compilation::NativeCompilationGrammar::StringMap(
                NativeScalarScope::PublicMember,
            ),
            operation: crate::SemanticOperationId::Invoke,
            body: crate::native_compilation::NativeBodyCompilation::Inherit,
        };
        assert_eq!(
            spec.original_argument_name_effects_in_context(
                &words,
                1,
                Some(dialect),
                NativeCompilationContext::default(),
            ),
            NativeOriginalCompilerNameEffects::Visits(vec![Step::Word(Word::Original(3))])
        );
    }

    #[test]
    fn named_lookup_precedes_the_existing_original_word_effect_visits() {
        let source = "string map {a b c d} [rename x y]";
        let dialect = crate::InvocationDialect::for_version(TclVersion::V9_0);
        let parsed = tcl_lexer::native_script_words_in(
            tcl_lexer::SourceImage::native(source.as_bytes()),
            tcl_lexer::Span::new(0, u32::try_from(source.len()).unwrap()),
            tcl_lexer::LexerConfig::from_grammar(dialect.lexer_grammar),
        )
        .unwrap();
        let words = NativeCompilerWords::capture(
            &parsed.commands[0].words,
            dialect.native_string_protocol().unwrap(),
        )
        .unwrap();
        let spec = NativeCompilationSpec {
            grammar: crate::native_compilation::NativeCompilationGrammar::StringMap(
                NativeScalarScope::PublicMember,
            ),
            operation: crate::SemanticOperationId::Invoke,
            body: crate::native_compilation::NativeBodyCompilation::Inherit,
        };
        let NativeOriginalCompilerNameEffects::FixedLookupThenVisits { lookup, visits } = spec
            .original_argument_name_effects_in_context(
                &words,
                1,
                Some(dialect),
                NativeCompilationContext::default(),
            )
        else {
            panic!("selected Basic2Arg must retain both independent effects");
        };
        assert_eq!(*lookup, STRING_MAP_IMPLEMENTATION);
        for body in [
            crate::native_compilation::NativeBodyCompilation::ScriptObject,
            crate::native_compilation::NativeBodyCompilation::ProcedureObject,
            crate::native_compilation::NativeBodyCompilation::Direct,
        ] {
            let foreign = NativeCompilationSpec { body, ..spec };
            assert_eq!(
                foreign.original_argument_name_effects(&words, 1, Some(dialect)),
                NativeOriginalCompilerNameEffects::Unknown
            );
        }
        let foreign = NativeCompilationSpec {
            operation: crate::SemanticOperationId::Intrinsic(crate::IntrinsicId::StringIs),
            ..spec
        };
        assert_eq!(
            foreign.original_argument_name_effects(&words, 1, Some(dialect)),
            NativeOriginalCompilerNameEffects::Unknown
        );
        assert_eq!(
            visits,
            vec![
                crate::native_control_compilation::NativeControlPreparationStep::Word(
                    crate::native_compiler_word_projection::NativeCompilerWordOperand::Original(2)
                ),
                crate::native_control_compilation::NativeControlPreparationStep::Word(
                    crate::native_compiler_word_projection::NativeCompilerWordOperand::Original(3)
                )
            ]
        );
        assert_eq!(
            select_original(&words, 0, NativeScalarScope::PublicMember, TclVersion::V9_0),
            NativeCompilationSelection::Unknown
        );
        assert_eq!(
            select_original(
                &words,
                usize::MAX,
                NativeScalarScope::PublicMember,
                TclVersion::V9_0
            ),
            NativeCompilationSelection::Unknown
        );
    }
}
