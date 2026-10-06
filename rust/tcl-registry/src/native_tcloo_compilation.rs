// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original native `TclOO` helper compiler geometry, independent of method entry.

use crate::native_compilation::{
    NativeCompilationGuard, NativeCompilationSelection, NativeCompilationWordShape,
};
use crate::native_compiler_words::NativeCompilerWords;
use crate::native_instruction_plan::NativeArgumentListStep;
use crate::{InvocationDialect, SemanticOperationId};
use tcl_dialect::TclVersion;

/// Original C `TclOO` `self` helper table used by its native Index getter.
pub const SELF_SUBCOMMANDS: &[&str] = &[
    "call",
    "caller",
    "class",
    "filter",
    "method",
    "namespace",
    "next",
    "object",
    "target",
];

/// Actual C `TclOO` helper compiler registration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NativeTclOoHelper {
    /// Invoke the next implementation with the original complete word vector.
    Next,
    /// Invoke the selected later class with the original complete word vector.
    NextTo,
    /// Read the current method object's retained original command-name header.
    SelfObject,
    /// Look up an original object and return its class name.
    ObjectClass,
    /// Look up an original object and return its private namespace.
    ObjectNamespace,
    /// Test the original object operand after a compile-known category.
    ObjectIsObject,
    /// Read the original object creation epoch (C9.1).
    ObjectCreationId,
}

/// Selected native object introspection operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NativeTclOoObjectInfo {
    /// Return the class object's retained cached name header.
    Class,
    /// Produce the object's actual private namespace result.
    Namespace,
    /// Test whether the original command operand denotes an object.
    IsObject,
    /// Produce a wide integer from the actual creation epoch.
    CreationId,
}

impl NativeTclOoObjectInfo {
    /// Standalone object predicates return the actual interpreter's execution constant.
    /// Conditional-jump peepholes consume the truth directly instead of producing a header.
    #[must_use]
    pub const fn uses_execution_constant(self, version: TclVersion) -> bool {
        matches!(self, Self::IsObject)
            && matches!(
                version,
                TclVersion::V8_6 | TclVersion::V9_0 | TclVersion::V9_1
            )
    }
}

/// Portable instructions of the selected original helper compiler.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeTclOoInstruction {
    /// Require the actual current method frame and return its original name.
    SelfObject,
    /// Require that same frame first, discard its name, then read current namespace.
    SelfNamespace,
    /// Compile exactly the original object operand and apply its native getter.
    ObjectInfo {
        /// Selected actual object getter or predicate.
        operation: NativeTclOoObjectInfo,
        /// Original word or parser-expanded source member.
        operand: crate::native_compiler_word_projection::NativeCompilerWordOperand,
    },
    /// Compile all original words, including the invocation head, in source order.
    Next {
        /// Whether the selected instruction includes an explicit class operand.
        class: bool,
        /// Original complete-vector operands consumed by this instruction.
        words: Vec<crate::native_compiler_word_projection::NativeCompilerWordOperand>,
        /// C9.1 List/Concat steps index `words`; absence uses its immediate count.
        list: Option<Vec<NativeArgumentListStep>>,
    },
}

/// Select only original source geometries accepted by the actual helper hook.
/// This metadata establishes neither registration nor an active method frame.
#[must_use]
pub fn select(
    helper: NativeTclOoHelper,
    shapes: &[NativeCompilationWordShape],
    literal: Option<&[u8]>,
    version: TclVersion,
    operation: SemanticOperationId,
) -> NativeCompilationSelection {
    use NativeCompilationSelection as Selection;
    if version < TclVersion::V8_6 {
        return Selection::Generic;
    }
    if shapes.contains(&NativeCompilationWordShape::Opaque) {
        return Selection::Unknown;
    }
    let expanded = shapes.contains(&NativeCompilationWordShape::Expanded);
    let valid = match helper {
        NativeTclOoHelper::ObjectClass | NativeTclOoHelper::ObjectNamespace => {
            !expanded && shapes.len() == 1
        }
        NativeTclOoHelper::ObjectCreationId => {
            version >= TclVersion::V9_1 && !expanded && shapes.len() == 1
        }
        NativeTclOoHelper::ObjectIsObject => {
            !expanded
                && shapes.len() == 2
                && matches!(
                    shapes[0],
                    NativeCompilationWordShape::Literal
                        | NativeCompilationWordShape::QuotedLiteral
                        | NativeCompilationWordShape::BracedLiteral
                )
                && literal.is_some_and(|value| {
                    value.len() >= if version >= TclVersion::V9_1 { 2 } else { 1 }
                        && b"object".starts_with(value)
                })
        }
        NativeTclOoHelper::Next | NativeTclOoHelper::NextTo => {
            (!matches!(helper, NativeTclOoHelper::NextTo) || !shapes.is_empty())
                && (version >= TclVersion::V9_1 || (!expanded && shapes.len() < 255))
        }
        NativeTclOoHelper::SelfObject => {
            !expanded
                && (shapes.is_empty()
                    || (shapes.len() == 1
                        && matches!(
                            shapes[0],
                            NativeCompilationWordShape::Literal
                                | NativeCompilationWordShape::QuotedLiteral
                                | NativeCompilationWordShape::BracedLiteral
                        )
                        && literal.is_some_and(|value| {
                            !value.is_empty()
                                && (b"object".starts_with(value) || b"namespace".starts_with(value))
                        })))
        }
    };
    if valid {
        Selection::Inline {
            operation,
            guard: NativeCompilationGuard::BeforeArguments,
        }
    } else {
        Selection::Generic
    }
}

