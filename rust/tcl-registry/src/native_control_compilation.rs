// SPDX-License-Identifier: AGPL-3.0-or-later
//! Ordered original preparation for native structured-control instructions.

use crate::native_compilation::NativeCompiledBodyContext;
use crate::native_compiler_word_projection::NativeCompilerWordOperand;
use tcl_lexer::Span;

/// Original literal predicate observed by the native compiler before pruning.
/// Retaining this metadata allocates no native header or executable operand.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeControlBooleanProbe {
    /// Exact original compiler operand, including parser expansion provenance.
    pub operand: NativeCompilerWordOperand,
    /// Complete compile-known original value supplied to the fresh Boolean probe.
    pub literal: Vec<u8>,
    /// Actual compiler decision, independently checked against evaluation policy.
    pub value: bool,
}

impl NativeControlBooleanProbe {
    /// Match the original projected operand and value without generated source.
    #[must_use]
    pub fn matches_original(
        &self,
        words: &crate::native_compiler_words::NativeCompilerWords<'_>,
        version: tcl_dialect::TclVersion,
    ) -> bool {
        crate::native_compiler_word_projection::project_native_compiler_words(words, version)
            .is_ok_and(|words| {
                words.iter().any(|word| {
                    word.operand == self.operand
                        && word.literal.as_deref() == Some(self.literal.as_slice())
                })
            })
    }
}

/// A compiler visit, distinct from execution and native handler authority.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeControlPreparationStep {
    /// Retain an original predicate decision at this position. This produces
    /// no literal, local, child visit, getter side effect or runtime header.
    BooleanProbe(NativeControlBooleanProbe),
    /// Reserve a named native scalar local at this exact visit.
    DeclareLocal(Vec<u8>),
    /// Reserve one native anonymous compiler temporary.
    DeclareAnonymousLocal,
    /// Compile this original operand, retaining its substitutions and expansion.
    Word(NativeCompilerWordOperand),
    /// Register the selected compiler's literal at this visit.
    Literal(Vec<u8>),
    /// Register a private native Integer object without string reconstruction.
    Integer(i64),
    /// Register a private native List with its original ordered members.
    List(Vec<Vec<u8>>),
    /// Compile an original body in its selected lexical exception environment.
    Script {
        /// Original compiler operand containing this body.
        operand: NativeCompilerWordOperand,
        /// Body extent in the original source image.
        span: Span,
        /// Native exception/loop environment entered while compiling the body.
        context: NativeCompiledBodyContext,
    },
    /// C8.4 catch prepares a body speculatively. A child compiler rejection
    /// retains completed literal/local visits, rolls body instructions back,
    /// and selects the actual generic invocation instead of publishing it.
    SpeculativeScript {
        /// Original compiler operand containing this body.
        operand: NativeCompilerWordOperand,
        /// Body extent in the original source image.
        span: Span,
        /// Native exception environment entered for the speculative body.
        context: NativeCompiledBodyContext,
    },
    /// Compile the selected original expression after all preceding visits.
    Expression(NativeCompilerWordOperand),
}

/// Native inline selection or a genuine compiler decline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeControlOutcome<T> {
    /// Emit the retained instruction recipe after its ordered preparation.
    Inline(T),
    /// Roll back instructions while retaining completed literal/local visits.
    Generic,
    /// The native compiler publishes a guest compile-time failure.
    Rejected(crate::native_compilation::NativeCompilationFailure),
}

/// Control compiler selection and its exact retained preparation prefix.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeControlCompilation<T> {
    /// Selected native instruction recipe or genuine decline.
    pub outcome: NativeControlOutcome<T>,
    /// Original preparation order, including visits retained after decline.
    pub preparations: Vec<NativeControlPreparationStep>,
}

/// Original `LocalScalar` name geometry, without a runtime frame or cell.
pub use tcl_syntax::naming::NativeCompiledScalarName as NativeLocalScalarProjection;

/// Share native counted-name selection with all structured-control compilers.
#[must_use]
pub fn project_native_local_scalar(
    original: &[u8],
    version: tcl_dialect::TclVersion,
) -> NativeLocalScalarProjection<'_> {
    tcl_syntax::naming::NativeCompiledVariableRecipe::C(version)
        .scalar_name(original)
        .expect("selected C compiler recipe")
}

/// Reach an operand's body extent in the same original source image.
///
/// # Errors
/// Returns missing original geometry rather than reparsing generated text.
pub fn native_control_body_span(
    words: &crate::native_compiler_words::NativeCompilerWords<'_>,
    operand: &NativeCompilerWordOperand,
) -> Result<Span, crate::native_compiler_word_projection::NativeCompilerProjectionUnavailable> {
    use crate::native_compiler_word_projection::NativeCompilerProjectionUnavailable;
    match operand {
        NativeCompilerWordOperand::Original(index) => words
            .original_words()
            .get(*index)
            .ok_or(NativeCompilerProjectionUnavailable::SourceGeometry)?
            .content_span()
            .map_err(|_| NativeCompilerProjectionUnavailable::SourceGeometry),
        NativeCompilerWordOperand::LiteralExpansion { value_span, .. } => Ok(*value_span),
    }
}
