// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original foreach/lmap compiler declarations, iterator operands and ranges.

use crate::native_compilation::{
    NativeCompilationContext, NativeCompilationFailure, NativeCompilationFrame,
    NativeCompilationMode, NativeCompilationWordShape, NativeCompiledBodyContext,
};
use crate::native_compiler_word_projection::{
    NativeCompilerProjectionUnavailable, NativeCompilerWordOperand, NativeProjectedCompilerWord,
    project_native_compiler_words,
};
use crate::native_compiler_words::NativeCompilerWords;
use crate::native_control_compilation::{
    NativeControlCompilation, NativeControlOutcome, NativeControlPreparationStep,
    native_control_body_span, project_native_local_scalar,
};
use tcl_dialect::TclVersion;
use tcl_lexer::Span;

/// Result policy of an original native iterator instruction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeEachCollection {
    /// Discard body results and publish the registered empty string.
    Foreach,
    /// Append each normally completed body result to the retained accumulator.
    Lmap,
}

/// Actual native compiled iterator ownership and callback refresh boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeCompiledEachStorage {
    /// C8.4 reads temporary locals and refreshes members before each setter.
    LocalRefetch,
    /// C8.5 reads temporary locals and retains a List copy for each group.
    LocalGroupCopy,
    /// C8.6+ copies shared input headers once and retains stack iterator lists.
    StackLists,
}

/// Select the original foreach opcodes' storage owner, independently from
/// generic foreach handlers and original object authentication.
#[must_use]
pub const fn native_compiled_each_storage(version: TclVersion) -> NativeCompiledEachStorage {
    match version {
        TclVersion::V8_4 => NativeCompiledEachStorage::LocalRefetch,
        TclVersion::V8_5 => NativeCompiledEachStorage::LocalGroupCopy,
        TclVersion::V8_6 | TclVersion::V9_0 | TclVersion::V9_1 => {
            NativeCompiledEachStorage::StackLists
        }
    }
}

/// One auxiliary iterator group, retaining its evaluated value-list operand.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeEachGroup {
    /// Counted scalar names in the native auxiliary table's order.
    pub variables: Vec<Vec<u8>>,
    /// Original operand evaluated before entering any loop exception range.
    pub values: NativeCompilerWordOperand,
}

/// Original compiled iterator, independent of handler registration authority.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeEachInstruction {
    /// Compiler release selecting temporary and iterator storage.
    pub version: TclVersion,
    /// Body-result collection policy.
    pub collection: NativeEachCollection,
    /// Original auxiliary groups, widths and value operands.
    pub groups: Vec<NativeEachGroup>,
    /// Original body operand, never generated source.
    pub body: NativeCompilerWordOperand,
    /// Exact original body extent protected by the loop exception range.
    pub body_span: Span,
    /// C8.4/8.5 reserve value-list and loop-counter local temporaries.
    pub local_temporaries: bool,
}

/// Missing original compiler evidence, distinct from a genuine decline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeEachUnavailable {
    /// An operand index does not identify the original vector.
    Geometry,
    /// The physical compiler environment is not established.
    Context,
    /// Original native parser expansion or body extent is unavailable.
    Projection(NativeCompilerProjectionUnavailable),
}

fn simple(shape: NativeCompilationWordShape) -> bool {
    matches!(
        shape,
        NativeCompilationWordShape::Literal
            | NativeCompilationWordShape::QuotedLiteral
            | NativeCompilationWordShape::BracedLiteral
    )
}

/// Select an original native iterator and retain every completed declaration
/// visit even when a later variable list declines inline compilation.
///
/// # Errors
/// Returns absent source/frame evidence without guessing executable operands.
pub fn compile_native_each(
    words: &NativeCompilerWords<'_>,
    operand_from: usize,
    version: TclVersion,
    context: NativeCompilationContext,
    collection: NativeEachCollection,
) -> Result<NativeControlCompilation<NativeEachInstruction>, NativeEachUnavailable> {
    use NativeControlOutcome::{Generic, Rejected};
    let mut selected = NativeControlCompilation {
        outcome: Generic,
        preparations: Vec::new(),
    };
    if operand_from == 0 || operand_from > words.original_words().len() {
        return Err(NativeEachUnavailable::Geometry);
    }
    if context.mode == NativeCompilationMode::Direct
        || context.frame == NativeCompilationFrame::ScriptCode
        || (collection == NativeEachCollection::Lmap && version < TclVersion::V8_6)
    {
        return Ok(selected);
    }
    if context.mode != NativeCompilationMode::BytecodeObject
        || context.frame != NativeCompilationFrame::ProcedureCode
    {
        return Err(NativeEachUnavailable::Context);
    }
    let projected =
        project_native_compiler_words(words, version).map_err(NativeEachUnavailable::Projection)?;
    if projected
        .iter()
        .any(|word| word.shape == NativeCompilationWordShape::Expanded)
    {
        return Ok(selected);
    }
    let operands = projected
        .iter()
        .filter(|word| match word.operand {
            NativeCompilerWordOperand::Original(index) => index >= operand_from,
            NativeCompilerWordOperand::LiteralExpansion { original_word, .. } => {
                original_word >= operand_from
            }
        })
        .collect::<Vec<_>>();
    if operands.len() < 3 || operands.len() % 2 != 1 {
        if version == TclVersion::V8_4 {
            selected.outcome = Rejected(NativeCompilationFailure {
                message: Some(
                    "wrong # args: should be \"foreach varList list ?varList list ...? command\""
                        .to_owned(),
                ),
                error_code: None,
                error_info: None,
            });
        }
        return Ok(selected);
    }
    let body = operands.last().ok_or(NativeEachUnavailable::Geometry)?;
    if !simple(body.shape) {
        return Ok(selected);
    }
    let Some(groups) = prepare_each_groups(&operands, version, &mut selected) else {
        return Ok(selected);
    };
    finish_native_each(words, groups, body, version, collection, selected)
}

