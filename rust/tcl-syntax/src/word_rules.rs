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

mod compiled_word;
pub use compiled_word::{CompiledWordSubstitution, compiled_word_substitution};
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
    original_word_unicode_value_with_variables(word, |_, _| None)
}

/// Source value shape in a variable-name argument, without a variable place.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OriginalVariableNameWordForm {
    /// A substitution-free Unicode source value. Element splitting describes
    /// this value only; it supplies no source boundary or runtime lookup.
    Static {
        /// Decoded source value, independently of Native string encoding.
        value: Vec<u8>,
        /// The shared value-name grammar partitions this as an element name.
        element: bool,
    },
    /// Possible evaluated array shape: literal root and delimiters surround
    /// dynamic index source. Values, effects, errors and name policy stay unknown.
    CompoundArray {
        /// Decoded literal root data, without namespace or frame lookup.
        root: Vec<u8>,
    },
    /// The source does not retain a literal scalar or array-root name.
    Dynamic,
}

/// Original source-display geometry for one variable-name operand.
/// The whole word retains its image/channel, grouping and full configuration;
/// literal extents always address original bytes, never cooked-value offsets.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalVariableNameWordGeometry {
    original: tcl_lexer::NativeWord,
    form: OriginalVariableNameWordForm,
    literals: Vec<tcl_lexer::Span>,
}

impl OriginalVariableNameWordGeometry {
    /// The unchanged original operand and its complete syntax owner.
    #[must_use]
    pub const fn original_word(&self) -> &tcl_lexer::NativeWord {
        &self.original
    }

    /// Source value form, without a selected variable, cell or operation.
    #[must_use]
    pub const fn form(&self) -> &OriginalVariableNameWordForm {
        &self.form
    }

    /// Contiguous original literal fragments of an admitted name shape.
    /// Substitution introducers and script/index children are excluded.
    #[must_use]
    pub fn literal_spans(&self) -> &[tcl_lexer::Span] {
        &self.literals
    }
}

/// Describe original Document syntax in a variable-name argument position.
/// The caller must independently retain that argument role and its source
/// context. This supplies no C compiler operand, name policy, physical cell,
/// selected handler, evaluated index, completed read or edit authority.
#[must_use]
pub fn original_variable_name_word_geometry(
    word: &tcl_lexer::NativeWord,
) -> Option<OriginalVariableNameWordGeometry> {
    use tcl_lexer::{ExecutablePart, SourceChannel, Span, WordKind};
    if word.image().channel() != SourceChannel::Document || word.group().expand {
        return None;
    }
    word.image().try_text().ok()?;
    let mut literals: Vec<Span> = Vec::new();
    let mut prefix = Vec::new();
    let mut suffix = Vec::new();
    let mut dynamic = false;
    if word.group().kind == WordKind::Braced {
        literals.push(word.content_span().ok()?);
    } else {
        let arena = word.executable_parts();
        for component in arena.list(arena.root()) {
            match &component.part {
                ExecutablePart::Text(_) => {
                    let value = original_unicode_text_component(arena, component)?;
                    if !dynamic {
                        prefix.extend_from_slice(&value);
                    }
                    suffix.extend_from_slice(&value);
                    if let Some(previous) = literals.last_mut()
                        && previous.end() == component.span.start()
                    {
                        *previous = Span::new(previous.start(), component.span.end());
                    } else {
                        literals.push(component.span);
                    }
                }
                ExecutablePart::Variable { .. } | ExecutablePart::Command { .. } => {
                    dynamic = true;
                    suffix.clear();
                }
                _ => return None,
            }
        }
    }
    let form = if !dynamic {
        let value = original_static_word_unicode_value(word)?;
        let element = crate::naming::split_element_ref_bytes(&value).is_some();
        OriginalVariableNameWordForm::Static { value, element }
    } else if suffix.last() == Some(&b')')
        && let Some(open) = prefix.iter().position(|byte| *byte == b'(')
    {
        OriginalVariableNameWordForm::CompoundArray {
            root: prefix[..open].to_vec(),
        }
    } else {
        literals.clear();
        OriginalVariableNameWordForm::Dynamic
    };
    Some(OriginalVariableNameWordGeometry {
        original: word.clone(),
        form,
        literals,
    })
}

