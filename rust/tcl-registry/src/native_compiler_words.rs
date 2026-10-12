// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original byte words for native compiler descriptors, independent of argv.

use crate::InvocationWord;
use crate::native_compilation::NativeCompilationWordShape;
use tcl_dialect::TclVersion;
use tcl_lexer::{ExecutablePart, ExecutableText, NativeWord, NativeWordError, WordKind};
use tcl_syntax::backslash::NativeArenaTextUnavailable;
use tcl_syntax::native_string::NativeStringProtocol;

/// Unavailable original word ownership, never a native compiler decline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeCompilerWordsUnavailable {
    /// An invocation has no original head word.
    MissingHead,
    /// Words do not retain one image, configuration and ordered source layout.
    SourceOwnership,
    /// The grouped word cannot expose the selected original content extent.
    Word(NativeWordError),
    /// The independently selected source-string decoder cannot form a value.
    Text(NativeArenaTextUnavailable),
}

/// Original native List construction, independently of command registration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeCompiledListRecipe {
    /// Zero operands push the registered empty string object.
    EmptyString,
    /// Original operands are evaluated and assembled by the List opcode.
    DynamicElements,
    /// C8.6+ constructs a private constant List from these static members.
    PrivateConstant {
        /// Exact member value bytes, preserving their order and empty values.
        members: Vec<Vec<u8>>,
    },
}

/// Original List operand layout cannot be supplied to the native compiler.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeCompiledListUnavailable {
    /// The index does not address operands in this complete original vector.
    OperandGeometry,
    /// Native parser list-expansion projection has not been retained.
    Expansion,
}

/// Frozen original compiler words and static byte values. Indices address the
/// complete original vector, including its head; they never address evaluated
/// or expanded argv. A missing literal means substitution, not opaque text.
/// This carrier grants no command registration, handler or object-class proof.
#[derive(Debug, Clone)]
pub struct NativeCompilerWords<'w> {
    original: &'w [NativeWord],
    shapes: Vec<NativeCompilationWordShape>,
    literals: Vec<Option<Vec<u8>>>,
    protocol: NativeStringProtocol,
}

impl<'w> NativeCompilerWords<'w> {
    /// Retain one original command and form static values through the shared
    /// source-channel and native string owners. `protocol` is the source-word
    /// issuer, independent of physical compiler and command-name protocols.
    ///
    /// # Errors
    /// Rejects mixed ownership or unavailable value decoding. Opaque bytes in
    /// a static value remain a successful literal capture.
    pub fn capture(
        original: &'w [NativeWord],
        protocol: NativeStringProtocol,
    ) -> Result<Self, NativeCompilerWordsUnavailable> {
        let head = original
            .first()
            .ok_or(NativeCompilerWordsUnavailable::MissingHead)?;
        if original
            .iter()
            .any(|word| word.image() != head.image() || word.config() != head.config())
            || original
                .windows(2)
                .any(|pair| pair[0].span().end() > pair[1].span().start())
        {
            return Err(NativeCompilerWordsUnavailable::SourceOwnership);
        }
        let mut shapes = Vec::with_capacity(original.len());
        let mut literals = Vec::with_capacity(original.len());
        for word in original {
            let (shape, literal) = capture_word(word, protocol)?;
            shapes.push(shape);
            literals.push(literal);
        }
        Ok(Self {
            original,
            shapes,
            literals,
            protocol,
        })
    }