fn object_info_operation(helper: NativeTclOoHelper) -> Option<NativeTclOoObjectInfo> {
    match helper {
        NativeTclOoHelper::ObjectClass => Some(NativeTclOoObjectInfo::Class),
        NativeTclOoHelper::ObjectNamespace => Some(NativeTclOoObjectInfo::Namespace),
        NativeTclOoHelper::ObjectIsObject => Some(NativeTclOoObjectInfo::IsObject),
        NativeTclOoHelper::ObjectCreationId => Some(NativeTclOoObjectInfo::CreationId),
        NativeTclOoHelper::Next | NativeTclOoHelper::NextTo | NativeTclOoHelper::SelfObject => None,
    }
}

/// Retain the instruction recipe after independently authenticated selection.
/// Operands retain original word indices or parser-expanded source members;
/// List steps index this retained compiler vector, never evaluated argv.
#[must_use]
pub fn instruction(
    helper: NativeTclOoHelper,
    words: &NativeCompilerWords<'_>,
    operand_from: usize,
    dialect: InvocationDialect,
) -> Option<NativeTclOoInstruction> {
    if operand_from != 1 && object_info_operation(helper).is_none() {
        return None;
    }
    let version = dialect.tcl_version?;
    let projected =
        crate::native_compiler_word_projection::project_native_compiler_words(words, version)
            .ok()?;
    let shapes: Vec<_> = projected
        .iter()
        .skip(operand_from)
        .map(|word| word.shape)
        .collect();
    let literal = projected
        .get(operand_from)
        .and_then(|word| word.literal.as_deref());
    if !matches!(
        select(
            helper,
            &shapes,
            literal,
            version,
            SemanticOperationId::Invoke
        ),
        NativeCompilationSelection::Inline { .. }
    ) {
        return None;
    }
    if let Some(operation) = object_info_operation(helper) {
        let index = operand_from + usize::from(operation == NativeTclOoObjectInfo::IsObject);
        return Some(NativeTclOoInstruction::ObjectInfo {
            operation,
            operand: projected.get(index)?.operand.clone(),
        });
    }
    Some(match helper {
        NativeTclOoHelper::ObjectClass
        | NativeTclOoHelper::ObjectNamespace
        | NativeTclOoHelper::ObjectIsObject
        | NativeTclOoHelper::ObjectCreationId => return None,
        NativeTclOoHelper::SelfObject => {
            if literal.is_some_and(|value| b"namespace".starts_with(value)) {
                NativeTclOoInstruction::SelfNamespace
            } else {
                NativeTclOoInstruction::SelfObject
            }
        }
        NativeTclOoHelper::Next | NativeTclOoHelper::NextTo => {
            let list = (version >= TclVersion::V9_1
                && (shapes.contains(&NativeCompilationWordShape::Expanded)
                    || u32::try_from(projected.len()).is_err()))
            .then(|| {
                crate::native_instruction_plan::argument_list_steps_for_expansion(
                    projected.iter().enumerate().map(|(index, word)| {
                        (index, word.shape == NativeCompilationWordShape::Expanded)
                    }),
                    dialect,
                )
            });
            NativeTclOoInstruction::Next {
                class: helper == NativeTclOoHelper::NextTo,
                words: projected.into_iter().map(|word| word.operand).collect(),
                list,
            }
        }
    })
}

