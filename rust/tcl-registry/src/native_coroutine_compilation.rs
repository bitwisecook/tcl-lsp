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

//! Original native coroutine operands, namespace capture and suspension stack.

use crate::native_compilation::{
    NativeCompilationContext, NativeCompilationFrame, NativeCompilationGrammar,
    NativeCompilationWordShape,
};
use crate::native_compiler_word_projection::{
    NativeCompilerWordOperand, project_native_compiler_words,
};
use crate::native_compiler_words::NativeCompilerWords;
use tcl_dialect::TclVersion;

/// Original compiler work; admission and runtime coroutine capability are separate.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum NativeCoroutineStep {
    /// Evaluate the unchanged operand.
    Word(NativeCompilerWordOperand),
    /// Register the compile-known original C9.1 tailcall target as a command literal.
    CommandWord {
        /// Unchanged original operand whose literal is registered.
        operand: NativeCompilerWordOperand,
        /// Exact compile-known original literal bytes.
        bytes: Vec<u8>,
    },
    /// Push the compiler's registered empty string.
    Empty,
    /// Produce the original namespace object before any subsequent operand.
    CurrentNamespace,
    /// Construct a fresh List from the last count original stack objects.
    List(usize),
    /// Concatenate the two original Lists on the stack.
    Concat,
    /// Schedule the complete original stack; legacy replaces its head at execution.
    Tailcall {
        /// Original namespace placeholder or namespace object plus target argv.
        count: usize,
        /// Replace the evaluated original head with the executing procedure namespace.
        legacy: bool,
    },
    /// Schedule or cancel from the same original namespace-prefixed List.
    TailcallList,
    /// Suspend with the same original value, resuming into its stack position.
    Yield,
    /// Suspend and invoke from the same original namespace-prefixed List.
    YieldTo,
    /// Push the current original coroutine command name without argument work.
    Name,
}

/// Actual original coroutine compiler recipe, without provider or header grants.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NativeCoroutineInstruction {
    /// Actual C compiler release.
    pub version: TclVersion,
    /// Ordered preparation and execution operations.
    pub steps: Vec<NativeCoroutineStep>,
}

/// Native decline and unavailable original source remain distinct.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeCoroutineCompilationUnavailable {
    /// The actual release, frame, arity or expansion declines this compiler.
    Generic,
    /// Original parser operands cannot be authenticated.
    Geometry,
}

/// Prepare an independently selected C coroutine compiler from original words.
///
/// # Errors
/// Returns native decline or missing original operand geometry.
pub fn compile_native_coroutine(
    words: &NativeCompilerWords<'_>,
    operand_from: usize,
    grammar: NativeCompilationGrammar,
    dialect: crate::InvocationDialect,
    context: NativeCompilationContext,
) -> Result<NativeCoroutineInstruction, NativeCoroutineCompilationUnavailable> {
    use NativeCoroutineCompilationUnavailable as Unavailable;
    use NativeCoroutineStep as Step;
    let version = dialect.tcl_version.ok_or(Unavailable::Generic)?;
    if dialect.family() != Some(tcl_dialect::model::Family::Tcl) || version < TclVersion::V8_6 {
        return Err(Unavailable::Generic);
    }
    if operand_from != 1 && grammar != NativeCompilationGrammar::InfoCoroutine {
        return Err(Unavailable::Geometry);
    }
    let projected =
        project_native_compiler_words(words, version).map_err(|_| Unavailable::Geometry)?;
    let operands = projected.get(operand_from..).ok_or(Unavailable::Geometry)?;
    let expanded = operands
        .iter()
        .any(|word| word.shape == NativeCompilationWordShape::Expanded);
    let mut steps = Vec::new();
    match grammar {
        NativeCompilationGrammar::InfoCoroutine => {
            if !operands.is_empty() {
                return Err(Unavailable::Generic);
            }
            steps.push(Step::Name);
        }
        NativeCompilationGrammar::CoroutineYield => {
            if expanded || operands.len() > 1 {
                return Err(Unavailable::Generic);
            }
            steps.push(
                operands
                    .first()
                    .map_or(Step::Empty, |word| Step::Word(word.operand.clone())),
            );
            steps.push(Step::Yield);
        }
        NativeCompilationGrammar::Tailcall => {
            if !matches!(context.frame, NativeCompilationFrame::ProcedureCode) {
                return Err(Unavailable::Generic);
            }
            if version < TclVersion::V9_1 {
                if expanded || operands.is_empty() || projected.len() >= 256 {
                    return Err(Unavailable::Generic);
                }
                steps.extend(
                    projected
                        .iter()
                        .map(|word| Step::Word(word.operand.clone())),
                );
                steps.push(Step::Tailcall {
                    count: projected.len(),
                    legacy: true,
                });
            } else {
                steps.push(Step::CurrentNamespace);
                if !expanded
                    && !operands.is_empty()
                    && projected.len()
                        <= crate::native_compilation::NativeTailcallStack::LIST_SEGMENT_LIMIT
                {
                    append_operands(&mut steps, operands, true);
                    steps.push(Step::Tailcall {
                        count: projected.len(),
                        legacy: false,
                    });
                } else {
                    append_list(&mut steps, operands, dialect, true);
                    steps.push(Step::TailcallList);
                }
            }
        }
        NativeCompilationGrammar::CoroutineRelay => {
            if version < TclVersion::V9_1 && (expanded || operands.is_empty()) {
                return Err(Unavailable::Generic);
            }
            steps.push(Step::CurrentNamespace);
            append_list(&mut steps, operands, dialect, false);
            steps.push(Step::YieldTo);
        }
        _ => return Err(Unavailable::Geometry),
    }
    Ok(NativeCoroutineInstruction { version, steps })
}