    /// The complete original lexical vector, including source/capture extents.
    #[must_use]
    pub fn original_words(&self) -> &'w [NativeWord] {
        self.original
    }

    /// Compiler token shapes in the same complete-vector address space.
    #[must_use]
    pub fn shapes(&self) -> &[NativeCompilationWordShape] {
        &self.shapes
    }

    /// Exact static value bytes; substituted words return `None`. A static
    /// empty byte string remains `Some(&[])`, distinct from missing evidence.
    #[must_use]
    pub fn literal(&self, original_index: usize) -> Option<&[u8]> {
        self.literals.get(original_index)?.as_deref()
    }

    /// Original source extent of a static native-value range. Channel units
    /// map through the shared literal owner; an escape maps only at its whole
    /// selected token boundaries. Folded braced continuations and ambiguous
    /// boundaries decline. This is readonly correspondence, without edit or
    /// compilation authority.
    #[must_use]
    pub fn original_literal_extent(
        &self,
        original_index: usize,
        native: std::ops::Range<usize>,
    ) -> Option<tcl_lexer::Span> {
        let value = self.literal(original_index)?;
        if native.start > native.end || native.end > value.len() {
            return None;
        }
        let word = self.original.get(original_index)?;
        if word.group().kind == WordKind::Braced {
            let content = word.content_span().ok()?;
            let raw = word.image().bytes().get(content.as_range())?;
            let literal = tcl_syntax::backslash::native_source_literal_bytes(
                raw,
                word.image().channel(),
                self.protocol,
            )
            .ok()?;
            if literal.as_ref() != value {
                return None;
            }
            let extent = tcl_syntax::backslash::native_source_literal_extent(
                raw,
                word.image().channel(),
                self.protocol,
                native,
            )?;
            return Some(tcl_lexer::Span::new(
                content
                    .start()
                    .checked_add(u32::try_from(extent.start).ok()?)?,
                content
                    .start()
                    .checked_add(u32::try_from(extent.end).ok()?)?,
            ));
        }
        let arena = word.executable_parts();
        let mut offset = 0_usize;
        let mut start = None;
        let mut end = None;
        for part in arena.list(arena.root()) {
            let bytes = tcl_syntax::backslash::native_arena_text(
                arena,
                part,
                word.config().escapes,
                self.protocol,
            )
            .ok()?;
            let next = offset.checked_add(bytes.len())?;
            let point = |boundary: usize| {
                let local = boundary.checked_sub(offset)?;
                if local > bytes.len() {
                    return None;
                }
                let local = match &part.part {
                    ExecutablePart::Text(ExecutableText::Original) => {
                        let raw = arena.bytes(part.span)?;
                        tcl_syntax::backslash::native_source_literal_extent(
                            raw,
                            arena.image().channel(),
                            self.protocol,
                            local..local,
                        )?
                        .start
                    }
                    ExecutablePart::Text(ExecutableText::Decoded(_)) => {
                        let raw = arena.bytes(part.span)?;
                        tcl_syntax::backslash::native_source_string_extent(
                            raw,
                            arena.image().channel(),
                            word.config().escapes,
                            self.protocol,
                            local..local,
                        )?
                        .start
                    }
                    _ => return None,
                };
                part.span.start().checked_add(u32::try_from(local).ok()?)
            };
            for (boundary, selected) in [(native.start, &mut start), (native.end, &mut end)] {
                if offset <= boundary && boundary <= next {
                    let original = point(boundary)?;
                    if selected.is_some_and(|previous| previous != original) {
                        return None;
                    }
                    *selected = Some(original);
                }
            }
            offset = next;
        }
        (offset == value.len()).then_some(())?;
        Some(tcl_lexer::Span::new(start?, end?))
    }

    /// Independently selected source-word recipe used to form static values.
    #[must_use]
    pub const fn source_protocol(&self) -> NativeStringProtocol {
        self.protocol
    }

    /// Retain the selected compiler's fresh known-word construction steps.
    #[must_use]
    pub fn known_word_literal(
        &self,
        index: usize,
    ) -> Option<tcl_runtime_api::native_return_literal::NativeKnownWordLiteral> {
        use tcl_runtime_api::native_return_literal::NativeKnownWordLiteral;
        let value = self.literal(index)?;
        let word = self.original.get(index)?;
        let composite = self.shapes[index] == NativeCompilationWordShape::BackslashLiteral;
        let pieces = if composite && word.group().kind != WordKind::Braced {
            let arena = word.executable_parts();
            arena
                .list(arena.root())
                .iter()
                .map(|part| {
                    tcl_syntax::backslash::native_arena_text(
                        arena,
                        part,
                        word.config().escapes,
                        self.protocol,
                    )
                    .ok()
                    .map(std::borrow::Cow::into_owned)
                })
                .collect::<Option<Vec<_>>>()?
        } else {
            vec![value.to_vec()]
        };
        Some(NativeKnownWordLiteral { pieces, composite })
    }

    /// Select the List object construction recipe after independent compiler
    /// selection. C8.4/8.5 require a procedure compiler; the descriptor checks
    /// that frame requirement. This method grants no hook or frame authority.
    ///
    /// # Errors
    /// Rejects invalid operand indices and unprojected original expansions.
    pub fn list_recipe(
        &self,
        operand_from: usize,
        physical_version: TclVersion,
    ) -> Result<NativeCompiledListRecipe, NativeCompiledListUnavailable> {
        if operand_from == 0 || operand_from > self.original.len() {
            return Err(NativeCompiledListUnavailable::OperandGeometry);
        }
        let shapes = &self.shapes[operand_from..];
        if shapes.contains(&NativeCompilationWordShape::Expanded) {
            return Err(NativeCompiledListUnavailable::Expansion);
        }
        if shapes.is_empty() {
            return Ok(NativeCompiledListRecipe::EmptyString);
        }
        if physical_version >= TclVersion::V8_6
            && let Some(members) = self.literals[operand_from..].iter().cloned().collect()
        {
            return Ok(NativeCompiledListRecipe::PrivateConstant { members });
        }
        Ok(NativeCompiledListRecipe::DynamicElements)
    }

    /// Native compile-time increment amount from a fresh private string object.
    /// C8.4/8.5 use `GetInt`; C8.6+ use `GetWide`. C8.6 separately requires an
    /// integer cache. C9.1 accepts any compile-time-known original word;
    /// earlier compilers require `SIMPLE_WORD`. This proves no runtime cell read.
    #[must_use]
    pub fn increment_immediate(
        &self,
        original_index: usize,
        physical_version: TclVersion,
    ) -> Option<i32> {
        use tcl_syntax::scalar_getter::{
            NativeScalarCache, NativeScalarGetterKind, NativeScalarGetterProtocol,
            NativeScalarGetterValue,
        };
        let shape = *self.shapes.get(original_index)?;
        if shape == NativeCompilationWordShape::Expanded
            || (physical_version < TclVersion::V9_1
                && !matches!(
                    shape,
                    NativeCompilationWordShape::Literal
                        | NativeCompilationWordShape::QuotedLiteral
                        | NativeCompilationWordShape::BracedLiteral
                ))
        {
            return None;
        }
        let bytes = self.literal(original_index)?;
        if physical_version == TclVersion::V8_4
            && !tcl_syntax::scalar_getter::compiler_integer_prefix84(bytes)
        {
            return None;
        }
        let kind = if physical_version <= TclVersion::V8_5 {
            NativeScalarGetterKind::Int
        } else {
            NativeScalarGetterKind::Wide
        };
        let conversion = NativeScalarGetterProtocol::for_tcl_version(physical_version)
            .fresh_conversion(kind, bytes)?;
        if physical_version == TclVersion::V8_6
            && !matches!(
                conversion.cache(),
                Some(NativeScalarCache::Number(tcl_syntax::number::Number::Int(
                    _
                )))
            )
        {
            return None;
        }
        let NativeScalarGetterValue::Wide(value) = conversion.outcome().ok()? else {
            return None;
        };
        if (-127..=127).contains(&value) {
            i32::try_from(value).ok()
        } else {
            None
        }
    }

    pub(super) fn checked_words(&self) -> Option<Vec<InvocationWord<'_>>> {
        self.shapes
            .iter()
            .zip(&self.literals)
            .map(|(shape, value)| {
                if *shape == NativeCompilationWordShape::Expanded {
                    Some(InvocationWord::Expanded)
                } else if let Some(bytes) = value {
                    std::str::from_utf8(bytes).ok().map(InvocationWord::Literal)
                } else {
                    Some(InvocationWord::Dynamic)
                }
            })
            .collect()
    }
}

