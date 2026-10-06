// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original dictionary scope compiler geometry and ordered preparation.

use crate::native_compilation::{
    NativeCompilationContext, NativeCompilationFrame, NativeCompilationWordShape,
    NativeCompiledBodyContext,
};
use crate::native_compiler_word_projection::{
    NativeCompilerWordOperand, NativeProjectedCompilerWord, project_native_compiler_words,
};
use crate::native_compiler_words::NativeCompilerWords;
use crate::native_control_compilation::{
    NativeControlCompilation, NativeControlOutcome, NativeControlPreparationStep as Step,
    native_control_body_span, project_native_local_scalar,
};
use crate::native_dictionary::NativeDictionaryCommand;
use tcl_dialect::TclVersion;

#[cfg(test)]
mod tests;

/// Dictionary root operand selected by the actual native compiler.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeDictionaryScopeReceiver {
    /// A declared actual native scalar slot; the name is never an argv object.
    Local(Vec<u8>),
    /// An original variable-name operand, retained across the body.
    Stack(NativeCompilerWordOperand),
}

/// Native dictionary update auxiliary targets or with expansion state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeDictionaryScopeKind {
    /// Ordered keys and physical destination declarations in private auxiliary data.
    Update {
        /// Original key operands, evaluated in native compiler order.
        keys: Vec<NativeCompilerWordOperand>,
        /// Ordered physical destination names retained by auxiliary data.
        targets: Vec<Vec<u8>>,
    },
    /// Original key path and native anonymous temporaries.
    With {
        /// Original nested dictionary key operands.
        path: Vec<NativeCompilerWordOperand>,
        /// Whether the original literal body has no executable statements.
        empty_body: bool,
    },
}

/// Scope operation whose concrete consumers install actual slots independently.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeDictionaryScopeInstruction {
    /// Original native compiler release.
    pub version: TclVersion,
    /// Native local or stack receiver geometry.
    pub receiver: NativeDictionaryScopeReceiver,
    /// Actual update or with instruction geometry.
    pub kind: NativeDictionaryScopeKind,
    /// Original literal body operand.
    pub body: NativeCompilerWordOperand,
    /// Body extent in the unchanged source image.
    pub body_span: tcl_lexer::Span,
}

/// A scope recipe requires authentic original parser and compiler-frame evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeDictionaryScopeUnavailable {
    /// This command is not a scope compiler.
    Operation,
    /// The original parser or native frame is unavailable.
    Geometry,
}

fn simple(shape: NativeCompilationWordShape) -> bool {
    matches!(
        shape,
        NativeCompilationWordShape::Literal
            | NativeCompilationWordShape::QuotedLiteral
            | NativeCompilationWordShape::BracedLiteral
    )
}

fn local(
    word: &NativeProjectedCompilerWord,
    version: TclVersion,
    frame: NativeCompilationFrame,
    preparations: &mut Vec<Step>,
) -> Option<Vec<u8>> {
    if frame != NativeCompilationFrame::ProcedureCode || !simple(word.shape) {
        return None;
    }
    let projected = project_native_local_scalar(word.literal.as_deref()?, version);
    if let Some(name) = projected.declaration {
        preparations.push(Step::DeclareLocal(name.to_vec()));
    }
    projected
        .scalar
        .then(|| word.literal.as_ref().expect("literal scalar").clone())
}

fn prepare_update(
    operands: &[NativeProjectedCompilerWord],
    version: TclVersion,
    context: NativeCompilationContext,
    preparations: &mut Vec<Step>,
) -> Option<(NativeDictionaryScopeReceiver, NativeDictionaryScopeKind)> {
    let count = operands.len();
    let body = &operands[count - 1];
    let Some(name) = local(&operands[0], version, context.frame, preparations) else {
        return None;
    };
    let receiver = NativeDictionaryScopeReceiver::Local(name);
    let mut keys = Vec::new();
    let mut targets = Vec::new();
    for pair in operands[1..count - 1].chunks_exact(2) {
        let Some(name) = local(&pair[1], version, context.frame, preparations) else {
            return None;
        };
        keys.push(pair[0].operand.clone());
        targets.push(name);
    }
    if !simple(body.shape) {
        return None;
    }
    preparations.extend(keys.iter().cloned().map(Step::Word));
    let kind = NativeDictionaryScopeKind::Update { keys, targets };
    Some((receiver, kind))
}

fn prepare_with(
    operands: &[NativeProjectedCompilerWord],
    version: TclVersion,
    context: NativeCompilationContext,
    preparations: &mut Vec<Step>,
) -> Result<
    Option<(NativeDictionaryScopeReceiver, NativeDictionaryScopeKind)>,
    NativeDictionaryScopeUnavailable,
