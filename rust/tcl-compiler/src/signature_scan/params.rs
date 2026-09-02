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

//! Parameter-list parser for Tcl proc declarations.
//!
//! Splits a parameter-list string (the literal `args` argument to `proc`) into
//! [`ParamDef`] records. A proc / method parameter list is list-parsed by Tcl,
//! so this mirrors that two-level grammar: the list is split into element
//! *specs*, and each spec is itself a list whose first element is the parameter
//! name and whose remainder is the optional default (`a`, `{name default}`, and
//! the escaped forms such as `a\ b` — which Tcl reads as name `a` default `b`).

use tcl_lexer::backslash_subst;
use tcl_syntax::formal_params::{FormalParameter, FormalParameterError, parse_formal_parameters};
use tcl_syntax::list::{find_element, join_list};
use tcl_syntax::word_rules::WordValueRules;

use super::types::ParamDef;

/// Whether a routine's parameter-list **word** is *literal* — data Tcl passes
/// through untouched — rather than **computed** from a run-time value.
///
/// This is the one predicate every tier shares (issue #1107): the analyser,
/// the signature scanner, and the LSP's cursor classifier all decide
/// "literal parameter list?" here, so they cannot drift apart about what a
/// quoted or braced word means.
///
/// Literal ⟺ the word is a *single* [`TokenType::Str`] (brace-quoted) or
/// [`TokenType::Esc`] (bareword, or a quoted word with no substitutions)
/// token. Any `$` or `[` either lexes the word as [`TokenType::Var`] /
/// [`TokenType::Cmd`] or splits it into several tokens, which is exactly the
/// computed case.
///
/// Verified on tclsh 9.0.4 and 8.6.16:
///
/// ```tcl
/// proc makeargs {} { return {a b} }
/// proc p [makeargs] { … }   ;# computed — info args p → a b
/// set params {x y}
/// proc q $params { … }      ;# computed
/// proc r "m n" { … }        ;# LITERAL — info args r → m n
/// proc s {a {b 1}} { … }    ;# literal
/// ```
///
/// `proc r "m n" {…}` is the case the position classifier used to get wrong:
/// a substitution-free quoted word really does declare `m` and `n`.
#[must_use]
pub fn param_word_is_literal(kind: tcl_lexer::TokenType, single_token_word: bool) -> bool {
    single_token_word && matches!(kind, tcl_lexer::TokenType::Str | tcl_lexer::TokenType::Esc)
}

/// [`param_word_is_literal`] decided from the parameter-list word's **raw
/// source text**, for a consumer that holds source rather than a segmented
/// command (the LSP's cursor-position classifier).
///
/// Rather than re-deriving the rule with a character scan — which is how the
/// position classifier came to call the substitution-free quoted list
/// `proc r "m n" {…}` computed, contradicting both the oracle and the
/// analyser — this lexes `word` and applies the *same* token test. A word
/// that does not lex at all (a stray delimiter mid-edit) is treated as
/// computed: nothing about it can be trusted as a declaration.
#[must_use]
pub fn param_word_text_is_literal(word: &str) -> bool {
    // dialect-drift-ok: the LSP cursor-position classifier that owns this
    // predicate (`tcl_lsp_core::definition::param_word_position`) carries only
    // `AnalysisResult::dialect`, a dialect *name*; resolving it here would be
    // the name lookup this lane exists to remove. The test the predicate makes
    // — one word, `Str` or `Esc` — is invariant across every modelled grammar.
    let Ok(tokens) = tcl_lexer::Lexer::new(word).tokenise_all() else {
        return false;
    };
    let mut words = tokens.iter().filter(|t| {
        !matches!(
            t.kind,
            tcl_lexer::TokenType::Sep
                | tcl_lexer::TokenType::Eol
                | tcl_lexer::TokenType::Eof
                | tcl_lexer::TokenType::Comment
        )
    });
    let Some(first) = words.next() else {
        // An empty (or whitespace-only) word is the empty parameter list —
        // literal, and declares nothing.
        return true;
    };
    words.next().is_none() && param_word_is_literal(first.kind, true)
}