fn capture_word(
    word: &NativeWord,
    protocol: NativeStringProtocol,
) -> Result<(NativeCompilationWordShape, Option<Vec<u8>>), NativeCompilerWordsUnavailable> {
    use NativeCompilationWordShape as Shape;
    let arena = word.executable_parts();
    let parts = arena.list(arena.root());
    let static_word = parts
        .iter()
        .all(|part| matches!(part.part, ExecutablePart::Text(_)));
    if !static_word {
        return Ok((
            if word.group().expand {
                Shape::Expanded
            } else {
                Shape::Substituted
            },
            None,
        ));
    }
    let (mut shape, value) = if word.group().kind == WordKind::Braced {
        let span = word
            .content_span()
            .map_err(NativeCompilerWordsUnavailable::Word)?;
        let original = arena
            .bytes(span)
            .ok_or(NativeCompilerWordsUnavailable::SourceOwnership)?;
        let value = tcl_syntax::backslash::native_source_braced_word_bytes(
            original,
            word.image().channel(),
            word.config().brace_backslash_newline,
            protocol,
        )
        .map_err(|error| {
            NativeCompilerWordsUnavailable::Text(
                tcl_syntax::backslash::NativeArenaTextUnavailable::Source(error),
            )
        })?;
        let continuation = value.as_ref()
            != tcl_syntax::backslash::native_source_literal_bytes(
                original,
                word.image().channel(),
                protocol,
            )
            .map_err(|error| {
                NativeCompilerWordsUnavailable::Text(
                    tcl_syntax::backslash::NativeArenaTextUnavailable::Source(error),
                )
            })?
            .as_ref();
        (
            if continuation && word.config().brace_backslash_newline.folds() {
                Shape::BackslashLiteral
            } else {
                Shape::BracedLiteral
            },
            value.into_owned(),
        )
    } else {
        let mut value = Vec::new();
        for part in parts {
            value.extend_from_slice(
                &tcl_syntax::backslash::native_arena_text(
                    arena,
                    part,
                    word.config().escapes,
                    protocol,
                )
                .map_err(NativeCompilerWordsUnavailable::Text)?,
            );
        }
        let shape = if parts
            .iter()
            .any(|part| matches!(part.part, ExecutablePart::Text(ExecutableText::Decoded(_))))
        {
            Shape::BackslashLiteral
        } else if word.group().kind == WordKind::Quoted {
            Shape::QuotedLiteral
        } else {
            Shape::Literal
        };
        (shape, value)
    };
    if word.group().expand {
        shape = Shape::Expanded;
    }
    Ok((shape, Some(value)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native_compilation::{
        NativeBodyCompilation, NativeCompilationContext, NativeCompilationFrame,
        NativeCompilationGrammar, NativeCompilationMode, NativeCompilationSelection,
        NativeCompilationSpec,
    };
    use tcl_lexer::{LexerConfig, SourceImage, Span, native_script_words_in};

    fn words(source: &[u8]) -> Vec<NativeWord> {
        words_for_version(source, TclVersion::V9_0)
    }

    fn words_for_version(source: &[u8], version: TclVersion) -> Vec<NativeWord> {
        let image = SourceImage::native(source);
        let end = u32::try_from(image.len()).unwrap();
        let profile =
            tcl_dialect::DialectProfile::find(&format!("tcl{}", version.version_string())).unwrap();
        native_script_words_in(
            image,
            Span::new(0, end),
            LexerConfig::from_grammar(profile.grammar),
        )
        .unwrap()
        .commands
        .remove(0)
        .words
    }

    #[test]
    fn original_document_words_match_native_character_channel_values() {
        // Native proof: naming.source.document-native-utf-ingress
        // docs/design/analysis/name-resolution-proofs/document-native-utf-ingress.md
        let source = include_bytes!("../../tcl-syntax/tests/data/native_source_ingress/source.tcl");
        let observations =
            include_str!("../../tcl-syntax/tests/data/native_source_ingress/observations.tsv");
        for version in TclVersion::ALL {
            let profile = tcl_dialect::DialectProfile::find(version.dialect_name()).unwrap();
            for channel in [
                tcl_lexer::SourceChannel::Document,
                tcl_lexer::SourceChannel::NativeValue,
            ] {
                let image = SourceImage::from_bytes(source.as_slice(), channel);
                let commands = native_script_words_in(
                    image.clone(),
                    Span::new(0, u32::try_from(image.len()).unwrap()),
                    LexerConfig::from_grammar(profile.grammar),
                )
                .unwrap()
                .commands;
                for command in commands {
                    let captured = NativeCompilerWords::capture(
                        &command.words,
                        NativeStringProtocol::C(version),
                    )
                    .unwrap();
                    let name = std::str::from_utf8(captured.literal(1).unwrap()).unwrap();
                    let path = if channel == tcl_lexer::SourceChannel::Document {
                        "character-channel-source"
                    } else {
                        "counted-native-source"
                    };
                    let row = observations
                        .lines()
                        .filter(|row| !row.starts_with('#'))
                        .map(|row| row.split('\t').collect::<Vec<_>>())
                        .find(|row| {
                            row[0].starts_with(version.version_string())
                                && row[1] == path
                                && row[2] == name
                        })
                        .unwrap();
                    let expected = row[3]
                        .as_bytes()
                        .as_chunks::<2>()
                        .0
                        .iter()
                        .map(|pair| {
                            u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap()
                        })
                        .collect::<Vec<_>>();
                    assert_eq!(
                        captured.literal(2),
                        Some(expected.as_slice()),
                        "{version:?} {channel:?} {name}"
                    );
                    assert_eq!(command.words[2].image(), &image);
                    assert_eq!(captured.source_protocol(), NativeStringProtocol::C(version));
                }
            }
        }
    }

    #[test]
    fn byte_literals_remain_static_and_source_shapes_remain_original() {
        let original = words(b"list \xff\0tail \\u0000 $x {*}[get]");
        let captured =
            NativeCompilerWords::capture(&original, NativeStringProtocol::C(TclVersion::V9_0))
                .unwrap();
        assert_eq!(captured.literal(1), Some(b"\xff\0tail".as_slice()));
        assert_eq!(captured.literal(2), Some(b"\xc0\x80".as_slice()));
        assert_eq!(captured.literal(3), None);
        assert_eq!(
            captured.shapes(),
            &[
                NativeCompilationWordShape::Literal,
                NativeCompilationWordShape::Literal,
                NativeCompilationWordShape::BackslashLiteral,
                NativeCompilationWordShape::Substituted,
                NativeCompilationWordShape::Expanded,
            ]
        );
        assert_eq!(captured.original_words()[4].written_bytes(), b"{*}[get]");
    }

    #[test]
    fn original_literal_extents_preserve_grouping_escapes_and_channel_units() {
        // Implementation contract: naming.source.native-string-original-extents
        // docs/design/analysis/name-resolution-proofs/native-string-original-extents.md
        use tcl_syntax::naming::{NativeNameProtocol, NativeVariableInputForm};
        for source in [
            b"set ::N::v\\uD800(k) value".as_slice(),
            b"set \"::N::v\\uD800(k)\" value",
            b"set {::N::v\xed\xa0\x80(k)} value",
        ] {
            let original = words(source);
            let captured =
                NativeCompilerWords::capture(&original, NativeStringProtocol::C(TclVersion::V9_0))
                    .unwrap();
            let extent = NativeNameProtocol::C(TclVersion::V9_0)
                .variable_root_tail_extent(NativeVariableInputForm::Combined(
                    captured.literal(1).unwrap(),
                ))
                .unwrap();
            let span = captured.original_literal_extent(1, extent).unwrap();
            let expected = if source.contains(&b'\\') {
                b"v\\uD800".as_slice()
            } else {
                b"v\xed\xa0\x80".as_slice()
            };
            assert_eq!(
                original[1].image().bytes().get(span.as_range()),
                Some(expected)
            );
        }
        let original = words(b"set \\uD800 value");
        let captured =
            NativeCompilerWords::capture(&original, NativeStringProtocol::C(TclVersion::V9_0))
                .unwrap();
        assert_eq!(captured.literal(1), Some(b"\xed\xa0\x80".as_slice()));
        assert!(captured.original_literal_extent(1, 0..1).is_none());
        assert_eq!(
            original[1].image().bytes().get(
                captured
                    .original_literal_extent(1, 0..3)
                    .unwrap()
                    .as_range()
            ),
            Some(b"\\uD800".as_slice())
        );
        let original = words(b"set {a\\\n b} value");
        let captured =
            NativeCompilerWords::capture(&original, NativeStringProtocol::C(TclVersion::V9_0))
                .unwrap();
        assert!(captured.original_literal_extent(1, 0..1).is_none());

        let source = b"set {v\0tail} value";
        let image = SourceImage::document(std::str::from_utf8(source).unwrap());
        let original = native_script_words_in(
            image.clone(),
            Span::new(0, u32::try_from(image.len()).unwrap()),
            LexerConfig::from_grammar(tcl_dialect::DialectProfile::find("tcl9.0").unwrap().grammar),
        )
        .unwrap()
        .commands
        .remove(0)
        .words;
        let captured =
            NativeCompilerWords::capture(&original, NativeStringProtocol::C(TclVersion::V9_0))
                .unwrap();
        assert_eq!(captured.literal(1), Some(b"v\xc0\x80tail".as_slice()));
        assert!(captured.original_literal_extent(1, 1..2).is_none());
        assert_eq!(
            image.bytes().get(
                captured
                    .original_literal_extent(1, 1..3)
                    .unwrap()
                    .as_range()
            ),
            Some(b"\0".as_slice())
        );
    }

    #[test]
    fn shape_only_compiler_accepts_opaque_values_without_granting_other_grammars() {
        let original = words(b"list \xff\0tail");
        let captured =
            NativeCompilerWords::capture(&original, NativeStringProtocol::C(TclVersion::V9_0))
                .unwrap();
        let context = NativeCompilationContext {
            mode: NativeCompilationMode::BytecodeObject,
            frame: NativeCompilationFrame::ProcedureCode,
            ..Default::default()
        };
        let spec = NativeCompilationSpec {
            grammar: NativeCompilationGrammar::ArgumentList,
            operation: crate::SemanticOperationId::Intrinsic(crate::IntrinsicId::ListConstruct),
            body: NativeBodyCompilation::Inherit,
        };
        let dialect = Some(crate::InvocationDialect::for_version(TclVersion::V9_0));
        assert!(matches!(
            spec.select_native_words(&captured, 1, dialect, context),
            NativeCompilationSelection::Inline { .. }
        ));
        assert_eq!(
            NativeCompilationSpec {
                grammar: NativeCompilationGrammar::Return,
                ..spec
            }
            .select_native_words(&captured, 1, dialect, context),
            NativeCompilationSelection::Unknown
        );
        assert_eq!(
            spec.select_native_words(&captured, 0, dialect, context),
            NativeCompilationSelection::Unknown
        );
        let expanded = words(b"list {*}{A B}");
        let expanded = NativeCompilerWords::capture(&expanded, captured.source_protocol()).unwrap();
        assert_eq!(
            spec.select_native_words(&expanded, 1, dialect, context),
            NativeCompilationSelection::Unknown
        );
    }
    #[test]
    fn list_recipes_retain_version_frame_and_original_member_values() {
        let spec = NativeCompilationSpec {
            grammar: NativeCompilationGrammar::ArgumentList,
            operation: crate::SemanticOperationId::Intrinsic(crate::IntrinsicId::ListConstruct),
            body: NativeBodyCompilation::Inherit,
        };
        for version in [
            TclVersion::V8_4,
            TclVersion::V8_5,
            TclVersion::V8_6,
            TclVersion::V9_0,
            TclVersion::V9_1,
        ] {
            for source in [b"list".as_slice(), b"list A\xff {}", b"list $x"] {
                let original = words_for_version(source, version);
                let view =
                    NativeCompilerWords::capture(&original, NativeStringProtocol::C(version))
                        .unwrap();
                let recipe = view.list_recipe(1, version).unwrap();
                assert_eq!(
                    recipe,
                    if source == b"list" {
                        NativeCompiledListRecipe::EmptyString
                    } else if source == b"list $x" || version <= TclVersion::V8_5 {
                        NativeCompiledListRecipe::DynamicElements
                    } else {
                        NativeCompiledListRecipe::PrivateConstant {
                            members: vec![b"A\xff".to_vec(), vec![]],
                        }
                    }
                );
                for frame in [
                    NativeCompilationFrame::ScriptCode,
                    NativeCompilationFrame::ProcedureCode,
                    NativeCompilationFrame::Unknown,
                ] {
                    let selected = spec.select_native_words(
                        &view,
                        1,
                        Some(crate::InvocationDialect::for_version(version)),
                        NativeCompilationContext {
                            mode: NativeCompilationMode::BytecodeObject,
                            frame,
                            ..Default::default()
                        },
                    );
                    if version <= TclVersion::V8_5 && frame != NativeCompilationFrame::ProcedureCode
                    {
                        assert_eq!(
                            selected,
                            if frame == NativeCompilationFrame::Unknown {
                                NativeCompilationSelection::Unknown
                            } else {
                                NativeCompilationSelection::Generic
                            }
                        );
                    } else {
                        assert!(matches!(
                            selected,
                            NativeCompilationSelection::Inline { .. }
                        ));
                    }
                }
            }
        }
        let original = words(b"list {*}{A B}");
        let view =
            NativeCompilerWords::capture(&original, NativeStringProtocol::C(TclVersion::V9_1))
                .unwrap();
        assert_eq!(
            view.list_recipe(1, TclVersion::V9_1),
            Err(NativeCompiledListUnavailable::Expansion)
        );
        assert_eq!(
            view.list_recipe(0, TclVersion::V9_1),
            Err(NativeCompiledListUnavailable::OperandGeometry)
        );
    }

    #[test]
    fn increment_immediates_follow_original_token_and_native_getter_rules() {
        for version in [
            TclVersion::V8_4,
            TclVersion::V8_5,
            TclVersion::V8_6,
            TclVersion::V9_0,
            TclVersion::V9_1,
        ] {
            for (source, expected) in [
                (b"incr x -127".as_slice(), Some(-127)),
                (b"incr x 127", Some(127)),
                (b"incr x -128", None),
                (b"incr x 128", None),
                (b"incr x $amount", None),
                (b"incr x 1.0", None),
            ] {
                let original = words_for_version(source, version);
                let view =
                    NativeCompilerWords::capture(&original, NativeStringProtocol::C(version))
                        .unwrap();
                assert_eq!(
                    view.increment_immediate(2, version),
                    expected,
                    "{version:?} {source:?}"
                );
            }
            let original = words_for_version(b"incr x \\x31", version);
            let view =
                NativeCompilerWords::capture(&original, NativeStringProtocol::C(version)).unwrap();
            assert_eq!(
                view.increment_immediate(2, version),
                (version >= TclVersion::V9_1).then_some(1)
            );
            let original = words_for_version(b"incr x 4294967295", version);
            let view =
                NativeCompilerWords::capture(&original, NativeStringProtocol::C(version)).unwrap();
            assert_eq!(
                view.increment_immediate(2, version),
                (version <= TclVersion::V8_5).then_some(-1)
            );
        }
    }
}
