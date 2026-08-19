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

//! Compile-time evaluator for `[subst -nocommands {template}]`.
//!
//! Used by the lowering hook when we see a `proc $var [subst
//! -nocommands {…}]` shape and want to materialise the body string
//! at compile time instead of deferring to the runtime interpreter.
//!
//! Semantics (matching `tclsh 9.0`'s `subst -nocommands`):
//!
//! * `$var` / `${var}` — variable substitution. The name is looked
//!   up in the supplied const-map; a miss refuses the whole
//!   evaluation by returning `None` (the caller keeps the dynamic
//!   dispatch path in that case).
//! * `\…` — standard backslash processing via
//!   [`tcl_lexer::backslash_subst`]. Handles `\n \t \xNN \uNNNN`
//!   and octal / continuation-line forms.
//! * `[` / `]` — ordinary literal characters. `-nocommands` disables
//!   *command* substitution only, so `[` no longer opens a command
//!   substitution: it (and `]`) are copied verbatim while any `$var`
//!   / `\escape` *inside* the brackets is still substituted. This
//!   mirrors the VM's `subst_command` with `commands = false`
//!   (`subst.rs`), where the `[` arm is gated on `commands` and a
//!   bare `[` falls through to the literal-copy path. There is no
//!   bracket matching and an unbalanced `[` is not an error.
//! * `$a(b)` — array references are refused.
//! * `$::ns::var` — namespace-qualified var refs are refused.

use std::collections::HashMap;
use std::hash::BuildHasher;

/// Evaluate a `subst -nocommands` template at compile time.
///
/// Returns the substituted string on success, or `None` if any
/// condition above refuses the evaluation. Refusal is always safe
/// — the caller falls back to runtime dispatch, preserving the
/// original semantics.
///
/// *`const_map`* maps variable names (without the leading `$`) to
/// their literal string values. Names are looked up with their
/// `{…}`-stripped form, so `${foo}` and `$foo` resolve the same
/// entry.
#[must_use]
pub fn subst_nocommands<S: BuildHasher>(
    template: &str,
    const_map: &HashMap<String, String, S>,
    braced_var: tcl_dialect::BracedVarStyle,
) -> Option<String> {
    let bytes = template.as_bytes();
    let n = bytes.len();
    let mut out = String::with_capacity(n);
    let mut i = 0usize;
    while i < n {
        let c = bytes[i];
        if c == b'\\' {
            // Defer to the shared backslash processor for the single
            // following escape; the canonical extent rule covers the
            // continuation-line (LF / CR / CRLF) and octal / hex /
            // unicode forms, so decode always sees one whole escape.
            let j = tcl_lexer::backslash_escape_end(template, i);
            let decoded = tcl_lexer::backslash_subst(&template[i..j]);
            out.push_str(&decoded);
            i = j;
            continue;
        }
        if c == b'$' {
            // ``$$`` — Tcl treats first ``$`` as start of a name,
            // and if no name follows, leaves it literal.
            if i + 1 >= n {
                out.push('$');
                i += 1;
                continue;
            }
            let nxt = bytes[i + 1];
            if nxt == b'{' {
                // The name starts just past the `${`; the closer comes from
                // the shared owner under this document's release rule. The old
                // first-`}` scan answered for 8.x at every release; declining
                // (this function's failure mode) is safe, but it declined a
                // reference the 9.x lexer had already spanned as one whole
                // name (issue #1604).
                let tcl_lexer::BracedVarEnd::Closed(close) =
                    tcl_lexer::braced_var_name_end(bytes, i + 2, braced_var)
                else {
                    return None;
                };
                let name = &template[i + 2..close];
                if is_complex_var_name(name) {
                    return None;
                }
                let value = const_map.get(name)?;
                out.push_str(value);
                i = close + 1;
                continue;
            }
            if is_name_byte(nxt) {
                let mut j = i + 1;
                while j < n && is_name_byte(bytes[j]) {
                    j += 1;
                }
                let name = &template[i + 1..j];
                // Array reference ``$name(index)`` — refuse.
                if j < n && bytes[j] == b'(' {
                    return None;
                }
                // Namespace qualifier ``$a::b`` — refuse.
                if j + 1 < n && bytes[j] == b':' && bytes[j + 1] == b':' {
                    return None;
                }
                let value = const_map.get(name)?;
                out.push_str(value);
                i = j;
                continue;
            }
            // ``$::name`` — refuse.
            if nxt == b':' {
                return None;
            }
            // ``$`` followed by a non-name character — leave the
            // ``$`` literal and continue.
            out.push('$');
            i += 1;
            continue;
        }
        // `[` and `]` are literal under -nocommands (command substitution
        // is disabled); `$`/`\` inside them were already handled above, so
        // they need no special case — they fall through to the copy path.
        // ASCII fast path; for non-ASCII we read as a char.
        if c < 128 {
            out.push(c as char);
            i += 1;
        } else {
            let ch = template[i..].chars().next().unwrap();
            out.push(ch);
            i += ch.len_utf8();
        }
    }
    Some(out)
}

fn is_name_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

