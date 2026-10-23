// SPDX-License-Identifier: AGPL-3.0-or-later
//! Native Return instruction selection from original compiler words.

use crate::{
    native_compilation::{
        NativeCompilationContext, NativeCompilationFrame, NativeCompilationWordShape,
    },
    native_compiler_words::NativeCompilerWords,
};
use tcl_cmd_core::{
    CmdError,
    native_return_merge::{self, NativeReturnMergeObjects},
    return_options::{CompletionCodeCache, ReturnOptionsOps, ReturnOptionsProtocol},
};
use tcl_dialect::TclVersion;
use tcl_runtime_api::native_return_literal::NativeReturnOptionsLiteral;
use tcl_syntax::{
    native_string::NativeStringProtocol,
    scalar_getter::{NativeScalarGetterKind, NativeScalarGetterProtocol, NativeScalarGetterValue},
};

/// Original ordered options operand and native instruction branch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeReturnOptionsOperand {
    /// Already-merged private Dictionary, manufactured once for the unit.
    Static(NativeReturnOptionsLiteral),
    /// Exact special `return -options opts result`: opts word remains original.
    StackWord(usize),
    /// C86+ evaluates original option words then constructs one List.
    StackPairs(std::ops::Range<usize>),
}
/// Selected Return exit, separate from result construction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeReturnExit {
    /// Finish the current unit with the result at the native DONE instruction.
    Done,
    /// Continue normally after a level-zero successful return.
    Fallthrough,
    /// Apply statically merged controls through `RETURN_IMM`.
    Immediate,
    /// Merge the original stack options through `RETURN_STK`.
    Stack,
}
/// Portable Return recipe; this does not grant compiler or handler authority.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeReturnInstruction {
    /// Original result word index, or None for the native empty result.
    pub value_word: Option<usize>,
    /// Original or privately merged options operand in native evaluation order.
    pub options: NativeReturnOptionsOperand,
    /// Selected completion instruction after result and options preparation.
    pub exit: NativeReturnExit,
}

impl NativeReturnInstruction {
    /// Original source words visited by the selected compiler: stack options
    /// precede the result. Private merged options and the default empty result
    /// create literals without compiling a source substitution.
    pub fn compiler_word_visits(&self) -> impl Iterator<Item = usize> + '_ {
        let (word, pairs) = match &self.options {
            NativeReturnOptionsOperand::Static(_) => (None, 0..0),
            NativeReturnOptionsOperand::StackWord(word) => (Some(*word), 0..0),
            NativeReturnOptionsOperand::StackPairs(words) => (None, words.clone()),
        };
        word.into_iter().chain(pairs).chain(self.value_word)
    }
}
/// Genuine compiler decline is distinct from unavailable evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeReturnCompilationUnavailable {
    /// The native return compiler declines and retains generic invocation.
    Generic,
    /// Original word indices or source extents are inconsistent.
    Geometry,
    /// Required native option-merging evidence is unavailable.
    Capability,
}

