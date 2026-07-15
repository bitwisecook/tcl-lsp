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

//! Tokenisation of a `switch` command's braced pattern/body list.
//!
//! The shared home for the brace-aware scan that the refactoring transforms
//! (`switch_to_dict`, data-group extraction) and the MCP data-group scan use to
//! read a `switch` body given as one braced word. Two byte-identical private
//! copies (in `tcl-lsp-core` and `tcl-mcp`) previously lived apart; they are
//! centralised here so the tokenisation cannot drift.
//!
//! This is deliberately a *lossy, forgiving* scan for transform heuristics —
//! tokens keep their `{…}`/`"…"` delimiters and malformed input never errors —
//! not the strict list grammar. For real `Tcl_SplitList` semantics use
//! [`crate::list`].

/// Split a braced switch body into whitespace/brace/quote-delimited tokens.
///
/// `{…}` groups nest and are returned with their braces; `"…"` groups honour
/// backslash escapes and are returned with their quotes; anything else is a
/// bare run up to whitespace or a brace. Unterminated groups extend to the end
/// of the text rather than erroring.
#[must_use]
pub fn tokenise_switch_body(text: &str) -> Vec<String> {
    let bytes = text.as_bytes();
    let n = bytes.len();
    let mut tokens = Vec::new();
    let mut i = 0;
    while i < n {
        while i < n && matches!(bytes[i], b' ' | b'\t' | b'\n' | b'\r') {
            i += 1;
        }
        if i >= n {
            break;
        }
        match bytes[i] {
            b'{' => {
                let start = i;
                let mut depth = 1;
                i += 1;
                while i < n && depth > 0 {
                    match bytes[i] {
                        b'{' => depth += 1,
                        b'}' => depth -= 1,
                        _ => {}
                    }
                    i += 1;
                }
                tokens.push(text[start..i].to_owned());
            }
            b'"' => {
                let start = i;
                i += 1;
                while i < n && bytes[i] != b'"' {
                    if bytes[i] == b'\\' {
                        i += 1;
                    }
                    i += 1;
                }
                if i < n {
                    i += 1;
                }
                tokens.push(text[start..i].to_owned());
            }
            _ => {
                let start = i;
                while i < n && !matches!(bytes[i], b' ' | b'\t' | b'\n' | b'\r' | b'{' | b'}') {
                    i += 1;
                }
                tokens.push(text[start..i].to_owned());
            }
        }
    }
    tokens
}

/// Parse `{pat1 {body1} pat2 {body2} …}` into pattern/body pairs.
///
/// Strips one layer of surrounding braces if present, tokenises with
/// [`tokenise_switch_body`], and pairs the tokens up; a dangling odd token is
/// dropped.
#[must_use]
pub fn parse_braced_pairs(text: &str) -> Vec<(String, String)> {
    let mut text = text.trim();
    if text.len() >= 2 && text.starts_with('{') && text.ends_with('}') {
        text = text[1..text.len() - 1].trim();
    }
    let tokens = tokenise_switch_body(text);
    let mut pairs = Vec::new();
    let mut i = 0;
    while i + 1 < tokens.len() {
        pairs.push((tokens[i].clone(), tokens[i + 1].clone()));
        i += 2;
    }
    pairs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokenises_bare_braced_and_quoted_words() {
        assert_eq!(
            tokenise_switch_body("a {b c} \"d e\" f"),
            ["a", "{b c}", "\"d e\"", "f"]
        );
        // Braces nest; the group keeps its delimiters.
        assert_eq!(
            tokenise_switch_body("x {a {b} c} y"),
            ["x", "{a {b} c}", "y"]
        );
        // A `\"` inside quotes does not terminate the quoted token.
        assert_eq!(tokenise_switch_body(r#""a\"b" c"#), [r#""a\"b""#, "c"]);
    }

    #[test]
    fn tokenises_forgivingly_at_the_end_of_input() {
        // Unterminated groups run to the end instead of erroring.
        assert_eq!(tokenise_switch_body("{a b"), ["{a b"]);
        assert_eq!(tokenise_switch_body("\"a b"), ["\"a b"]);
        assert_eq!(tokenise_switch_body("   "), Vec::<String>::new());
    }

    #[test]
    fn parses_pattern_body_pairs() {
        assert_eq!(
            parse_braced_pairs("{ GET {set h a} POST {set h b} }"),
            [
                ("GET".to_owned(), "{set h a}".to_owned()),
                ("POST".to_owned(), "{set h b}".to_owned()),
            ]
        );
        // Without the outer braces the pairing is the same.
        assert_eq!(
            parse_braced_pairs("a {x} default {y}"),
            [
                ("a".to_owned(), "{x}".to_owned()),
                ("default".to_owned(), "{y}".to_owned()),
            ]
        );
        // A dangling odd token (a pattern with no body) is dropped.
        assert_eq!(
            parse_braced_pairs("{a {x} b}"),
            [("a".to_owned(), "{x}".to_owned())]
        );
    }
}
