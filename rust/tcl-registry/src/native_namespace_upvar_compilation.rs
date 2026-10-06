// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original C8.5 namespace-upvar compiler visits and partial local preparation.

use crate::native_compilation::{
    NativeCompilationContext, NativeCompilationFrame, NativeCompilationMode,
    NativeCompilationWordShape,
};
use crate::native_compiler_word_projection::{
    NativeCompilerWordOperand, NativeProjectedCompilerWord, project_native_compiler_words,
};
use crate::native_compiler_words::NativeCompilerWords;
use crate::native_namespace_binding_compilation::{
    NativeNamespaceBinding, NativeNamespaceBindingCompilation, NativeNamespaceBindingKind,
    NativeNamespaceBindingOutcome, NativeNamespaceBindingUnavailable, NativeNamespaceBindingVisit,
};
use tcl_dialect::TclVersion;
use tcl_syntax::naming::NativeCompiledVariableRecipe;

/// Project the monolithic C8.5 compiler over one unchanged original vector.
///
/// The caller independently authenticates the installed compileProc, original
/// procedure context and source issuer. Visits describe compilation, without
/// granting variable cells, evaluated arguments or namespace tokens.
///
/// # Errors
/// Returns original parser geometry obligations instead of fabricated decline.
pub fn compile_native_namespace_upvar(
    words: &NativeCompilerWords<'_>,
    operand_from: usize,
    version: TclVersion,
    context: NativeCompilationContext,
) -> Result<NativeNamespaceBindingCompilation, NativeNamespaceBindingUnavailable> {
    let mut result = NativeNamespaceBindingCompilation {
        kind: NativeNamespaceBindingKind::Upvar,
        namespace: None,
        outcome: NativeNamespaceBindingOutcome::Generic,
        bindings: Vec::new(),
        visits: Vec::new(),
    };
    if operand_from == 0 || operand_from > words.original_words().len() {
        return Err(NativeNamespaceBindingUnavailable::OperandGeometry);
    }
    if version < TclVersion::V8_5
        || context.mode == NativeCompilationMode::Direct
        || context.frame == NativeCompilationFrame::ScriptCode
    {
        return Ok(result);
    }
    if version != TclVersion::V8_5
        || context.mode != NativeCompilationMode::BytecodeObject
        || context.frame != NativeCompilationFrame::ProcedureCode
    {
        result.outcome = NativeNamespaceBindingOutcome::Unknown;
        return Ok(result);
    }
    let projected = project_native_compiler_words(words, version)
        .map_err(NativeNamespaceBindingUnavailable::Projection)?;
    if projected
        .iter()
        .any(|word| word.shape == NativeCompilationWordShape::Expanded)
    {
        return Ok(result);
    }
    let operands: Vec<_> = projected
        .iter()
        .filter(|word| match word.operand {
            NativeCompilerWordOperand::Original(index) => index >= operand_from,
            NativeCompilerWordOperand::LiteralExpansion { original_word, .. } => {
                original_word >= operand_from
            }
        })
        .collect();
    if operands.len() < 4
        || operands.len() % 2 != 0
        || raw_word(words, operands[0]) != Some(b"upvar".as_slice())
    {
        return Ok(result);
    }
    let namespace = operands[1].operand.clone();
    result.namespace = Some(namespace.clone());
    result
        .visits
        .push(NativeNamespaceBindingVisit::Word(namespace));
    for pair in operands[2..].as_chunks::<2>().0 {
        let other = pair[0].operand.clone();
        result
            .visits
            .push(NativeNamespaceBindingVisit::Word(other.clone()));
        if !prepare_local_pair(&mut result, pair[1], other, version) {
            return Ok(result);
        }
    }
    result.outcome = NativeNamespaceBindingOutcome::Inline;
    Ok(result)
}

fn prepare_local_pair(
    result: &mut NativeNamespaceBindingCompilation,
    local: &NativeProjectedCompilerWord,
    other: NativeCompilerWordOperand,
    version: TclVersion,
) -> bool {
    if !matches!(
        local.shape,
        NativeCompilationWordShape::Literal
            | NativeCompilationWordShape::QuotedLiteral
            | NativeCompilationWordShape::BracedLiteral
    ) {
        return false;
    }
    let Some(value) = local.literal.as_deref() else {
        return false;
    };
    let scalar = NativeCompiledVariableRecipe::C(version)
        .scalar_name(value)
        .expect("C scalar-name recipe");
    if let Some(name) = scalar.declaration {
        result
            .visits
            .push(NativeNamespaceBindingVisit::DeclareLocal(name.to_vec()));
    }
    if !scalar.scalar {
        return false;
    }
    result.bindings.push(NativeNamespaceBinding {
        name: other,
        local: value.to_vec(),
        value: None,
    });
    true
}