fn is_complex_var_name(name: &str) -> bool {
    if name.is_empty() {
        return true;
    }
    if name.contains("::")
        || name.contains('(')
        || name.contains(')')
        || name.contains('$')
        || name.contains('[')
    {
        return true;
    }
    !name.bytes().all(is_name_byte)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The default document's `${…}` close rule.
    const STYLE: tcl_dialect::BracedVarStyle = tcl_dialect::BracedVarStyle::Tcl9Nesting;

    fn map_of(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
            .collect()
    }

    #[test]
    fn simple_var_substitution() {
        let m = map_of(&[("name", "world")]);
        assert_eq!(
            subst_nocommands("hello $name", &m, STYLE).as_deref(),
            Some("hello world")
        );
    }

    #[test]
    fn braced_var_substitution() {
        let m = map_of(&[("name", "world")]);
        assert_eq!(
            subst_nocommands("hello ${name}", &m, STYLE).as_deref(),
            Some("hello world")
        );
    }

    #[test]
    fn missing_var_returns_none() {
        let m: HashMap<String, String> = HashMap::new();
        assert!(subst_nocommands("$missing", &m, STYLE).is_none());
    }

    #[test]
    fn brackets_kept_literal() {
        let m: HashMap<String, String> = HashMap::new();
        assert_eq!(
            subst_nocommands("[cmd arg]", &m, STYLE).as_deref(),
            Some("[cmd arg]")
        );
    }

    #[test]
    fn unbalanced_bracket_is_literal() {
        // -nocommands disables command substitution, so `[` is an
        // ordinary character and an unclosed `[` is NOT an error
        // (matches tclsh: `subst -nocommands {[unbalanced}` → `[unbalanced`).
        let m: HashMap<String, String> = HashMap::new();
        assert_eq!(
            subst_nocommands("[unbalanced", &m, STYLE).as_deref(),
            Some("[unbalanced")
        );
    }

    #[test]
    fn vars_inside_brackets_are_substituted() {
        // `$field` inside `[...]` still substitutes under
        // -nocommands; only the command is not executed. `\$obj` decodes to
        // a literal `$obj`. Mirrors tclsh:
        //   subst -nocommands {[dict get \$obj $field]} → [dict get $obj email]
        let m = map_of(&[("field", "email")]);
        assert_eq!(
            subst_nocommands(r"[dict get \$obj $field]", &m, STYLE).as_deref(),
            Some("[dict get $obj email]")
        );
    }

    #[test]
    fn missing_var_inside_brackets_refused() {
        // A variable miss anywhere (even inside brackets) refuses the whole
        // evaluation so the caller keeps the runtime dispatch path.
        let m: HashMap<String, String> = HashMap::new();
        assert!(subst_nocommands("[cmd $missing]", &m, STYLE).is_none());
    }

    #[test]
    fn backslash_before_multibyte_char() {
        // `\é` must not split the 2-byte `é` and panic; the
        // backslash is dropped and the following char is emitted verbatim.
        let m: HashMap<String, String> = HashMap::new();
        assert_eq!(subst_nocommands(r"\é", &m, STYLE).as_deref(), Some("é"));
        assert_eq!(subst_nocommands(r"a\€b", &m, STYLE).as_deref(), Some("a€b"));
        assert_eq!(subst_nocommands(r"\🎉", &m, STYLE).as_deref(), Some("🎉"));
    }

    #[test]
    fn array_ref_refused() {
        let m = map_of(&[("a", "x")]);
        assert!(subst_nocommands("$a(idx)", &m, STYLE).is_none());
    }

    #[test]
    fn namespace_qualified_refused() {
        let m = map_of(&[("a", "x")]);
        assert!(subst_nocommands("$a::b", &m, STYLE).is_none());
        assert!(subst_nocommands("$::name", &m, STYLE).is_none());
    }

    #[test]
    fn dollar_dollar_keeps_first_literal() {
        // ``$$`` — first ``$`` followed by ``$`` (not a name char).
        // The first ``$`` stays literal, then the second ``$`` is
        // also followed by end-of-string, so it stays literal too.
        let m: HashMap<String, String> = HashMap::new();
        assert_eq!(subst_nocommands("$$", &m, STYLE).as_deref(), Some("$$"));
    }

    #[test]
    fn backslash_n_decoded() {
        let m: HashMap<String, String> = HashMap::new();
        assert_eq!(subst_nocommands(r"\n", &m, STYLE).as_deref(), Some("\n"));
    }

    #[test]
    fn backslash_x_hex() {
        let m: HashMap<String, String> = HashMap::new();
        assert_eq!(subst_nocommands(r"\x41", &m, STYLE).as_deref(), Some("A"));
    }

    #[test]
    fn backslash_x_hex_capped_at_two_digits() {
        // Tcl 9 caps `\x` at two hex digits (TclParseBackslash): the rest of
        // the digit run is literal text.
        let m: HashMap<String, String> = HashMap::new();
        assert_eq!(
            subst_nocommands(r"\x41BC", &m, STYLE).as_deref(),
            Some("ABC")
        );
    }

    #[test]
    fn backslash_newline_continuation_collapses_crlf_like_lf() {
        // A `\<newline>` continuation (LF, CR, or CRLF) plus the whitespace
        // after it collapses to a single space, matching tclsh.
        let m: HashMap<String, String> = HashMap::new();
        assert_eq!(
            subst_nocommands("a\\\n   b", &m, STYLE).as_deref(),
            Some("a b")
        );
        assert_eq!(
            subst_nocommands("a\\\r\n\t b", &m, STYLE).as_deref(),
            Some("a b")
        );
        assert_eq!(
            subst_nocommands("a\\\rb", &m, STYLE).as_deref(),
            Some("a b")
        );
    }

    #[test]
    fn empty_template() {
        let m: HashMap<String, String> = HashMap::new();
        assert_eq!(subst_nocommands("", &m, STYLE).as_deref(), Some(""));
    }

    #[test]
    fn mixed_var_and_brackets() {
        let m = map_of(&[("name", "foo")]);
        assert_eq!(
            subst_nocommands("$name [list a b]", &m, STYLE).as_deref(),
            Some("foo [list a b]")
        );
    }

    #[test]
    fn empty_braced_var_refused() {
        let m: HashMap<String, String> = HashMap::new();
        assert!(subst_nocommands("${}", &m, STYLE).is_none());
    }
}