> {
    use NativeDictionaryScopeUnavailable as Unavailable;
    let count = operands.len();
    let body = &operands[count - 1];
    if !simple(body.shape) {
        return Ok(None);
    }
    let bytes = body.literal.as_deref().ok_or(Unavailable::Geometry)?;
    let empty_body = if version >= TclVersion::V9_1 {
        tcl_syntax::native_tcl_utf::NativeTclUtf::for_version(version)
            .decode_units(bytes)
            .into_iter()
            .all(|unit| matches!(unit, 9..=13 | 32))
    } else {
        bytes
            .iter()
            .all(|byte| matches!(byte, b' ' | b'\t' | b'\r' | b'\n'))
    };
    if !empty_body && context.frame != NativeCompilationFrame::ProcedureCode {
        return Ok(None);
    }
    let receiver = if let Some(name) = local(&operands[0], version, context.frame, preparations) {
        NativeDictionaryScopeReceiver::Local(name)
    } else {
        NativeDictionaryScopeReceiver::Stack(operands[0].operand.clone())
    };
    let path = operands[1..count - 1]
        .iter()
        .map(|word| word.operand.clone())
        .collect::<Vec<_>>();
    if !empty_body {
        if matches!(receiver, NativeDictionaryScopeReceiver::Stack(_)) {
            preparations.push(Step::DeclareAnonymousLocal);
        }
        if !path.is_empty() {
            preparations.push(Step::DeclareAnonymousLocal);
        }
        preparations.push(Step::DeclareAnonymousLocal);
    }
    if let NativeDictionaryScopeReceiver::Stack(operand) = &receiver {
        preparations.push(Step::Word(operand.clone()));
    }
    preparations.extend(path.iter().cloned().map(Step::Word));
    if path.is_empty() {
        preparations.push(Step::Literal(Vec::new()));
    }
    let kind = NativeDictionaryScopeKind::With { path, empty_body };
    Ok(Some((receiver, kind)))
}

/// Retain actual declarations, original operands and protected body visits.
/// The recipe neither resolves runtime names nor constructs command argv.
///
/// # Errors
/// Declines unavailable original token or physical compiler-frame evidence.
pub fn compile_native_dictionary_scope(
    command: NativeDictionaryCommand,
    words: &NativeCompilerWords<'_>,
    operand_from: usize,
    version: TclVersion,
    context: NativeCompilationContext,
) -> Result<
    NativeControlCompilation<NativeDictionaryScopeInstruction>,
    NativeDictionaryScopeUnavailable,
> {
    use NativeDictionaryScopeUnavailable as Unavailable;
    if !matches!(
        command,
        NativeDictionaryCommand::Update | NativeDictionaryCommand::With
    ) {
        return Err(Unavailable::Operation);
    }
    let mut selected = NativeControlCompilation {
        outcome: NativeControlOutcome::Generic,
        preparations: Vec::new(),
    };
    if version < command.hook_from() {
        return Ok(selected);
    }
    let projected =
        project_native_compiler_words(words, version).map_err(|_| Unavailable::Geometry)?;
    let operands = projected.get(operand_from..).ok_or(Unavailable::Geometry)?;
    if operands
        .iter()
        .any(|word| word.shape == NativeCompilationWordShape::Expanded)
    {
        return Ok(selected);
    }
    let count = operands.len();
    if (command == NativeDictionaryCommand::Update && (count < 4 || !count.is_multiple_of(2)))
        || (command == NativeDictionaryCommand::With && count < 2)
    {
        return Ok(selected);
    }
    if context.frame == NativeCompilationFrame::Unknown {
        return Err(Unavailable::Geometry);
    }
    let body = &operands[count - 1];
    let geometry = if command == NativeDictionaryCommand::Update {
        prepare_update(operands, version, context, &mut selected.preparations)
    } else {
        prepare_with(operands, version, context, &mut selected.preparations)?
    };
    let Some((receiver, kind)) = geometry else {
        return Ok(selected);
    };
    let body_span =
        native_control_body_span(words, &body.operand).map_err(|_| Unavailable::Geometry)?;
    if !matches!(
        kind,
        NativeDictionaryScopeKind::With {
            empty_body: true,
            ..
        }
    ) {
        selected.preparations.push(Step::Script {
            operand: body.operand.clone(),
            span: body_span,
            context: NativeCompiledBodyContext::ExceptionRange,
        });
    }
    if let NativeDictionaryScopeKind::With { path, empty_body } = &kind {
        if path.is_empty() && !empty_body {
            selected.preparations.push(Step::Literal(Vec::new()));
            selected.preparations.push(Step::Literal(Vec::new()));
        }
        if *empty_body {
            if path.is_empty() {
                selected.preparations.push(Step::Literal(Vec::new()));
            }
            selected.preparations.push(Step::Literal(Vec::new()));
        }
    }
    selected.outcome = NativeControlOutcome::Inline(NativeDictionaryScopeInstruction {
        version,
        receiver,
        kind,
        body: body.operand.clone(),
        body_span,
    });
    Ok(selected)
}
