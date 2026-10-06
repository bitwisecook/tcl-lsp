// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original native dictionary lookup operand and stack geometry.

use crate::native_compiler_word_projection::{
    NativeCompilerWordOperand, project_native_compiler_words,
};
use crate::native_compiler_words::NativeCompilerWords;
use crate::native_dictionary::NativeDictionaryCommand;
use tcl_dialect::TclVersion;

/// Actual compiled dictionary lookup operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NativeDictionaryLookupKind {
    /// Lookup a nonempty key path.
    Get,
    /// Test a nonempty key path.
    Exists,
    /// Lookup a nonempty key path followed by its original default operand.
    GetDefault,
}

/// Ordered original dictionary lookup preparation, independent of its values.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NativeDictionaryLookupInstruction {
    /// Selected actual native operation.
    pub kind: NativeDictionaryLookupKind,
    /// Dictionary, ordered keys, then the default when applicable.
    pub operands: Vec<NativeCompilerWordOperand>,
    /// Actual key count consumed by the dictionary instruction.
    pub key_count: u32,
}

/// Original dictionary compiler selection frontier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeDictionaryCompilationUnavailable {
    /// The actual native compiler declines this command/arity/source form.
    Generic,
    /// The command's portable operation or original parser geometry is unavailable.
    Unavailable,
}

/// Retain the actual lookup compiler's original ordered operands.
/// No dictionary parsing, getter or callback runs at this compiler boundary.
///
/// # Errors
/// Distinguishes genuine compiler decline from unavailable operand/operation data.
pub fn compile_native_dictionary_lookup(
    command: NativeDictionaryCommand,
    words: &NativeCompilerWords<'_>,
    operand_from: usize,
    version: TclVersion,
) -> Result<NativeDictionaryLookupInstruction, NativeDictionaryCompilationUnavailable> {
    use NativeDictionaryCompilationUnavailable as Error;
    let kind = match command {
        NativeDictionaryCommand::Get => NativeDictionaryLookupKind::Get,
        NativeDictionaryCommand::Exists => NativeDictionaryLookupKind::Exists,
        NativeDictionaryCommand::GetDefault | NativeDictionaryCommand::GetWithDefault => {
            NativeDictionaryLookupKind::GetDefault
        }
        _ => return Err(Error::Unavailable),
    };
    if version < command.hook_from() {
        return Err(Error::Generic);
    }
    let original = project_native_compiler_words(words, version).map_err(|_| Error::Unavailable)?;
    let operands = original.get(operand_from..).ok_or(Error::Unavailable)?;
    if operands
        .iter()
        .any(|word| word.shape == crate::native_compilation::NativeCompilationWordShape::Expanded)
    {
        return Err(Error::Generic);
    }
    let overhead = if kind == NativeDictionaryLookupKind::GetDefault {
        2
    } else {
        1
    };
    let count = operands
        .len()
        .checked_sub(overhead)
        .filter(|count| *count > 0)
        .ok_or(Error::Generic)?;
    let key_count = u32::try_from(count).map_err(|_| Error::Generic)?;
    Ok(NativeDictionaryLookupInstruction {
        kind,
        operands: operands.iter().map(|word| word.operand.clone()).collect(),
        key_count,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_lexer::Span;
    use tcl_lexer::{LexerConfig, SourceImage, native_script_words_in};
    use tcl_syntax::native_string::NativeStringProtocol;

    fn original(source: &[u8], version: TclVersion) -> Vec<tcl_lexer::NativeWord> {
        let profile = tcl_dialect::DialectProfile::find(version.dialect_profile_name()).unwrap();
        native_script_words_in(
            SourceImage::native(source),
            Span::new(0, u32::try_from(source.len()).unwrap()),
            LexerConfig::from_grammar(profile.grammar),
        )
        .unwrap()
        .commands
        .remove(0)
        .words
    }

    #[test]
    fn dictionary_lookup_preserves_original_key_order_and_parser_expansion() {
        for version in [
            TclVersion::V8_5,
            TclVersion::V8_6,
            TclVersion::V9_0,
            TclVersion::V9_1,
        ] {
            for command in [
                NativeDictionaryCommand::Get,
                NativeDictionaryCommand::Exists,
            ] {
                let source = b"dict member $dictionary {*}{a b}";
                let native = original(source, version);
                let words = NativeCompilerWords::capture(&native, NativeStringProtocol::C(version))
                    .unwrap();
                let selection = compile_native_dictionary_lookup(command, &words, 2, version);
                if version < command.hook_from() {
                    assert_eq!(
                        selection,
                        Err(NativeDictionaryCompilationUnavailable::Generic)
                    );
                    continue;
                }
                let recipe = selection.unwrap();
                assert_eq!(recipe.key_count, 2);
                assert_eq!(recipe.operands[0], NativeCompilerWordOperand::Original(2));
                for (operand, expected) in recipe.operands[1..].iter().zip([b"a", b"b"]) {
                    let NativeCompilerWordOperand::LiteralExpansion {
                        original_word,
                        value_span,
                        value,
                    } = operand
                    else {
                        panic!("original parser member");
                    };
                    assert_eq!(*original_word, 3);
                    assert_eq!(value.as_slice(), expected.as_slice());
                    assert_eq!(&source[value_span.as_range()], expected);
                }
                let original = original(b"dict member $dictionary {*}$keys", version);
                let words =
                    NativeCompilerWords::capture(&original, NativeStringProtocol::C(version))
                        .unwrap();
                assert_eq!(
                    compile_native_dictionary_lookup(command, &words, 2, version),
                    Err(NativeDictionaryCompilationUnavailable::Generic)
                );
            }
        }
    }

    #[test]
    fn dictionary_default_operand_is_evaluated_after_every_original_key() {
        for version in [
            TclVersion::V8_4,
            TclVersion::V8_5,
            TclVersion::V8_6,
            TclVersion::V9_0,
            TclVersion::V9_1,
        ] {
            let original = original(b"dict member $dictionary $key $default", version);
            let words =
                NativeCompilerWords::capture(&original, NativeStringProtocol::C(version)).unwrap();
            let selected = compile_native_dictionary_lookup(
                NativeDictionaryCommand::GetDefault,
                &words,
                2,
                version,
            );
            if version < TclVersion::V9_0 {
                assert_eq!(
                    selected,
                    Err(NativeDictionaryCompilationUnavailable::Generic)
                );
            } else {
                let recipe = selected.unwrap();
                assert_eq!(recipe.key_count, 1);
                assert_eq!(
                    recipe.operands,
                    [
                        NativeCompilerWordOperand::Original(2),
                        NativeCompilerWordOperand::Original(3),
                        NativeCompilerWordOperand::Original(4)
                    ]
                );
            }
        }
    }
}