struct Symbolic {
    version: TclVersion,
}
impl ReturnOptionsOps for Symbolic {
    type Value = Vec<u8>;
    fn bytes(&mut self, value: &Self::Value) -> Result<Vec<u8>, CmdError> {
        Ok(value.clone())
    }
    fn list(&mut self, value: &Self::Value) -> Result<Vec<Self::Value>, CmdError> {
        tcl_syntax::list::split_native_list_bytes(value, NativeStringProtocol::C(self.version))
            .map(|items| {
                items
                    .into_iter()
                    .map(std::borrow::Cow::into_owned)
                    .collect()
            })
            .map_err(|_| CmdError::new("invalid native list"))
    }
    fn new_string(&mut self, bytes: &[u8]) -> Self::Value {
        bytes.to_vec()
    }
    fn integer_probe(&mut self, value: &Self::Value, wide: bool) -> Result<Option<i64>, CmdError> {
        Ok(NativeScalarGetterProtocol::for_tcl_version(self.version)
            .fresh_conversion(
                if wide {
                    NativeScalarGetterKind::Wide
                } else {
                    NativeScalarGetterKind::Int
                },
                value,
            )
            .and_then(|conversion| match conversion.outcome() {
                Ok(NativeScalarGetterValue::Wide(value)) => Some(value),
                _ => None,
            }))
    }
    fn completion_code_cache(&self, _: &Self::Value) -> Option<i32> {
        None
    }
    fn adopt_completion_code_cache(
        &mut self,
        _: &Self::Value,
        _: CompletionCodeCache,
    ) -> Result<(), CmdError> {
        Ok(())
    }
}
impl NativeReturnMergeObjects for Symbolic {
    type Dictionary = Vec<(Vec<u8>, Vec<u8>)>;
    fn fresh_dictionary(&mut self) -> Result<Self::Dictionary, CmdError> {
        Ok(Vec::new())
    }
    fn put(
        &mut self,
        root: &mut Self::Dictionary,
        key: &Self::Value,
        value: &Self::Value,
    ) -> Result<(), CmdError> {
        if let Some(pair) = root.iter_mut().find(|pair| pair.0 == *key) {
            pair.1.clone_from(value);
        } else {
            root.push((key.clone(), value.clone()));
        }
        Ok(())
    }
    fn get(
        &mut self,
        root: &Self::Dictionary,
        key: &[u8],
    ) -> Result<Option<Self::Value>, CmdError> {
        Ok(root
            .iter()
            .find(|pair| pair.0 == key)
            .map(|pair| pair.1.clone()))
    }
    fn remove(&mut self, root: &mut Self::Dictionary, key: &[u8]) -> Result<(), CmdError> {
        root.retain(|pair| pair.0 != key);
        Ok(())
    }
    fn dictionary_pairs(
        &mut self,
        value: &Self::Value,
    ) -> Result<Vec<(Self::Value, Self::Value)>, CmdError> {
        let items = self.list(value)?;
        if !items.len().is_multiple_of(2) {
            return Err(CmdError::new("invalid dictionary"));
        }
        let mut root = Vec::new();
        for pair in items.as_chunks::<2>().0 {
            self.put(&mut root, &pair[0], &pair[1])?;
        }
        Ok(root)
    }
    fn size(&mut self, root: &Self::Dictionary) -> Result<usize, CmdError> {
        Ok(root.len())
    }
    fn finish(&mut self, _: Self::Dictionary) -> Self::Value {
        Vec::new()
    }
}

/// Validate static original option values through the same selected merger.
/// This projects controls only and never issues physical object authority.
pub(crate) fn static_controls(
    arguments: &[Vec<u8>],
    version: TclVersion,
) -> Result<(i32, i32, usize), CmdError> {
    let mut symbolic = Symbolic { version };
    let merged = native_return_merge::merge(
        &mut symbolic,
        if version == TclVersion::V8_5 {
            ReturnOptionsProtocol::Tcl85
        } else {
            ReturnOptionsProtocol::Tcl86Plus
        },
        arguments,
    )?;
    Ok((merged.code, merged.level, merged.size))
}

/// Validate a fresh compiler-only completion-code object through the same
/// original Int/index selection as reached return options. This projects its
/// value without issuing a retained runtime object or cache receipt.
pub(crate) fn static_completion_code(value: &[u8], version: TclVersion) -> Result<i32, CmdError> {
    tcl_cmd_core::return_options::parse_completion_code(
        &mut Symbolic { version },
        ReturnOptionsProtocol::Tcl86Plus,
        &value.to_vec(),
    )
}

