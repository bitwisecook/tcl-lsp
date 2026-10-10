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

//! Shared splitting of an [`tcl_registry::ArgRole::LambdaLiteral`] argument
//! (`apply`'s `{argList body ?namespace?}` shape) into its list elements'
//! absolute spans, so every consumer that walks such an argument — folding,
//! formatting, minification, declaration-scanning, the iRules object-
//! reference walker, the semantic-token highlighter — splits it the same
//! way instead of mis-reading the whole literal as if it were script source.
//!
//! Before this shape had its own role, several generic `ArgRole::Body`
//! walkers re-segmented the *entire* `{argList body}` blob as a script:
//! `apply {dir { puts $dir }} …` was read as one statement whose command
//! name is `dir` and whose one argument is `{puts $dir}` — the parameter
//! word masquerading as a command head. Since `dir` never resolves to a
//! registered command, recursion stops there and the real body is never
//! reached. Splitting the list here — element 0 is the
//! parameter list, element 1 is the body script — lets each consumer recurse
//! into the real body directly.

use std::borrow::Cow;

use tcl_lexer::{Span, Token, TokenType};
use tcl_syntax::list::Element;

/// The list elements of a lambda literal, as absolute byte spans into the
/// original source.
///
/// `params` is present whenever the literal parses as a list at all.
/// `body` / `namespace` are `None` when the list has fewer elements — a
/// malformed lambda that would itself error under `apply` at runtime, or a
/// list truncated mid-edit — so callers that need "is this a usable 2-or-3
/// element lambda" should check `body.is_some()` rather than assuming it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LambdaLiteralElements {
    /// Element 0 — the parameter list (not code: a plain word or a braced
    /// `{name ?default...?}` list, never recursed as script).
    pub params: Span,
    /// Element 1 — the body script, when present.
    pub body: Option<Span>,
    /// `true` when [`Self::body`] was written as a `{braced}` list element.
    /// See [`Self::braced_body`].
    pub body_braced: bool,
    /// Element 2 — the namespace the body runs in, when present.
    pub namespace: Option<Span>,
}

impl LambdaLiteralElements {
    /// [`Self::body`], but only for a `{braced}` body element — the one
    /// shape whose script text is the source text verbatim at exactly this
    /// span's offsets.
    ///
    /// A bare or double-quoted body element goes through list-element
    /// decoding before `apply` ever evaluates it, so its backslash escapes
    /// collapse first: `apply {{} if\ \{$x\}\ \{puts a\}}`'s real body is
    /// `if {$x} {puts a}`, three words, while its *source* slice is a single
    /// escaped word. A consumer that re-parses the slice in place and reports
    /// source spans back — the refactor code-action descent, the dispatch
    /// call-site scan — would be reading a script that does not exist. Those
    /// consumers take this accessor and skip the element instead; consumers
    /// that want the real value regardless of shape take
    /// [`split_lambda_literal_decoded`], which has no spans to get wrong.
    #[must_use]
    pub fn braced_body(&self) -> Option<Span> {
        self.body.filter(|_| self.body_braced)
    }
}

/// Split a lambda-literal token into its list elements' absolute spans.
///
/// `tok` must be the braced literal argument itself (a `Str` token) — a
/// `$var` / `[cmd]`-computed lambda can't be split statically, and this
/// returns `None` for that case, matching the guard every
/// `ArgRole::LambdaLiteral` consumer applies before calling this. Also
/// returns `None` when the literal's content isn't parseable as a Tcl list
/// at all (an unmatched brace/quote in the parameter-list element).
#[must_use]
pub fn split_lambda_literal(source: &str, tok: Token) -> Option<LambdaLiteralElements> {
    let (content_start, params_el, body_el, namespace_el) = locate_elements(source, tok)?;
    let to_span = |el: &Element| -> Option<Span> {
        Some(Span::new(
            content_start + u32::try_from(el.value.start).ok()?,
            content_start + u32::try_from(el.value.end).ok()?,
        ))
    };
    Some(LambdaLiteralElements {
        params: to_span(&params_el)?,
        body: body_el.as_ref().and_then(to_span),
        body_braced: body_el.as_ref().is_some_and(|el| el.braced),
        namespace: namespace_el.as_ref().and_then(to_span),
    })
}

