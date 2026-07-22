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
//! registered command, recursion stopped there and the real body was never
//! reached (issue #954). Splitting the list here — element 0 is the
//! parameter list, element 1 is the body script — lets each consumer recurse
//! into the real body directly.

use std::borrow::Cow;

use tcl_lexer::{Span, Token, TokenType};
use tcl_syntax::list::{Element, find_element};

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
    /// Element 2 — the namespace the body runs in, when present.
    pub namespace: Option<Span>,
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
        namespace: namespace_el.as_ref().and_then(to_span),
    })
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

    let params_el = find_element(text, 0).ok().flatten()?;
    let body_el = find_element(text, params_el.next).ok().flatten();
    let namespace_el = body_el
        .as_ref()
        .and_then(|el| find_element(text, el.next).ok().flatten());

    Some((content_start, params_el, body_el, namespace_el))
}

/// A lambda literal's list elements, decoded to the actual value Tcl's list
/// parser produces — a `{braced}` element verbatim, a bare/quoted element
/// with its backslash escapes collapsed ([`tcl_lexer::backslash_subst`]).
///
/// Reconstruction consumers (minification, formatting) that rewrite an
/// element and reassemble the literal need this rather than
/// [`LambdaLiteralElements`]'s raw spans: reprocessing a non-literal
/// element's still-escaped source spelling directly as list/script text
/// silently changes what it means (codex review of #954's follow-up —
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
    fn dynamic_lambda_is_not_split() {
        let src = "apply $lambda /tmp";
        let cmds = segment_commands(src);
        let tok = cmds[0].argv[1];
        assert!(split_lambda_literal(src, tok).is_none());
    }

    /// Codex review of #954's follow-up: a bare body element's backslash
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
