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

//! Original native dictionary compiler operands and stack geometry.

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

/// `TclCompileDictCreateCmd` stack recipe.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum NativeDictionaryCreateInstruction {
    /// Every word is a literal: the dictionary's canonical string, then
    /// `dup` and `dictVerify`.
    Constant(Vec<u8>),
    /// C9.1: push the empty dictionary, then each key and value and `dictPut`.
    Put(Vec<NativeCompilerWordOperand>),
    /// C8.6/C9.0 in a procedure: build in an anonymous local with `dictSet`,
    /// then load and unset it.
    Worker(Vec<NativeCompilerWordOperand>),
}

/// Plan `dict create` as the release's compiler does.
///
/// # Errors
/// Native decline for an unsupported release, odd arity or expansion;
/// unavailable for geometry or for a non-constant C8.6/C9.0 script-level
/// form, which the compiler hands to its basic invocation instead.
pub fn compile_native_dictionary_create(
    words: &NativeCompilerWords<'_>,
    operand_from: usize,
    version: TclVersion,
    context: crate::native_compilation::NativeCompilationContext,
) -> Result<NativeDictionaryCreateInstruction, NativeDictionaryCompilationUnavailable> {
    use crate::native_compilation::{
        NativeCompilationFrame as Frame, NativeCompilationWordShape as Shape,
    };
    use NativeDictionaryCompilationUnavailable as Error;
    if version < NativeDictionaryCommand::Create.hook_from() {
        return Err(Error::Generic);
    }
    let original = project_native_compiler_words(words, version).map_err(|_| Error::Unavailable)?;
    let operands = original.get(operand_from..).ok_or(Error::Unavailable)?;
    if operands.iter().any(|word| word.shape == Shape::Expanded) || operands.len() % 2 != 0 {
        return Err(Error::Generic);
    }
    let literals = operands
        .iter()
        .map(|word| word.literal.as_deref())
        .collect::<Option<Vec<_>>>();
    if let Some(literals) = literals {
        return Ok(NativeDictionaryCreateInstruction::Constant(
            canonical_dictionary(&literals),
        ));
    }
    let operands = operands.iter().map(|word| word.operand.clone()).collect();
    if version >= TclVersion::V9_1 {
        return Ok(NativeDictionaryCreateInstruction::Put(operands));
    }
    if context.frame == Frame::ProcedureCode {
        Ok(NativeDictionaryCreateInstruction::Worker(operands))
    } else {
        Err(Error::Unavailable)
    }
}

/// `UpdateStringOfDict` over `Tcl_DictObjPut` in written order: a repeated
/// key keeps its first position and takes the last value; only the first
/// key quotes a leading `#`.
fn canonical_dictionary(literals: &[&[u8]]) -> Vec<u8> {
    let mut entries: Vec<(&[u8], &[u8])> = Vec::with_capacity(literals.len() / 2);
    let mut positions = std::collections::HashMap::new();
    for &[key, value] in literals.as_chunks::<2>().0 {
        if let Some(&index) = positions.get(key) {
            entries[index] = (key, value);
        } else {
            positions.insert(key, entries.len());
            entries.push((key, value));
        }
    }
    let mut out = Vec::new();
    for (index, (key, value)) in entries.iter().enumerate() {
        if index > 0 {
            out.push(b' ');
        }
        tcl_syntax::list::append_list_element(&mut out, key, index == 0);
        out.push(b' ');
        tcl_syntax::list::append_list_element(&mut out, value, false);
    }
    out
}

/// Actual dictionary mutation opcode operating on one retained compiled local.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NativeDictionaryMutationKind {
    /// Store the final value through an ordered key path.
    Set,
    /// Remove the final member through an ordered key path.
    Unset,
    /// Concatenate the original values before appending one prepared object.
    Append,
    /// Append exactly one original element to a member list.
    Lappend,
    /// Increment the original member by the compiler's signed immediate.
    Incr(i32),
}

/// Original receiver and stack operands accepted by a native mutation compiler.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NativeDictionaryMutationInstruction {
    /// Actual native instruction.
    pub kind: NativeDictionaryMutationKind,
    /// Counted original scalar name selecting the genuine physical local slot.
    pub receiver: Vec<u8>,
    /// Original keys followed by the original value operands, in visit order.
    pub operands: Vec<NativeCompilerWordOperand>,
    /// Number of original path keys consumed by the mutation instruction.
    pub key_count: u32,
}

/// Retain original mutation preparation, including declarations before decline.
/// This compiler receipt grants neither physical slot nor variable authority.
///
/// # Errors
/// Refuses missing original parser geometry or an unknown native frame.
pub fn compile_native_dictionary_mutation(
    command: NativeDictionaryCommand,
    words: &NativeCompilerWords<'_>,
    operand_from: usize,
    version: TclVersion,
    context: crate::native_compilation::NativeCompilationContext,
) -> Result<
    crate::native_control_compilation::NativeControlCompilation<
        NativeDictionaryMutationInstruction,
    >,
    NativeDictionaryCompilationUnavailable,