/// Lambda source geometry from one genuine complete original braced word.
/// The parent's full configuration selects list syntax; no child word key,
/// invocation, fresh frame or native body preparation is manufactured.
#[must_use]
pub fn split_original_lambda_literal(
    word: &tcl_lexer::NativeWord,
) -> Option<LambdaLiteralElements> {
    // Implementation contract: naming.source.original-editor-body-structure
    // docs/design/analysis/name-resolution-proofs/original-editor-body-structure.md

    if word.group().kind != tcl_lexer::WordKind::Braced {
        return None;
    }
    let content = word.content_span().ok()?;
    let source = word.image().try_text().ok()?;
    let text = source.get(content.as_range())?;
    let (params, body, namespace) = locate_list_elements(text, word.config().list_parse)?;
    let to_span = |element: &Element| -> Option<Span> {
        Some(Span::new(
            content
                .start()
                .checked_add(u32::try_from(element.value.start).ok()?)?,
            content
                .start()
                .checked_add(u32::try_from(element.value.end).ok()?)?,
        ))
    };
    Some(LambdaLiteralElements {
        params: to_span(&params)?,
        body: body.as_ref().and_then(to_span),
        body_braced: body.as_ref().is_some_and(|element| element.braced),
        namespace: namespace.as_ref().and_then(to_span),
    })
}

fn locate_list_elements(
    text: &str,
    syntax: tcl_dialect::ListParse,
) -> Option<(Element, Option<Element>, Option<Element>)> {
    let params = tcl_syntax::list::find_element_with_syntax(text, 0, syntax)
        .ok()
        .flatten()?;
    let body = tcl_syntax::list::find_element_with_syntax(text, params.next, syntax)
        .ok()
        .flatten();
    let namespace = body.as_ref().and_then(|element| {
        tcl_syntax::list::find_element_with_syntax(text, element.next, syntax)
            .ok()
            .flatten()
    });
    Some((params, body, namespace))
}

/// Shared element-location logic for [`split_lambda_literal`] and
/// [`split_lambda_literal_decoded`]: locate the (params, ?body?, ?namespace?)
/// list elements of a lambda literal, keeping each [`Element`]'s `literal`
/// flag — whether it was `{brace}`-delimited (verbatim) vs bare/quoted
/// (backslash escapes need collapsing) — that a `Span`-only result would
/// discard. Returns the absolute byte offset the elements' (source-relative)
/// spans are anchored to, alongside the elements themselves.
fn locate_elements(
    source: &str,
    tok: Token,
) -> Option<(u32, Element, Option<Element>, Option<Element>)> {
    if tok.kind != TokenType::Str {
        return None;
    }
    let content_start = tok.span.start() + u32::from(tok.content_offset);
    let content_end = tok
        .span
        .end()
        .min(u32::try_from(source.len()).unwrap_or(u32::MAX));
    if content_end < content_start {
        return None;
    }
    let text = source.get(content_start as usize..content_end as usize)?;

    let (params, body, namespace) = locate_list_elements(text, tcl_dialect::ListParse::Strict)?;
    Some((content_start, params, body, namespace))
}

/// A lambda literal's list elements, decoded to the actual value Tcl's list
/// parser produces — a `{braced}` element verbatim, a bare/quoted element
/// with its backslash escapes collapsed ([`tcl_lexer::backslash_subst`]).
///
/// Reconstruction consumers (minification, formatting) that rewrite an
/// element and reassemble the literal need this rather than
/// [`LambdaLiteralElements`]'s raw spans: reprocessing a non-literal
/// element's still-escaped source spelling directly as list/script text
/// silently changes what it means:
/// `apply {{} puts\ hi}`'s real body is `puts hi`, not the literal text
/// `puts\ hi`, since list-element decoding collapses the `\ ` into a space
/// *before* the result is ever parsed as a script).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedLambdaLiteral<'a> {
    /// Element 0's decoded value (the parameter list).
    pub params: Cow<'a, str>,
    /// Element 1's decoded value (the body script), when present.
    pub body: Option<Cow<'a, str>>,
    /// Element 2's decoded value (the namespace), when present.
    pub namespace: Option<Cow<'a, str>>,
}

/// Decoded lambda values from the actual complete static parent input.
/// The input must own `word`; its independently selected native value/list
/// recipe supplies bytes. Values without an exact UTF-8 view decline rather than changing
/// native units. These owned values supply no cooked source offsets or body admission.
#[must_use]
pub fn split_original_lambda_literal_decoded(
    word: &tcl_lexer::NativeWord,
    input: &crate::signature_scan::scope::SignatureSourceNameInput,
) -> Option<DecodedLambdaLiteral<'static>> {
    // Implementation contract: naming.source.original-editor-body-structure
    // docs/design/analysis/name-resolution-proofs/original-editor-body-structure.md

    let crate::signature_scan::scope::SignatureSourceNameInput::OriginalWord(key) = input else {
        return None;
    };
    if key.original_word() != word
        || word.group().kind != tcl_lexer::WordKind::Braced
        || input.original_list_element(3).is_some()
    {
        return None;
    }
    let decode = |ordinal| {
        let child = input.original_list_element(ordinal)?;
        std::str::from_utf8(child.bytes())
            .ok()
            .map(|text| Cow::Owned(text.to_owned()))
    };
    Some(DecodedLambdaLiteral {
        params: decode(0)?,
        body: Some(decode(1)?),
        namespace: if input.original_list_element(2).is_some() {
            Some(decode(2)?)
        } else {
            None
        },
    })
}