fn raw_word<'a>(
    words: &'a NativeCompilerWords<'_>,
    word: &NativeProjectedCompilerWord,
) -> Option<&'a [u8]> {
    match &word.operand {
        NativeCompilerWordOperand::Original(index) => words
            .original_words()
            .get(*index)
            .map(tcl_lexer::NativeWord::bytes),
        NativeCompilerWordOperand::LiteralExpansion {
            original_word,
            value_span,
            ..
        } => words
            .original_words()
            .get(*original_word)?
            .image()
            .bytes()
            .get(value_span.as_range()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_lexer::{LexerConfig, SourceImage, Span, native_script_words_in};
    use tcl_syntax::native_string::NativeStringProtocol;

    mod inputs {
        include!("../tests/data/native_namespace_upvar_compilation/cases.rs");
    }

    fn recipe(
        source: &[u8],
        context: NativeCompilationContext,
    ) -> NativeNamespaceBindingCompilation {
        let profile = tcl_dialect::DialectProfile::find("tcl8.5").unwrap();
        let command = native_script_words_in(
            SourceImage::native(source),
            Span::new(0, u32::try_from(source.len()).unwrap()),
            LexerConfig::from_grammar(profile.grammar),
        )
        .unwrap()
        .commands
        .remove(0);
        let words =
            NativeCompilerWords::capture(&command.words, NativeStringProtocol::C(TclVersion::V8_5))
                .unwrap();
        compile_native_namespace_upvar(&words, 1, TclVersion::V8_5, context).unwrap()
    }

    fn context() -> NativeCompilationContext {
        NativeCompilationContext {
            mode: NativeCompilationMode::BytecodeObject,
            frame: NativeCompilationFrame::ProcedureCode,
            ..Default::default()
        }
    }

    #[test]
    fn original_c85_namespace_upvar_matches_15_native_compiler_frontiers() {
        let mut count = 0;
        for row in include_str!("../tests/data/native_namespace_upvar_compilation/8.5.19.tsv")
            .lines()
            .skip(1)
        {
            let fields: Vec<_> = row.split('\t').collect();
            let index: usize = fields[0].parse().unwrap();
            let (label, source) = inputs::CASES[index];
            let selected = recipe(source, context());
            let native_count: usize = fields[1].parse().unwrap();
            assert_eq!(
                selected.outcome == NativeNamespaceBindingOutcome::Inline,
                native_count != 0,
                "{label}"
            );
            if native_count != 0 {
                assert_eq!(selected.bindings.len(), native_count, "{label}");
                assert!(selected.namespace.is_some(), "{label}");
            }
            count += 1;
        }
        assert_eq!(count, 15);
        let selected = recipe(inputs::CASES[10].1, context());
        assert_eq!(
            selected.visits,
            vec![
                NativeNamespaceBindingVisit::Word(NativeCompilerWordOperand::Original(2)),
                NativeNamespaceBindingVisit::Word(NativeCompilerWordOperand::Original(3)),
                NativeNamespaceBindingVisit::DeclareLocal(b"good".to_vec()),
                NativeNamespaceBindingVisit::Word(NativeCompilerWordOperand::Original(5)),
                NativeNamespaceBindingVisit::DeclareLocal(b"bad".to_vec()),
            ]
        );
        assert_eq!(selected.bindings.len(), 1);
    }

    #[test]
    fn namespace_upvar_requires_original_procedure_compiler_context() {
        let source = b"namespace upvar ::N x local";
        for frame in [
            NativeCompilationFrame::ScriptCode,
            NativeCompilationFrame::Unknown,
        ] {
            let selected = recipe(source, NativeCompilationContext { frame, ..context() });
            assert!(selected.visits.is_empty());
            assert_eq!(
                selected.outcome,
                if frame == NativeCompilationFrame::ScriptCode {
                    NativeNamespaceBindingOutcome::Generic
                } else {
                    NativeNamespaceBindingOutcome::Unknown
                }
            );
        }
        let direct = recipe(
            source,
            NativeCompilationContext {
                mode: NativeCompilationMode::Direct,
                ..context()
            },
        );
        assert_eq!(direct.outcome, NativeNamespaceBindingOutcome::Generic);
        assert!(direct.namespace.is_none());
    }
}