> {
    use crate::native_compilation::{
        NativeCompilationFrame as Frame, NativeCompilationWordShape as Shape,
    };
    use crate::native_control_compilation::{
        NativeControlCompilation as Compilation, NativeControlOutcome as Outcome,
        NativeControlPreparationStep as Step,
    };
    use NativeDictionaryCommand as Command;
    use NativeDictionaryCompilationUnavailable as Error;
    let mut preparations = Vec::new();
    let declined = |preparations| {
        Ok(Compilation {
            outcome: Outcome::Generic,
            preparations,
        })
    };
    if !matches!(
        command,
        Command::Set | Command::Unset | Command::Append | Command::Lappend | Command::Incr
    ) {
        return Err(Error::Unavailable);
    }
    if version < command.hook_from() {
        return declined(preparations);
    }
    let original = project_native_compiler_words(words, version).map_err(|_| Error::Unavailable)?;
    let operands = original.get(operand_from..).ok_or(Error::Unavailable)?;
    if operands.iter().any(|word| word.shape == Shape::Expanded) {
        return declined(preparations);
    }
    let count = operands.len();
    let valid = mutation_operand_count(command, count, version);
    if !valid {
        return declined(preparations);
    }
    if context.frame == Frame::Unknown {
        return Err(Error::Unavailable);
    }
    if context.frame != Frame::ProcedureCode {
        return declined(preparations);
    }
    let mut immediate = None;
    if command == Command::Incr && version > TclVersion::V8_5 {
        immediate = mutation_increment_amount(operands, version);
        if immediate.is_none() {
            return declined(preparations);
        }
    }
    let Some(name) = mutation_receiver(&operands[0], version, &mut preparations)? else {
        return declined(preparations);
    };
    if command == Command::Incr && version == TclVersion::V8_5 {
        immediate = mutation_increment_amount(operands, version);
        if immediate.is_none() {
            return declined(preparations);
        }
    }
    let kind = match command {
        Command::Set => NativeDictionaryMutationKind::Set,
        Command::Unset => NativeDictionaryMutationKind::Unset,
        Command::Append => NativeDictionaryMutationKind::Append,
        Command::Lappend => NativeDictionaryMutationKind::Lappend,
        Command::Incr => NativeDictionaryMutationKind::Incr(immediate.ok_or(Error::Unavailable)?),
        _ => return Err(Error::Unavailable),
    };
    let stack = if command == Command::Incr {
        &operands[1..2]
    } else {
        &operands[1..]
    };
    let operands = stack
        .iter()
        .map(|word| word.operand.clone())
        .collect::<Vec<_>>();
    preparations.extend(operands.iter().cloned().map(Step::Word));
    let keys = match command {
        Command::Set => count - 2,
        Command::Unset => count - 1,
        _ => 1,
    };
    Ok(Compilation {
        outcome: Outcome::Inline(NativeDictionaryMutationInstruction {
            kind,
            receiver: name.to_vec(),
            operands,
            key_count: u32::try_from(keys).map_err(|_| Error::Unavailable)?,
        }),
        preparations,
    })
}

fn mutation_receiver<'a>(
    operand: &'a crate::native_compiler_word_projection::NativeProjectedCompilerWord,
    version: TclVersion,
    preparations: &mut Vec<crate::native_control_compilation::NativeControlPreparationStep>,
) -> Result<Option<&'a [u8]>, NativeDictionaryCompilationUnavailable> {
    use crate::native_compilation::NativeCompilationWordShape as Shape;
    use crate::native_control_compilation::{
        NativeControlPreparationStep as Step, project_native_local_scalar,
    };
    if !matches!(
        operand.shape,
        Shape::Literal | Shape::QuotedLiteral | Shape::BracedLiteral
    ) {
        return Ok(None);
    }
    let name = operand
        .literal
        .as_deref()
        .ok_or(NativeDictionaryCompilationUnavailable::Unavailable)?;
    let scalar = project_native_local_scalar(name, version);
    if let Some(declaration) = scalar.declaration {
        preparations.push(Step::DeclareLocal(declaration.to_vec()));
    }
    Ok(scalar.scalar.then_some(name))
}

fn mutation_operand_count(
    command: NativeDictionaryCommand,
    count: usize,
    version: TclVersion,
) -> bool {
    use NativeDictionaryCommand as Command;
    match command {
        Command::Set => count >= 3,
        Command::Unset => count >= 2,
        Command::Append => {
            (3..=if version == TclVersion::V8_5 { 257 } else { 99 }).contains(&count)
        }
        Command::Lappend => count == 3,
        Command::Incr => (2..=3).contains(&count),
        _ => false,
    }
}

