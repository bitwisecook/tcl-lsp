// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original executable words in a region of one retained source image.

use crate::{
    LeadingBom, Lexer, LexerConfig, NativeWord, NativeWordError, NestedBodyCutMemo, ParseCut,
    ParseCutUnavailable, SourceImage, SourceSegmentationPurpose, Span, Token,
    first_parse_cut_image_in_checked, group_commands_bytes,
    parse_cut::first_parse_cut_region_in_checked_with_memo,
};

/// Complete command words retaining the original whole-image address space.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeScriptCommandWords {
    /// Full original command extent, including the last word's closing delimiter.
    pub span: Span,
    /// Original terminator-delimited source extent, including trailing trivia.
    /// Absent for a vector assembled without its containing lexical region.
    /// This supplies no native frame, execution or command-log authority.
    pub source_extent: Option<Span>,
    /// Exact original `Eol` token that terminates this command, when the
    /// selected source region contains one. EOF-terminated commands have no
    /// token; synthetic vectors without a containing region also use `None`.
    /// The span includes the source delimiter and grants no execution fact.
    pub terminator: Option<Span>,
    /// Original lexical words and their authoritative executable arenas.
    pub words: Vec<NativeWord>,
}

/// An authentic malformed command after the complete-command prefix.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeScriptWordCut {
    /// Original whole-image offset where the malformed command starts.
    pub command_start: u32,
    /// Native parse message and whole-image offsets. `command` is the ordinal
    /// within the selected region, not within a containing script.
    pub cut: ParseCut,
}

/// Checked original command-at-a-time lexical plan for one source region.
/// This grants no selected command, compiler hook, variable frame or effects.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeScriptWordsPlan {
    source: SourceImage,
    config: LexerConfig,
    parse_config: LexerConfig,
    checked_words: std::sync::Arc<[CheckedWordGeometry]>,
    /// Complete commands that precede the malformed tail, or every command.
    pub commands: Vec<NativeScriptCommandWords>,
    /// Authentic guest syntax failure, separate from unavailable ownership.
    pub fatal_tail: Option<NativeScriptWordCut>,
}

// Sealed at construction from the recursively checked positive prefix. Public
// commands/fatal_tail may be edited by callers, so they cannot issue this proof.
#[derive(Debug, Clone, PartialEq, Eq)]
struct CheckedWordGeometry {
    group: crate::WordSpan,
    tokens: Box<[Token]>,
}
#[derive(Clone, Copy)]
enum SyntaxValidation {
    InspectRecursively,
    InheritedCommand { parse_config: LexerConfig },
}

impl NativeScriptWordsPlan {
    /// Borrow the original whole source image and its input channel, including
    /// plans with no complete commands. This grants no execution authority.
    #[must_use]
    pub const fn source(&self) -> &SourceImage {
        &self.source
    }
    /// Reborrow a real command-substitution body of an original word in this
    /// plan's recursively checked positive prefix. Every Command part, including
    /// variable-index substitutions, remains eligible; arbitrary braced bodies
    /// and spans are not. This is syntax only, with no Native entry or execution.
    ///
    /// # Errors
    /// Refuses foreign image/channel/config/word/part geometry, or unavailable
    /// inherited lexical ownership. The complete parent configuration remains
    /// on child words; the local parser purpose is explicitly `NestedScript`.
    pub fn reborrow_command_substitution_plan(
        &self,
        word: &NativeWord,
        body: Span,
    ) -> Result<Self, NativeScriptWordsUnavailable> {
        if word.image() != &self.source || word.config() != self.config
            || !self.checked_words.iter().any(|checked| checked.group == *word.group() && checked.tokens.as_ref() == word.tokens())
            || body.start() < word.span().start() || body.end() > word.span().end()
            || !word.executable_parts().all_parts().any(|part| matches!(part.part, crate::ExecutablePart::Command { body: actual } if actual == body))
        { return Err(NativeScriptWordsUnavailable::SourceGeometry); }
        capture_native_script_words(
            self.source.clone(),
            body,
            self.config,
            SourceSegmentationPurpose::NestedScript,
            SyntaxValidation::InheritedCommand {
                parse_config: SourceSegmentationPurpose::NestedScript
                    .config(self.parse_config)
                    .normalized(),
            },
            None,
        )
    }
}

