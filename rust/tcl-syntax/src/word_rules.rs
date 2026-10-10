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

//! The dialect's answer to "what does this *word* mean as a value" — the
//! single owner of the two axes every layer needs and none should re-derive.
//!
//! [`LexerGrammar`] states the axes; [`crate::list`] and [`crate::backslash`]
//! hold the algorithms. This module is the join: one type carrying both axes,
//! with the operations on it, so the lexer, the compiler, codegen, the runtime
//! and the tooling all get the *same* answer for the same document.
//!
//! It exists because they did not. `brace_backslash_newline` reached only
//! lowering's parameter/variable-list helpers while `Codegen::push_lit_verbatim`,
//! the taint walker and the signature scanner each called the unconditional
//! collapse; `list_parse` reached nothing at all while the VM's list
//! conversions called the strict splitter. Three consumers, three answers to a
//! question the dialect owns — the same shape
//! [`tcl_registry::CommandSpec::return_type_for_call`] ends for per-call
//! result types. A consumer
//! that needs either axis takes a `WordValueRules` and asks it; it does not
//! reach for [`crate::list::split_list`] or
//! [`crate::backslash::collapse_brace_continuations_str`] directly and decide
//! for itself.
//!
//! The axes travel together because the two questions are one question at
//! every call site that splits a word-shaped list: the brace rule decides what
//! bytes the word contains, and the list rule decides how those bytes divide.
//! Answering one per dialect and the other per hardcoded default is precisely
//! the bug this replaces.

mod source_list;
pub use source_list::{OriginalSourceListElement, original_static_word_list_elements};

use crate::list::ListError;
use std::borrow::Cow;
use tcl_dialect::{BraceBackslashNewline, LexerGrammar, ListParse};

/// The word-value rules of one dialect.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct WordValueRules {
    /// Whether a `\<newline>` inside a braced word folds to a space.
    pub brace: BraceBackslashNewline,
    /// Whether malformed list text raises or is split anyway.
    pub list: ListParse,
}

impl Default for WordValueRules {
    /// Every build of the Tcl core: fold the continuation, raise on malformed
    /// list text. A caller with no dialect in hand gets C Tcl, matching what
    /// the unconditional call sites do regardless of dialect.
    fn default() -> Self {
        Self::TCL
    }
}

/// If `word` is a single balanced brace group spanning the whole word
/// (`{` … its matching `}` at the final byte), return the inner slice.
///
/// The one owner of "is this word a braced literal", asked by both sides of
/// the compile: the VM's `subst_word` strips such a word's braces and returns
/// the content with *all* substitution suppressed, and codegen must decide the
/// same question the same way — for a braced `expr` operand, and before it
/// pushes any word the VM will read back.
///
/// The balance walk is the whole point, and the reason this is a function
/// rather than a two-byte test each caller writes for itself. `{` first and
/// `}` last is only the *necessary* condition: in `{}${z}` the leading brace
/// matches at byte 1, so the word is not a braced literal and stripping it
/// yields the unbalanced `}${z}`. Codegen's braced-`expr`-operand arm did
/// exactly that, and `switch -- "{}$z" …` raised `missing close-brace for
/// variable name` on a script both oracles run (its `expr` sibling ran a
/// `[cmd]` inside what it had already decided was a literal).
///
/// Dialect-blind by construction: a backslash escapes the next byte on every
/// ladder, and brace *balance* is not one of the axes [`WordValueRules`]
/// carries — the continuation collapse that is stays on
/// [`WordValueRules::collapse_braced_word`].
#[must_use]
pub fn whole_braced_word(word: &str) -> Option<&str> {
    whole_braced_word_bytes(word.as_bytes()).and_then(|inner| core::str::from_utf8(inner).ok())
}