/// Parse a Tcl proc argument list string into [`ParamDef`] records.
///
/// Handles both bare-word (`a b c`) and braced-with-default
/// (`{name default}`) forms. The input is the verbatim text of the
/// proc's parameter argument; outer whitespace is tolerated.
///
/// `rules` are the document dialect's word-value rules — the brace
/// `\<newline>` axis and the list-parse axis — so the same parameter-list
/// text divides the way the document's own runtime divides it. A caller
/// that genuinely has no document (a doctest, a fixed internal literal)
/// passes [`WordValueRules::TCL`].
///
/// ```
/// use tcl_compiler::signature_scan::params::parse_param_list;
/// use tcl_syntax::word_rules::WordValueRules;
/// let params = parse_param_list("a {b 1} c", WordValueRules::TCL);
/// assert_eq!(params.len(), 3);
/// assert_eq!(params[1].name, "b");
/// assert_eq!(params[1].default_value.as_deref(), Some("1"));
/// ```
#[must_use]
pub fn parse_param_list(param_str: &str, rules: WordValueRules) -> Vec<ParamDef> {
    // A parameter list is a **braced word** at the command level, so on a
    // dialect that folds them (`BraceBackslashNewline::Folds` — every Tcl
    // core build and the F5 fork) a backslash-newline line continuation has
    // already collapsed to a space before Tcl list-parses the list. The list
    // grammar treats a backslash as escaping the next byte, so the collapse
    // must run first — otherwise `a b\<newline>c` would parse as the two-word
    // element `b c` instead of the two params `b`, `c` (issue #743). JimTcl
    // keeps the bytes, and `rules` is what knows which.
    let collapsed = rules.collapse_braced_word(param_str);
    // Top-level split: each element is one parameter *spec*.
    let Ok(specs) = rules.split_list(&collapsed) else {
        // Unbalanced braces / quotes (common while a list is being typed) —
        // fall back to a tolerant scan so we still surface partial params.
        return parse_param_list_lenient(&collapsed);
    };
    specs
        .iter()
        .filter_map(|spec| spec_to_param(spec))
        .collect()
}

/// Bind call arguments to Tcl procedure formals.
///
/// Missing fixed arguments use their declared defaults.  A final formal named
/// `args` receives every remaining argument as one correctly quoted Tcl list,
/// including the empty list when there are no remaining arguments.  `None`
/// actuals carry an unknown run-time value through the binding; an unknown
/// element of `args` therefore makes the packed list unknown without making
/// the call's arity invalid.
///
/// Returns `None` only when the call has too few required arguments or too
/// many arguments for a non-variadic signature.
///
/// `rules` are the document dialect's word-value rules: a declared default is
/// list text, and which dialect reads it decides whether malformed text raises
/// or is split anyway.
#[must_use]
pub fn bind_proc_formals(
    params: &[ParamDef],
    actuals: &[Option<String>],
    rules: WordValueRules,
) -> Option<Vec<(String, Option<String>)>> {
    let variadic = params.last().is_some_and(|param| param.name == "args");
    let fixed_len = params.len().saturating_sub(usize::from(variadic));
    if actuals.len() > fixed_len && !variadic {
        return None;
    }

    let mut bound = Vec::with_capacity(params.len());
    for (idx, param) in params.iter().take(fixed_len).enumerate() {
        let value = if let Some(value) = actuals.get(idx) {
            value.clone()
        } else {
            let raw = param.default_value.as_deref()?;
            let mut values = rules.split_list(raw).ok()?;
            if values.len() != 1 {
                return None;
            }
            Some(values.remove(0).into_owned())
        };
        bound.push((param.name.clone(), value));
    }
    if variadic {
        let rest = &actuals[fixed_len.min(actuals.len())..];
        let packed = rest
            .iter()
            .cloned()
            .collect::<Option<Vec<_>>>()
            .map(join_list);
        bound.push(("args".to_owned(), packed));
    }
    Some(bound)
}

/// Parse a complete parameter list with Tcl's strict execution semantics.
///
/// Unlike [`parse_param_list`], this rejects malformed lists, invalid formal
/// names, and parameter specifiers with other than one or two fields.  The
/// lenient API deliberately remains separate so incomplete editor buffers can
/// still contribute signature and name information.  Defaults returned here
/// are decoded Tcl list values rather than source-preserving display text.
///
/// `rules` supply the document dialect's brace `\<newline>` axis for the
/// pre-pass.  The *split* stays strict deliberately: this is the
/// runtime-validity oracle behind E006, and a lenient list parse would report
/// a malformed declaration as well-formed.
pub fn parse_param_list_strict(
    param_str: &str,
    rules: WordValueRules,
) -> Result<Vec<FormalParameter>, FormalParameterError> {
    let collapsed = rules.collapse_braced_word(param_str);
    parse_formal_parameters(&collapsed)
}

/// Turn one parameter *spec* (a list element value, delimiters already
/// stripped) into a [`ParamDef`]. The spec is itself a list: its first element
/// is the parameter name; any remaining text is the (lenient) default value.
/// So `a` → name `a`; `x 1` / `{x 1}` → name `x` default `1`; `a b` (from an
/// escaped `a\ b`) → name `a` default `b`; and a braced-verbatim `a\ b` (from
/// `{a\ b}`) → the single name `a b`. Returns `None` for an empty / nameless
/// spec (which Tcl itself rejects).
fn spec_to_param(spec: &str) -> Option<ParamDef> {
    let first = find_element(spec, 0).ok().flatten()?;
    let name_raw = spec.get(first.value.clone())?;
    let name = if first.literal {
        name_raw.to_string()
    } else {
        backslash_subst(name_raw).into_owned()
    };
    if name.is_empty() {
        return None;
    }
    let rest = spec.get(first.next..).unwrap_or("").trim();
    let (has_default, default_value) = if rest.is_empty() {
        (false, None)
    } else {
        (true, Some(rest.to_string()))
    };
    Some(ParamDef {
        name,
        has_default,
        default_value,
    })
}