/// Unavailable lexical ownership, never an invented guest syntax failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeScriptWordsUnavailable {
    /// The region or a rebased component has no valid whole-image extent.
    SourceGeometry,
    /// The shared checked parse-cut owner could not inspect the source.
    ParseOwnership(ParseCutUnavailable),
    /// A complete command's original word could not be retained.
    Word(NativeWordError),
}

/// Lex one genuine source region, retaining whole-image token spans.
/// The selected grammar applies to the same original bytes; an inner region's
/// leading BOM is source content. This supplies syntax geometry only, with no
/// selected command, parse-completion, compiler, frame or execution authority.
///
/// # Errors
/// Returns unavailable source geometry or a selected-grammar lexical failure.
pub fn source_region_tokens_in(
    image: &SourceImage,
    region: Span,
    config: LexerConfig,
) -> Result<Vec<Token>, NativeScriptWordsUnavailable> {
    // naming.cli.original-source-highlight-geometry
    // docs/design/analysis/name-resolution-proofs/cli-original-source-highlight-geometry.md
    let (_, _, mut tokens) =
        local_region_tokens(image, region, config, SourceSegmentationPurpose::WholeInput)?;
    for token in &mut tokens {
        token.span = rebase_span(token.span, region)?;
    }
    Ok(tokens)
}

fn local_region_tokens(
    image: &SourceImage,
    region: Span,
    config: LexerConfig,
    purpose: SourceSegmentationPurpose,
) -> Result<(SourceImage, LexerConfig, Vec<Token>), NativeScriptWordsUnavailable> {
    let original = image
        .bytes()
        .get(region.as_range())
        .ok_or(NativeScriptWordsUnavailable::SourceGeometry)?;
    let local = SourceImage::from_bytes(original, image.channel());
    let mut config = purpose.config(config).normalized();
    if region.start() != 0 {
        config.leading_bom = LeadingBom::Content;
    }
    let tokens = Lexer::with_source_image(&local, config)
        .tokenise_all()
        .map_err(|error| {
            NativeScriptWordsUnavailable::ParseOwnership(ParseCutUnavailable::LexicalStream(error))
        })?;
    Ok((local, config, tokens))
}

/// Capture original words in `region` with global spans in the same `image`.
/// The region is already selected by a lexical owner, such as a bracket body's
/// span. Tokens are grouped in region-local coordinates, then rebased once;
/// every returned word retains the original full image and its input channel.
/// No child source is escaped, normalised, or converted through Unicode.
/// Native compilation/admission is a separate obligation of the consumer.
///
/// # Errors
/// Returns unavailable source/word ownership. Authentic malformed syntax is
/// returned in `fatal_tail`, after the complete-command prefix.
pub fn native_script_words_in(
    image: SourceImage,
    region: Span,
    config: LexerConfig,
) -> Result<NativeScriptWordsPlan, NativeScriptWordsUnavailable> {
    native_script_words_in_with_purpose(
        image,
        region,
        config,
        SourceSegmentationPurpose::WholeInput,
    )
}

/// Capture words with an explicitly issued local script purpose while every
/// returned word retains the original image and complete parent configuration.
/// A body at byte zero is still a body; its coordinate supplies no prologue or
/// execution authority. Region geometry and checked parse failures are unchanged.
///
/// # Errors
/// Returns the same unavailable geometry/word ownership as the whole-input view.
pub fn native_script_words_in_with_purpose(
    image: SourceImage,
    region: Span,
    config: LexerConfig,
    purpose: SourceSegmentationPurpose,
) -> Result<NativeScriptWordsPlan, NativeScriptWordsUnavailable> {
    capture_native_script_words(
        image,
        region,
        config,
        purpose,
        SyntaxValidation::InspectRecursively,
        None,
    )
}