fn merged_control_values(
    version: TclVersion,
    arguments: &[Vec<u8>],
) -> Result<(i32, i32, usize), NativeReturnCompilationUnavailable> {
    use NativeReturnCompilationUnavailable as Unavailable;
    let (code, level, size) = if version == TclVersion::V8_4 {
        (0, 1, 0)
    } else {
        let mut symbolic = Symbolic { version };
        let merged = native_return_merge::merge(
            &mut symbolic,
            if version == TclVersion::V8_5 {
                ReturnOptionsProtocol::Tcl85
            } else {
                ReturnOptionsProtocol::Tcl86Plus
            },
            arguments,
        )
        .map_err(|error| {
            if error.native_access_refusal().is_some() {
                Unavailable::Capability
            } else {
                Unavailable::Generic
            }
        })?;
        (merged.code, merged.level, merged.size)
    };
    Ok((code, level, size))
}

/// Project the native compiler's option merge or ordered stack branch.
/// # Errors
/// Reports an authentic generic decline separately from missing geometry.
pub fn native_return_instruction(
    words: &NativeCompilerWords<'_>,
    operand_from: usize,
    version: TclVersion,
    context: NativeCompilationContext,
) -> Result<NativeReturnInstruction, NativeReturnCompilationUnavailable> {
    use NativeReturnCompilationUnavailable as Unavailable;
    let count = words
        .original_words()
        .len()
        .checked_sub(operand_from)
        .ok_or(Unavailable::Geometry)?;
    if operand_from == 0 {
        return Err(Unavailable::Geometry);
    }
    let pair_count = count & !1;
    let value_word = (count % 2 == 1).then_some(operand_from + pair_count);
    if version == TclVersion::V8_4
        && pair_count == 0
        && context.frame == NativeCompilationFrame::ProcedureCode
        && context.catch_depth.is_none()
    {
        return Err(Unavailable::Capability);
    }
    if version == TclVersion::V8_4
        && (pair_count != 0
            || context.frame != NativeCompilationFrame::ProcedureCode
            || context.catch_depth != Some(0))
    {
        return Err(Unavailable::Generic);
    }
    if version != TclVersion::V8_4
        && count == 3
        && words.literal(operand_from) == Some(b"-options")
        && matches!(
            words.shapes()[operand_from],
            NativeCompilationWordShape::Literal
                | NativeCompilationWordShape::QuotedLiteral
                | NativeCompilationWordShape::BracedLiteral
        )
    {
        return Ok(NativeReturnInstruction {
            value_word,
            options: NativeReturnOptionsOperand::StackWord(operand_from + 1),
            exit: NativeReturnExit::Stack,
        });
    }
    let Some(arguments) = (operand_from..operand_from + pair_count)
        .map(|index| words.literal(index).map(<[u8]>::to_vec))
        .collect::<Option<Vec<_>>>()
    else {
        return if version >= TclVersion::V8_6 {
            Ok(NativeReturnInstruction {
                value_word,
                options: NativeReturnOptionsOperand::StackPairs(
                    operand_from..operand_from + pair_count,
                ),
                exit: NativeReturnExit::Stack,
            })
        } else {
            Err(Unavailable::Generic)
        };
    };
    let (code, level, size) = merged_control_values(version, &arguments)?;
    let known = (operand_from..operand_from + pair_count)
        .map(|index| words.known_word_literal(index))
        .collect::<Option<Vec<_>>>()
        .ok_or(Unavailable::Capability)?;
    let exit = if pair_count == 0
        && context.frame == NativeCompilationFrame::ProcedureCode
        && context.catch_depth == Some(0)
    {
        NativeReturnExit::Done
    } else if code == 0 && level == 0 && size == 0 {
        NativeReturnExit::Fallthrough
    } else {
        NativeReturnExit::Immediate
    };
    Ok(NativeReturnInstruction {
        value_word,
        options: NativeReturnOptionsOperand::Static(NativeReturnOptionsLiteral {
            protocol: NativeStringProtocol::C(version),
            words: known,
            code,
            level,
            size,
        }),
        exit,
    })
}