/// Source-only decoded lambda fields under the complete original word grammar.
/// This supplies Unicode lexical presentation for explicit Logical consumers;
/// no Native value/list recipe, key, cooked extent or activation is issued.
#[must_use]
pub fn split_original_lambda_literal_lexical_decoded(
    word: &tcl_lexer::NativeWord,
) -> Option<DecodedLambdaLiteral<'static>> {
    // naming.editor.logical-formatting-context
    // docs/design/analysis/name-resolution-proofs/logical-formatting-context.md
    if word.image().channel() != tcl_lexer::SourceChannel::Document
        || word.group().kind != tcl_lexer::WordKind::Braced
        || word.group().expand
    {
        return None;
    }
    let content = word.content_span().ok()?;
    let raw = word.image().bytes().get(content.as_range())?;
    let value = tcl_syntax::backslash::source_braced_word_bytes(
        raw,
        word.image().channel(),
        word.config().brace_backslash_newline,
    );
    let text = std::str::from_utf8(&value).ok()?;
    let elements = tcl_syntax::word_rules::WordValueRules::from_config(&word.config())
        .split_list(text)
        .ok()?;
    if !(2..=3).contains(&elements.len()) {
        return None;
    }
    Some(DecodedLambdaLiteral {
        params: Cow::Owned(elements[0].to_string()),
        body: Some(Cow::Owned(elements[1].to_string())),
        namespace: elements.get(2).map(|value| Cow::Owned(value.to_string())),
    })
}

