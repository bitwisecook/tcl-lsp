// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original executable words in a region of one retained source image.

use crate::{
    LeadingBom, Lexer, LexerConfig, NativeWord, NativeWordError, ParseCut, ParseCutUnavailable,
    SourceImage, Span, Token, first_parse_cut_image_in_checked, group_commands_bytes,
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
    /// Complete commands that precede the malformed tail, or every command.
    pub commands: Vec<NativeScriptCommandWords>,
    /// Authentic guest syntax failure, separate from unavailable ownership.
    pub fatal_tail: Option<NativeScriptWordCut>,
}

impl NativeScriptWordsPlan {
    /// Borrow the original whole source image and its input channel, including
    /// plans with no complete commands. This grants no execution authority.
    #[must_use]
    pub const fn source(&self) -> &SourceImage {
        &self.source
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
    let (_, _, mut tokens) = local_region_tokens(image, region, config)?;
    for token in &mut tokens {
        token.span = rebase_span(token.span, region)?;
    }
    Ok(tokens)
}

fn local_region_tokens(
    image: &SourceImage,
    region: Span,
    config: LexerConfig,
) -> Result<(SourceImage, LexerConfig, Vec<Token>), NativeScriptWordsUnavailable> {
    let original = image
        .bytes()
        .get(region.as_range())
        .ok_or(NativeScriptWordsUnavailable::SourceGeometry)?;
    let local = SourceImage::from_bytes(original, image.channel());
    let mut config = config.normalized();
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
    )?;
    let groups = group_commands_bytes(&tokens, local.bytes(), parse_config);
    let cut = first_parse_cut_image_in_checked(&groups, &tokens, &local, parse_config)
        .map_err(NativeScriptWordsUnavailable::ParseOwnership)?;
    let complete_count = cut.map_or(groups.len(), |cut| cut.command);
    let fatal_tail = if let Some(mut cut) = cut {
        let command = groups
            .get(cut.command)
            .ok_or(NativeScriptWordsUnavailable::SourceGeometry)?;
        let command_start = rebase_offset(command.span.start(), region)?;
        cut.offset = rebase_offset(cut.offset, region)?;
        cut.term = rebase_offset(cut.term, region)?;
        Some(NativeScriptWordCut { command_start, cut })
    } else {
        None
    };
    let source_extents = groups
        .iter()
        .take(complete_count)
        .map(|command| {
            let extent = command
                .source_extent(&tokens, original.len())
                .ok_or(NativeScriptWordsUnavailable::SourceGeometry)?;
            rebase_span(extent, region)
        })
        .collect::<Result<Vec<_>, _>>()?;
    for token in &mut tokens {
        token.span = rebase_span(token.span, region)?;
    }
    let mut commands = Vec::with_capacity(complete_count);
    for (command, source_extent) in groups.iter().take(complete_count).zip(source_extents) {
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
            words,
        });
    }
    Ok(NativeScriptWordsPlan {
        source: image,
        commands,
        fatal_tail,
    })
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