/// [`native_script_words_in_with_purpose`] sharing clean nested command-body
/// verdicts of the same whole image through `memo`. Validation is still the
/// full recursive walk; only a body the same walk already completed clean
/// under the identical whole-image extent and configuration is not re-lexed.
///
/// # Errors
/// Returns the same unavailable geometry/word ownership as the unmemoised view.
pub fn native_script_words_in_with_purpose_and_memo(
    image: SourceImage,
    region: Span,
    config: LexerConfig,
    purpose: SourceSegmentationPurpose,
    memo: &NestedBodyCutMemo,
) -> Result<NativeScriptWordsPlan, NativeScriptWordsUnavailable> {
    capture_native_script_words(
        image,
        region,
        config,
        purpose,
        SyntaxValidation::InspectRecursively,
        Some(memo),
    )
}

fn capture_native_script_words(
    image: SourceImage,
    region: Span,
    config: LexerConfig,
    purpose: SourceSegmentationPurpose,
    validation: SyntaxValidation,
    memo: Option<&NestedBodyCutMemo>,
) -> Result<NativeScriptWordsPlan, NativeScriptWordsUnavailable> {
    let original = image
        .bytes()
        .get(region.as_range())
        .ok_or(NativeScriptWordsUnavailable::SourceGeometry)?;
    let (local, parse_config, mut tokens) = local_region_tokens(
        &image,
        region,
        LexerConfig {
            strict_quoting: false,
            ..config
        },
        purpose,
    )?;
    let groups = group_commands_bytes(&tokens, local.bytes(), parse_config);
    let cut = match validation {
        SyntaxValidation::InspectRecursively => match memo {
            Some(memo) => first_parse_cut_region_in_checked_with_memo(
                &groups,
                &tokens,
                &local,
                parse_config,
                &image,
                region.start(),
                memo,
            ),
            None => first_parse_cut_image_in_checked(&groups, &tokens, &local, parse_config),
        }
        .map_err(NativeScriptWordsUnavailable::ParseOwnership)?,
        SyntaxValidation::InheritedCommand {
            parse_config: expected,
        } => {
            if parse_config != expected {
                return Err(NativeScriptWordsUnavailable::SourceGeometry);
            }
            None
        }
    };
    let complete_count = cut.map_or(groups.len(), |cut| cut.command);
    let fatal_tail = cut
        .map(|cut| rebase_fatal_tail(cut, &groups, region))
        .transpose()?;
    let source_extents = groups
        .iter()
        .take(complete_count)
        .map(|command| rebase_source_extent(command, &tokens, original.len(), region))
        .collect::<Result<Vec<_>, _>>()?;
    let terminators = groups
        .iter()
        .take(complete_count)
        .map(|command| rebase_terminator(command, &tokens, region))
        .collect::<Result<Vec<_>, _>>()?;
    for token in &mut tokens {
        token.span = rebase_span(token.span, region)?;
    }
    let mut commands = Vec::with_capacity(complete_count);
    for ((command, source_extent), terminator) in groups
        .iter()
        .take(complete_count)
        .zip(source_extents)
        .zip(terminators)
    {
        let words = command
            .words
            .iter()
            .map(|word| {
                let mut word = word.clone();
                word.span = rebase_span(word.span, region)?;
                NativeWord::from_group(image.clone(), config, &tokens, &word)
                    .map_err(NativeScriptWordsUnavailable::Word)
            })
            .collect::<Result<Vec<_>, _>>()?;
        let start = rebase_offset(command.span.start(), region)?;
        let end = words.last().map_or(start, |word| word.span().end());
        commands.push(NativeScriptCommandWords {
            span: Span::new(start, end),
            source_extent: Some(source_extent),
            terminator,
            words,
        });
    }
    let checked_words = commands
        .iter()
        .flat_map(|command| &command.words)
        .map(|word| CheckedWordGeometry {
            group: word.group().clone(),
            tokens: word.tokens().into(),
        })
        .collect::<Vec<_>>()
        .into();
    Ok(NativeScriptWordsPlan {
        source: image,
        config,
        parse_config,
        checked_words,
        commands,
        fatal_tail,
    })
}

/// Rebase an authentic malformed command after the complete prefix.
fn rebase_fatal_tail(
    mut cut: ParseCut,
    groups: &[crate::CommandSpan],
    region: Span,
) -> Result<NativeScriptWordCut, NativeScriptWordsUnavailable> {
    let command = groups
        .get(cut.command)
        .ok_or(NativeScriptWordsUnavailable::SourceGeometry)?;
    let command_start = rebase_offset(command.span.start(), region)?;
    cut.offset = rebase_offset(cut.offset, region)?;
    cut.term = rebase_offset(cut.term, region)?;
    Ok(NativeScriptWordCut { command_start, cut })
}