/// Decoded counterpart of [`split_lambda_literal`] — see
/// [`DecodedLambdaLiteral`]. Returns `None` under the same conditions as
/// [`split_lambda_literal`] (a `$var`/`[cmd]`-computed lambda, or content
/// that isn't parseable as a Tcl list).
#[must_use]
pub fn split_lambda_literal_decoded(source: &str, tok: Token) -> Option<DecodedLambdaLiteral<'_>> {
    let (content_start, params_el, body_el, namespace_el) = locate_elements(source, tok)?;
    let decode = |el: &Element| -> Option<Cow<'_, str>> {
        let start = content_start as usize + el.value.start;
        let end = content_start as usize + el.value.end;
        let raw = source.get(start..end)?;
        Some(if el.literal {
            Cow::Borrowed(raw)
        } else {
            tcl_lexer::backslash_subst(raw)
        })
    };
    Some(DecodedLambdaLiteral {
        params: decode(&params_el)?,
        body: body_el.as_ref().and_then(decode),
        namespace: namespace_el.as_ref().and_then(decode),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::segmenter::segment_commands;

    #[test]
    fn original_lexical_lambda_decoding_retains_word_config_and_channel() {
        // naming.editor.logical-formatting-context
        // docs/design/analysis/name-resolution-proofs/logical-formatting-context.md
        let source = "apply {a {puts a\\
 b}}";
        let config = tcl_lexer::LexerConfig::default();
        let word = |image, config| {
            let end = u32::try_from(source.len()).unwrap();
            let plan = tcl_lexer::native_script_words_in(image, Span::new(0, end), config).unwrap();
            plan.commands[0].words[1].clone()
        };
        let folded = word(tcl_lexer::SourceImage::document(source), config);
        assert_eq!(
            split_original_lambda_literal_lexical_decoded(&folded)
                .unwrap()
                .body
                .as_deref(),
            Some("puts a b")
        );
        let literal = word(
            tcl_lexer::SourceImage::document(source),
            tcl_lexer::LexerConfig {
                brace_backslash_newline: tcl_dialect::BraceBackslashNewline::Literal,
                ..config
            },
        );
        assert_eq!(
            split_original_lambda_literal_lexical_decoded(&literal)
                .unwrap()
                .body
                .as_deref(),
            Some(
                "puts a\\
 b"
            )
        );
        let native = word(tcl_lexer::SourceImage::native(source.as_bytes()), config);
        assert!(split_original_lambda_literal_lexical_decoded(&native).is_none());
    }

    fn lambda_tok(src: &str) -> Token {
        // `apply <lambda> …` — the lambda literal is argv[1].
        let cmds = segment_commands(src);
        cmds[0].argv[1]
    }

    #[test]
    fn splits_params_and_body() {
        let src = "apply {dir {puts $dir}} /tmp";
        let elems = split_lambda_literal(src, lambda_tok(src)).unwrap();
        assert_eq!(
            &src[elems.params.start() as usize..elems.params.end() as usize],
            "dir"
        );
        let body = elems.body.unwrap();
        assert_eq!(
            &src[body.start() as usize..body.end() as usize],
            "puts $dir"
        );
        assert!(elems.namespace.is_none());
    }

    #[test]
    fn splits_braced_multi_param_list() {
        let src = "apply {{x y} {return [expr {$x+$y}]}} 1 2";
        let elems = split_lambda_literal(src, lambda_tok(src)).unwrap();
        assert_eq!(
            &src[elems.params.start() as usize..elems.params.end() as usize],
            "x y"
        );
        let body = elems.body.unwrap();
        assert_eq!(
            &src[body.start() as usize..body.end() as usize],
            "return [expr {$x+$y}]"
        );
    }

    #[test]
    fn splits_namespace_element() {
        let src = "apply {dir {puts $dir} ::foo} /tmp";
        let elems = split_lambda_literal(src, lambda_tok(src)).unwrap();
        let ns = elems.namespace.unwrap();
        assert_eq!(&src[ns.start() as usize..ns.end() as usize], "::foo");
    }

    #[test]
    fn params_only_has_no_body() {
        let src = "apply {dir}";
        let elems = split_lambda_literal(src, lambda_tok(src)).unwrap();
        assert!(elems.body.is_none());
        assert!(elems.namespace.is_none());
    }

    #[test]
    fn braced_body_is_offered_for_in_place_reparse() {
        let src = "apply {{} {Factory make}}";
        let elems = split_lambda_literal(src, lambda_tok(src)).unwrap();
        assert!(elems.body_braced);
        let body = elems.braced_body().unwrap();
        assert_eq!(
            &src[body.start() as usize..body.end() as usize],
            "Factory make"
        );
    }

    /// A bare body element with backslash escapes is
    /// decoded by the list parser before `apply` evaluates it, so its source
    /// slice is not the script that runs — an in-place re-parse would read
    /// `if\ \{$x\}\ \{puts\ a\}` as one word.  `braced_body` withholds it.
    #[test]
    fn escaped_bare_body_is_withheld_from_in_place_reparse() {
        let src = r"apply {{} if\ \{$x\}\ \{puts\ a\}}";
        let elems = split_lambda_literal(src, lambda_tok(src)).unwrap();
        assert!(elems.body.is_some());
        assert!(!elems.body_braced);
        assert!(elems.braced_body().is_none());
    }

    /// A double-quoted body element is likewise not brace-delimited: it is
    /// decoded before evaluation, so it is withheld too.
    #[test]
    fn quoted_body_is_withheld_from_in_place_reparse() {
        let src = r#"apply {{} "Factory make"}"#;
        let elems = split_lambda_literal(src, lambda_tok(src)).unwrap();
        assert!(elems.body.is_some());
        assert!(!elems.body_braced);
        assert!(elems.braced_body().is_none());
    }

    #[test]
    fn dynamic_lambda_is_not_split() {
        let src = "apply $lambda /tmp";
        let cmds = segment_commands(src);
        let tok = cmds[0].argv[1];
        assert!(split_lambda_literal(src, tok).is_none());
    }

    /// A bare body element's backslash
    /// escapes must be collapsed to get the value Tcl's list parser (and
    /// then `apply`'s script evaluator) actually sees — `puts\ hi`'s real
    /// runtime body is `puts hi` (a two-word command), not the literal
    /// source text `puts\ hi` (which would parse as one word if re-lexed
    /// verbatim as script source).
    #[test]
    fn decodes_bare_body_backslash_escape() {
        let src = r"apply {{} puts\ hi}";
        let decoded = split_lambda_literal_decoded(src, lambda_tok(src)).unwrap();
        assert_eq!(decoded.params, "");
        assert_eq!(decoded.body.as_deref(), Some("puts hi"));
    }

    #[test]
    fn braced_elements_are_not_decoded() {
        let src = "apply {dir {puts $dir}} /tmp";
        let decoded = split_lambda_literal_decoded(src, lambda_tok(src)).unwrap();
        assert_eq!(decoded.params, "dir");
        assert_eq!(decoded.body.as_deref(), Some("puts $dir"));
    }

    #[test]
    fn decodes_bare_namespace_backslash_escape() {
        let src = r"apply {dir {puts $dir} ::ns\ x} /tmp";
        let decoded = split_lambda_literal_decoded(src, lambda_tok(src)).unwrap();
        assert_eq!(decoded.namespace.as_deref(), Some("::ns x"));
    }

    #[test]
    fn decoded_dynamic_lambda_is_not_split() {
        let src = "apply $lambda /tmp";
        let cmds = segment_commands(src);
        let tok = cmds[0].argv[1];
        assert!(split_lambda_literal_decoded(src, tok).is_none());
    }
}