/// Tolerant fallback used when a parameter list does not parse as a well-formed
/// Tcl list (an unmatched brace / quote, typically mid-edit). Splits on
/// whitespace and line continuations, treating a leading `{` as a
/// `{name default}` spec, so a partially-typed signature still yields as many
/// params as can be recovered.
fn parse_param_list_lenient(param_str: &str) -> Vec<ParamDef> {
    let mut params: Vec<ParamDef> = Vec::new();
    let text = param_str.trim();
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        while let Some(len) = separator_len(bytes, i) {
            i += len;
        }
        if i >= bytes.len() {
            break;
        }
        if bytes[i] == b'{' {
            let mut level: u32 = 1;
            i += 1;
            let start = i;
            while i < bytes.len() && level > 0 {
                match bytes[i] {
                    b'{' => level += 1,
                    b'}' => level -= 1,
                    _ => {}
                }
                i += 1;
            }
            let inner_end = if level == 0 { i - 1 } else { i };
            let inner = text[start..inner_end].trim();
            if inner.is_empty() {
                continue;
            }
            if let Some((name, default)) = split_first_whitespace(inner) {
                params.push(ParamDef {
                    name: name.to_string(),
                    has_default: true,
                    default_value: Some(default.to_string()),
                });
            } else {
                params.push(ParamDef {
                    name: inner.to_string(),
                    has_default: false,
                    default_value: None,
                });
            }
        } else {
            let start = i;
            i = scan_bare_word(bytes, i);
            let word = &text[start..i];
            if !word.is_empty() {
                params.push(ParamDef {
                    name: word.to_string(),
                    has_default: false,
                    default_value: None,
                });
            }
        }
    }
    params
}

/// Source spans of each parameter *name*, in declaration order, within the
/// **raw** param-list literal `raw` (exactly as it appears in source, e.g.
/// `{a b}` or `{a {b 1}}` — one optional outer brace layer is stripped). Each
/// returned [`tcl_lexer::Span`] is offset by `base` (the literal's start byte
/// offset in the document), so the spans point at the parameter names in the
/// original source. The order matches [`parse_param_list`], so the two can be
/// zipped.
///
/// This exists so go-to-definition / references / rename on a formal parameter
/// resolve to the parameter *name* in the declaration, not the proc name or the
/// whole method body (issue #727).
#[must_use]
pub fn param_name_spans(raw: &str, base: u32) -> Vec<tcl_lexer::Span> {
    let bytes = raw.as_bytes();
    let n = bytes.len();
    // Strip exactly one outer `{…}` brace layer if the whole literal is braced.
    let (lo, hi) = if n >= 2 && bytes[0] == b'{' && bytes[n - 1] == b'}' {
        (1, n - 1)
    } else {
        (0, n)
    };
    // Absolute source offset of a byte index within `raw`.
    let abs = |off: usize| base.saturating_add(u32::try_from(off).unwrap_or(u32::MAX));
    let mut out = Vec::new();
    let mut i = lo;
    while i < hi {
        while let Some(len) = separator_len(&bytes[..hi], i) {
            i += len;
        }
        if i >= hi {
            break;
        }
        if bytes[i] == b'{' {
            // Braced spec `{name ?default?}`, taken verbatim: the name is its
            // first list sub-element, so the span covers `b` in `{b 1}` and the
            // whole `a\ b` in `{a\ b}` (whose decoded name is `a b`).
            let inner_start = i + 1;
            let mut level: u32 = 1;
            let mut j = inner_start;
            while j < hi && level > 0 {
                match bytes[j] {
                    b'{' => level += 1,
                    b'}' => level -= 1,
                    _ => {}
                }
                j += 1;
            }
            let inner_end = if level == 0 { j - 1 } else { j };
            if let Some(inner) = raw.get(inner_start..inner_end)
                && let Ok(Some(el)) = find_element(inner, 0)
                && el.value.end > el.value.start
            {
                out.push(tcl_lexer::Span::new(
                    abs(inner_start + el.value.start),
                    abs(inner_start + el.value.end),
                ));
            }
            i = j;
        } else {
            // Bare spec: the name is the element's first sub-element, so an
            // escaped-whitespace spec `a\ b` spans only `a` (its default `b`
            // must stay outside the rename / go-to-definition range).
            let start = i;
            i = scan_bare_word(&bytes[..hi], i);
            let name_len = bare_name_len(&bytes[start..i]);
            if name_len > 0 {
                out.push(tcl_lexer::Span::new(abs(start), abs(start + name_len)));
            }
        }
    }
    out
}

