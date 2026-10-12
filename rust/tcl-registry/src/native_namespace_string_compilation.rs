// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original namespace string compiler operands and counted string instructions.

use crate::native_compilation::NativeCompilationWordShape as Shape;
use crate::native_compiler_word_projection::{
    NativeCompilerWordOperand, project_native_compiler_words,
};
use crate::native_compiler_words::NativeCompilerWords;
use tcl_dialect::TclVersion;

/// Selected namespace compiler string operation, without registration authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NativeNamespaceStringOperation {
    /// Original last-separator search followed by the native character range.
    Tail,
    /// Original separator-run scan followed by the native prefix range.
    Qualifiers,
}

impl NativeNamespaceStringOperation {
    /// Actual selected private compiler registration, without lookup authority.
    #[must_use]
    pub const fn implementation(
        self,
    ) -> crate::native_compilation::NativeCompilerImplementationLookup {
        match self {
            Self::Tail => NAMESPACE_TAIL_IMPLEMENTATION,
            Self::Qualifiers => NAMESPACE_QUALIFIERS_IMPLEMENTATION,
        }
    }
}

/// One original operand followed by an independently selected compiler program.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NativeNamespaceStringInstruction {
    /// Selected native compiler program.
    pub operation: NativeNamespaceStringOperation,
    /// Original source operand or native parser literal expansion member.
    pub operand: NativeCompilerWordOperand,
}

/// Actual private Tail compiler registration, independent of mutable spelling.
pub const NAMESPACE_TAIL_IMPLEMENTATION:
    crate::native_compilation::NativeCompilerImplementationLookup =
    crate::native_compilation::NativeCompilerImplementationLookup {
        ensemble: "::namespace",
        member: "tail",
        slot: "::tcl::namespace::tail",
        command: "namespace",
        prepended: &["tail"],
    };

/// Actual private Qualifiers compiler registration, independent of spelling.
pub const NAMESPACE_QUALIFIERS_IMPLEMENTATION:
    crate::native_compilation::NativeCompilerImplementationLookup =
    crate::native_compilation::NativeCompilerImplementationLookup {
        ensemble: "::namespace",
        member: "qualifiers",
        slot: "::tcl::namespace::qualifiers",
        command: "namespace",
        prepended: &["qualifiers"],
    };