#[cfg(test)]
mod original_geometry_tests {
    #[test]
    fn original_lambda_geometry_uses_its_parent_selected_list_grammar() {
        // Implementation contract: naming.source.original-editor-body-structure
        // docs/design/analysis/name-resolution-proofs/original-editor-body-structure.md
        let source = tcl_lexer::SourceImage::document("apply {{} {puts x}tail}");
        for (profile, expected) in [("tcl8.6", None), ("jim0.84", Some("puts x"))] {
            let profile =
                tcl_registry::model::ingress::resolve_environment(profile).analyser_profile();
            let config = tcl_lexer::LexerConfig::for_profile(Some(profile));
            let plan = tcl_lexer::native_script_words_in(
                source.clone(),
                tcl_lexer::Span::new(0, u32::try_from(source.len()).unwrap()),
                config,
            )
            .unwrap();
            let lambda = &plan.commands[0].words[1];
            let body = super::split_original_lambda_literal(lambda)
                .and_then(|elements| elements.braced_body())
                .and_then(|span| source.try_text().ok()?.get(span.as_range()));
            assert_eq!(body, expected);
        }
    }
}

#[cfg(test)]
mod original_decoded_tests {
    #[test]
    fn original_lambda_decoding_retains_parent_native_units_without_cooked_geometry() {
        // Implementation contract: naming.source.original-editor-body-structure
        // docs/design/analysis/name-resolution-proofs/original-editor-body-structure.md
        for name in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim0.84"] {
            let profile =
                tcl_registry::model::ingress::resolve_environment(name).analyser_profile();
            let dialect = tcl_registry::InvocationDialect::of_profile(profile);
            let config = tcl_lexer::LexerConfig::for_profile(Some(profile));
            let image = tcl_lexer::SourceImage::document("apply {{} puts\\ hi ::N}");
            let plan = tcl_lexer::native_script_words_in(
                image.clone(),
                tcl_lexer::Span::new(0, u32::try_from(image.len()).unwrap()),
                config,
            )
            .unwrap();
            let word = &plan.commands[0].words[1];
            let recipe = dialect.authored_name_policy().unwrap();
            let key =
                crate::signature_scan::scope::SignatureSourceNameKey::from_original_native_word(
                    word,
                    tcl_syntax::word_rules::WordValueRules::from_config(&config),
                    recipe,
                )
                .unwrap();
            let input = crate::signature_scan::scope::SignatureSourceNameInput::OriginalWord(key);
            let decoded = super::split_original_lambda_literal_decoded(word, &input).unwrap();
            assert_eq!(decoded.params, "");
            assert_eq!(decoded.body.as_deref(), Some("puts hi"));
            assert_eq!(decoded.namespace.as_deref(), Some("::N"));
            assert!(
                super::split_original_lambda_literal(word)
                    .unwrap()
                    .braced_body()
                    .is_none()
            );
            let foreign = &plan.commands[0].words[0];
            assert!(super::split_original_lambda_literal_decoded(foreign, &input).is_none());
            let astral = tcl_lexer::SourceImage::document("apply {{} {puts 😀}}");
            let plan = tcl_lexer::native_script_words_in(
                astral.clone(),
                tcl_lexer::Span::new(0, u32::try_from(astral.len()).unwrap()),
                config,
            )
            .unwrap();
            let word = &plan.commands[0].words[1];
            let key =
                crate::signature_scan::scope::SignatureSourceNameKey::from_original_native_word(
                    word,
                    tcl_syntax::word_rules::WordValueRules::from_config(&config),
                    recipe,
                )
                .unwrap();
            let input = crate::signature_scan::scope::SignatureSourceNameInput::OriginalWord(key);
            let decoded = super::split_original_lambda_literal_decoded(word, &input);
            if dialect
                .tcl_version
                .is_some_and(|version| version < tcl_dialect::TclVersion::V9_0)
            {
                assert!(
                    decoded.is_none(),
                    "{name}: native surrogate units have no UTF-8 view"
                );
            } else {
                assert_eq!(decoded.unwrap().body.as_deref(), Some("puts 😀"), "{name}");
            }
        }
    }
}