/// [`param_name_spans`] computed directly from the parameter-list *token*,
/// rather than a hand-sliced `(raw, base)` pair.
///
/// `param_name_spans` expects `raw` to be either a bare word or the
/// parameter list's **full** `{…}`-delimited text (both braces present) —
/// but a braced (`Str`) token's span does not hold that: the lexer's
/// inner-end convention starts the span *at* the opening `{` while ending it
/// one byte short of the closing `}`, so `source[tok.span]` alone yields a
/// one-sided slice like `"{f z xs"` for `{f z xs}`. Fed straight into
/// `param_name_spans`, that mismatched leading `{` with no matching closer
/// makes its outer-brace check fail and its scanner misread the stray `{` as
/// opening a single nested spec that swallows every remaining parameter —
/// only the first name ever gets a real span, every later one silently falls
/// back to whatever the caller uses as a default.
///
/// Skipping exactly `tok.content_offset` leading bytes (the same field
/// [`tcl_lexer::SourceMap::token_text`] strips) yields the pure inner
/// content for both braced and bare tokens uniformly — no delimiters, so
/// `param_name_spans`'s own brace handling never has to guess. The `{}`
/// empty-list degenerate (whose span is widened to cover the closing brace,
/// leaving a bare `"}"` remainder after the strip) is clamped to `""`,
/// mirroring `token_text`'s identical clamp.
#[must_use]
pub fn param_name_spans_for_token(source: &str, tok: tcl_lexer::Token) -> Vec<tcl_lexer::Span> {
    let content_start = tok
        .span
        .start()
        .saturating_add(u32::from(tok.content_offset));
    let Some(raw) = source.get(content_start as usize..tok.span.end() as usize) else {
        return Vec::new();
    };
    let raw = if tok.kind == tcl_lexer::TokenType::Str && raw == "}" {
        ""
    } else {
        raw
    };
    param_name_spans(raw, content_start)
}

#[inline]
fn is_whitespace_byte(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | b'\r')
}

/// Length of the element separator starting at `bytes[i]`, if any.
///
/// A separator is either a single whitespace byte, or a **backslash-newline
/// line continuation** (`\<LF>`, `\<CR><LF>`, or `\<CR>`).  In Tcl list
/// parsing — which is how a `proc` / method parameter list is split — a
/// backslash-newline collapses to element-separating whitespace, so a long
/// parameter list wrapped across lines with `\` yields distinct parameters
/// (`{a b\<newline>c}` → `a`, `b`, `c`), not a bogus `b\` name (issue #743).
/// Every other backslash escape keeps the following byte inside the element
/// (see [`scan_bare_word`]).
#[inline]
fn separator_len(bytes: &[u8], i: usize) -> Option<usize> {
    let b = *bytes.get(i)?;
    if is_whitespace_byte(b) {
        return Some(1);
    }
    if b == b'\\' {
        return match bytes.get(i + 1) {
            Some(b'\r') if bytes.get(i + 2) == Some(&b'\n') => Some(3),
            Some(b'\n' | b'\r') => Some(2),
            _ => None,
        };
    }
    None
}

/// Scan one bare (unbraced) list element starting at `bytes[i]`, returning the
/// index one past its final byte.  Stops at the next [`separator_len`]
/// separator; a backslash that escapes any non-newline byte (`a\ b`, `a\tb`)
/// consumes both bytes so the escaped whitespace stays within the element,
/// mirroring Tcl's `TclFindElement`.
#[inline]
fn scan_bare_word(bytes: &[u8], mut i: usize) -> usize {
    while i < bytes.len() && separator_len(bytes, i).is_none() {
        // A backslash before a non-newline byte escapes it into the element.
        i += if bytes[i] == b'\\' && i + 1 < bytes.len() {
            2
        } else {
            1
        };
    }
    i
}

/// Byte length of the parameter *name* within one bare list element `elem` —
/// the element's first list sub-element. A backslash escape that decodes to
/// Tcl list whitespace (`\ `, `\t`, `\n`, `\r`, `\v`, `\f`, and the literal
/// forms) ends the name and begins the default, so `a\ b` → name length 1
/// (`a`); every other escape stays part of the name (`a\\b` → 4). Mirrors the
/// second level of [`parse_param_list`]'s spec parse so the name span stays
/// aligned with the decoded [`ParamDef`].
fn bare_name_len(elem: &[u8]) -> usize {
    let mut i = 0;
    while i < elem.len() {
        match elem[i] {
            b'\\' => match elem.get(i + 1) {
                // Escapes decoding to list whitespace: literal whitespace bytes
                // and the `\t \n \r \v \f` letter escapes.
                Some(
                    b' ' | b'\t' | b'\n' | b'\r' | 0x0b | 0x0c | b't' | b'n' | b'r' | b'v' | b'f',
                ) => return i,
                // Any other escape (or a trailing lone `\`) stays in the name.
                Some(_) => i += 2,
                None => i += 1,
            },
            // An unescaped whitespace byte would have ended the element already,
            // but guard so the name never runs into a following field.
            b if is_whitespace_byte(b) => return i,
            _ => i += 1,
        }
    }
    elem.len()
}