/// Rebase a command's terminator-delimited source extent, including trivia.
fn rebase_source_extent(
    command: &crate::CommandSpan,
    tokens: &[Token],
    source_len: usize,
    region: Span,
) -> Result<Span, NativeScriptWordsUnavailable> {
    let extent = command
        .source_extent(tokens, source_len)
        .ok_or(NativeScriptWordsUnavailable::SourceGeometry)?;
    rebase_span(extent, region)
}

/// Rebase a command's original `Eol` terminator token, when it has one.
fn rebase_terminator(
    command: &crate::CommandSpan,
    tokens: &[Token],
    region: Span,
) -> Result<Option<Span>, NativeScriptWordsUnavailable> {
    command
        .terminator
        .map(|index| {
            let token = tokens
                .get(index)
                .filter(|token| token.kind == crate::TokenType::Eol)
                .ok_or(NativeScriptWordsUnavailable::SourceGeometry)?;
            rebase_span(token.span, region)
        })
        .transpose()
}

fn rebase_offset(local: u32, region: Span) -> Result<u32, NativeScriptWordsUnavailable> {
    if local > region.end() - region.start() {
        return Err(NativeScriptWordsUnavailable::SourceGeometry);
    }
    region
        .start()
        .checked_add(local)
        .ok_or(NativeScriptWordsUnavailable::SourceGeometry)
}