/// Prepare original value operands and the compiled loop body after declarations.
fn finish_native_each(
    words: &NativeCompilerWords<'_>,
    groups: Vec<NativeEachGroup>,
    body: &NativeProjectedCompilerWord,
    version: TclVersion,
    collection: NativeEachCollection,
    mut selected: NativeControlCompilation<NativeEachInstruction>,
) -> Result<NativeControlCompilation<NativeEachInstruction>, NativeEachUnavailable> {
    use NativeControlOutcome::Inline;
    use NativeControlPreparationStep as Step;
    let local_temporaries = version < TclVersion::V8_6;
    if local_temporaries {
        selected
            .preparations
            .extend((0..=groups.len()).map(|_| Step::DeclareAnonymousLocal));
    }
    if version == TclVersion::V8_4 {
        for group in &groups {
            for name in &group.variables {
                selected.preparations.push(Step::DeclareLocal(name.clone()));
            }
        }
    }
    for group in &groups {
        selected.preparations.push(Step::Word(group.values.clone()));
    }
    let body_span = native_control_body_span(words, &body.operand)
        .map_err(NativeEachUnavailable::Projection)?;
    selected.preparations.push(Step::Script {
        operand: body.operand.clone(),
        span: body_span,
        context: NativeCompiledBodyContext::Loop,
    });
    if collection == NativeEachCollection::Foreach {
        selected.preparations.push(Step::Literal(Vec::new()));
    }
    selected.outcome = Inline(NativeEachInstruction {
        version,
        collection,
        groups,
        body: body.operand.clone(),
        body_span,
        local_temporaries,
    });
    Ok(selected)
}

/// Preserve declaration visits even when a later original variable list declines.
fn prepare_each_groups(
    operands: &[&NativeProjectedCompilerWord],
    version: TclVersion,
    selected: &mut NativeControlCompilation<NativeEachInstruction>,
) -> Option<Vec<NativeEachGroup>> {
    let strings = tcl_syntax::native_string::NativeStringProtocol::C(version);
    let mut groups = Vec::new();
    for pair in operands[..operands.len() - 1].as_chunks::<2>().0 {
        let names = pair[0];
        if version == TclVersion::V8_4 && !simple(names.shape) {
            return None;
        }
        let value = names.literal.as_ref()?;
        let names = match tcl_syntax::list::split_native_list_bytes(value, strings) {
            Ok(names) if !names.is_empty() => names,
            Ok(_) => return None,
            Err(error) => {
                if version == TclVersion::V8_4 {
                    selected.outcome = NativeControlOutcome::Rejected(NativeCompilationFailure {
                        message: match error {
                            tcl_syntax::list::ListError::UnmatchedBrace
                            | tcl_syntax::list::ListError::UnmatchedQuote => {
                                Some(error.message().to_owned())
                            }
                            _ => None,
                        },
                        error_code: None,
                        error_info: None,
                    });
                }
                return None;
            }
        };
        let mut variables = Vec::new();
        for name in names {
            let projection = project_native_local_scalar(&name, version);
            if version > TclVersion::V8_4
                && let Some(declaration) = projection.declaration
            {
                selected
                    .preparations
                    .push(NativeControlPreparationStep::DeclareLocal(
                        declaration.to_vec(),
                    ));
            }
            if !projection.scalar {
                return None;
            }
            variables.push(
                projection
                    .declaration
                    .expect("native scalar local")
                    .to_vec(),
            );
        }
        groups.push(NativeEachGroup {
            variables,
            values: pair[1].operand.clone(),
        });
    }
    Some(groups)
}