/// Project the original one-operand compiler program. The caller independently
/// authenticates its registered token, compiler hook and implementation path.
#[must_use]
pub fn compile_native_namespace_string(
    words: &NativeCompilerWords<'_>,
    operand_from: usize,
    operation: NativeNamespaceStringOperation,
    version: TclVersion,
) -> Option<NativeNamespaceStringInstruction> {
    if version < TclVersion::V8_6 || operand_from == 0 {
        return None;
    }
    let projected = project_native_compiler_words(words, version).ok()?;
    let [operand] = projected.get(operand_from..)? else {
        return None;
    };
    if matches!(operand.shape, Shape::Expanded | Shape::Opaque) {
        return None;
    }
    Some(NativeNamespaceStringInstruction {
        operation,
        operand: operand.operand.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native_compilation::{
        NativeCompilationContext, NativeCompilationFrame, NativeCompilationMode,
        NativeCompilationSelection,
    };
    use crate::native_instruction_plan::{
        NativeInstructionPlan, native_registered_worker_instruction_plan,
    };

    #[test]
    fn original_namespace_tail_recipe_retains_dynamic_and_counted_operands() {
        // naming.namespace.original-counted-tail-compiler-and-runtime
        // docs/design/analysis/name-resolution-proofs/original-counted-tail-compiler-and-runtime.md
        for (version, engine) in [
            (TclVersion::V8_4, "tcl8.4"),
            (TclVersion::V8_5, "tcl8.5"),
            (TclVersion::V8_6, "tcl8.6"),
            (TclVersion::V9_0, "tcl9.0"),
            (TclVersion::V9_1, "tcl9.1"),
        ] {
            let dialect = crate::InvocationDialect::for_version(version);
            let registry = crate::model::ingress::static_context_for_profile(
                crate::model::ingress::resolve_environment(engine).unit_profile(),
            )
            .commands();
            let spec = registry
                .get("namespace")
                .unwrap()
                .subcommands
                .iter()
                .find(|member| member.name == "tail")
                .unwrap()
                .native_compilation
                .unwrap();
            if version >= TclVersion::V8_6 {
                assert_eq!(
                    registry.native_compilation_for_registration("tcl::namespace::tail", dialect),
                    Some(spec)
                );
            }
            let context = NativeCompilationContext {
                mode: NativeCompilationMode::BytecodeObject,
                frame: NativeCompilationFrame::ProcedureCode,
                loop_depth: 0,
                catch_depth: Some(0),
            };
            for source in [
                b"namespace tail $name".as_slice(),
                b"namespace tail {a::\xed\xa0\x80}",
                b"namespace tail {a\0::tail}",
            ] {
                let parsed = tcl_lexer::native_script_words_in(
                    tcl_lexer::SourceImage::native(source),
                    tcl_lexer::Span::new(0, u32::try_from(source.len()).unwrap()),
                    tcl_lexer::LexerConfig::from_grammar(dialect.lexer_grammar),
                )
                .unwrap();
                let words = NativeCompilerWords::capture(
                    &parsed.commands[0].words,
                    dialect.native_string_protocol().unwrap(),
                )
                .unwrap();
                let selection =
                    spec.select_registered_worker_native_words(&words, 2, Some(dialect), context);
                if version < TclVersion::V8_6 {
                    assert_eq!(selection, NativeCompilationSelection::Generic);
                    continue;
                }
                assert!(matches!(
                    selection,
                    NativeCompilationSelection::Inline { .. }
                ));
                let NativeInstructionPlan::NamespaceString(recipe) =
                    native_registered_worker_instruction_plan(
                        spec, selection, &words, 2, dialect, context,
                    )
                    .unwrap()
                else {
                    panic!("original Tail program");
                };
                assert_eq!(recipe.operand, NativeCompilerWordOperand::Original(2));
                assert_eq!(recipe.operation, NativeNamespaceStringOperation::Tail);
                assert!(
                    compile_native_namespace_string(
                        &words,
                        1,
                        NativeNamespaceStringOperation::Tail,
                        version
                    )
                    .is_none()
                );
            }
        }
    }
    #[test]
    // Native proof: naming.namespace.original-counted-qualifiers-compiler-and-runtime
    // docs/design/analysis/name-resolution-proofs/original-counted-qualifiers-compiler-and-runtime.md
    fn original_namespace_qualifiers_recipe_retains_dynamic_and_counted_operands() {
        // naming.namespace.original-counted-qualifiers-compiler-and-runtime
        // docs/design/analysis/name-resolution-proofs/original-counted-qualifiers-compiler-and-runtime.md
        for (version, engine) in [
            (TclVersion::V8_4, "tcl8.4"),
            (TclVersion::V8_5, "tcl8.5"),
            (TclVersion::V8_6, "tcl8.6"),
            (TclVersion::V9_0, "tcl9.0"),
            (TclVersion::V9_1, "tcl9.1"),
        ] {
            let dialect = crate::InvocationDialect::for_version(version);
            let registry = crate::model::ingress::static_context_for_profile(
                crate::model::ingress::resolve_environment(engine).unit_profile(),
            )
            .commands();
            let spec = registry
                .get("namespace")
                .unwrap()
                .subcommands
                .iter()
                .find(|member| member.name == "qualifiers")
                .unwrap()
                .native_compilation
                .unwrap();
            if version >= TclVersion::V8_6 {
                assert_eq!(
                    registry
                        .native_compilation_for_registration("tcl::namespace::qualifiers", dialect),
                    Some(spec)
                );
            }
            let context = NativeCompilationContext {
                mode: NativeCompilationMode::BytecodeObject,
                frame: NativeCompilationFrame::ProcedureCode,
                loop_depth: 0,
                catch_depth: Some(0),
            };
            for source in [
                b"namespace qualifiers $name".as_slice(),
                b"namespace qualifiers {a::\xed\xa0\x80}",
                b"namespace qualifiers {a\0::qualifiers}",
            ] {
                let parsed = tcl_lexer::native_script_words_in(
                    tcl_lexer::SourceImage::native(source),
                    tcl_lexer::Span::new(0, u32::try_from(source.len()).unwrap()),
                    tcl_lexer::LexerConfig::from_grammar(dialect.lexer_grammar),
                )
                .unwrap();
                let words = NativeCompilerWords::capture(
                    &parsed.commands[0].words,
                    dialect.native_string_protocol().unwrap(),
                )
                .unwrap();
                let selection =
                    spec.select_registered_worker_native_words(&words, 2, Some(dialect), context);
                if version < TclVersion::V8_6 {
                    assert_eq!(selection, NativeCompilationSelection::Generic);
                    continue;
                }
                assert!(matches!(
                    selection,
                    NativeCompilationSelection::Inline { .. }
                ));
                let NativeInstructionPlan::NamespaceString(recipe) =
                    native_registered_worker_instruction_plan(
                        spec, selection, &words, 2, dialect, context,
                    )
                    .unwrap()
                else {
                    panic!("original Qualifiers program");
                };
                assert_eq!(recipe.operand, NativeCompilerWordOperand::Original(2));
                assert_eq!(recipe.operation, NativeNamespaceStringOperation::Qualifiers);
                assert!(
                    compile_native_namespace_string(
                        &words,
                        1,
                        NativeNamespaceStringOperation::Qualifiers,
                        version
                    )
                    .is_none()
                );
            }
        }
    }
    #[test]
    fn original_namespace_qualifiers_declines_arity_and_unresolved_expansion() {
        // naming.namespace.original-counted-qualifiers-compiler-and-runtime
        // docs/design/analysis/name-resolution-proofs/original-counted-qualifiers-compiler-and-runtime.md
        for version in [TclVersion::V8_6, TclVersion::V9_0, TclVersion::V9_1] {
            let dialect = crate::InvocationDialect::for_version(version);
            for source in [
                b"namespace qualifiers".as_slice(),
                b"namespace qualifiers a b",
                b"namespace qualifiers {*}$values",
            ] {
                let parsed = tcl_lexer::native_script_words_in(
                    tcl_lexer::SourceImage::native(source),
                    tcl_lexer::Span::new(0, u32::try_from(source.len()).unwrap()),
                    tcl_lexer::LexerConfig::from_grammar(dialect.lexer_grammar),
                )
                .unwrap();
                let words = NativeCompilerWords::capture(
                    &parsed.commands[0].words,
                    dialect.native_string_protocol().unwrap(),
                )
                .unwrap();
                assert!(
                    compile_native_namespace_string(
                        &words,
                        2,
                        NativeNamespaceStringOperation::Qualifiers,
                        version
                    )
                    .is_none()
                );
            }
        }
    }
}