fn rebase_span(local: Span, region: Span) -> Result<Span, NativeScriptWordsUnavailable> {
    Ok(Span::new(
        rebase_offset(local.start(), region)?,
        rebase_offset(local.end(), region)?,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn original_region_tokens_keep_global_offsets_and_opaque_source_bytes() {
        // naming.cli.original-source-highlight-geometry
        // docs/design/analysis/name-resolution-proofs/cli-original-source-highlight-geometry.md
        let image =
            SourceImage::native(b"prefix[puts \xff\0; # comment\nset x $value]suffix".as_slice());
        let end =
            u32::try_from(image.bytes().iter().position(|byte| *byte == b']').unwrap()).unwrap();
        let region = Span::new(7, end);
        let tokens = source_region_tokens_in(
            &image,
            region,
            LexerConfig {
                base_offset: 500,
                ..LexerConfig::default()
            },
        )
        .unwrap();
        assert_eq!(tokens[0].span, Span::new(7, 11));
        assert_eq!(&image.bytes()[tokens[0].span.as_range()], b"puts");
        assert!(tokens.iter().any(|token| {
            token.kind == crate::TokenType::Comment
                && &image.bytes()[token.span.as_range()] == b"# comment"
        }));
        assert!(
            tokens
                .iter()
                .any(|token| &image.bytes()[token.span.as_range()] == b"\xff\0")
        );
        assert!(
            tokens
                .iter()
                .all(|token| region.start() <= token.span.start()
                    && token.span.end() <= region.end())
        );
        assert!(
            source_region_tokens_in(&image, Span::new(0, end + 99), LexerConfig::default())
                .is_err()
        );
    }

    #[test]
    fn empty_and_fatal_plans_retain_the_original_image_without_word_owners() {
        for channel in [
            crate::SourceChannel::NativeValue,
            crate::SourceChannel::Document,
        ] {
            for bytes in [&b""[..], &b"set x {"[..]] {
                let image = SourceImage::from_bytes(bytes, channel);
                let owner = image.shared_bytes();
                let plan = native_script_words_in(
                    image,
                    Span::new(0, u32::try_from(bytes.len()).unwrap()),
                    LexerConfig::default(),
                )
                .unwrap();
                assert_eq!(plan.commands, [] as [NativeScriptCommandWords; 0]);
                assert_eq!(plan.fatal_tail.is_some(), !bytes.is_empty());
                assert_eq!(plan.source().bytes(), bytes);
                assert_eq!(plan.source().channel(), channel);
                assert!(std::sync::Arc::ptr_eq(
                    &owner,
                    &plan.source().shared_bytes()
                ));
            }
        }
    }

    #[test]
    fn original_command_extent_rebases_inside_its_genuine_region() {
        // naming.runtime.original-command-source-extent
        // docs/design/analysis/name-resolution-proofs/runtime-original-command-source-extent.md
        let image =
            SourceImage::native(b"outer [  error {marker}   ; list \"done\" \t] tail".as_slice());
        let region = Span::new(
            7,
            u32::try_from(image.bytes().iter().position(|byte| *byte == b']').unwrap()).unwrap(),
        );
        let plan = native_script_words_in(image.clone(), region, LexerConfig::default()).unwrap();
        assert!(plan.fatal_tail.is_none());
        assert_eq!(plan.commands.len(), 2);
        for (command, words, source) in [
            (
                &plan.commands[0],
                &b"error {marker}"[..],
                &b"error {marker}   "[..],
            ),
            (
                &plan.commands[1],
                &b"list \"done\""[..],
                &b"list \"done\" \t"[..],
            ),
        ] {
            assert_eq!(&image.bytes()[command.span.as_range()], words);
            let extent = command.source_extent.unwrap();
            assert_eq!(&image.bytes()[extent.as_range()], source);
            assert!(region.start() <= extent.start() && extent.end() <= region.end());
            assert!(command.words.iter().all(|word| word.image() == &image));
        }
    }

    #[test]
    fn bracket_region_retains_whole_image_bytes_and_complete_prefix() {
        let image = SourceImage::native(b"outer [echo \xff\0tail; set y {a}b] suffix".as_slice());
        let start = 7;
        let end = u32::try_from(
            image
                .bytes()
                .iter()
                .rposition(|byte| *byte == b']')
                .unwrap(),
        )
        .unwrap();
        let plan =
            native_script_words_in(image.clone(), Span::new(start, end), LexerConfig::default())
                .unwrap();
        assert_eq!(plan.commands.len(), 1);
        assert_eq!(plan.commands[0].span.start(), start);
        assert_eq!(plan.commands[0].words[1].bytes(), b"\xff\0tail");
        assert_eq!(plan.commands[0].words[1].image(), &image);
        let tail = plan.fatal_tail.unwrap();
        assert_eq!(tail.command_start, 20);
        assert_eq!(image.bytes()[tail.cut.term as usize], b'b');
        assert_eq!(tail.cut.message, crate::EXTRA_AFTER_CLOSE_BRACE);
    }

    #[test]
    fn child_region_keeps_expansion_closers_and_bom_as_data() {
        let image = SourceImage::native(b"[\xef\xbb\xbf {*}[list \xff]]".as_slice());
        let config = LexerConfig {
            leading_bom: LeadingBom::Skip,
            ..LexerConfig::default()
        };
        let plan = native_script_words_in(
            image.clone(),
            Span::new(1, u32::try_from(image.len()).unwrap() - 1),
            config,
        )
        .unwrap();
        assert!(plan.fatal_tail.is_none());
        let words = &plan.commands[0].words;
        assert_eq!(words[0].bytes(), b"\xef\xbb\xbf");
        assert_eq!(words[1].written_bytes(), b"{*}[list \xff]");
        assert!(words[1].group().expand);
    }
}

#[cfg(test)]
mod original_script_purpose_tests {
    use super::*;

    #[test]
    fn original_script_purpose_keeps_parent_words_at_zero_and_body_offsets() {
        // naming.source.original-configured-script-completeness
        // docs/design/analysis/name-resolution-proofs/source-original-configured-script-completeness.md
        // Source geometry and parent configuration only; no Native entry grant.
        let config = LexerConfig {
            leading_bom: LeadingBom::Skip,
            strict_quoting: true,
            braced_var: crate::BracedVarStyle::FirstClose,
            base_offset: 77,
            ..LexerConfig::default()
        };
        let image = SourceImage::document("\u{feff}puts ${a{b}");
        let region = Span::new(0, u32::try_from(image.len()).unwrap());
        let whole = native_script_words_in(image.clone(), region, config).unwrap();
        let body = native_script_words_in_with_purpose(
            image.clone(),
            region,
            config,
            SourceSegmentationPurpose::NestedScript,
        )
        .unwrap();
        assert!(whole.fatal_tail.is_none());
        assert!(body.fatal_tail.is_none());
        assert_eq!(whole.commands.len(), 1);
        assert_eq!(body.commands.len(), 1);
        assert_eq!(whole.commands[0].words[0].bytes(), b"puts");
        assert_eq!(whole.commands[0].span.start(), 3);
        assert_eq!(body.commands[0].words[0].bytes(), "\u{feff}puts".as_bytes());
        assert_eq!(body.commands[0].span.start(), 0);
        assert_ne!(whole.commands[0].words, body.commands[0].words);
        for plan in [&whole, &body] {
            assert_eq!(plan.commands[0].words.len(), 2);
            assert_eq!(plan.commands[0].words[1].bytes(), b"${a{b}");
            for word in &plan.commands[0].words {
                assert_eq!(word.image(), &image);
                assert_eq!(word.config(), config);
            }
        }
        let parent = SourceImage::document("eval {\u{feff}puts value}");
        let parent_plan = native_script_words_in(
            parent.clone(),
            Span::new(0, u32::try_from(parent.len()).unwrap()),
            config,
        )
        .unwrap();
        let body_region = parent_plan.commands[0].words[1].content_span().unwrap();
        let child = native_script_words_in_with_purpose(
            parent.clone(),
            body_region,
            config,
            SourceSegmentationPurpose::NestedScript,
        )
        .unwrap();
        assert!(child.fatal_tail.is_none());
        assert_eq!(child.commands.len(), 1);
        assert_eq!(child.commands[0].span.start(), body_region.start());
        assert_eq!(
            child.commands[0].words[0].bytes(),
            "\u{feff}puts".as_bytes()
        );
        assert_eq!(child.commands[0].words[1].bytes(), b"value");
        for word in &child.commands[0].words {
            assert_eq!(word.image(), &parent);
            assert_eq!(word.config(), config);
        }
        assert!(matches!(
            native_script_words_in_with_purpose(
                parent.clone(),
                Span::new(0, u32::try_from(parent.len()).unwrap() + 1),
                config,
                SourceSegmentationPurpose::NestedScript,
            ),
            Err(NativeScriptWordsUnavailable::SourceGeometry)
        ));
        let content = LexerConfig {
            leading_bom: LeadingBom::Content,
            ..config
        };
        assert_eq!(
            native_script_words_in(image, region, content)
                .unwrap()
                .commands[0]
                .words[0]
                .bytes(),
            "\u{feff}puts".as_bytes()
        );
    }
}

#[cfg(test)]
mod checked_substitution_reborrow_tests {
    use super::*;

    fn plan(image: SourceImage, config: LexerConfig) -> NativeScriptWordsPlan {
        let end = u32::try_from(image.bytes().len()).unwrap();
        native_script_words_in(image, Span::new(0, end), config).unwrap()
    }

    #[test]
    fn checked_child_matches_recursive_capture_in_both_original_channels() {
        for channel in [
            crate::SourceChannel::Document,
            crate::SourceChannel::NativeValue,
        ] {
            let image = SourceImage::from_bytes(b"set x [list [list value]]".as_slice(), channel);
            let config = LexerConfig {
                base_offset: 700,
                strict_quoting: true,
                ..LexerConfig::default()
            };
            let parent = plan(image.clone(), config);
            let word = &parent.commands[0].words[2];
            let body = word
                .executable_parts()
                .all_parts()
                .find_map(|part| match part.part {
                    crate::ExecutablePart::Command { body } => Some(body),
                    _ => None,
                })
                .unwrap();
            let inherited = parent
                .reborrow_command_substitution_plan(word, body)
                .unwrap();
            let checked = native_script_words_in_with_purpose(
                image,
                body,
                config,
                SourceSegmentationPurpose::NestedScript,
            )
            .unwrap();
            assert_eq!(inherited, checked);
            assert!(
                inherited
                    .commands
                    .iter()
                    .flat_map(|command| &command.words)
                    .all(|word| word.config() == config)
            );
            let nested_word = &inherited.commands[0].words[1];
            let nested_body = nested_word
                .executable_parts()
                .all_parts()
                .find_map(|part| match part.part {
                    crate::ExecutablePart::Command { body } => Some(body),
                    _ => None,
                })
                .unwrap();
            assert_eq!(
                inherited
                    .reborrow_command_substitution_plan(nested_word, nested_body)
                    .unwrap(),
                native_script_words_in_with_purpose(
                    inherited.source().clone(),
                    nested_body,
                    config,
                    SourceSegmentationPurpose::NestedScript
                )
                .unwrap()
            );
        }
    }

    #[test]
    fn variable_index_substitution_is_in_the_actual_all_parts_inventory() {
        let parent = plan(
            SourceImage::document("set x $arr([list value])"),
            LexerConfig::default(),
        );
        let word = &parent.commands[0].words[2];
        let body = word
            .executable_parts()
            .all_parts()
            .find_map(|part| match part.part {
                crate::ExecutablePart::Command { body } => Some(body),
                _ => None,
            })
            .unwrap();
        let child = parent
            .reborrow_command_substitution_plan(word, body)
            .unwrap();
        assert_eq!(
            &child.source().bytes()[child.commands[0].span.as_range()],
            b"list value"
        );
    }

    #[test]
    fn inert_body_and_foreign_word_cannot_inherit_recursive_validation() {
        let parent = plan(
            SourceImage::document("set x {[list value]}"),
            LexerConfig::default(),
        );
        let word = &parent.commands[0].words[2];
        assert!(
            parent
                .reborrow_command_substitution_plan(word, Span::new(8, 18))
                .is_err()
        );
        let parent = plan(
            SourceImage::document("set x [list value]"),
            LexerConfig::default(),
        );
        let word = &parent.commands[0].words[2];
        let body = word
            .executable_parts()
            .all_parts()
            .find_map(|part| match part.part {
                crate::ExecutablePart::Command { body } => Some(body),
                _ => None,
            })
            .unwrap();
        let foreign = plan(
            SourceImage::native(parent.source().bytes()),
            LexerConfig::default(),
        );
        assert!(
            parent
                .reborrow_command_substitution_plan(&foreign.commands[0].words[2], body)
                .is_err()
        );
        let other_config = plan(
            parent.source().clone(),
            LexerConfig {
                base_offset: 1,
                ..LexerConfig::default()
            },
        );
        assert!(
            parent
                .reborrow_command_substitution_plan(&other_config.commands[0].words[2], body)
                .is_err()
        );
        let mut altered = parent.clone();
        altered.commands.clear();
        altered.fatal_tail = None;
        // Public presentation edits do not destroy a genuine immutable prefix proof.
        assert!(
            altered
                .reborrow_command_substitution_plan(word, body)
                .is_ok()
        );
        // Nor can inserting a foreign word manufacture one.
        altered.commands.push(foreign.commands[0].clone());
        assert!(
            altered
                .reborrow_command_substitution_plan(&altered.commands[0].words[2], body)
                .is_err()
        );
    }
    #[test]
    fn public_fatal_tail_edits_do_not_admit_an_unchecked_command() {
        let image = SourceImage::document("set good [list value]; set bad [list {a}b]");
        let config = LexerConfig::default();
        let mut parent = plan(image.clone(), config);
        assert!(parent.fatal_tail.is_some());
        let good = parent.commands[0].words[2].clone();
        let good_body = good
            .executable_parts()
            .all_parts()
            .find_map(|part| match part.part {
                crate::ExecutablePart::Command { body } => Some(body),
                _ => None,
            })
            .unwrap();
        assert!(
            parent
                .reborrow_command_substitution_plan(&good, good_body)
                .is_ok()
        );
        let tokens = Lexer::with_source_image(&image, config)
            .tokenise_all()
            .unwrap();
        let commands = group_commands_bytes(&tokens, image.bytes(), config);
        let unvalidated =
            NativeWord::from_group(image, config, &tokens, &commands[1].words[2]).unwrap();
        let bad_body = unvalidated
            .executable_parts()
            .all_parts()
            .find_map(|part| match part.part {
                crate::ExecutablePart::Command { body } => Some(body),
                _ => None,
            })
            .unwrap();
        parent.fatal_tail = None;
        parent.commands[0].words.push(unvalidated.clone());
        assert!(
            parent
                .reborrow_command_substitution_plan(&unvalidated, bad_body)
                .is_err()
        );
    }
}