/// Unicode source value with each substitution resolved from its original component.
/// The callback retains the complete image, grammar and scalar/array form;
/// unavailable commands, indices or values never become replacement text.
#[must_use]
pub fn original_word_unicode_value_with_variables(
    word: &tcl_lexer::NativeWord,
    mut variable: impl FnMut(
        &tcl_lexer::ExecutablePartArena,
        &tcl_lexer::SpannedExecutablePart,
    ) -> Option<Vec<u8>>,
) -> Option<Vec<u8>> {
    use tcl_lexer::{ExecutablePart, SourceChannel, WordKind};
    if word.image().channel() != SourceChannel::Document || word.group().expand {
        return None;
    }
    word.image().try_text().ok()?;
    let mut value = Vec::new();
    if word.group().kind == WordKind::Braced {
        value = original_static_word_source_bytes(word)?;
    } else {
        let arena = word.executable_parts();
        for component in arena.list(arena.root()) {
            let bytes = match &component.part {
                ExecutablePart::Text(_) => original_unicode_text_component(arena, component)?,
                ExecutablePart::Variable { .. } => variable(arena, component)?,
                _ => return None,
            };
            value.extend_from_slice(&bytes);
        }
    }
    std::str::from_utf8(&value).ok()?;
    Some(value)
}

/// Decode a static standalone word fragment using an explicit grouping fact.
/// `fragment` is the caller's post-grouping source text, not a complete script
/// or an already evaluated object. Braced fragments apply only the selected
/// brace continuation rule; other fragments must contain only configured
/// literal/escape components. Variable, command, expression and malformed
/// components are unavailable. Empty and sigil data remain counted values.
/// This syntax query supplies no original-source receipt, selected handler,
/// Native compiler eligibility, local slot, physical cell or evaluation proof.
#[must_use]
pub fn standalone_static_word_fragment_value(
    fragment: &str,
    braced: bool,
    config: tcl_lexer::LexerConfig,
) -> Option<String> {
    use tcl_lexer::{ExecutablePartArena, SourceChannel, SourceImage, Span, SubstFlags};
    if braced {
        let value = crate::backslash::source_braced_word_bytes(
            fragment.as_bytes(),
            SourceChannel::Document,
            config.brace_backslash_newline,
        );
        return String::from_utf8(value.into_owned()).ok();
    }
    let end = u32::try_from(fragment.len()).ok()?;
    let arena = ExecutablePartArena::decompose(
        SourceImage::document(fragment),
        Span::new(0, end),
        SubstFlags::default(),
        config,
    )
    .ok()?;
    let mut value = Vec::new();
    for component in arena.list(arena.root()) {
        value.extend_from_slice(&original_unicode_text_component(&arena, component)?);
    }
    String::from_utf8(value).ok()
}

/// Literal Unicode prefix data from genuine original partial-word geometry.
/// The lexical issuer retains grouping, complete replacement extent and full
/// source configuration; decoding supplies no offsets, completed word or name.
#[must_use]
pub fn original_word_cursor_literal_prefix(
    prefix: &tcl_lexer::word_parts::WordCursorPrefix,
) -> Option<String> {
    if prefix.image().channel() != tcl_lexer::SourceChannel::Document {
        return None;
    }
    let source = prefix.image().try_text().ok()?;
    let fragment = source.get(prefix.partial_span().as_range())?;
    standalone_static_word_fragment_value(
        fragment,
        prefix.kind() == tcl_lexer::WordKind::Braced,
        prefix.config(),
    )
}

#[cfg(test)]
mod original_word_cursor_value_tests {
    use super::*;

    #[test]
    fn cursor_prefix_values_keep_original_unicode_escapes_and_grouping_separate() {
        for (marked, expected) in [
            (r"caf\u00e9\ name<|>suffix", "café name"),
            (r"{literal\n<|>suffix}", r"literal\n"),
            (r#""quoted\ name<|>suffix""#, "quoted name"),
            ("{unfinished name<|>", "unfinished name"),
        ] {
            let cursor = marked.find("<|>").unwrap();
            let source = marked.replacen("<|>", "", 1);
            let image = tcl_lexer::SourceImage::document(&source);
            let prefix = tcl_lexer::word_parts::word_cursor_prefix_in(
                &image,
                tcl_lexer::Span::new(0, image.len() as u32),
                cursor as u32,
                tcl_lexer::LexerConfig::default(),
            )
            .unwrap()
            .unwrap();
            assert_eq!(
                original_word_cursor_literal_prefix(&prefix).as_deref(),
                Some(expected)
            );
            assert_eq!(
                prefix.container_span(),
                tcl_lexer::Span::new(0, image.len() as u32)
            );
            let native = tcl_lexer::SourceImage::native(image.shared_bytes());
            let prefix = tcl_lexer::word_parts::word_cursor_prefix_in(
                &native,
                tcl_lexer::Span::new(0, native.len() as u32),
                cursor as u32,
                tcl_lexer::LexerConfig::default(),
            )
            .unwrap()
            .unwrap();
            assert!(original_word_cursor_literal_prefix(&prefix).is_none());
        }
    }
}

#[cfg(test)]
mod lexical_unicode_source_word_tests {
    use super::*;