/// Select native parser expansion before the helper compiler observes words.
/// Missing original expansion geometry retains uncertainty.
#[must_use]
pub fn select_original(
    helper: NativeTclOoHelper,
    words: &NativeCompilerWords<'_>,
    operand_from: usize,
    version: TclVersion,
    operation: SemanticOperationId,
) -> NativeCompilationSelection {
    if operand_from != 1 && object_info_operation(helper).is_none() {
        return NativeCompilationSelection::Unknown;
    }
    let Ok(projected) =
        crate::native_compiler_word_projection::project_native_compiler_words(words, version)
    else {
        return NativeCompilationSelection::Unknown;
    };
    let shapes: Vec<_> = projected
        .iter()
        .skip(operand_from)
        .map(|word| word.shape)
        .collect();
    select(
        helper,
        &shapes,
        projected
            .get(operand_from)
            .and_then(|word| word.literal.as_deref()),
        version,
        operation,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn original_nested_object_info_projection_matches_native_specialized_words() {
        use crate::native_compiler_word_projection::NativeCompilerWordOperand as Operand;
        use tcl_lexer::{LexerConfig, SourceImage, Span, native_script_words_in};
        use tcl_syntax::native_string::NativeStringProtocol;
        for version in [TclVersion::V8_6, TclVersion::V9_0, TclVersion::V9_1] {
            let profile =
                tcl_dialect::DialectProfile::find(version.dialect_profile_name()).unwrap();
            for (source, helper, from, index, inline) in [
                (
                    b"info object class $x".as_slice(),
                    NativeTclOoHelper::ObjectClass,
                    3,
                    3,
                    true,
                ),
                (
                    b"info object namespace $x",
                    NativeTclOoHelper::ObjectNamespace,
                    3,
                    3,
                    true,
                ),
                (
                    b"info object creationid $x",
                    NativeTclOoHelper::ObjectCreationId,
                    3,
                    3,
                    version == TclVersion::V9_1,
                ),
                (
                    b"info object isa object $x",
                    NativeTclOoHelper::ObjectIsObject,
                    3,
                    4,
                    true,
                ),
                (
                    b"info object isa o $x",
                    NativeTclOoHelper::ObjectIsObject,
                    3,
                    4,
                    version != TclVersion::V9_1,
                ),
                (
                    b"info object isa ob $x",
                    NativeTclOoHelper::ObjectIsObject,
                    3,
                    4,
                    true,
                ),
                (
                    b"info object isa \\x6f $x",
                    NativeTclOoHelper::ObjectIsObject,
                    3,
                    4,
                    false,
                ),
                (
                    b"info object isa $category $x",
                    NativeTclOoHelper::ObjectIsObject,
                    3,
                    4,
                    false,
                ),
                (
                    b"info object class $x extra",
                    NativeTclOoHelper::ObjectClass,
                    3,
                    3,
                    false,
                ),
            ] {
                let mut parsed = native_script_words_in(
                    SourceImage::native(source),
                    Span::new(0, u32::try_from(source.len()).unwrap()),
                    LexerConfig::from_grammar(profile.grammar),
                )
                .unwrap();
                let words = parsed.commands.remove(0).words;
                let captured =
                    NativeCompilerWords::capture(&words, NativeStringProtocol::C(version)).unwrap();
                let selected = select_original(
                    helper,
                    &captured,
                    from,
                    version,
                    SemanticOperationId::Invoke,
                );
                assert_eq!(
                    matches!(selected, NativeCompilationSelection::Inline { .. }),
                    inline,
                    "{version:?} {source:?}"
                );
                let plan = instruction(
                    helper,
                    &captured,
                    from,
                    InvocationDialect::for_version(version),
                );
                assert_eq!(plan.is_some(), inline, "{version:?} {source:?}");
                if let Some(NativeTclOoInstruction::ObjectInfo { operand, .. }) = plan {
                    assert_eq!(operand, Operand::Original(index), "{version:?} {source:?}");
                }
            }
        }
        for version in [TclVersion::V8_4, TclVersion::V8_5] {
            assert_eq!(
                select(
                    NativeTclOoHelper::ObjectClass,
                    &[NativeCompilationWordShape::Substituted],
                    None,
                    version,
                    SemanticOperationId::Invoke
                ),
                NativeCompilationSelection::Generic
            );
        }
    }

    #[test]
    fn original_helper_projection_matches_native_direct_and_expanded_instructions() {
        use tcl_lexer::{LexerConfig, SourceImage, Span, native_script_words_in};
        use tcl_syntax::native_string::NativeStringProtocol;
        for version in [TclVersion::V8_6, TclVersion::V9_0, TclVersion::V9_1] {
            let profile =
                tcl_dialect::DialectProfile::find(version.dialect_profile_name()).unwrap();
            for (source, helper, inline, list) in [
                (
                    b"self".as_slice(),
                    NativeTclOoHelper::SelfObject,
                    true,
                    false,
                ),
                (b"self n", NativeTclOoHelper::SelfObject, true, false),
                (b"self {}", NativeTclOoHelper::SelfObject, false, false),
                (b"self \\x6f", NativeTclOoHelper::SelfObject, false, false),
                (b"self $which", NativeTclOoHelper::SelfObject, false, false),
                (b"next", NativeTclOoHelper::Next, true, false),
                (b"nextto", NativeTclOoHelper::NextTo, false, false),
                (b"nextto B", NativeTclOoHelper::NextTo, true, false),
                (
                    b"next {*}$args",
                    NativeTclOoHelper::Next,
                    version == TclVersion::V9_1,
                    version == TclVersion::V9_1,
                ),
                (
                    b"nextto B {*}$args",
                    NativeTclOoHelper::NextTo,
                    version == TclVersion::V9_1,
                    version == TclVersion::V9_1,
                ),
                (b"next {*}{a b}", NativeTclOoHelper::Next, true, false),
                (b"nextto {*}{B a}", NativeTclOoHelper::NextTo, true, false),
            ] {
                let image = SourceImage::native(source);
                let original = native_script_words_in(
                    image,
                    Span::new(0, u32::try_from(source.len()).unwrap()),
                    LexerConfig::from_grammar(profile.grammar),
                )
                .unwrap()
                .commands
                .remove(0)
                .words;
                let captured =
                    NativeCompilerWords::capture(&original, NativeStringProtocol::C(version))
                        .unwrap();
                let selected =
                    select_original(helper, &captured, 1, version, SemanticOperationId::Invoke);
                assert_eq!(
                    matches!(selected, NativeCompilationSelection::Inline { .. }),
                    inline,
                    "{version:?} {source:?}"
                );
                let recipe = instruction(
                    helper,
                    &captured,
                    1,
                    InvocationDialect::for_version(version),
                );
                assert_eq!(recipe.is_some(), inline, "{version:?} {source:?}");
                if let Some(NativeTclOoInstruction::Next {
                    words, list: steps, ..
                }) = recipe
                {
                    assert_eq!(steps.is_some(), list, "{version:?} {source:?}");
                    if source == b"next {*}{a b}" || source == b"nextto {*}{B a}" {
                        assert_eq!(words.len(), 3);
                        assert!(matches!(words[1],crate::native_compiler_word_projection::NativeCompilerWordOperand::LiteralExpansion{..}));
                    }
                }
            }
        }
    }

    #[test]
    fn helper_compilation_retains_native_simple_words_and_release_bounds() {
        use NativeCompilationWordShape as Shape;
        for version in [TclVersion::V8_6, TclVersion::V9_0, TclVersion::V9_1] {
            let selected = |helper, shapes: &[Shape], literal| {
                select(
                    helper,
                    shapes,
                    literal,
                    version,
                    SemanticOperationId::Invoke,
                )
            };
            assert!(matches!(
                selected(NativeTclOoHelper::SelfObject, &[], None),
                NativeCompilationSelection::Inline { .. }
            ));
            for value in [b"o".as_slice(), b"object", b"n", b"namespace"] {
                assert!(matches!(
                    selected(
                        NativeTclOoHelper::SelfObject,
                        &[Shape::BracedLiteral],
                        Some(value)
                    ),
                    NativeCompilationSelection::Inline { .. }
                ));
                assert_eq!(
                    selected(
                        NativeTclOoHelper::SelfObject,
                        &[Shape::BackslashLiteral],
                        Some(value)
                    ),
                    NativeCompilationSelection::Generic
                );
            }
            for value in [b"".as_slice(), b"objectX", b"method"] {
                assert_eq!(
                    selected(
                        NativeTclOoHelper::SelfObject,
                        &[Shape::Literal],
                        Some(value)
                    ),
                    NativeCompilationSelection::Generic
                );
            }
            assert_eq!(
                selected(NativeTclOoHelper::NextTo, &[], None),
                NativeCompilationSelection::Generic
            );
            assert_eq!(
                matches!(
                    selected(NativeTclOoHelper::Next, &[Shape::Expanded], None),
                    NativeCompilationSelection::Inline { .. }
                ),
                version == TclVersion::V9_1
            );
            assert_eq!(
                matches!(
                    selected(
                        NativeTclOoHelper::Next,
                        &vec![Shape::Substituted; 255],
                        None
                    ),
                    NativeCompilationSelection::Inline { .. }
                ),
                version == TclVersion::V9_1
            );
        }
        assert_eq!(
            select(
                NativeTclOoHelper::SelfObject,
                &[],
                None,
                TclVersion::V8_5,
                SemanticOperationId::Invoke
            ),
            NativeCompilationSelection::Generic
        );
    }
}