/// Content of a balanced whole-word brace group in original byte source.
/// Delimiters and escapes are inspected without decoding opaque content.
#[must_use]
pub fn whole_braced_word_bytes(b: &[u8]) -> Option<&[u8]> {
    let n = b.len();
    if n < 2 || b[0] != b'{' || b[n - 1] != b'}' {
        return None;
    }
    let mut depth = 0usize;
    let mut i = 0usize;
    while i < n {
        match b[i] {
            b'\\' => {
                i += 2;
                continue;
            }
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                // A premature return to depth 0 means the leading `{` does not
                // match the trailing `}`, so this is not a whole-word literal.
                if depth == 0 && i != n - 1 {
                    return None;
                }
            }
            _ => {}
        }
        i += 1;
    }
    if depth == 0 { Some(&b[1..n - 1]) } else { None }
}

impl WordValueRules {
    /// Every build of the Tcl core, and the F5 fork.
    pub const TCL: Self = Self {
        brace: BraceBackslashNewline::Folds,
        list: ListParse::Strict,
    };

    /// `JimTcl`, every modelled release.
    pub const JIM: Self = Self {
        brace: BraceBackslashNewline::Literal,
        list: ListParse::Lenient,
    };

    /// The rules a compiled or pack-declared grammar states.
    #[must_use]
    pub fn from_grammar(grammar: &LexerGrammar) -> Self {
        Self {
            brace: grammar.brace_backslash_newline,
            list: grammar.list_parse,
        }
    }

    /// The rules a live lexer configuration carries — the form production
    /// callers use, since a document's config is what they already hold.
    #[must_use]
    pub fn from_config(config: &tcl_lexer::LexerConfig) -> Self {
        Self {
            brace: config.brace_backslash_newline,
            list: config.list_parse,
        }
    }

    /// The rules of a dialect named at the compile's entry point, or C Tcl
    /// when the caller has none.
    ///
    /// The counterpart of [`BraceBackslashNewline::of_dialect_name`] and the
    /// other axis constructors, and here for the same stated reason: "no
    /// dialect means the default rule" is written once, so a layer cannot
    /// answer differently and read the same bytes under a rule the document
    /// was not lexed with.
    #[must_use]
    pub fn of_dialect_name(name: Option<&str>) -> Self {
        Self::from_grammar(&tcl_dialect::grammar_of_dialect_name(name))
    }

    /// The rules of an already-resolved profile, or C Tcl when the caller has
    /// none — for the layers that carry an `Option<&DialectProfile>`.
    #[must_use]
    pub fn of_profile(profile: Option<&tcl_dialect::DialectProfile>) -> Self {
        profile.map_or(Self::TCL, |p| Self::from_grammar(&p.grammar))
    }