    #[test]
    fn literal_source_words_preserve_unicode_data_and_original_formatting_policy() {
        for dialect in ["tcl8.4", "tcl8.6", "tcl9.0", "jim", "f5-irules"] {
            let config = tcl_lexer::LexerConfig::for_dialect(dialect);
            for value in [
                "plain",
                "two words",
                "",
                "café 😀",
                r"cash$[call]\path",
                "{unbalanced",
                r"literal\uD800",
                "line\nname",
                "crlf\r\nname",
                "\u{feff}name",
            ] {
                let spelling = lexical_unicode_source_word(value, config).unwrap();
                let image = tcl_lexer::SourceImage::document(&spelling);
                let plan = tcl_lexer::native_script_words_in(
                    image,
                    tcl_lexer::Span::new(0, spelling.len() as u32),
                    config,
                )
                .unwrap();
                assert!(plan.fatal_tail.is_none());
                let [command] = plan.commands.as_slice() else {
                    panic!("one command")
                };
                let [word] = command.words.as_slice() else {
                    panic!("one word")
                };
                assert_eq!(
                    original_static_word_unicode_value(word).as_deref(),
                    Some(value.as_bytes()),
                    "{dialect}: {value:?}"
                );
                if let Some(ascii) = lexical_ascii_source_word(value, config) {
                    assert_eq!(spelling, ascii);
                }
            }
        }
    }
}

/// One original scalar reference with an exact decoded literal prefix.
/// Original grouping and executable components remain authoritative; this
/// classifies source syntax and supplies no variable read or name lookup.
#[must_use]
pub fn original_scalar_variable_with_prefix(
    word: &tcl_lexer::NativeWord,
    prefixes: &[&[u8]],
) -> Option<Vec<u8>> {
    let mut name = None;
    let prefix = original_word_unicode_value_with_variables(word, |arena, component| {
        let tcl_lexer::ExecutablePart::Variable {
            name: span,
            index: None,
        } = component.part
        else {
            return None;
        };
        let bytes = arena.bytes(span)?;
        if name.is_some() || crate::naming::split_element_ref_bytes(bytes).is_some() {
            return None;
        }
        name = Some(bytes.to_vec());
        Some(Vec::new())
    })?;
    prefixes
        .contains(&prefix.as_slice())
        .then_some(name?)
        .filter(|name| !name.is_empty())
}

fn original_unicode_text_component(
    arena: &tcl_lexer::ExecutablePartArena,
    component: &tcl_lexer::SpannedExecutablePart,
) -> Option<Vec<u8>> {
    use tcl_lexer::{ExecutablePart, ExecutableText, SourceChannel};
    let value = match &component.part {
        ExecutablePart::Text(ExecutableText::Original) => crate::backslash::source_literal_bytes(
            arena.bytes(component.span)?,
            SourceChannel::Document,
        )
        .into_owned(),
        ExecutablePart::Text(ExecutableText::Decoded(bytes)) => {
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
                    arena.config().escapes,
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
                        arena.config().escapes,
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
            bytes.clone()
        }
        _ => return None,
    };
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

/// Render Unicode data as one literal source word under the complete supplied
/// grammar. Existing list formatting is retained when it proves the exact
/// value; quoted source is independently checked when that proposal cannot.
/// This authoring purpose supplies no Native name recipe or existing identity.
#[must_use]
pub fn lexical_unicode_source_word(value: &str, config: tcl_lexer::LexerConfig) -> Option<String> {
    let proposal = crate::list::list_element(value);
    if lexical_unicode_source_word_matches(&proposal, value, config) {
        return Some(proposal);
    }
    let fragment = crate::backslash::literal_quoted_source_fragment(value, config.escapes)?;
    let proposal = format!("\"{fragment}\"");
    lexical_unicode_source_word_matches(&proposal, value, config).then_some(())?;
    Some(proposal)
}

/// Whether one complete configured source word has exactly the supplied
/// Unicode value. This is source syntax correspondence, not dispatch, value
/// observation or edit authority.
#[must_use]
pub fn lexical_unicode_source_word_matches(
    spelling: &str,
    value: &str,
    config: tcl_lexer::LexerConfig,
) -> bool {
    let admits = || -> Option<()> {
        let end = u32::try_from(spelling.len()).ok()?;
        let image = tcl_lexer::SourceImage::document(spelling);
        let plan =
            tcl_lexer::native_script_words_in(image, tcl_lexer::Span::new(0, end), config).ok()?;
        let [command] = plan.commands.as_slice() else {
            return None;
        };
        let [word] = command.words.as_slice() else {
            return None;
        };
        (plan.fatal_tail.is_none()
            && !word.group().expand
            && word.span() == tcl_lexer::Span::new(0, end)
            && original_static_word_unicode_value(word)?.as_slice() == value.as_bytes())
        .then_some(())
    };
    admits().is_some()
}

/// Whether a complete configured proposal retains one original word's ordered
/// literal values and substitution components. Command/expression bodies and
/// array references retain exact source bytes; scalar names retain their exact
/// literal data. Bare proposals containing literal brace data are outside this
/// conservative source subset. This grants syntax correspondence only, without a value,
/// getter, evaluation-erasure, Native object or edit permission.
#[must_use]
pub fn lexical_source_word_preserves_components(
    original: &tcl_lexer::NativeWord,
    proposal: &str,
) -> bool {
    let admits = || -> Option<()> {
        let end = u32::try_from(proposal.len()).ok()?;
        let plan = tcl_lexer::native_script_words_in_with_purpose(
            tcl_lexer::SourceImage::document(proposal),
            tcl_lexer::Span::new(0, end),
            original.config(),
            tcl_lexer::SourceSegmentationPurpose::NestedScript,
        )
        .ok()?;
        let [command] = plan.commands.as_slice() else {
            return None;
        };
        let [proposed] = command.words.as_slice() else {
            return None;
        };
        let components = source_word_components(proposed)?;
        if proposed.group().kind == tcl_lexer::WordKind::Bare
            && components.iter().any(|component| {
                matches!(component, SourceWordComponent::Text(value) if value.contains(&b'{') || value.contains(&b'}'))
            })
        {
            return None;
        }
        (plan.fatal_tail.is_none()
            && proposed.span() == tcl_lexer::Span::new(0, end)
            && source_word_components(original)? == components)
            .then_some(())
    };
    admits().is_some()
}

#[derive(PartialEq, Eq)]
enum SourceWordComponent {
    Text(Vec<u8>),
    Scalar(Vec<u8>),
    Array(Vec<u8>),
    Command(Vec<u8>),
    Expression(Vec<u8>),
}

fn source_word_components(word: &tcl_lexer::NativeWord) -> Option<Vec<SourceWordComponent>> {
    source_word_components_replacing(word, None)
}

fn source_word_components_replacing(
    word: &tcl_lexer::NativeWord,
    replacement: Option<(tcl_lexer::Span, &[u8])>,
) -> Option<Vec<SourceWordComponent>> {
    use tcl_lexer::{ExecutablePart, SourceChannel};
    if word.image().channel() != SourceChannel::Document || word.group().expand {
        return None;
    }
    word.image().try_text().ok()?;
    if replacement.is_none()
        && let Some(value) = original_static_word_unicode_value(word)
    {
        return Some(vec![SourceWordComponent::Text(value)]);
    }
    let arena = word.executable_parts();
    let mut components = Vec::new();
    let mut replaced = false;
    for component in arena.list(arena.root()) {
        let selected = if let Some((span, value)) =
            replacement.filter(|(span, _)| *span == component.span)
        {
            let _ = span;
            if replaced || !matches!(component.part, ExecutablePart::Variable { index: None, .. }) {
                return None;
            }
            replaced = true;
            SourceWordComponent::Text(value.to_vec())
        } else {
            match component.part {
                ExecutablePart::Text(_) => {
                    let value = original_unicode_text_component(arena, component)?;
                    if value.is_empty() {
                        continue;
                    }
                    if let Some(SourceWordComponent::Text(previous)) = components.last_mut() {
                        previous.extend(value);
                        continue;
                    }
                    SourceWordComponent::Text(value)
                }
                ExecutablePart::Variable { name, index: None } => {
                    SourceWordComponent::Scalar(arena.bytes(name)?.to_vec())
                }
                ExecutablePart::Variable { index: Some(_), .. } => {
                    SourceWordComponent::Array(arena.bytes(component.span)?.to_vec())
                }
                ExecutablePart::Command { body } => {
                    SourceWordComponent::Command(arena.bytes(body)?.to_vec())
                }
                ExecutablePart::Expression { expression } => {
                    SourceWordComponent::Expression(arena.bytes(expression)?.to_vec())
                }
                ExecutablePart::ParseError(_) => return None,
            }
        };
        if let SourceWordComponent::Text(value) = &selected
            && let Some(SourceWordComponent::Text(previous)) = components.last_mut()
        {
            previous.extend(value);
        } else {
            components.push(selected);
        }
    }
    (replacement.is_none() || replaced).then_some(components)
}

#[cfg(test)]
mod original_metadata_tests {
    use super::*;
    use tcl_lexer::{LexerConfig, SourceImage, Span};

    #[test]
    fn complete_component_correspondence_keeps_substitutions_and_full_word_boundaries() {
        let config = LexerConfig::for_dialect("tcl9.0");
        for (source, proposal, expected) in [
            (r#""${x}-[puts {a b}]""#, r#"$x-[puts {a b}]"#, true),
            (r#""${x}tail""#, "$xtail", false),
            (r#""${x}(key)""#, "$x(key)", false),
            (r#""$arr($key)""#, "$arr($other)", false),
            (r#""[puts one]""#, "[puts two]", false),
            (r#""a\ b""#, r"a\ b", true),
            (r#""a b""#, "a b", false),
            (r#""$x""#, "$x;puts altered", false),
            (r#""$x""#, "$x ", false),
            (r#""literal""#, "{*}{literal}", false),
            (r##""#literal""##, "#literal", false),
            (r#""\$x""#, "$x", false),
        ] {
            let original = word(SourceImage::document(source), config);
            assert_eq!(
                lexical_source_word_preserves_components(&original, proposal),
                expected,
                "{source} -> {proposal}"
            );
        }
        let jim = LexerConfig::for_dialect("jim");
        let original = word(SourceImage::document("\"$x\u{0b}text\""), jim);
        assert!(lexical_source_word_preserves_components(
            &original,
            "$x\u{0b}text"
        ));
        let original = word(SourceImage::document("\"$x\u{0b}text\""), config);
        assert!(!lexical_source_word_preserves_components(
            &original,
            "$x\u{0b}text"
        ));
    }

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
    fn variable_name_display_geometry_retains_source_components_and_value_shape_separately() {
        let config = LexerConfig {
            base_line: 17,
            ..LexerConfig::default()
        };
        for (source, value, element) in [
            (r"a\ b", "a b", false),
            ("{a b}", "a b", false),
            (r"a\(key\)", "a(key)", true),
            ("{a($index)}", "a($index)", true),
            ("{名 字}", "名 字", false),
        ] {
            let original = word(SourceImage::document(source), config);
            let geometry = original_variable_name_word_geometry(&original).unwrap();
            assert_eq!(geometry.original_word(), &original);
            assert_eq!(geometry.original_word().config(), config);
            assert_eq!(
                geometry.form(),
                &OriginalVariableNameWordForm::Static {
                    value: value.as_bytes().to_vec(),
                    element,
                }
            );
            let expected = if source.starts_with('{') {
                &source[1..source.len() - 1]
            } else {
                source
            };
            let spans = geometry.literal_spans();
            assert_eq!(spans.len(), 1);
            assert_eq!(&source[spans[0].as_range()], expected);
        }
        for (source, fragments) in [
            ("a($index[choose])", vec!["a(", ")"]),
            (r"a\x28$index\x29", vec![r"a\x28", r"\x29"]),
        ] {
            let original = word(SourceImage::document(source), config);
            let geometry = original_variable_name_word_geometry(&original).unwrap();
            assert_eq!(
                geometry.form(),
                &OriginalVariableNameWordForm::CompoundArray {
                    root: b"a".to_vec()
                }
            );
            assert_eq!(
                geometry
                    .literal_spans()
                    .iter()
                    .map(|span| &source[span.as_range()])
                    .collect::<Vec<_>>(),
                fragments
            );
        }
        for source in ["${root}($index)", "a${root}($index)", "a($index)tail"] {
            let original = word(SourceImage::document(source), config);
            let geometry = original_variable_name_word_geometry(&original).unwrap();
            assert_eq!(geometry.form(), &OriginalVariableNameWordForm::Dynamic);
            assert!(geometry.literal_spans().is_empty());
        }
        let original = word(SourceImage::native(b"a($index)".to_vec()), config);
        assert!(original_variable_name_word_geometry(&original).is_none());
        let expanded = word(SourceImage::document("{*}a($index)"), config);
        assert!(original_variable_name_word_geometry(&expanded).is_none());
    }

    #[test]
    fn original_scalar_prefix_shape_keeps_grouping_and_real_variable_components() {
        let config = LexerConfig::default();
        for source in [
            "$name",
            "${name}",
            "::$name",
            "::${name}",
            r#""::$name""#,
            r"\x3a\x3a$name",
        ] {
            let original = word(SourceImage::document(source), config);
            assert_eq!(
                original_scalar_variable_with_prefix(&original, &[b"", b"::"]),
                Some(b"name".to_vec()),
                "{source}"
            );
        }
        for source in [
            "{$name}",
            "{::$name}",
            ":::$name",
            "prefix$name",
            "$name-suffix",
            "$name(index)",
            "${name(index)}",
            "$name$name",
            "[opaque]",
            r"\$name",
        ] {
            let original = word(SourceImage::document(source), config);
            assert!(
                original_scalar_variable_with_prefix(&original, &[b"", b"::"]).is_none(),
                "{source}"
            );
        }
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

#[cfg(test)]
mod standalone_fragment_tests {
    use super::*;

    #[test]
    fn standalone_static_fragments_keep_grouping_and_configured_substitutions_separate() {
        let config = tcl_lexer::LexerConfig::for_dialect("tcl9.0");
        for value in [
            "",
            "$literal",
            "${literal}",
            "[data]",
            "array($index)",
            "two words",
            "a\0b",
        ] {
            assert_eq!(
                standalone_static_word_fragment_value(value, true, config).as_deref(),
                Some(value)
            );
        }
        assert_eq!(
            standalone_static_word_fragment_value(r"prefix\$value", false, config).as_deref(),
            Some("prefix$value")
        );
        for value in [
            "prefix$value",
            "[set name]",
            "array($index)",
            "${unfinished",
        ] {
            assert!(
                standalone_static_word_fragment_value(value, false, config).is_none(),
                "{value}"
            );
        }
        assert_eq!(
            standalone_static_word_fragment_value("", false, config).as_deref(),
            Some("")
        );
        assert!(
            standalone_static_word_fragment_value(
                "$(1 + 2)",
                false,
                tcl_lexer::LexerConfig::for_dialect("jim")
            )
            .is_none()
        );
        // C reads the empty-root array element; Jim evaluates an expression.
        // Both are computed source operands. Grouped literal data is separate.
        assert!(standalone_static_word_fragment_value("$(1 + 2)", false, config).is_none());
        assert_eq!(
            standalone_static_word_fragment_value("$(1 + 2)", true, config).as_deref(),
            Some("$(1 + 2)")
        );
    }
}

/// Check an intended Unicode literal in one exact original scalar component's
/// actual whole-word container. Existing components, channel and full grammar
/// remain owned by that word; array-index children and expansion refuse.
/// This is syntax correspondence only, not binding, movement or edit permission.
#[must_use]
pub fn lexical_source_scalar_literal_substitution(
    original: &tcl_lexer::NativeWord,
    reference: tcl_lexer::Span,
    value: &str,
) -> bool {
    let admits = || -> Option<()> {
        if value.is_empty()
            || !lexical_unicode_source_word_matches(value, value, original.config().nested())
        {
            return None;
        }
        let expected =
            source_word_components_replacing(original, Some((reference, value.as_bytes())))?;
        let from = usize::try_from(reference.start().checked_sub(original.span().start())?).ok()?;
        let to = usize::try_from(reference.end().checked_sub(original.span().start())?).ok()?;
        let mut proposal = std::str::from_utf8(original.written_bytes())
            .ok()?
            .to_owned();
        proposal.get(from..to)?;
        proposal.replace_range(from..to, value);
        let end = u32::try_from(proposal.len()).ok()?;
        let plan = tcl_lexer::native_script_words_in_with_purpose(
            tcl_lexer::SourceImage::document(&proposal),
            tcl_lexer::Span::new(0, end),
            original.config(),
            tcl_lexer::SourceSegmentationPurpose::NestedScript,
        )
        .ok()?;
        let [command] = plan.commands.as_slice() else {
            return None;
        };
        let [word] = command.words.as_slice() else {
            return None;
        };
        (plan.fatal_tail.is_none()
            && word.span() == tcl_lexer::Span::new(0, end)
            && source_word_components(word)? == expected)
            .then_some(())
    };
    admits().is_some()
}

#[cfg(test)]
mod scalar_literal_construction_tests {
    use super::*;
    use tcl_lexer::{LexerConfig, SourceImage, Span};
    fn word(source: &str) -> tcl_lexer::NativeWord {
        let plan = tcl_lexer::native_script_words_in(
            SourceImage::document(source),
            Span::new(0, u32::try_from(source.len()).unwrap()),
            LexerConfig::for_dialect("tcl9.0"),
        )
        .unwrap();
        plan.commands[0].words[0].clone()
    }
    #[test]
    fn actual_scalar_container_retains_neighbors_and_rejects_activation_or_index_children() {
        let receiver = word("pre-${x}-$other");
        assert!(lexical_source_scalar_literal_substitution(
            &receiver,
            Span::new(4, 8),
            "VALUE"
        ));
        assert!(lexical_source_scalar_literal_substitution(
            &receiver,
            Span::new(4, 8),
            "a#b"
        ));
        assert!(!lexical_source_scalar_literal_substitution(
            &receiver,
            Span::new(4, 8),
            "a b"
        ));
        assert!(!lexical_source_scalar_literal_substitution(
            &receiver,
            Span::new(4, 8),
            "$run"
        ));
        let receiver = word("$arr($x)");
        assert!(!lexical_source_scalar_literal_substitution(
            &receiver,
            Span::new(5, 7),
            "VALUE"
        ));
        let receiver = word("{$x}");
        assert!(!lexical_source_scalar_literal_substitution(
            &receiver,
            Span::new(1, 3),
            "VALUE"
        ));
    }
}

/// An original braced scalar followed by a proposed substituted array index.
/// The original receiver and the proposed array receiver remain distinct;
/// this supplies source geometry, not a variable cell, read or edit grant.
#[derive(Debug, Clone)]
pub struct BracedVariableArraySource {
    original: tcl_lexer::NativeWord,
    span: tcl_lexer::Span,
    scalar: String,
    root: String,
    index: String,
    adjacent: bool,
}

impl BracedVariableArraySource {
    /// Genuine complete original word, retaining its image/channel/configuration.
    #[must_use]
    pub fn original_word(&self) -> &tcl_lexer::NativeWord {
        &self.original
    }
    /// Exact original scalar-and-suffix region proposed for replacement.
    #[must_use]
    pub const fn span(&self) -> tcl_lexer::Span {
        self.span
    }
    /// Original literal scalar name; this is not the proposed array root.
    #[must_use]
    pub fn scalar_name(&self) -> &str {
        &self.scalar
    }
    /// Literal root of the deliberately proposed array receiver.
    #[must_use]
    pub fn array_root(&self) -> &str {
        &self.root
    }
    /// Original bytes intended as the proposed substituted index source.
    #[must_use]
    pub fn index_source(&self) -> &str {
        &self.index
    }
    /// Whether the original index suffix lies outside the braced scalar.
    #[must_use]
    pub const fn has_adjacent_index(&self) -> bool {
        self.adjacent
    }
}

/// Read braced-scalar array-intent shapes from genuine executable components.
/// Shared variable/index scanners own all delimiters and substitutions. An
/// unavailable or malformed proposed index supplies no shape; the proposed
/// receiver never retroactively identifies the original scalar variable.
#[must_use]
pub fn braced_variable_array_sources(
    word: &tcl_lexer::NativeWord,
) -> Vec<BracedVariableArraySource> {
    let source = word.image().bytes();
    let config = word.config();
    word.executable_parts()
        .all_parts()
        .filter_map(|part| {
            let tcl_lexer::ExecutablePart::Variable { index: None, .. } = part.part else {
                return None;
            };
            let start = usize::try_from(part.span.start()).ok()?;
            if source.get(start..start.checked_add(2)?)? != b"${" {
                return None;
            }
            let original = tcl_lexer::scan_var_ref(source, start, config).ok()??;
            (original.index.is_none() && original.next == part.span.end() as usize).then_some(())?;
            let scalar = std::str::from_utf8(original.name).ok()?;
            let (root, index, end, adjacent) = if let Some((root, index)) =
                crate::naming::split_element_ref_bytes(original.name)
            {
                let root = std::str::from_utf8(root).ok()?;
                let index = std::str::from_utf8(index).ok()?;
                let extent = tcl_lexer::Span::new(0, u32::try_from(index.len()).ok()?);
                let parts = tcl_lexer::ExecutablePartArena::decompose(
                    tcl_lexer::SourceImage::document(index),
                    extent,
                    tcl_lexer::word_parts::SubstFlags::default(),
                    config,
                )
                .ok()?;
                let mut dynamic = false;
                for component in parts.all_parts() {
                    match component.part {
                        tcl_lexer::ExecutablePart::ParseError(_) => return None,
                        tcl_lexer::ExecutablePart::Variable { .. }
                        | tcl_lexer::ExecutablePart::Command { .. }
                        | tcl_lexer::ExecutablePart::Expression { .. } => dynamic = true,
                        tcl_lexer::ExecutablePart::Text(_) => {}
                    }
                }
                dynamic.then_some(())?;
                (root.to_owned(), index.to_owned(), original.next, false)
            } else {
                let suffix = source.get(original.next..word.span().end() as usize)?;
                (suffix.first() == Some(&b'(')).then_some(())?;
                // This is intentionally a NEW array-reference proposal. The fixed
                // root only asks the existing index scanner for the original suffix.
                let mut proposal = b"$a".to_vec();
                proposal.extend_from_slice(suffix);
                let proposed = tcl_lexer::scan_var_ref(&proposal, 0, config).ok()??;
                let index = std::str::from_utf8(proposed.index?).ok()?.to_owned();
                let consumed = proposed.next.checked_sub(2)?;
                (
                    scalar.to_owned(),
                    index,
                    original.next.checked_add(consumed)?,
                    true,
                )
            };
            ((!root.is_empty() || adjacent) && end <= word.span().end() as usize).then_some(())?;
            Some(BracedVariableArraySource {
                original: word.clone(),
                span: tcl_lexer::Span::new(part.span.start(), u32::try_from(end).ok()?),
                scalar: scalar.to_owned(),
                root,
                index,
                adjacent,
            })
        })
        .collect()
}

#[cfg(test)]
mod braced_array_source_controls {
    use super::*;

    fn word(source: &str) -> tcl_lexer::NativeWord {
        let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
        let mut config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
        config.strict_quoting = !config.strict_quoting;
        let mut plan = tcl_lexer::native_script_words_in(
            tcl_lexer::SourceImage::document(source),
            tcl_lexer::Span::new(0, u32::try_from(source.len()).unwrap()),
            config,
        )
        .unwrap();
        plan.commands.remove(0).words.remove(1)
    }

    #[test]
    fn original_scalar_and_proposed_array_keep_distinct_configured_geometry() {
        for (source, root, index, adjacent) in [
            ("puts ${arr}(a\\)b)", "arr", "a\\)b", true),
            ("puts ${arr($index)}", "arr", "$index", false),
            ("puts \"before ${arr}(x) after\"", "arr", "x", true),
        ] {
            let word = word(source);
            let proposals = braced_variable_array_sources(&word);
            let [proposal] = proposals.as_slice() else {
                panic!("{source}")
            };
            assert_eq!(proposal.original_word(), &word);
            assert_eq!(proposal.array_root(), root);
            assert_eq!(proposal.index_source(), index);
            assert_eq!(proposal.has_adjacent_index(), adjacent);
            assert_eq!(
                source.get(proposal.span().as_range()),
                Some(if adjacent {
                    if index == "x" {
                        "${arr}(x)"
                    } else {
                        "${arr}(a\\)b)"
                    }
                } else {
                    "${arr($index)}"
                })
            );
        }
        assert!(braced_variable_array_sources(&word("puts {${arr}($index)}")).is_empty());
        assert!(braced_variable_array_sources(&word("puts ${arr(fixed)}")).is_empty());
    }
}

/// A deliberately changed source word after removing one literal leading space
/// from an original quoted operand. Only one static word or a single original
/// scalar/array reference is supported. The configured component owner checks
/// the new standalone word; this supplies no value, list or edit equivalence.
#[must_use]
pub fn quoted_leading_space_source_word(original: &tcl_lexer::NativeWord) -> Option<String> {
    if original.group().kind != tcl_lexer::WordKind::Quoted || original.group().expand {
        return None;
    }
    let content = original.content_span().ok()?;
    let raw = original.image().bytes().get(content.as_range())?;
    let suffix = std::str::from_utf8(raw.strip_prefix(b" ")?).ok()?;
    if suffix.is_empty() {
        return None;
    }
    let components = source_word_components(original)?;
    let proposal = match components.as_slice() {
        [SourceWordComponent::Text(value)] => {
            let value = std::str::from_utf8(value.strip_prefix(b" ")?).ok()?;
            if value.is_empty() {
                return None;
            }
            lexical_unicode_source_word(value, original.config())?
        }
        [
            SourceWordComponent::Text(space),
            SourceWordComponent::Scalar(_) | SourceWordComponent::Array(_),
        ] if space == b" " => suffix.to_owned(),
        _ => return None,
    };
    // The receiver is an operand, not a new command head. Static data is
    // deliberately encoded; only the resulting configured word is parsed.
    let plan = tcl_lexer::native_script_words_in_with_purpose(
        tcl_lexer::SourceImage::document(&proposal),
        tcl_lexer::Span::new(0, u32::try_from(proposal.len()).ok()?),
        original.config(),
        tcl_lexer::SourceSegmentationPurpose::NestedScript,
    )
    .ok()?;
    let [command] = plan.commands.as_slice() else {
        return None;
    };
    let [word] = command.words.as_slice() else {
        return None;
    };
    let mut expected = components;
    let Some(SourceWordComponent::Text(first)) = expected.first_mut() else {
        return None;
    };
    first.remove(0);
    if first.is_empty() {
        expected.remove(0);
    }
    (plan.fatal_tail.is_none() && source_word_components(word)? == expected).then_some(proposal)
}

#[cfg(test)]
mod configured_append_piece_controls {
    use super::*;

    #[test]
    fn actual_quoted_component_keeps_variable_phase_and_refuses_composite_payloads() {
        let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
        let mut config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
        config.strict_quoting = !config.strict_quoting;
        let comment_piece = "\" #item\"";
        let plan = tcl_lexer::native_script_words_in(
            tcl_lexer::SourceImage::document(comment_piece),
            tcl_lexer::Span::new(0, u32::try_from(comment_piece.len()).unwrap()),
            config,
        )
        .unwrap();
        let proposed = quoted_leading_space_source_word(&plan.commands[0].words[0]).unwrap();
        assert!(lexical_unicode_source_word_matches(
            &proposed, "#item", config
        ));
        for (source, expected) in [
            ("\" $item\"", Some("$item")),
            ("\" item\"", Some("item")),
            ("\" $arr(k)\"", Some("$arr(k)")),
            ("\" $a $b\"", None),
            ("\" [list $a]\"", None),
            ("\"  $a\"", None),
        ] {
            let plan = tcl_lexer::native_script_words_in(
                tcl_lexer::SourceImage::document(source),
                tcl_lexer::Span::new(0, u32::try_from(source.len()).unwrap()),
                config,
            )
            .unwrap();
            let word = &plan.commands[0].words[0];
            assert_eq!(
                quoted_leading_space_source_word(word).as_deref(),
                expected,
                "{source}"
            );
        }
    }
}