fn mutation_increment_amount(
    operands: &[crate::native_compiler_word_projection::NativeProjectedCompilerWord],
    version: TclVersion,
) -> Option<i32> {
    use crate::native_compilation::NativeCompilationWordShape as Shape;
    use tcl_syntax::scalar_getter::{
        NativeScalarGetterKind, NativeScalarGetterProtocol, NativeScalarGetterValue,
    };
    if operands.len() == 2 {
        return Some(1);
    }
    let word = &operands[2];
    if matches!(version, TclVersion::V8_6 | TclVersion::V9_0)
        && !matches!(
            word.shape,
            Shape::Literal | Shape::QuotedLiteral | Shape::BracedLiteral
        )
    {
        return None;
    }
    let conversion = NativeScalarGetterProtocol::for_tcl_version(version)
        .fresh_conversion(NativeScalarGetterKind::Int, word.literal.as_deref()?)?;
    let NativeScalarGetterValue::Wide(value) = conversion.outcome().ok()? else {
        return None;
    };
    i32::try_from(value).ok()
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
    fn mutation(
        source: &[u8],
        command: NativeDictionaryCommand,
        version: TclVersion,
    ) -> crate::native_control_compilation::NativeControlCompilation<
        NativeDictionaryMutationInstruction,
    > {
        let original = original(source, version);
        let words =
            NativeCompilerWords::capture(&original, NativeStringProtocol::C(version)).unwrap();
        compile_native_dictionary_mutation(
            command,
            &words,
            2,
            version,
            crate::native_compilation::NativeCompilationContext {
                frame: crate::native_compilation::NativeCompilationFrame::ProcedureCode,
                ..Default::default()
            },
        )
        .unwrap()
    }

    #[test]
    fn original_dictionary_mutations_retain_receiver_then_ordered_key_value_visits() {
        use crate::native_control_compilation::{
            NativeControlOutcome as Outcome, NativeControlPreparationStep as Step,
        };
        for version in [
            TclVersion::V8_5,
            TclVersion::V8_6,
            TclVersion::V9_0,
            TclVersion::V9_1,
        ] {
            let compiled = mutation(
                b"dict set d $first $second $value",
                NativeDictionaryCommand::Set,
                version,
            );
            let Outcome::Inline(recipe) = compiled.outcome else {
                panic!("native set compiler");
            };
            assert_eq!(recipe.kind, NativeDictionaryMutationKind::Set);
            assert_eq!(recipe.receiver, b"d");
            assert_eq!(recipe.key_count, 2);
            assert_eq!(
                compiled.preparations,
                [
                    Step::DeclareLocal(b"d".to_vec()),
                    Step::Word(NativeCompilerWordOperand::Original(3)),
                    Step::Word(NativeCompilerWordOperand::Original(4)),
                    Step::Word(NativeCompilerWordOperand::Original(5))
                ]
            );
            let declined = mutation(
                b"dict set a(element) k v",
                NativeDictionaryCommand::Set,
                version,
            );
            assert_eq!(declined.outcome, Outcome::Generic);
            assert_eq!(declined.preparations, [Step::DeclareLocal(b"a".to_vec())]);
        }
    }

    #[test]
    fn original_dictionary_increment_decline_preserves_actual_release_declaration_order() {
        use crate::native_control_compilation::{
            NativeControlOutcome as Outcome, NativeControlPreparationStep as Step,
        };
        for version in [
            TclVersion::V8_5,
            TclVersion::V8_6,
            TclVersion::V9_0,
            TclVersion::V9_1,
        ] {
            let declined = mutation(
                b"dict incr d k $amount",
                NativeDictionaryCommand::Incr,
                version,
            );
            assert_eq!(declined.outcome, Outcome::Generic);
            assert_eq!(
                declined.preparations,
                if version == TclVersion::V8_5 {
                    vec![Step::DeclareLocal(b"d".to_vec())]
                } else {
                    Vec::new()
                }
            );
            let compiled = mutation(
                b"dict incr d k 0xffffffff",
                NativeDictionaryCommand::Incr,
                version,
            );
            let Outcome::Inline(recipe) = compiled.outcome else {
                panic!("actual native Int probe");
            };
            assert_eq!(recipe.kind, NativeDictionaryMutationKind::Incr(-1));
            assert_eq!(recipe.operands, [NativeCompilerWordOperand::Original(3)]);
            let compiled = mutation(
                b"dict incr d k \\x32",
                NativeDictionaryCommand::Incr,
                version,
            );
            assert_eq!(
                matches!(compiled.outcome, Outcome::Inline(_)),
                matches!(version, TclVersion::V8_5 | TclVersion::V9_1)
            );
        }
    }

    #[test]
    fn original_dictionary_lappend_arity_and_unset_hook_are_actual_compiler_frontiers() {
        use crate::native_control_compilation::NativeControlOutcome as Outcome;
        for version in [
            TclVersion::V8_4,
            TclVersion::V8_5,
            TclVersion::V8_6,
            TclVersion::V9_0,
            TclVersion::V9_1,
        ] {
            assert_eq!(
                mutation(
                    b"dict lappend d k v w",
                    NativeDictionaryCommand::Lappend,
                    version
                )
                .outcome,
                Outcome::Generic
            );
            assert_eq!(
                matches!(
                    mutation(b"dict unset d k", NativeDictionaryCommand::Unset, version).outcome,
                    Outcome::Inline(_)
                ),
                version >= TclVersion::V8_6
            );
        }
    }
}