/// Split on the first run of whitespace. Returns `None` when the input
/// contains no whitespace.
fn split_first_whitespace(s: &str) -> Option<(&str, &str)> {
    let bytes = s.as_bytes();
    let split_at = bytes.iter().position(|b| is_whitespace_byte(*b))?;
    let name = &s[..split_at];
    let rest = s[split_at..].trim_start();
    Some((name, rest))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Issue #1107 — the two views of the literalness rule (from a lexed
    /// token, and from raw source text) must agree on every shape, or the
    /// analyser and the LSP's cursor classifier drift apart again.
    #[test]
    fn literalness_views_agree_and_match_the_oracle() {
        // (word, literal?) — verified on tclsh 9.0.4 / 8.6.16.
        let cases: &[(&str, bool)] = &[
            // TP — literal lists.
            ("{a b}", true),
            ("{a {b 1} args}", true),
            ("{}", true),
            ("args", true),
            (r#""m n""#, true), // `info args r` → `m n`
            (r#""""#, true),
            // TN — computed lists.
            ("[makeargs]", false),
            ("$params", false),
            ("${params}", false),
            (r#""$a $b""#, false),
            ("a$b", false),
            ("[a][b]", false),
        ];
        for &(word, expect_literal) in cases {
            assert_eq!(
                param_word_text_is_literal(word),
                expect_literal,
                "text view disagrees for {word:?}"
            );
            // The token view, fed the word as the real lexer sees it.
            let tokens: Vec<tcl_lexer::Token> = tcl_lexer::Lexer::new(word)
                .tokenise_all()
                .expect("lexes")
                .into_iter()
                .filter(|t| {
                    !matches!(
                        t.kind,
                        tcl_lexer::TokenType::Sep
                            | tcl_lexer::TokenType::Eol
                            | tcl_lexer::TokenType::Eof
                    )
                })
                .collect();
            let token_view = match tokens.as_slice() {
                [] => true,
                [only] => param_word_is_literal(only.kind, true),
                _ => false,
            };
            assert_eq!(
                token_view, expect_literal,
                "token view disagrees for {word:?} (tokens {tokens:?})"
            );
        }
    }

    /// FP guard — a bare word containing a `"` that is *not* at word start is
    /// ordinary literal text in Tcl (the quote is only special as the first
    /// character of a word), so it must stay literal. The character scan this
    /// replaced rejected any word containing a `"`.
    #[test]
    fn literalness_bare_word_with_interior_quote_is_literal() {
        assert!(param_word_text_is_literal("a\"b"));
    }

    #[test]
    fn empty_input_yields_no_params() {
        assert!(parse_param_list("", WordValueRules::TCL).is_empty());
        assert!(parse_param_list("   \t\n  ", WordValueRules::TCL).is_empty());
    }

    #[test]
    fn three_bare_names() {
        let params = parse_param_list("a b c", WordValueRules::TCL);
        assert_eq!(params.len(), 3);
        assert_eq!(params[0].name, "a");
        assert!(!params[0].has_default);
        assert_eq!(params[1].name, "b");
        assert_eq!(params[2].name, "c");
        assert!(params.iter().all(|p| p.default_value.is_none()));
    }

    #[test]
    fn braced_with_default() {
        let params = parse_param_list("{a default}", WordValueRules::TCL);
        assert_eq!(params.len(), 1);
        assert_eq!(params[0].name, "a");
        assert!(params[0].has_default);
        assert_eq!(params[0].default_value.as_deref(), Some("default"));
    }

    #[test]
    fn param_name_spans_match_names_in_raw_literal() {
        use tcl_lexer::Span;
        // Outer braces stripped; spans are offset by `base` and point at names.
        let raw = "{arg1 arg2}";
        let spans = param_name_spans(raw, 100);
        assert_eq!(spans, vec![Span::new(101, 105), Span::new(106, 110)]);
        // `{name default}` → the name only.
        let raw2 = "{a {b 1} c}";
        let spans2 = param_name_spans(raw2, 0);
        // a @1..2, b @4..5 (inside inner braces), c @9..10
        assert_eq!(
            spans2,
            vec![Span::new(1, 2), Span::new(4, 5), Span::new(9, 10)]
        );
        // Empty list.
        assert!(param_name_spans("{}", 0).is_empty());
        // Unbraced single word.
        assert_eq!(param_name_spans("args", 5), vec![Span::new(5, 9)]);
    }

    #[test]
    fn braced_single_element_no_default() {
        let params = parse_param_list("{a}", WordValueRules::TCL);
        assert_eq!(params.len(), 1);
        assert_eq!(params[0].name, "a");
        assert!(!params[0].has_default);
        assert!(params[0].default_value.is_none());
    }

    #[test]
    fn mixed_bare_and_braced_default() {
        let params = parse_param_list("a {b 1} c", WordValueRules::TCL);
        assert_eq!(params.len(), 3);
        assert_eq!(params[0].name, "a");
        assert!(!params[0].has_default);
        assert_eq!(params[1].name, "b");
        assert!(params[1].has_default);
        assert_eq!(params[1].default_value.as_deref(), Some("1"));
        assert_eq!(params[2].name, "c");
        assert!(!params[2].has_default);
    }

    #[test]
    fn whitespace_padding_tolerated() {
        let params = parse_param_list("  a   b  ", WordValueRules::TCL);
        assert_eq!(params.len(), 2);
        assert_eq!(params[0].name, "a");
        assert_eq!(params[1].name, "b");
    }

    #[test]
    fn backslash_newline_continuation_splits_params() {
        // Issue #743: a long parameter list wrapped with `\` at end-of-line.
        // Tcl list parsing treats the backslash-newline as element-separating
        // whitespace, so the last name on the wrapped line is `ddrtol`, not
        // `ddrtol\`.
        let params = parse_param_list("a b ddrtol\\\n            ddatol c", WordValueRules::TCL);
        let names: Vec<&str> = params.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names, vec!["a", "b", "ddrtol", "ddatol", "c"]);
        assert!(params.iter().all(|p| !p.name.contains('\\')));

        // `\r\n` line endings behave identically.
        let crlf = parse_param_list("x\\\r\ny", WordValueRules::TCL);
        let crlf_names: Vec<&str> = crlf.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(crlf_names, vec!["x", "y"]);
    }

    #[test]
    fn escaped_whitespace_spec_yields_name_and_default() {
        // A bare spec with an escaped space is one list element `a b`, which
        // Tcl then reads as a two-field spec: name `a`, default `b`. Verified
        // against tclsh: `proc p {a\ b c}` → `info args` = `a c`, and
        // `info default p a d` sets d = `b`.
        let params = parse_param_list("a\\ b c", WordValueRules::TCL);
        assert_eq!(params.len(), 2);
        assert_eq!(params[0].name, "a");
        assert!(params[0].has_default);
        assert_eq!(params[0].default_value.as_deref(), Some("b"));
        assert_eq!(params[1].name, "c");
        assert!(!params[1].has_default);
        // An escaped tab behaves the same way (the tab is a field separator
        // inside the spec).
        let tabbed = parse_param_list("a\\tb", WordValueRules::TCL);
        assert_eq!(tabbed.len(), 1);
        assert_eq!(tabbed[0].name, "a");
        assert_eq!(tabbed[0].default_value.as_deref(), Some("b"));
    }

    #[test]
    fn braced_spec_with_escaped_space_is_one_name() {
        // A braced spec is taken verbatim, then list-parsed: `{a\ b}` → the
        // single element `a b`, i.e. a parameter literally named `a b` with no
        // default. Verified against tclsh: `proc q {{a\ b} c}` → `info args`
        // = `{a b} c`.
        let params = parse_param_list("{a\\ b} c", WordValueRules::TCL);
        assert_eq!(params.len(), 2);
        assert_eq!(params[0].name, "a b");
        assert!(!params[0].has_default);
        assert_eq!(params[1].name, "c");
    }

    #[test]
    fn unbalanced_braces_fall_back_gracefully() {
        // A half-typed list must not panic and should still recover params.
        let params = parse_param_list("a {b", WordValueRules::TCL);
        let names: Vec<&str> = params.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names, vec!["a", "b"]);
    }

    #[test]
    fn strict_parser_rejects_runtime_invalid_names_but_lenient_parser_recovers() {
        let strict = parse_param_list_strict("a {b::c default}", WordValueRules::TCL);
        assert!(matches!(
            strict,
            Err(FormalParameterError::NotSimpleName { ref name }) if name == "b::c"
        ));

        let lenient = parse_param_list("a {b::c default}", WordValueRules::TCL);
        assert_eq!(
            lenient
                .iter()
                .map(|param| param.name.as_str())
                .collect::<Vec<_>>(),
            vec!["a", "b::c"]
        );
    }

    #[test]
    fn param_name_spans_skip_continuation_backslash() {
        use tcl_lexer::Span;
        // `{a b\<newline>c}` — the span for `b` must not include the trailing
        // backslash, and `c` is a separate parameter.
        let raw = "{a b\\\nc}";
        let spans = param_name_spans(raw, 0);
        // a @1..2, b @3..4 (backslash at 4 excluded), c @6..7
        assert_eq!(
            spans,
            vec![Span::new(1, 2), Span::new(3, 4), Span::new(6, 7)]
        );
    }

    #[test]
    fn param_name_spans_align_with_decoded_specs() {
        use tcl_lexer::Span;
        // Bare escaped-whitespace spec: the name span must cover only `a`, not
        // the whole `a\ b` element, so rename / go-to-definition on `$a` never
        // touch the default text.
        let raw = "{a\\ b c}";
        let spans = param_name_spans(raw, 0);
        // a @1..2 (the `\ b` default is excluded), c @6..7.
        assert_eq!(spans, vec![Span::new(1, 2), Span::new(6, 7)]);
        // The span count and order stay aligned with the parsed params.
        assert_eq!(
            spans.len(),
            parse_param_list("a\\ b c", WordValueRules::TCL).len()
        );

        // Braced escaped-whitespace spec: the whole `a\ b` is the name (decoded
        // `a b`), so the span covers it verbatim.
        let braced = "{{a\\ b} c}";
        let bspans = param_name_spans(braced, 0);
        // `a\ b` @2..6, c @8..9.
        assert_eq!(bspans, vec![Span::new(2, 6), Span::new(8, 9)]);
        assert_eq!(
            bspans.len(),
            parse_param_list("{a\\ b} c", WordValueRules::TCL).len()
        );
    }

    #[test]
    fn default_value_preserves_internal_whitespace() {
        let params = parse_param_list("{name default with spaces}", WordValueRules::TCL);
        assert_eq!(params.len(), 1);
        assert_eq!(
            params[0].default_value.as_deref(),
            Some("default with spaces")
        );
    }

    #[test]
    fn proc_formal_binding_uses_defaults_and_checks_arity() {
        let params = parse_param_list("required {optional {two words}}", WordValueRules::TCL);
        let bound = bind_proc_formals(&params, &[Some("value".to_owned())], WordValueRules::TCL);
        assert_eq!(
            bound,
            Some(vec![
                ("required".to_owned(), Some("value".to_owned())),
                ("optional".to_owned(), Some("two words".to_owned())),
            ])
        );
        assert_eq!(bind_proc_formals(&params, &[], WordValueRules::TCL), None);
        assert_eq!(
            bind_proc_formals(
                &params,
                &[
                    Some("one".to_owned()),
                    Some("two".to_owned()),
                    Some("three".to_owned()),
                ],
                WordValueRules::TCL,
            ),
            None
        );
    }

    /// The dialect decides how a parameter list divides, and the two answers
    /// are visibly different for the *same* text.
    ///
    /// `{a b\<newline>c}` is a proc's parameter word. Every Tcl core build
    /// folds the `\<newline>` to a space *before* the word is list-parsed
    /// (`BraceBackslashNewline::Folds`), so it declares three required
    /// parameters. `JimTcl` keeps the bytes (`Literal`, to preserve line
    /// numbers), so the list splits into two elements and the second is a
    /// `{name default}` spec — two parameters, the second optional. Parsing a
    /// Jim document under the default grammar would silently report the wrong
    /// arity, which is the drift this parameter exists to stop.
    ///
    /// The rules come from the compiled catalogue rather than the constants,
    /// so this fails if the grammar's `brace_backslash_newline` axis moves.
    #[test]
    fn a_braced_continuation_splits_per_the_documents_brace_rule() {
        use tcl_dialect::model::{Family, grammar};

        let param_word = "a b\\\nc";

        let tcl = WordValueRules::from_grammar(&grammar(Family::Tcl, Family::Tcl.releases()[0]));
        let folded = parse_param_list(param_word, tcl);
        assert_eq!(
            folded.iter().map(|p| p.name.as_str()).collect::<Vec<_>>(),
            ["a", "b", "c"],
            "a folding dialect declares three required parameters"
        );
        assert!(folded.iter().all(|p| !p.has_default));

        let jim = WordValueRules::from_grammar(&grammar(Family::Jim, Family::Jim.releases()[0]));
        assert_ne!(jim.brace, tcl.brace, "the catalogue axes must differ");
        let literal = parse_param_list(param_word, jim);
        assert_eq!(
            literal.iter().map(|p| p.name.as_str()).collect::<Vec<_>>(),
            ["a", "b"],
            "a literal-continuation dialect declares two parameters"
        );
        assert_eq!(literal[1].default_value.as_deref(), Some("c"));
        assert!(literal[1].has_default);
    }

    #[test]
    fn proc_formal_binding_packs_zero_one_and_many_args_as_a_tcl_list() {
        let params = parse_param_list("head args", WordValueRules::TCL);
        for (tail, expected) in [
            (Vec::<&str>::new(), Vec::<&str>::new()),
            (vec!["one"], vec!["one"]),
            (
                vec!["two words", "brace { value", r"back\slash", ""],
                vec!["two words", "brace { value", r"back\slash", ""],
            ),
        ] {
            let mut actuals = vec![Some("head".to_owned())];
            actuals.extend(tail.into_iter().map(|value| Some(value.to_owned())));
            let Some(bound) = bind_proc_formals(&params, &actuals, WordValueRules::TCL) else {
                panic!("valid variadic call did not bind");
            };
            let Some(Some(packed)) = bound.last().map(|(_, value)| value) else {
                panic!("args binding was not a known Tcl list");
            };
            let Ok(elements) = WordValueRules::TCL.split_list(packed) else {
                panic!("args binding was not list-parseable");
            };
            assert_eq!(elements, expected);
        }
    }

    #[test]
    fn proc_formal_binding_preserves_unknown_variadic_values() {
        let params = parse_param_list("args", WordValueRules::TCL);
        assert_eq!(
            bind_proc_formals(
                &params,
                &[Some("known".to_owned()), None],
                WordValueRules::TCL
            ),
            Some(vec![("args".to_owned(), None)])
        );
    }

    /// Lex `source` and return the *n*th non-separator, non-EOL token — for
    /// `proc <name> <params> <body>` sources, index 2 is always the
    /// parameter-list word, matching how `handle_proc_command` /
    /// `handle_method_body` pull `arg_tokens[1]` / `mb.params_tok` from the
    /// real segmented command. Lexing real source (rather than
    /// hand-constructing a `Token`) is deliberate: the bug this guards
    /// against is specifically about what the *real* lexer's span/
    /// `content_offset` convention produces for a braced word, which a
    /// hand-built token could accidentally get "right" by construction.
    fn nth_word_token(source: &str, n: usize) -> tcl_lexer::Token {
        tcl_lexer::Lexer::new(source)
            .tokenise_all()
            .expect("lexes")
            .into_iter()
            .filter(|t| {
                !matches!(
                    t.kind,
                    tcl_lexer::TokenType::Sep | tcl_lexer::TokenType::Eol
                )
            })
            .nth(n)
            .expect("token exists")
    }

    #[test]
    fn param_name_spans_for_token_finds_every_param_not_just_first() {
        // TP — the exact regression this function exists to fix: a real
        // lexed `Str` (braced) param-list token's span starts *at* the `{`
        // but, per the inner-end convention, ends one byte short of the `}`
        // (confirmed against the live lexer, not assumed), so naively
        // slicing `source[tok.span]` and handing it to `param_name_spans`
        // yields a mismatched one-sided string that made every parameter
        // after the first silently lose its span.
        let source = "proc reduce {f z xs} {}";
        let params_tok = nth_word_token(source, 2);
        assert_eq!(params_tok.kind, tcl_lexer::TokenType::Str);
        // Confirm the inner-end convention actually holds for this token —
        // if the lexer's span contract ever changes, this test should fail
        // loudly here rather than the assertions below silently passing for
        // the wrong reason.
        assert_eq!(&source[params_tok.span.as_range()], "{f z xs");

        let spans = param_name_spans_for_token(source, params_tok);
        let names: Vec<&str> = spans.iter().map(|s| &source[s.as_range()]).collect();
        assert_eq!(names, vec!["f", "z", "xs"]);
        assert_eq!(
            spans.len(),
            parse_param_list("f z xs", WordValueRules::TCL).len()
        );
    }

    #[test]
    fn param_name_spans_for_token_two_param_minimal_repro() {
        // TP — the smallest possible reproduction (2 params is already
        // enough to expose the bug: the first swallows the rest, so the
        // second silently vanishes).
        let source = "proc p {a b} {}";
        let params_tok = nth_word_token(source, 2);
        let spans = param_name_spans_for_token(source, params_tok);
        let names: Vec<&str> = spans.iter().map(|s| &source[s.as_range()]).collect();
        assert_eq!(names, vec!["a", "b"]);
    }

    #[test]
    fn param_name_spans_for_token_handles_bare_single_param() {
        // FN guard — a single unbraced parameter (`Esc` token, no
        // delimiter, `content_offset == 0`) must keep working exactly as
        // before; this shape never triggered the bug, so it must not
        // regress.
        let source = "proc p a {}";
        let params_tok = nth_word_token(source, 2);
        assert_eq!(params_tok.kind, tcl_lexer::TokenType::Esc);
        let spans = param_name_spans_for_token(source, params_tok);
        let names: Vec<&str> = spans.iter().map(|s| &source[s.as_range()]).collect();
        assert_eq!(names, vec!["a"]);
    }

    #[test]
    fn param_name_spans_for_token_handles_empty_param_list() {
        // TN — a genuinely empty `{}` param list has no names to find at
        // all; the degenerate span (widened to cover the closing brace,
        // per the lexer's empty-wrapper convention) must clamp to zero
        // spans, not a spurious one-character `}` "parameter".
        let source = "proc p {} {}";
        let params_tok = nth_word_token(source, 2);
        assert_eq!(&source[params_tok.span.as_range()], "{}");
        let spans = param_name_spans_for_token(source, params_tok);
        assert!(spans.is_empty());
    }

    #[test]
    fn param_name_spans_for_token_handles_default_value_specs() {
        // FP guard — a mix of bare and braced-with-default specs must keep
        // each span anchored to the *name* only, excluding default text,
        // and stay index-aligned with `parse_param_list` even when a
        // default spec sits in the middle (not just at the end).
        let source = "proc p {a {b 1} c} {}";
        let params_tok = nth_word_token(source, 2);
        let spans = param_name_spans_for_token(source, params_tok);
        let names: Vec<&str> = spans.iter().map(|s| &source[s.as_range()]).collect();
        assert_eq!(names, vec!["a", "b", "c"]);
        let params = parse_param_list("a {b 1} c", WordValueRules::TCL);
        assert_eq!(spans.len(), params.len());
        for (span, p) in spans.iter().zip(params.iter()) {
            assert_eq!(&source[span.as_range()], p.name);
        }
    }

    #[test]
    fn param_name_spans_for_token_out_of_bounds_token_returns_empty() {
        // TN — a token whose span/content_offset don't fit inside `source`
        // (e.g. a stale span after an edit) must abstain, never panic or
        // index out of range.
        let bogus = tcl_lexer::Token {
            kind: tcl_lexer::TokenType::Str,
            span: tcl_lexer::Span::new(100, 200),
            content_offset: 1,
            in_quote: false,
        };
        assert!(param_name_spans_for_token("short", bogus).is_empty());
    }
}
