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

//! Variable references in source: stripping the sigil, `${…}` under each close rule, normalised base names and whole-reference queries.

use super::array_elements::{split_array_name_for_style, split_element_ref_bytes};

/// Strip a variable reference's substitution sigil (`$`, `${…}`) while
/// **keeping** any array-index suffix — the form an evaluator needs to read the
/// actual variable (`$arr(idx)` → `arr(idx)`, `${v}` → `v`, `$x` → `x`). Unlike
/// [`normalise_var_name`], the `(idx)` is preserved.
///
/// ```
/// use tcl_lexer::variable_names::var_reference;
/// assert_eq!(var_reference("$arr(idx)"), "arr(idx)");
/// assert_eq!(var_reference("${v}"), "v");
/// assert_eq!(var_reference("$x"), "x");
/// ```
#[must_use]
pub fn var_reference(name: &str) -> &str {
    var_reference_for_style(name, tcl_dialect::BracedVarStyle::default())
}

/// [`var_reference`] under an explicitly resolved `${…}` close rule.
#[must_use]
pub fn var_reference_for_style(name: &str, style: tcl_dialect::BracedVarStyle) -> &str {
    match braced_var_name(name, style) {
        Some(inner) => inner,
        None => name.strip_prefix('$').unwrap_or(name),
    }
}

/// The name inside a word whose **whole text** is one `${…}` reference.
///
/// Two readings, in this order, and the order is the point:
///
/// 1. The closer located by the shared owner
///    [`crate::braced_var_name_end`] under `style` — the free-text
///    reading, which is what a scanner walking raw source needs.
/// 2. Failing that, a word that nonetheless *is* a whole `${…}` span:
///    it opens `${` and ends `}`, so some lexer already delimited it, and a
///    span is **correct by construction** under whatever release produced it.
///
/// Step 2 exists because a token span and a free-text scan can disagree about
/// the same bytes when the caller's `style` is not the one that produced the
/// span. Under an 8.x dialect the lexer emits `${a{b}` as one `Var` (the name
/// `a{b`, with `c}` following as ordinary word text); reading that span with
/// the 9.x rule finds no closer at all, and answering "not a reference" would
/// name `{a{b}` — a variable the source never spells. Trusting the delimiters
/// the lexer already chose is strictly better than inventing that name, and it
/// can never override step 1: the owner returning `Unterminated` is exactly
/// the case where no closer exists under `style`.
///
/// A word with text *after* the closer is not one reference; step 1 still
/// yields its name (the caller decides what the remainder means), which is why
/// this returns only the name.
pub(super) fn braced_var_name(word: &str, style: tcl_dialect::BracedVarStyle) -> Option<&str> {
    if let Some((inner, _)) = split_braced_var_ref(word, style) {
        return Some(inner);
    }
    word.strip_prefix("${").and_then(|w| w.strip_suffix('}'))
}

/// Split a word that **begins** with a `${…}` reference into its inner name
/// and the text following the closing brace.
///
/// This is the one place `tcl-syntax` decodes the brace form. The closer is
/// located by the shared owner [`crate::braced_var_name_end`] under
/// `style`, never by a local scan: three independent private scans here
/// (`normalise_var_name` stripping the **last** `}` — the 9.x answer, at
/// every release — while `split_array_name` and `element_var_name_braced`
/// take the **first** — the 8.x answer, at every release) would disagree
/// with each other on the same bytes, so the crate would answer
/// `${a{b}c}` two ways.
///
/// `None` when the word does not open with `${`, or when the reference never
/// closes — an unterminated name is `Tcl_ParseVarName`'s
/// [`tcl_lexer::MISSING_CLOSE_BRACE_FOR_VAR`] error, not a name running to
/// end-of-input, so callers fall back to their non-braced reading rather than
/// inventing a name.
///
/// ```
/// use tcl_dialect::BracedVarStyle;
/// use tcl_lexer::variable_names::split_braced_var_ref;
/// // 9.x tracks nesting and `\X` pairs …
/// assert_eq!(
///     split_braced_var_ref("${a{b}c}", BracedVarStyle::Tcl9Nesting),
///     Some(("a{b}c", "")),
/// );
/// // … while the 8.x family ends the name at the first literal `}`.
/// assert_eq!(
///     split_braced_var_ref("${a{b}c}", BracedVarStyle::FirstClose),
///     Some(("a{b", "c}")),
/// );
/// assert_eq!(split_braced_var_ref("$a", BracedVarStyle::Tcl9Nesting), None);
/// assert_eq!(split_braced_var_ref("${a", BracedVarStyle::Tcl9Nesting), None);
/// ```
#[must_use]
pub fn split_braced_var_ref(
    word: &str,
    style: tcl_dialect::BracedVarStyle,
) -> Option<(&str, &str)> {
    if !word.starts_with("${") {
        return None;
    }
    // `2` is the byte just past the `${`, i.e. where the name starts. The
    // returned offset always lands on an ASCII `}`, so both slices cut on a
    // UTF-8 character boundary.
    match crate::braced_var_name_end(word.as_bytes(), 2, style) {
        crate::BracedVarEnd::Closed(end) => Some((&word[2..end], &word[end + 1..])),
        crate::BracedVarEnd::Unterminated => None,
    }
}

