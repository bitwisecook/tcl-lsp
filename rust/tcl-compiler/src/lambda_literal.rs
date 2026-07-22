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

use tcl_lexer::{Span, Token, TokenType};
use tcl_syntax::list::find_element;

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

    let to_span = |el: &tcl_syntax::list::Element| -> Option<Span> {
        Some(Span::new(
            content_start + u32::try_from(el.value.start).ok()?,
            content_start + u32::try_from(el.value.end).ok()?,
        ))
    };

    let params_el = find_element(text, 0).ok().flatten()?;
    let params = to_span(&params_el)?;

    let body_el = find_element(text, params_el.next).ok().flatten();
    let body = body_el.as_ref().and_then(to_span);
    let namespace = body_el
        .and_then(|el| find_element(text, el.next).ok().flatten())
        .and_then(|el| to_span(&el));

    Some(LambdaLiteralElements {
        params,
        body,
        namespace,
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
}