    /// Collapse a braced word's line continuations under this dialect.
    ///
    /// C Tcl folds `\<newline>` and any following blanks to one space; Jim
    /// keeps the bytes, deliberately, to preserve line numbers.
    #[must_use]
    pub fn collapse_braced_word(self, text: &str) -> Cow<'_, str> {
        crate::backslash::collapse_brace_continuations_str_for(text, self.brace)
    }

    /// Split list text under this dialect.
    ///
    /// `Lenient` never returns `Err` — `JimTcl`'s list parser does not raise —
    /// so a caller that has already established the dialect is Jim may
    /// `expect` on it; one that has not must handle the error as before.
    pub fn split_list(self, text: &str) -> Result<Vec<Cow<'_, str>>, ListError> {
        match self.list {
            ListParse::Strict => crate::list::split_list(text),
            ListParse::Lenient => Ok(crate::list::split_list_jim(text)),
        }
    }

    /// The **tolerant** element split under this dialect's list grammar —
    /// the best-effort sibling of [`Self::split_list`] for a fold or a
    /// scan that must still yield the elements *before* a malformed tail
    /// rather than nothing. Strict list parsing tolerates through
    /// [`crate::list::split_list_lenient`]; Jim's grammar is tolerant by construction
    /// ([`crate::list::split_list_jim`]), so the same axis chooses.
    #[must_use]
    pub fn split_list_tolerant(self, text: &str) -> Vec<Cow<'_, str>> {
        match self.list {
            ListParse::Strict => crate::list::split_list_lenient(text),
            ListParse::Lenient => crate::list::split_list_jim(text),
        }
    }

    /// The word-shaped-list helper: collapse the braced word, then split it,
    /// yielding owned names. `None` is "this dialect raises on this text",
    /// which a static consumer turns into a barrier and never into a guess.
    #[must_use]
    pub fn split_word_names(self, text: &str) -> Option<Vec<String>> {
        let collapsed = self.collapse_braced_word(text);
        self.split_list(&collapsed)
            .ok()
            .map(|v| v.into_iter().map(Cow::into_owned).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_dialect::model::{Family, grammar};

    /// The constants are what the compiled catalogue says, for every release
    /// on both ladders — so a consumer may use `WordValueRules::JIM` in a test
    /// without it drifting from the real grammar.
    #[test]
    fn the_constants_match_the_catalogue() {
        for &release in Family::Jim.releases() {
            assert_eq!(
                WordValueRules::from_grammar(&grammar(Family::Jim, release)),
                WordValueRules::JIM,
                "{release}"
            );
        }
        for &release in Family::Tcl.releases() {
            assert_eq!(
                WordValueRules::from_grammar(&grammar(Family::Tcl, release)),
                WordValueRules::TCL,
                "{release}"
            );
        }
    }

    /// A grammar and the config built from it must answer identically —
    /// the property that makes `from_config` safe for production callers.
    #[test]
    fn grammar_and_config_agree() {
        for family in [Family::Tcl, Family::Jim] {
            for &release in family.releases() {
                let g = grammar(family, release);
                assert_eq!(
                    WordValueRules::from_grammar(&g),
                    WordValueRules::from_config(&tcl_lexer::LexerConfig::from_grammar(g)),
                    "{family:?} {release}"
                );
            }
        }
    }

    /// `of_dialect_name` and `of_profile` agree for every dialect the legacy
    /// catalogue can name — and *cannot* agree for one it cannot.
    ///
    /// `jim` is not a catalogue profile: a grammar is a
    /// function of `(family, release, build)`, so an environment names the
    /// family and ladder instead, which means `of_profile` receives `None`
    /// for a Jim document and answers C Tcl. That asymmetry is a property of
    /// the `Option<&DialectProfile>` currency, not of this type, and it is why
    /// a layer should carry a resolved point rather than a profile handle.
    #[test]
    fn the_name_and_profile_constructors_agree_where_a_profile_exists() {
        for (name, expected) in [
            ("tcl8.6", WordValueRules::TCL),
            ("tcl9.0", WordValueRules::TCL),
            ("f5-irules", WordValueRules::TCL),
        ] {
            let profile = tcl_dialect::DialectProfile::find(name);
            assert!(profile.is_some(), "{name} is a catalogue profile");
            assert_eq!(
                WordValueRules::of_dialect_name(Some(name)),
                expected,
                "{name}"
            );
            assert_eq!(WordValueRules::of_profile(profile), expected, "{name}");
        }
        assert_eq!(WordValueRules::of_dialect_name(None), WordValueRules::TCL);
        assert_eq!(WordValueRules::of_profile(None), WordValueRules::TCL);
    }

    /// The name route sees Jim; the profile route structurally cannot.
    #[test]
    fn the_profile_route_cannot_name_jim() {
        assert!(tcl_dialect::DialectProfile::find("jim").is_none());
        assert_eq!(
            WordValueRules::of_dialect_name(Some("jim")),
            WordValueRules::JIM,
            "the name resolves through the environment model"
        );
    }

    /// The default is C Tcl, so a caller with no dialect behaves exactly as
    /// the unconditional helpers did.
    #[test]
    fn the_default_is_tcl() {
        assert_eq!(WordValueRules::default(), WordValueRules::TCL);
        assert_eq!(
            WordValueRules::default().collapse_braced_word("a\\\n  b"),
            "a b"
        );
        assert!(WordValueRules::default().split_list("a {b").is_err());
    }

    /// The two axes move together, and both are visible through one call.
    #[test]
    fn the_two_dialects_differ_on_both_axes() {
        let wrapped = "a b\\\nc";
        assert_eq!(
            WordValueRules::TCL.split_word_names(wrapped).unwrap(),
            vec!["a", "b", "c"]
        );
        assert_eq!(
            WordValueRules::JIM.split_word_names(wrapped).unwrap(),
            vec!["a", "b c"]
        );

        assert_eq!(WordValueRules::TCL.split_word_names("a {b"), None);
        assert_eq!(
            WordValueRules::JIM.split_word_names("a {b").unwrap(),
            vec!["a", "b"]
        );
    }

    #[test]
    fn whole_braced_bytes_preserve_opaque_counted_content() {
        assert_eq!(whole_braced_word_bytes(b"{\xff\0}"), Some(&b"\xff\0"[..]));
        assert_eq!(whole_braced_word_bytes(b"{\xff}tail"), None);
    }

    #[test]
    fn whole_braced_word_strips_a_balanced_whole_word_group() {
        assert_eq!(whole_braced_word("{abc}"), Some("abc"));
        assert_eq!(whole_braced_word("{}"), Some("")); // empty group
        assert_eq!(whole_braced_word("{a {b} c}"), Some("a {b} c"));
        // A brace the escape neutralises is content, not structure.
        assert_eq!(whole_braced_word(r"{a\}b}"), Some(r"a\}b"));
    }

    /// The condition the two-byte test cannot see, and the bug it caused: in
    /// `{}${z}` the leading brace closes at byte 1, so the word is not a braced
    /// literal — codegen stripped it anyway and produced the unbalanced
    /// `}${z}`, which the VM then refused (`switch -- "{}$z" …`).
    #[test]
    fn whole_braced_word_rejects_a_word_that_only_looks_delimited() {
        assert_eq!(whole_braced_word("{}${z}"), None);
        assert_eq!(whole_braced_word("{a} {b}"), None);
        assert_eq!(whole_braced_word("{}x{}"), None);
    }

    #[test]
    fn whole_braced_word_rejects_undelimited_or_unbalanced_words() {
        assert_eq!(whole_braced_word("abc"), None);
        assert_eq!(whole_braced_word("{abc"), None);
        assert_eq!(whole_braced_word("abc}"), None);
        assert_eq!(whole_braced_word("{{a}"), None);
        assert_eq!(whole_braced_word(""), None);
        assert_eq!(whole_braced_word("{"), None);
    }
}

/// Static ASCII metadata presentation from one authentic Document word.
/// This carries lexical facts only: no native name policy, value object, cell,
/// compiler eligibility or successful execution follows from these bytes.
#[must_use]
pub fn original_static_word_ascii_presentation(word: &tcl_lexer::NativeWord) -> Option<Vec<u8>> {
    let value = original_static_word_source_bytes(word)?;
    value.is_ascii().then_some(value)
}

/// Static source presentation bytes from an authentic Document word, using
/// the selected shared literal/escape/brace-continuation grammar. These
/// counted bytes supply no native object, lookup or successful evaluation.
#[must_use]
pub fn original_static_word_source_bytes(word: &tcl_lexer::NativeWord) -> Option<Vec<u8>> {
    use tcl_lexer::{ExecutablePart, ExecutableText, SourceChannel, WordKind};
    if word.image().channel() != SourceChannel::Document || word.group().expand {
        return None;
    }
    word.image().try_text().ok()?;
    let mut value = Vec::new();
    if word.group().kind == WordKind::Braced {
        let span = word.content_span().ok()?;
        let original = word.image().bytes().get(span.as_range())?;
        value.extend_from_slice(&crate::backslash::source_braced_word_bytes(
            original,
            SourceChannel::Document,
            word.config().brace_backslash_newline,
        ));
    } else {
        let arena = word.executable_parts();
        for component in arena.list(arena.root()) {
            match &component.part {
                ExecutablePart::Text(ExecutableText::Original) => {
                    value.extend_from_slice(&crate::backslash::source_literal_bytes(
                        arena.bytes(component.span)?,
                        SourceChannel::Document,
                    ));
                }
                ExecutablePart::Text(ExecutableText::Decoded(bytes)) => {
                    value.extend_from_slice(bytes);
                }
                _ => return None,
            }
        }
    }
    Some(value)
}

/// Static Unicode value of one complete original Document word. Interpreted
/// numeric escapes must retain Unicode scalar values before any presentation
/// conversion; an unrepresentable unit cannot become a replacement-character
/// name or literal fact. Braced escapes remain literal under the retained
/// grammar. This supplies no Native value, naming policy or evaluation proof.
#[must_use]
pub fn original_static_word_unicode_value(word: &tcl_lexer::NativeWord) -> Option<Vec<u8>> {
    use tcl_lexer::{ExecutablePart, ExecutableText, SourceChannel, WordKind};
    let value = original_static_word_source_bytes(word)?;
    if word.group().kind != WordKind::Braced {
        let arena = word.executable_parts();
        for component in arena.list(arena.root()) {
            if !matches!(
                component.part,
                ExecutablePart::Text(ExecutableText::Decoded(_))
            ) {
                continue;
            }
            let raw = arena.bytes(component.span)?;
            let mut offset = 0;
            while offset < raw.len() {
                if raw[offset] != b'\\' {
                    offset += 1;
                    continue;
                }
                let fragment = tcl_lexer::source_backslash_fragment_in(
                    raw,
                    offset,
                    SourceChannel::Document,
                    word.config().escapes,
                    |_| tcl_lexer::EscapedInputUnit {
                        width: 1,
                        value: tcl_lexer::EscapedInputValue::CopyOriginal,
                    },
                )?;
                if let tcl_lexer::BackslashFragmentValue::Codepoint(unit) = fragment.value {
                    let scalar = char::from_u32(unit)?;
                    let mut encoded = [0; 4];
                    if crate::backslash::decode_bytes_in(
                        raw.get(offset..fragment.end)?,
                        word.config().escapes,
                    )
                    .as_ref()
                        != scalar.encode_utf8(&mut encoded).as_bytes()
                    {
                        return None;
                    }
                }
                if fragment.end <= offset || fragment.end > raw.len() {
                    return None;
                }
                offset = fragment.end;
            }
        }
    }
    std::str::from_utf8(&value).ok()?;
    Some(value)
}

/// Exact original Document coordinate of a static ASCII presentation boundary.
/// Parsed literal and escape components retain their own source extents. A
/// boundary inside a scanner-selected escape result or across ambiguous source
/// extents is unavailable. Transformed braced content supplies no invented coordinate.
/// This selects no Native value, naming purpose, lookup or edit authority.
#[must_use]
pub fn original_static_word_ascii_source_offset(
    word: &tcl_lexer::NativeWord,
    offset: usize,
) -> Option<u32> {
    use tcl_lexer::{ExecutablePart, ExecutableText, SourceChannel, WordKind};
    // naming.core.original-inlay-retained-context
    // docs/design/analysis/name-resolution-proofs/original-inlay-retained-context.md
    let value = original_static_word_ascii_presentation(word)?;
    if offset > value.len() {
        return None;
    }
    if word.group().kind == WordKind::Braced || value.is_empty() {
        let content = word.content_span().ok()?;
        let raw = word.image().bytes().get(content.as_range())?;
        if raw != value {
            return None;
        }
        return content.start().checked_add(u32::try_from(offset).ok()?);
    }
    let arena = word.executable_parts();
    let mut produced = 0_usize;
    let mut selected = None;
    for component in arena.list(arena.root()) {
        let (length, literal) = match &component.part {
            ExecutablePart::Text(ExecutableText::Original) => {
                let raw = arena.bytes(component.span)?;
                if crate::backslash::source_literal_bytes(raw, SourceChannel::Document).as_ref()
                    != raw
                {
                    return None;
                }
                (raw.len(), true)
            }
            ExecutablePart::Text(ExecutableText::Decoded(bytes)) => (bytes.len(), false),
            _ => return None,
        };
        let end = produced.checked_add(length)?;
        if (produced..=end).contains(&offset) {
            let local = offset - produced;
            let source = if literal {
                component
                    .span
                    .start()
                    .checked_add(u32::try_from(local).ok()?)?
            } else {
                let decoded = arena.text(component)?;
                let raw = arena.bytes(component.span)?;
                let original = decoded_ascii_source_offset(raw, decoded, word.config(), local)?;
                component
                    .span
                    .start()
                    .checked_add(u32::try_from(original).ok()?)?
            };
            if selected.is_some_and(|previous| previous != source) {
                return None;
            }
            selected = Some(source);
        }
        produced = end;
    }
    (produced == value.len()).then_some(())?;
    selected
}

// Recheck the complete retained decoded component using the shared lexical
// fragment owner. Each ASCII result has one whole original input extent; this
// supplies source geometry without selecting a native string or name protocol.
fn decoded_ascii_source_offset(
    raw: &[u8],
    decoded: &[u8],
    config: tcl_lexer::LexerConfig,
    offset: usize,
) -> Option<usize> {
    use tcl_lexer::{BackslashFragmentValue, EscapedInputUnit, EscapedInputValue, SourceChannel};
    if !raw.is_ascii() || !decoded.is_ascii() || offset > decoded.len() {
        return None;
    }
    let mut value = Vec::with_capacity(decoded.len());
    let mut position = 0;
    let mut selected = (offset == 0).then_some(0);
    while position < raw.len() {
        let (end, byte) = if raw[position] == b'\\' {
            let fragment = tcl_lexer::source_backslash_fragment_in(
                raw,
                position,
                SourceChannel::Document,
                config.escapes,
                |_| EscapedInputUnit {
                    width: 1,
                    value: EscapedInputValue::CopyOriginal,
                },
            )?;
            let byte = match fragment.value {
                BackslashFragmentValue::Byte(byte) => byte,
                BackslashFragmentValue::Codepoint(value) => u8::try_from(value).ok()?,
                BackslashFragmentValue::Literal(range) => {
                    let [byte] = raw.get(range)? else {
                        return None;
                    };
                    *byte
                }
            };
            (fragment.end, byte)
        } else if raw[position] == b'\r' {
            (
                position + 1 + usize::from(raw.get(position + 1) == Some(&b'\n')),
                b'\n',
            )
        } else {
            (position + 1, raw[position])
        };
        if end <= position || end > raw.len() || !byte.is_ascii() {
            return None;
        }
        value.push(byte);
        if value.len() == offset {
            selected = Some(end);
        }
        position = end;
    }
    (value == decoded).then_some(selected).flatten()
}

/// Render explicitly lexical ASCII declaration advice under the complete
/// supplied grammar. No native name policy or callable publication is inferred.
#[must_use]
pub fn lexical_ascii_source_word(value: &str, config: tcl_lexer::LexerConfig) -> Option<String> {
    if !value.is_ascii() {
        return None;
    }
    let spelling = crate::list::list_element(value);
    let image = tcl_lexer::SourceImage::document(&spelling);
    let plan = tcl_lexer::native_script_words_in(
        image,
        tcl_lexer::Span::new(0, u32::try_from(spelling.len()).ok()?),
        config,
    )
    .ok()?;
    let [command] = plan.commands.as_slice() else {
        return None;
    };
    let [word] = command.words.as_slice() else {
        return None;
    };
    (original_static_word_ascii_presentation(word)?.as_slice() == value.as_bytes())
        .then_some(spelling)
}

#[cfg(test)]
mod original_metadata_tests {
    use super::*;
    use tcl_lexer::{LexerConfig, SourceImage, Span};

    fn presentation(source: &str, config: LexerConfig) -> Option<Vec<u8>> {
        let image = SourceImage::document(source);
        let plan = tcl_lexer::native_script_words_in(
            image,
            Span::new(0, u32::try_from(source.len()).unwrap()),
            config,
        )
        .unwrap();
        original_static_word_ascii_presentation(&plan.commands[0].words[0])
    }

    fn word(image: SourceImage, config: LexerConfig) -> tcl_lexer::NativeWord {
        let length = u32::try_from(image.len()).unwrap();
        let plan = tcl_lexer::native_script_words_in(image, Span::new(0, length), config).unwrap();
        plan.commands[0].words[0].clone()
    }

    #[test]
    fn static_unicode_values_keep_original_units_separate_from_replacement_presentation() {
        // naming.source.original-static-unicode-value-projection
        // docs/design/analysis/name-resolution-proofs/source-original-static-unicode-value-projection.md
        // This checked Document view does not classify Native string values.
        for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            let point = tcl_dialect::model::DialectPoint::of_dialect_name(Some(profile))
                .expect("supported source grammar");
            let config = LexerConfig::for_file_grammar(point.grammar());
            if profile == "jim" {
                assert_eq!(config.escapes, tcl_dialect::EscapeSyntax::Jim);
            }
            for (source, expected) in [
                ("café", "café"),
                (r"caf\u00e9", "café"),
                (r"café\uFFFD", "café�"),
                ("café�", "café�"),
                (r"{café\uD800}", r"café\uD800"),
                (r"café\\uD800", r"café\uD800"),
            ] {
                let original = word(SourceImage::document(source), config);
                assert_eq!(
                    original_static_word_unicode_value(&original).as_deref(),
                    Some(expected.as_bytes()),
                    "{profile}: {source}"
                );
            }
            for source in [r"café\uD800", r#""café\uD801""#, r"café\uDC00"] {
                let original = word(SourceImage::document(source), config);
                assert!(original_static_word_source_bytes(&original).is_some());
                assert!(
                    original_static_word_unicode_value(&original).is_none(),
                    "{profile}: {source}"
                );
            }
            for source in ["$name", "[name]"] {
                let original = word(SourceImage::document(source), config);
                assert!(original_static_word_unicode_value(&original).is_none());
            }
            if config.expand_syntax {
                let original = word(SourceImage::document("{*}{café}"), config);
                assert!(original.group().expand);
                assert!(original_static_word_unicode_value(&original).is_none());
            }
            let original = word(SourceImage::native("café".as_bytes()), config);
            assert!(original_static_word_unicode_value(&original).is_none());
        }
        let original = word(SourceImage::document(r"café\uD800"), LexerConfig::default());
        assert_eq!(
            original_static_word_source_bytes(&original).as_deref(),
            Some("café�".as_bytes())
        );
        assert!(original_static_word_unicode_value(&original).is_none());
    }

    #[test]
    fn static_ascii_boundaries_keep_original_literal_and_escape_components() {
        // naming.core.original-inlay-retained-context
        // docs/design/analysis/name-resolution-proofs/original-inlay-retained-context.md
        let config = LexerConfig::default();
        for (source, expected) in [
            ("%d", [0, 1, 2]),
            (r"\x25d", [0, 4, 5]),
            (r#""\x25d""#, [1, 5, 6]),
            ("{%d}", [1, 2, 3]),
        ] {
            let original = word(SourceImage::document(source), config);
            assert_eq!(
                original_static_word_ascii_presentation(&original).as_deref(),
                Some(b"%d".as_slice())
            );
            for (offset, expected) in expected.into_iter().enumerate() {
                assert_eq!(
                    original_static_word_ascii_source_offset(&original, offset),
                    Some(expected),
                    "{source}"
                );
            }
            assert!(original_static_word_ascii_source_offset(&original, 3).is_none());
        }
    }

    #[test]
    fn static_ascii_boundaries_keep_selected_grammar_and_refuse_other_value_domains() {
        // naming.core.original-inlay-retained-context
        // docs/design/analysis/name-resolution-proofs/original-inlay-retained-context.md
        for (profile, expected) in [("tcl8.5", 6), ("tcl8.6", 4)] {
            let config = LexerConfig::for_profile(tcl_dialect::DialectProfile::find(profile));
            let original = word(SourceImage::document(r"\x4142"), config);
            assert_eq!(
                original_static_word_ascii_source_offset(&original, 1),
                Some(expected),
                "{profile}"
            );
        }
        let config = LexerConfig::default();
        for image in [
            SourceImage::native(b"%d".as_slice()),
            SourceImage::document("$format"),
            SourceImage::document("[format]"),
            SourceImage::document("{*}{%d}"),
            SourceImage::document("%😀"),
            SourceImage::document("{a\\\n%d}"),
        ] {
            let original = word(image, config);
            assert!(original_static_word_ascii_source_offset(&original, 1).is_none());
        }
    }

    #[test]
    fn static_ascii_boundaries_keep_aggregated_escapes_and_original_literal_tails() {
        // naming.core.original-inlay-retained-context
        // docs/design/analysis/name-resolution-proofs/original-inlay-retained-context.md
        for profile in [
            "tcl8.4",
            "tcl8.5",
            "tcl8.6",
            "tcl9.0",
            "tcl9.1",
            "jim",
            "f5-irules",
        ] {
            let config = LexerConfig::for_profile(tcl_dialect::DialectProfile::find(profile));
            for (source, base) in [(r"\u003a\u003aformat", 0), (r#""\u003a\u003aformat""#, 1)] {
                let original = word(SourceImage::document(source), config);
                assert_eq!(
                    original_static_word_ascii_presentation(&original).as_deref(),
                    Some(b"::format".as_slice())
                );
                for (offset, expected) in [
                    (0, base),
                    (1, base + 6),
                    (2, base + 12),
                    (5, base + 15),
                    (8, base + 18),
                ] {
                    assert_eq!(
                        original_static_word_ascii_source_offset(&original, offset),
                        Some(expected),
                        "{profile}: {source}"
                    );
                }
            }
        }
        let original = word(
            SourceImage::document(r"{\u003a\u003aformat}"),
            LexerConfig::default(),
        );
        assert_eq!(
            original_static_word_ascii_presentation(&original).as_deref(),
            Some(br"\u003a\u003aformat".as_slice())
        );
        assert_eq!(
            original_static_word_ascii_source_offset(&original, 2),
            Some(3)
        );
    }

    #[test]
    fn decoded_ascii_boundaries_refuse_channel_mismatch_and_non_ascii_units() {
        // naming.core.original-inlay-retained-context
        // docs/design/analysis/name-resolution-proofs/original-inlay-retained-context.md
        let config = LexerConfig::default();
        assert_eq!(
            decoded_ascii_source_offset(br"\x25d", b"%d", config, 1),
            Some(4)
        );
        assert_eq!(
            decoded_ascii_source_offset(br"\x25d", b"%x", config, 1),
            None
        );
        assert_eq!(
            decoded_ascii_source_offset(b"a\\\r\n b", b"a\r\n b", config, 1),
            None
        );
        assert_eq!(
            decoded_ascii_source_offset(br"\u00e9", b"e", config, 0),
            None
        );
        assert_eq!(
            decoded_ascii_source_offset(br"\x25d", b"%d", config, 3),
            None
        );
    }

    #[test]
    fn lexical_metadata_word_uses_full_document_grammar_without_native_policy() {
        // Implementation contract: naming.grammar.static-ascii-metadata (docs/design/analysis/name-resolution-proofs/static-ascii-metadata.md).
        let config = LexerConfig::default();
        for source in ["trim", "{trim}", "\"trim\"", r"tr\x69m"] {
            assert_eq!(
                presentation(source, config).as_deref(),
                Some(b"trim".as_slice())
            );
        }
        assert_eq!(
            presentation("{a\\\n b}", config).as_deref(),
            Some(b"a b".as_slice())
        );
        let literal = LexerConfig {
            brace_backslash_newline: tcl_dialect::BraceBackslashNewline::Literal,
            ..config
        };
        assert_eq!(
            presentation("{a\\\n b}", literal).as_deref(),
            Some(b"a\\\n b".as_slice())
        );
        for source in ["$selector", "[selector]", "{*}{trim}", "tr😀m"] {
            assert!(presentation(source, config).is_none());
        }
        let image = SourceImage::native(b"trim".as_slice());
        let plan = tcl_lexer::native_script_words_in(image, Span::new(0, 4), config).unwrap();
        assert!(original_static_word_ascii_presentation(&plan.commands[0].words[0]).is_none());
    }
}