/// Normalise a Tcl variable reference to its base name.
///
/// Strips leading `$`, `${…}` delimiters, and array index `(…)`
/// suffixes:
///
/// ```
/// use tcl_lexer::variable_names::normalise_var_name;
/// assert_eq!(normalise_var_name("$foo"), "foo");
/// assert_eq!(normalise_var_name("${bar}"), "bar");
/// assert_eq!(normalise_var_name("$arr(idx)"), "arr");
/// assert_eq!(normalise_var_name("plain"), "plain");
/// ```
///
/// Uses the **default** `${…}` close rule
/// ([`tcl_dialect::BracedVarStyle::default`] — 9.x nesting), which is the rule
/// a document with no explicit dialect is lexed under, so the name recovered
/// here agrees with the span the lexer produced. A caller that has already
/// resolved the document's dialect must pass it through
/// [`normalise_var_name_for_style`] instead: under an 8.x dialect the closer
/// moves, and reading it with the 9.x rule names a variable the source never
/// mentions.
#[must_use]
pub fn normalise_var_name(name: &str) -> &str {
    normalise_var_name_for_style(name, tcl_dialect::BracedVarStyle::default())
}

/// [`normalise_var_name`] under an explicitly resolved `${…}` close rule.
///
/// ```
/// use tcl_dialect::BracedVarStyle;
/// use tcl_lexer::variable_names::normalise_var_name_for_style;
/// assert_eq!(
///     normalise_var_name_for_style("${a{b}c}", BracedVarStyle::Tcl9Nesting),
///     "a{b}c",
/// );
/// assert_eq!(
///     normalise_var_name_for_style("${a{b}c}", BracedVarStyle::FirstClose),
///     "a{b",
/// );
/// ```
#[must_use]
pub fn normalise_var_name_for_style(name: &str, style: tcl_dialect::BracedVarStyle) -> &str {
    // The selected braced-name owner determines where the reference ends;
    // the shared closed-element partition decides whether parentheses denote
    // an array. Unmatched opens and suffix text remain scalar name bytes.
    split_array_name_for_style(name, style).0
}

/// Fixed identifier subset for emitting a variable-name word without quoting:
/// `[A-Za-z_][A-Za-z0-9_:]*`. This construction policy is deliberately narrower
/// than Tcl's accepted names and supplies no reference parsing or lookup proof.
///
/// ```
/// use tcl_lexer::variable_names::is_static_variable_source_word;
/// assert!(is_static_variable_source_word("ns::var"));
/// assert!(!is_static_variable_source_word("1abc"));
/// assert!(!is_static_variable_source_word("has space"));
/// ```
#[must_use]
pub fn is_static_variable_source_word(text: &str) -> bool {
    let mut iter = text.bytes();
    let Some(first) = iter.next() else {
        return false;
    };
    if !(first.is_ascii_alphabetic() || first == b'_') {
        return false;
    }
    iter.all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b':'))
}

/// Whether the complete source fragment is one variable reference under its grammar.
/// This shape supplies no selected cell, completed read or invocation authority.
#[must_use]
pub fn is_whole_variable_reference(source: &str, config: crate::LexerConfig) -> bool {
    matches!(
        crate::word_parts::whole_var_ref(source.as_bytes(), config),
        Ok(Some(_))
    )
}

/// One complete reference's literal name, without a separately substituted index.
/// Combined names such as `${a(k)}` remain exact name data; the selected lookup
/// must independently decide scalar/element semantics and authenticate the read.
#[must_use]
pub fn whole_variable_reference_literal_name(
    source: &str,
    config: crate::LexerConfig,
) -> Option<&str> {
    let reference = crate::word_parts::whole_var_ref(source.as_bytes(), config).ok()??;
    reference.index.is_none().then_some(())?;
    core::str::from_utf8(reference.name).ok()
}

/// A whole scalar reference's literal name under the selected source grammar.
/// Separate and combined array references decline; empty and literal sigil names
/// remain data. The caller must independently authenticate the source and read.
#[must_use]
pub fn whole_scalar_variable_reference_name(
    source: &str,
    config: crate::LexerConfig,
) -> Option<&str> {
    let name = whole_variable_reference_literal_name(source, config)?;
    split_element_ref_bytes(name.as_bytes())
        .is_none()
        .then_some(name)
}

/// A whole original substitution's analytical variable root under its grammar.
///
/// The selected lexer owns sigils, brace closers and separate array indexes.
/// A braced combined name is partitioned by [`split_element_ref_bytes`]; an
/// unmatched parenthesis remains part of its scalar name. UTF-8 and opaque
/// byte references use the same path, retaining NUL and literal dollar bytes.
/// Compound text declines; malformed references retain the scanner's error.
/// This source projection supplies no native cell, input extent or read proof.
pub fn variable_reference_root_bytes(
    source: &[u8],
    config: crate::LexerConfig,
) -> Result<Option<&[u8]>, &'static str> {
    let Some(reference) = crate::word_parts::whole_var_ref(source, config)? else {
        return Ok(None);
    };
    let root = if reference.index.is_some() {
        reference.name
    } else {
        split_element_ref_bytes(reference.name).map_or(reference.name, |(root, _)| root)
    };
    Ok(Some(root))
}

/// Whether an array-element key is a compile-time literal — no `$` variable
/// substitution, `[…]` command substitution, or `\` backslash substitution,
/// so the element it names is a single fixed variable (`arr(k)`, `arr(1)`),
/// not a runtime-selected or -decoded one (`arr($i)`, `arr(\x41)`).
///
/// Backslash sequences substitute in an unbraced word (`incr arr(\x41)`
/// increments `arr(A)` — tclsh 8.6/9.0 verified), so treating them as
/// literal would split one runtime element across two SSA symbols. They stay
/// on the conflated base instead; a *braced* word (`set {arr(\x41)} v`)
/// suppresses substitution and its key is literal via
/// [`element_var_name_braced`]'s flag.
#[must_use]
pub fn array_key_is_literal(key: &str) -> bool {
    !key.contains('$') && !key.contains('[') && !key.contains('\\')
}