fn operand_step(
    word: &crate::native_compiler_word_projection::NativeProjectedCompilerWord,
    command: bool,
) -> NativeCoroutineStep {
    if command
        && matches!(
            word.shape,
            NativeCompilationWordShape::Literal
                | NativeCompilationWordShape::QuotedLiteral
                | NativeCompilationWordShape::BracedLiteral
        )
        && let Some(bytes) = &word.literal
    {
        NativeCoroutineStep::CommandWord {
            operand: word.operand.clone(),
            bytes: bytes.clone(),
        }
    } else {
        NativeCoroutineStep::Word(word.operand.clone())
    }
}

fn append_operands(
    steps: &mut Vec<NativeCoroutineStep>,
    operands: &[crate::native_compiler_word_projection::NativeProjectedCompilerWord],
    command: bool,
) {
    steps.extend(
        operands
            .iter()
            .enumerate()
            .map(|(index, word)| operand_step(word, command && index == 0)),
    );
}

fn append_list(
    steps: &mut Vec<NativeCoroutineStep>,
    operands: &[crate::native_compiler_word_projection::NativeProjectedCompilerWord],
    dialect: crate::InvocationDialect,
    command: bool,
) {
    use crate::native_instruction_plan::NativeArgumentListStep;
    for step in crate::native_instruction_plan::argument_list_steps_for_expansion_with_prefix(
        operands
            .iter()
            .enumerate()
            .map(|(index, word)| (index, word.shape == NativeCompilationWordShape::Expanded)),
        dialect,
        1,
    ) {
        steps.push(match step {
            NativeArgumentListStep::Word(index) => {
                operand_step(&operands[index], command && index == 0)
            }
            NativeArgumentListStep::List(count) => NativeCoroutineStep::List(count),
            NativeArgumentListStep::Concat => NativeCoroutineStep::Concat,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_lexer::{LexerConfig, SourceImage, Span};
    use tcl_syntax::native_string::NativeStringProtocol;

    fn recipe(
        source: &[u8],
        grammar: NativeCompilationGrammar,
        version: TclVersion,
        frame: NativeCompilationFrame,
    ) -> Result<NativeCoroutineInstruction, NativeCoroutineCompilationUnavailable> {
        let profile =
            crate::native_test_provider::profile(&format!("tcl{}", version.version_string()));
        let image = SourceImage::native(source);
        let script = tcl_lexer::native_script_words_in(
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
        compile_native_coroutine(
            &words,
            1,
            grammar,
            crate::InvocationDialect::for_version(version),
            NativeCompilationContext {
                mode: crate::native_compilation::NativeCompilationMode::BytecodeObject,
                frame,
                loop_depth: 0,
                catch_depth: Some(0),
            },
        )
    }

    #[test]
    fn original_coroutine_recipes_preserve_namespace_capture_and_native_decline() {
        use NativeCompilationFrame as Frame;
        use NativeCompilationGrammar as Grammar;
        use NativeCoroutineStep as Step;
        for version in TclVersion::ALL {
            let tail = recipe(
                b"tailcall target [set seen VALUE]",
                Grammar::Tailcall,
                version,
                Frame::ProcedureCode,
            );
            if version < TclVersion::V8_6 {
                assert_eq!(tail, Err(NativeCoroutineCompilationUnavailable::Generic));
                continue;
            }
            let tail = tail.unwrap();
            if version < TclVersion::V9_1 {
                assert_eq!(
                    tail.steps.first(),
                    Some(&Step::Word(NativeCompilerWordOperand::Original(0)))
                );
                assert!(!tail.steps.contains(&Step::CurrentNamespace));
                assert_eq!(
                    tail.steps.last(),
                    Some(&Step::Tailcall {
                        count: 3,
                        legacy: true
                    })
                );
            } else {
                assert_eq!(tail.steps.first(), Some(&Step::CurrentNamespace));
                assert!(
                    matches!(&tail.steps[1],Step::CommandWord {operand:NativeCompilerWordOperand::Original(1),bytes} if bytes==b"target")
                );
                assert_eq!(
                    tail.steps.last(),
                    Some(&Step::Tailcall {
                        count: 3,
                        legacy: false
                    })
                );
            }
            assert_eq!(
                recipe(
                    b"tailcall target",
                    Grammar::Tailcall,
                    version,
                    Frame::ScriptCode
                ),
                Err(NativeCoroutineCompilationUnavailable::Generic)
            );
            assert_eq!(
                recipe(
                    b"yield",
                    Grammar::CoroutineYield,
                    version,
                    Frame::ProcedureCode
                )
                .unwrap()
                .steps,
                [Step::Empty, Step::Yield]
            );
            assert_eq!(
                recipe(
                    b"yield a b",
                    Grammar::CoroutineYield,
                    version,
                    Frame::ProcedureCode
                ),
                Err(NativeCoroutineCompilationUnavailable::Generic)
            );
        }
    }

    #[test]
    fn original_coroutine_expansion_and_empty_relay_follow_release_recipes() {
        use NativeCompilationFrame as Frame;
        use NativeCompilationGrammar as Grammar;
        use NativeCoroutineStep as Step;
        for version in [TclVersion::V8_6, TclVersion::V9_0, TclVersion::V9_1] {
            let expanded = recipe(
                b"tailcall {*}$args",
                Grammar::Tailcall,
                version,
                Frame::ProcedureCode,
            );
            if version < TclVersion::V9_1 {
                assert_eq!(
                    expanded,
                    Err(NativeCoroutineCompilationUnavailable::Generic)
                );
            } else {
                assert_eq!(
                    expanded.unwrap().steps,
                    [
                        Step::CurrentNamespace,
                        Step::List(1),
                        Step::Word(NativeCompilerWordOperand::Original(1)),
                        Step::Concat,
                        Step::TailcallList
                    ]
                );
            }
            let empty_relay = recipe(
                b"yieldto",
                Grammar::CoroutineRelay,
                version,
                Frame::ProcedureCode,
            );
            if version < TclVersion::V9_1 {
                assert_eq!(
                    empty_relay,
                    Err(NativeCoroutineCompilationUnavailable::Generic)
                );
            } else {
                assert_eq!(
                    empty_relay.unwrap().steps,
                    [Step::CurrentNamespace, Step::List(1), Step::YieldTo]
                );
            }
        }
    }
}

/// Original coroutine resume arity, independently of a printable command name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NativeCoroutineResumeArity {
    /// The coroutine is parked at a single optional `yield` value.
    SingleOptional,
    /// The coroutine is parked at an arbitrary `yieldto` argument vector.
    Arbitrary,
}

/// C9.0–9.1 pending-injection chronology, without frame or invocation authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NativeCoroutineInjectionProtocol(());
impl NativeCoroutineInjectionProtocol {
    /// Select the measured native callback protocol independently of CPP.
    #[must_use]
    pub fn select(dialect: crate::InvocationDialect) -> Option<Self> {
        // naming.coroutine.original-injection-public-completions
        // docs/design/analysis/name-resolution-proofs/coroutine-original-injection-public-completions.md
        matches!(
            dialect.native_name_protocol(),
            Some(tcl_syntax::naming::NativeNameProtocol::C(
                TclVersion::V9_0 | TclVersion::V9_1
            ))
        )
        .then_some(Self(()))
    }

    /// The injected kind reflects the retained suspension's resume arity.
    #[must_use]
    pub const fn kind(self, arity: NativeCoroutineResumeArity) -> &'static [u8] {
        match arity {
            NativeCoroutineResumeArity::SingleOptional => b"yield",
            NativeCoroutineResumeArity::Arbitrary => b"yieldto",
        }
    }

    /// Run newest pending callbacks first, replacing every incoming completion.
    /// An earlier callback's error does not skip the remaining callbacks.
    pub fn run_pending<T, R>(
        self,
        pending: &mut Vec<T>,
        mut completion: R,
        mut invoke: impl FnMut(T, R) -> R,
    ) -> R {
        while let Some(callback) = pending.pop() {
            completion = invoke(callback, completion);
        }
        completion
    }
}

#[cfg(test)]
mod injection_tests {
    use super::*;

    #[test]
    fn original_injection_chronology_replaces_errors_and_retains_suspend_kind() {
        // Native proof: naming.coroutine.original-injection-order-and-completion
        // docs/design/analysis/name-resolution-proofs/coroutine-original-injection-order-and-completion.md
        let dialect =
            crate::InvocationDialect::of_profile(crate::native_test_provider::profile("tcl9.1"));
        let selected = NativeCoroutineInjectionProtocol::select(dialect).unwrap();
        let mut callbacks = vec![("A", 0), ("B", 1)];
        let outcome = selected.run_pending(
            &mut callbacks,
            (0, Vec::new()),
            |(name, code), (_, mut log)| {
                log.push(name);
                (code, log)
            },
        );
        assert_eq!(outcome, (0, vec!["B", "A"]));
        assert!(callbacks.is_empty());
        assert_eq!(
            selected.kind(NativeCoroutineResumeArity::SingleOptional),
            b"yield"
        );
        assert_eq!(
            selected.kind(NativeCoroutineResumeArity::Arbitrary),
            b"yieldto"
        );
        // naming.coroutine.original-injection-public-completions
        // docs/design/analysis/name-resolution-proofs/coroutine-original-injection-public-completions.md
        let c90 = NativeCoroutineInjectionProtocol::select(crate::InvocationDialect::of_profile(
            crate::native_test_provider::profile("tcl9.0"),
        ))
        .unwrap();
        let mut earlier_pending = vec!["A", "B"];
        assert_eq!(
            c90.run_pending(&mut earlier_pending, "ORIGINAL", |name, _| name),
            "A"
        );
        assert!(earlier_pending.is_empty());
        assert_eq!(
            c90.kind(NativeCoroutineResumeArity::SingleOptional),
            b"yield"
        );
        assert_eq!(c90.kind(NativeCoroutineResumeArity::Arbitrary), b"yieldto");
        for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl"] {
            assert!(
                NativeCoroutineInjectionProtocol::select(crate::InvocationDialect::of_profile(
                    crate::native_test_provider::profile(profile),
                ))
                .is_none()
            );
        }
        let jim = crate::InvocationDialect::of_profile(
            crate::model::resolve_environment("jim").unit_profile(),
        );
        assert_eq!(
            jim.native_name_protocol(),
            Some(tcl_syntax::naming::NativeNameProtocol::Jim084),
        );
        assert!(NativeCoroutineInjectionProtocol::select(jim).is_none());
    }
}

#[cfg(test)]
mod coroutine_name_tests {
    use super::*;
    use tcl_lexer::{LexerConfig, SourceImage, Span, native_script_words_in};

    #[test]
    fn original_coroutine_name_compiler_keeps_zero_operand_recipe_and_native_decline() {
        // Source proof: naming.info.original-coroutine-name-compiler
        // docs/design/analysis/name-resolution-proofs/info-original-coroutine-name-compiler.md
        // The original hook/zero-worker-operand/opcode source is independent
        // from Native250 public coroutine holder/resumption observations.
        // The source/word receipt does not select an entry protocol.
        // This pure recipe control independently supplies script compilation;
        // no physical procedure frame or reached coroutine is claimed.
        let compilation = NativeCompilationContext {
            mode: crate::native_compilation::NativeCompilationMode::BytecodeObject,
            frame: crate::native_compilation::NativeCompilationFrame::ScriptCode,
            ..NativeCompilationContext::default()
        };
        for version in TclVersion::ALL {
            let dialect = crate::InvocationDialect::of_point(
                tcl_dialect::model::DialectPoint::for_tcl_version(version),
            );
            let profile = crate::native_test_provider::profile(version.dialect_profile_name());
            for (source, operand_from, empty, expansion) in [
                (b"info coroutine".as_slice(), 2, true, false),
                (b"::tcl::info::coroutine".as_slice(), 1, true, false),
                (b"info coroutine extra".as_slice(), 2, false, false),
                (b"info coroutine {*}$extra".as_slice(), 2, false, true),
            ] {
                let parsed = native_script_words_in(
                    SourceImage::native(source),
                    Span::new(0, u32::try_from(source.len()).unwrap()),
                    LexerConfig::from_grammar(profile.grammar),
                )
                .unwrap();
                if version == TclVersion::V8_4 && expansion {
                    // Software parser control: this unchanged source has no
                    // complete command under the selected C84 word grammar.
                    // A fatal tail cannot supply original compiler words.
                    assert!(parsed.commands.is_empty());
                    assert_eq!(
                        parsed.fatal_tail.unwrap().cut.message,
                        tcl_lexer::word_parts::EXTRA_AFTER_CLOSE_BRACE,
                    );
                    continue;
                }
                assert!(parsed.fatal_tail.is_none());
                assert_eq!(parsed.commands.len(), 1);
                let words = NativeCompilerWords::capture(
                    &parsed.commands[0].words,
                    dialect.native_source_string_protocol().unwrap(),
                )
                .unwrap();
                let spec = crate::native_compilation::NativeCompilationSpec {
                    grammar: NativeCompilationGrammar::InfoCoroutine,
                    operation: crate::SemanticOperationId::Invoke,
                    body: crate::native_compilation::NativeBodyCompilation::Inherit,
                };
                let selected = spec.select_registered_worker_native_words(
                    &words,
                    operand_from,
                    Some(dialect),
                    compilation,
                );
                assert_eq!(
                    selected,
                    if version >= TclVersion::V8_6 && empty {
                        crate::native_compilation::NativeCompilationSelection::Inline {
                            operation: spec.operation,
                            guard:
                                crate::native_compilation::NativeCompilationGuard::BeforeArguments,
                        }
                    } else {
                        crate::native_compilation::NativeCompilationSelection::Generic
                    }
                );
                if version >= TclVersion::V8_6 {
                    assert_eq!(
                        spec.select_registered_worker_native_words(
                            &words,
                            operand_from,
                            Some(dialect),
                            NativeCompilationContext::default(),
                        ),
                        crate::native_compilation::NativeCompilationSelection::Unknown,
                        "original words and a Native point cannot supply the missing entry mode"
                    );
                }
                let result = compile_native_coroutine(
                    &words,
                    operand_from,
                    NativeCompilationGrammar::InfoCoroutine,
                    dialect,
                    compilation,
                );
                if version >= TclVersion::V8_6 && empty {
                    assert_eq!(result.unwrap().steps, [NativeCoroutineStep::Name]);
                } else {
                    assert_eq!(result, Err(NativeCoroutineCompilationUnavailable::Generic));
                }
            }
        }
    }
}
